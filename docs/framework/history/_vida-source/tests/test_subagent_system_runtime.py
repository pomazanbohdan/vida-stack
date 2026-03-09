import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "subagent-system.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


class SubagentSystemRuntimeTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("subagent_system_runtime_test", SCRIPT_PATH)

    def test_models_hint_for_subagent_uses_dispatch_models_cache_path(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            cache_path = Path(tmpdir) / "models_cache.json"
            cache_path.write_text(json.dumps({"models": [{"slug": "gpt-5.1-codex-mini"}]}), encoding="utf-8")
            hints = self.module.models_hint_for_subagent(
                "any_cli",
                {"dispatch": {"models_cache_path": str(cache_path)}},
            )

        self.assertEqual(hints, ["gpt-5.1-codex-mini"])

    def test_web_search_probe_command_uses_configured_probe_fields(self) -> None:
        cmd, timeout_seconds, expect = self.module.subagent_web_search_probe_command(
            "qwen_cli",
            {
                "dispatch": {
                    "command": "qwen",
                    "static_args": ["-y", "-o", "text"],
                    "prompt_mode": "positional",
                    "web_search_mode": "provider_configured",
                    "web_probe_prompt": "Return exactly one line: VIDA_WEB_SEARCH_OK https://example.com",
                    "web_probe_expect_substring": "VIDA_WEB_SEARCH_OK",
                    "web_probe_timeout_seconds": 31,
                }
            },
        )

        self.assertEqual(cmd[:4], ["qwen", "-y", "-o", "text"])
        self.assertIn("VIDA_WEB_SEARCH_OK", cmd[-1])
        self.assertEqual(timeout_seconds, 31)
        self.assertEqual(expect, "VIDA_WEB_SEARCH_OK")

    def test_route_candidate_context_filters_capability_incompatible_lane_before_scoring(self) -> None:
        snapshot = {
            "subagents": {
                "writer_cli": {
                    "enabled": True,
                    "available": True,
                    "subagent_backend_class": "external_cli",
                    "write_scope": "scoped_only",
                    "capability_band": ["bounded_write_safe"],
                    "dispatch": {"command": "writer"},
                },
                "review_cli": {
                    "enabled": True,
                    "available": True,
                    "subagent_backend_class": "external_cli",
                    "write_scope": "none",
                    "capability_band": ["read_only", "review_safe"],
                    "dispatch": {"command": "review"},
                },
            },
            "scorecards": {},
            "agent_system": {
                "effective_mode": "hybrid",
                "max_parallel_agents": 2,
                "state_owner": "orchestrator_only",
                "scoring": self.module.thresholds({}),
            },
        }
        config = {
            "agent_system": {
                "routing": {
                    "implementation": {
                        "subagents": ["writer_cli", "review_cli"],
                        "write_scope": "scoped_only",
                        "dispatch_required": "optional",
                        "external_first_required": "no",
                    }
                }
            }
        }

        with mock.patch.object(self.module.capability_registry_runtime, "compatibility_for") as mocked_compatibility:
            mocked_compatibility.side_effect = [
                {
                    "compatible": True,
                    "reason": "ok",
                    "task_class": "implementation",
                    "subagent": "writer_cli",
                },
                {
                    "compatible": False,
                    "reason": "write_scope_mismatch,missing_required_capability_band",
                    "task_class": "implementation",
                    "subagent": "review_cli",
                },
            ]
            payload = self.module.route_candidate_context("implementation", snapshot, config, {})

        self.assertEqual([item["subagent"] for item in payload["candidates"]], ["writer_cli"])
        self.assertEqual(
            payload["suppressed_subagents"],
            [{"subagent": "review_cli", "reason": "capability_incompatible:write_scope_mismatch,missing_required_capability_band"}],
        )


if __name__ == "__main__":
    unittest.main()
