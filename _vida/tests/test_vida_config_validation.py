import importlib.util
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
CONFIG_SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "vida-config.py"
CONFIG_TEMPLATE_PATH = ROOT_DIR / "_vida" / "templates" / "vida.config.yaml.template"


def load_config_module():
    spec = importlib.util.spec_from_file_location("vida_config_validation_test", CONFIG_SCRIPT_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class VidaConfigValidationTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_config_module()

    def _base_config(self) -> dict:
        return {
            "project": {"id": "test-project", "overlay_version": 1},
            "protocol_activation": {"agent_system": True},
            "agent_system": {
                "init_on_boot": True,
                "mode": "hybrid",
                "state_owner": "orchestrator_only",
                "max_parallel_agents": 3,
                "subagents": {
                    "qwen_cli": {
                        "enabled": True,
                        "subagent_backend_class": "external_cli",
                        "role": "free_primary",
                        "orchestration_tier": "external_free",
                        "cost_priority": "highest",
                        "billing_tier": "free",
                        "speed_tier": "fast",
                        "quality_tier": "high",
                        "dispatch": {
                            "command": "qwen",
                            "output_mode": "stdout",
                            "prompt_mode": "positional",
                        },
                    }
                },
                "routing": {
                    "default": {
                        "subagents": ["qwen_cli"],
                    }
                },
                "scoring": {},
            },
        }

    def _add_external_subagent(self, cfg: dict, name: str, command: str) -> None:
        cfg["agent_system"]["subagents"][name] = {
            "enabled": True,
            "subagent_backend_class": "external_cli",
            "role": "free_primary",
            "orchestration_tier": "external_free",
            "cost_priority": "highest",
            "billing_tier": "free",
            "speed_tier": "fast",
            "quality_tier": "high",
            "dispatch": {
                "command": command,
                "output_mode": "stdout",
                "prompt_mode": "positional",
            },
        }

    def test_fanout_route_without_fanout_subagents_fails_validation(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["routing"]["research"] = {
            "subagents": ["qwen_cli"],
            "dispatch_required": "fanout_then_synthesize",
            "external_first_required": "yes",
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.research.fanout_subagents: required when dispatch_required=fanout_then_synthesize",
            errors,
        )

    def test_analysis_required_without_route_task_class_fails_validation(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["routing"]["implementation"] = {
            "subagents": ["qwen_cli"],
            "analysis_required": "yes",
            "analysis_external_first_required": "yes",
            "analysis_receipt_required": "yes",
            "dispatch_required": "external_first_review",
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.implementation.analysis_route_task_class: required when analysis_required=yes",
            errors,
        )
        self.assertIn(
            "agent_system.routing.implementation.analysis_fanout_subagents: required when analysis_required=yes and analysis_external_first_required=yes",
            errors,
        )

    def test_independent_verification_requires_route_task_class(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["routing"]["research"] = {
            "subagents": ["qwen_cli"],
            "dispatch_required": "external_first_review",
            "independent_verification_required": "yes",
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.research.verification_route_task_class: required when independent_verification_required=yes",
            errors,
        )

    def test_coach_required_requires_distinct_route_and_positive_pass_cap(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["routing"]["coach"] = {
            "subagents": ["qwen_cli"],
            "dispatch_required": "external_first_review",
        }
        cfg["agent_system"]["routing"]["implementation"] = {
            "subagents": ["qwen_cli"],
            "coach_required": "yes",
            "max_coach_passes": 0,
            "independent_verification_required": "yes",
            "verification_route_task_class": "coach",
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.implementation.coach_route_task_class: required when coach_required=yes",
            errors,
        )
        self.assertIn(
            "agent_system.routing.implementation.max_coach_passes: must be >= 1 when coach_required=yes",
            errors,
        )

    def test_coach_route_must_differ_from_verification_route(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["routing"]["coach"] = {
            "subagents": ["qwen_cli"],
            "dispatch_required": "external_first_review",
        }
        cfg["agent_system"]["routing"]["implementation"] = {
            "subagents": ["qwen_cli"],
            "coach_required": "yes",
            "coach_route_task_class": "coach",
            "max_coach_passes": 2,
            "independent_verification_required": "yes",
            "verification_route_task_class": "coach",
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.implementation.coach_route_task_class: must differ from verification_route_task_class when coach_required=yes and independent_verification_required=yes",
            errors,
        )

    def test_web_search_required_requires_web_capable_route_subagent(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["routing"]["research"] = {
            "subagents": ["qwen_cli"],
            "dispatch_required": "fanout_then_synthesize",
            "fanout_subagents": ["qwen_cli"],
            "fanout_min_results": 1,
            "web_search_required": "yes",
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.research.web_search_required: requires at least one route subagent with declared 'web_search' capability and dispatch wiring",
            errors,
        )

    def test_web_search_capability_requires_dispatch_wiring(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["subagents"]["qwen_cli"]["capability_band"] = ["read_only", "review_safe", "web_search"]

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.subagents.qwen_cli.dispatch.web_search_mode: required as 'flag' or 'provider_configured' when capability_band contains 'web_search'",
            errors,
        )

    def test_dispatch_web_search_mode_requires_capability_declaration(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["subagents"]["qwen_cli"]["dispatch"]["web_search_mode"] = "provider_configured"

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.subagents.qwen_cli.capability_band: must contain 'web_search' when dispatch.web_search_mode enables web search",
            errors,
        )

    def test_web_search_fanout_min_must_fit_wired_web_lanes(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["subagents"]["qwen_cli"]["capability_band"] = ["read_only", "review_safe", "web_search"]
        cfg["agent_system"]["subagents"]["qwen_cli"]["dispatch"]["web_search_mode"] = "provider_configured"
        cfg["agent_system"]["routing"]["research"] = {
            "subagents": ["qwen_cli"],
            "dispatch_required": "fanout_then_synthesize",
            "fanout_subagents": ["qwen_cli"],
            "fanout_min_results": 2,
            "web_search_required": "yes",
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.research.fanout_min_results: must be <= number of web-search-wired fanout_subagents when web_search_required=yes",
            errors,
        )

    def test_framework_self_diagnosis_enabled_requires_silent_mode_true(self) -> None:
        cfg = self._base_config()
        cfg["framework_self_diagnosis"] = {
            "enabled": True,
            "silent_mode": False,
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "framework_self_diagnosis.silent_mode: must be true when enabled=true",
            errors,
        )

    def test_autonomous_execution_cannot_disable_internal_boundary_analysis(self) -> None:
        cfg = self._base_config()
        cfg["autonomous_execution"] = {
            "next_task_boundary_analysis": False,
            "next_task_boundary_report": "brief_plan",
            "next_task_boundary_report_gating": False,
            "dependent_coverage_autoupdate": True,
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "autonomous_execution.next_task_boundary_analysis: may not be false because internal boundary analysis is framework-required",
            errors,
        )

    def test_autonomous_execution_report_off_forbids_report_gating(self) -> None:
        cfg = self._base_config()
        cfg["autonomous_execution"] = {
            "next_task_boundary_analysis": True,
            "next_task_boundary_report": "off",
            "next_task_boundary_report_gating": True,
            "dependent_coverage_autoupdate": True,
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "autonomous_execution.next_task_boundary_report_gating: must be false when next_task_boundary_report=off",
            errors,
        )

    def test_route_cross_references_unknown_subagents_and_routes_fail_validation(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["routing"]["analysis"] = {
            "subagents": ["qwen_cli", "ghost_cli"],
            "fanout_subagents": ["ghost_cli"],
            "dispatch_required": "fanout_then_synthesize",
            "bridge_fallback_subagent": "ghost_cli",
            "verification_route_task_class": "missing_review",
            "independent_verification_required": "yes",
        }
        cfg["agent_system"]["routing"]["implementation"] = {
            "subagents": ["qwen_cli"],
            "analysis_required": "yes",
            "analysis_route_task_class": "missing_analysis",
            "analysis_fanout_subagents": ["ghost_cli"],
            "analysis_external_first_required": "yes",
            "analysis_receipt_required": "yes",
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.analysis.subagents: unknown subagent 'ghost_cli'",
            errors,
        )
        self.assertIn(
            "agent_system.routing.analysis.fanout_subagents: unknown subagent 'ghost_cli'",
            errors,
        )
        self.assertIn(
            "agent_system.routing.analysis.bridge_fallback_subagent: unknown subagent 'ghost_cli'",
            errors,
        )
        self.assertIn(
            "agent_system.routing.analysis.verification_route_task_class: unknown route 'missing_review'",
            errors,
        )
        self.assertIn(
            "agent_system.routing.implementation.analysis_route_task_class: unknown route 'missing_analysis'",
            errors,
        )
        self.assertIn(
            "agent_system.routing.implementation.analysis_fanout_subagents: unknown subagent 'ghost_cli'",
            errors,
        )

    def test_invalid_dispatch_and_yes_no_flags_fail_validation(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["routing"]["research"] = {
            "subagents": ["qwen_cli"],
            "dispatch_required": "fanoutish",
            "analysis_required": "maybe",
            "external_first_required": "sometimes",
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.research.dispatch_required: expected one of ['bridge_or_critical_review', 'bridge_write_then_internal_if_expands', 'external_first_review', 'external_first_then_senior_arbitration', 'external_first_when_eligible', 'external_readonly_then_senior_writer', 'fanout_then_synthesize', 'local_or_external_first']",
            errors,
        )
        self.assertIn(
            "agent_system.routing.research.analysis_required: expected one of ['no', 'yes']",
            errors,
        )
        self.assertIn(
            "agent_system.routing.research.external_first_required: expected one of ['no', 'yes']",
            errors,
        )

    def test_local_preferred_route_cannot_claim_external_first_required(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["routing"]["status_diagnostic"] = {
            "subagents": ["qwen_cli"],
            "dispatch_required": "local_or_external_first",
            "external_first_required": "yes",
            "local_execution_allowed": "yes",
            "local_execution_preferred": "yes",
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.status_diagnostic.external_first_required: must be 'no' when local_execution_allowed=yes or local_execution_preferred=yes",
            errors,
        )

    def test_fallback_hops_must_cover_bridge_and_internal_escalation(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["routing"]["implementation"] = {
            "subagents": ["qwen_cli"],
            "dispatch_required": "external_first_review",
            "bridge_fallback_subagent": "qwen_cli",
            "internal_escalation_trigger": "scope_expands",
            "max_fallback_hops": 1,
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.implementation.max_fallback_hops: must be >= 2 to cover declared bridge_fallback_subagent and internal_escalation_trigger",
            errors,
        )

    def test_bridge_fallback_must_not_overlap_primary_fanout(self) -> None:
        cfg = self._base_config()
        cfg["agent_system"]["routing"]["analysis"] = {
            "subagents": ["qwen_cli"],
            "fanout_subagents": ["qwen_cli"],
            "dispatch_required": "fanout_then_synthesize",
            "bridge_fallback_subagent": "qwen_cli",
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.analysis.bridge_fallback_subagent: must not overlap fanout_subagents",
            errors,
        )

    def test_analysis_fanout_must_be_supported_by_analysis_route(self) -> None:
        cfg = self._base_config()
        self._add_external_subagent(cfg, "gemini_cli", "gemini")
        cfg["agent_system"]["routing"]["analysis"] = {
            "subagents": ["qwen_cli"],
        }
        cfg["agent_system"]["routing"]["implementation"] = {
            "subagents": ["qwen_cli"],
            "analysis_required": "yes",
            "analysis_route_task_class": "analysis",
            "analysis_fanout_subagents": ["qwen_cli", "gemini_cli"],
            "analysis_external_first_required": "yes",
            "analysis_receipt_required": "yes",
        }

        errors = self.module.validate_config(cfg)

        self.assertIn(
            "agent_system.routing.implementation.analysis_fanout_subagents: 'gemini_cli' must also be present in agent_system.routing.analysis.subagents",
            errors,
        )

    def test_bridge_fallback_may_use_known_subagent_outside_primary_order(self) -> None:
        cfg = self._base_config()
        self._add_external_subagent(cfg, "codex_cli", "codex")
        cfg["agent_system"]["routing"]["research"] = {
            "subagents": ["qwen_cli"],
            "dispatch_required": "external_first_review",
            "bridge_fallback_subagent": "codex_cli",
            "internal_escalation_trigger": "quality_gap",
            "max_fallback_hops": 2,
        }

        errors = self.module.validate_config(cfg)

        self.assertEqual(errors, [])

    def test_repository_overlay_validates(self) -> None:
        cfg = self.module.load_config(validate=False)

        errors = self.module.validate_config(cfg)

        self.assertEqual(errors, [])

    def test_repository_qwen_overlay_does_not_force_sandbox(self) -> None:
        cfg = self.module.load_config(validate=False)

        qwen_dispatch = cfg["agent_system"]["subagents"]["qwen_cli"]["dispatch"]
        static_args = qwen_dispatch.get("static_args", [])
        env = qwen_dispatch.get("env", {})

        self.assertNotIn("--sandbox", static_args)
        self.assertEqual(static_args, ["-y", "-o", "text"])
        self.assertEqual(env.get("HOME"), "/home/unnamed/project/vida-stack/.vida/data/qwen-home")
        self.assertEqual(qwen_dispatch.get("web_search_mode"), "provider_configured")

    def test_repository_gemini_overlay_does_not_force_sandbox(self) -> None:
        cfg = self.module.load_config(validate=False)

        gemini_dispatch = cfg["agent_system"]["subagents"]["gemini_cli"]["dispatch"]
        static_args = gemini_dispatch.get("static_args", [])
        probe_static_args = gemini_dispatch.get("probe_static_args", [])

        self.assertNotIn("--sandbox", static_args)
        self.assertNotIn("--sandbox", probe_static_args)
        self.assertEqual(gemini_dispatch.get("command"), "gemini")

    def test_repository_codex_overlay_wires_flag_based_web_search(self) -> None:
        cfg = self.module.load_config(validate=False)

        codex_cfg = cfg["agent_system"]["subagents"]["codex_cli"]
        codex_dispatch = codex_cfg["dispatch"]

        self.assertIn("web_search", codex_cfg.get("capability_band", []))
        self.assertEqual(codex_dispatch.get("pre_static_args"), ["-c", 'model_reasoning_effort="high"', "-a", "never"])
        self.assertEqual(codex_dispatch.get("subcommand"), "exec")
        self.assertEqual(codex_dispatch.get("web_search_mode"), "flag")
        self.assertEqual(codex_dispatch.get("web_search_flag"), "--search")

    def test_repository_overlay_declares_read_only_prep_route(self) -> None:
        cfg = self.module.load_config(validate=False)

        route = cfg["agent_system"]["routing"]["read_only_prep"]

        self.assertEqual(route["write_scope"], "none")
        self.assertEqual(route["dispatch_required"], "fanout_then_synthesize")
        self.assertEqual(route["external_first_required"], "yes")
        self.assertEqual(route["independent_verification_required"], "no")
        self.assertEqual(route["local_execution_allowed"], "no")

    def test_repository_overlay_prefers_qwen_and_codex_for_coach_review(self) -> None:
        cfg = self.module.load_config(validate=False)

        route = cfg["agent_system"]["routing"]["coach"]

        self.assertEqual(route["subagents"], "codex_cli,internal_subagents")
        self.assertEqual(route["fanout_subagents"], "codex_cli")
        self.assertEqual(route["fanout_min_results"], 1)

    def test_repository_overlay_declares_autonomous_boundary_defaults(self) -> None:
        cfg = self.module.load_config(validate=False)

        autonomous_execution = cfg["autonomous_execution"]

        self.assertTrue(autonomous_execution["next_task_boundary_analysis"])
        self.assertEqual(autonomous_execution["next_task_boundary_report"], "brief_plan")
        self.assertFalse(autonomous_execution["next_task_boundary_report_gating"])
        self.assertTrue(autonomous_execution["dependent_coverage_autoupdate"])

    def test_qwen_template_example_does_not_force_sandbox(self) -> None:
        template = CONFIG_TEMPLATE_PATH.read_text(encoding="utf-8")
        qwen_block = template.split("#     qwen_cli:", 1)[1].split("#     gemini_cli:", 1)[0]

        self.assertIn("#         command: qwen", qwen_block)
        self.assertIn("#           - -y", qwen_block)
        self.assertNotIn("#           - --sandbox", qwen_block)
        self.assertIn("#         web_search_mode: provider_configured", qwen_block)

    def test_template_declares_autonomous_boundary_defaults(self) -> None:
        template = CONFIG_TEMPLATE_PATH.read_text(encoding="utf-8")

        self.assertIn("autonomous_execution:", template)
        self.assertIn("next_task_boundary_analysis: true", template)
        self.assertIn("next_task_boundary_report: brief_plan", template)
        self.assertIn("next_task_boundary_report_gating: false", template)
        self.assertIn("dependent_coverage_autoupdate: true", template)

    def test_gemini_template_example_does_not_force_sandbox(self) -> None:
        template = CONFIG_TEMPLATE_PATH.read_text(encoding="utf-8")
        gemini_block = template.split("#     gemini_cli:", 1)[1].split("#     kilo_cli:", 1)[0]

        self.assertIn("#         command: gemini", gemini_block)
        self.assertNotIn("#           - --sandbox", gemini_block)
        self.assertIn("#           - -y", gemini_block)

    def test_codex_template_example_wires_flag_based_web_search(self) -> None:
        template = CONFIG_TEMPLATE_PATH.read_text(encoding="utf-8")
        codex_block = template.split("#     codex_cli:", 1)[1].split("#     qwen_cli:", 1)[0]

        self.assertIn("#         - web_search", codex_block)
        self.assertIn("#         pre_static_args:", codex_block)
        self.assertIn("#         subcommand: exec", codex_block)
        self.assertIn("#         web_search_mode: flag", codex_block)
        self.assertIn("#         web_search_flag: --search", codex_block)


if __name__ == "__main__":
    unittest.main()
