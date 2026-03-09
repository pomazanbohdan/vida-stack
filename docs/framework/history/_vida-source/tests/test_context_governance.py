import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "context-governance.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


class ContextGovernanceTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("context_governance_test", SCRIPT_PATH)

    def test_validate_sources_accepts_web_validated_and_runtime_entries(self) -> None:
        payload = self.module.validate_sources(
            [
                {
                    "source_class": "local_runtime",
                    "path": ".vida/logs/issue-contracts/unit-task.json",
                    "freshness": "current",
                    "provenance": "issue_contract_artifact",
                    "role_scope": "orchestrator",
                },
                {
                    "source_class": "web_validated",
                    "path": ".vida/logs/issue-contracts/unit-task.json",
                    "freshness": "validated",
                    "provenance": "issue_contract_artifact",
                    "role_scope": "orchestrator",
                },
            ]
        )

        self.assertTrue(payload["valid"])
        self.assertEqual(payload["summary"]["web_validated_count"], 1)

    def test_record_entry_updates_summary(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            self.module.STATE_PATH = Path(tmpdir) / "context-governance.json"
            entry = self.module.record_entry(
                task_id="mobile-2oy",
                phase="prepare_execution",
                sources=[
                    {
                        "source_class": "local_runtime",
                        "path": ".vida/logs/spec-intake/mobile-2oy.json",
                        "freshness": "current",
                        "provenance": "spec_intake_artifact",
                        "role_scope": "orchestrator",
                    }
                ],
                notes="implementation",
            )
            state = json.loads(self.module.STATE_PATH.read_text(encoding="utf-8"))

        self.assertEqual(entry["summary"]["source_count"], 1)
        self.assertEqual(state["summary"]["task_count"], 1)
        self.assertEqual(state["summary"]["by_source_class"]["local_runtime"], 1)


if __name__ == "__main__":
    unittest.main()
