import importlib.util
import unittest
from pathlib import Path
from unittest import mock


ROOT_DIR = Path(__file__).resolve().parents[2]
DISPATCH_PATH = ROOT_DIR / "_vida" / "scripts" / "subagent-dispatch.py"
SYSTEM_PATH = ROOT_DIR / "_vida" / "scripts" / "subagent-system.py"


def load_dispatch_module():
    spec = importlib.util.spec_from_file_location("subagent_dispatch_budget_test", DISPATCH_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


def load_system_module():
    spec = importlib.util.spec_from_file_location("subagent_system_budget_test", SYSTEM_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class BudgetEnforcementTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.dispatch = load_dispatch_module()
        cls.system = load_system_module()

    def test_route_budget_preflight_blocks_route_overages(self) -> None:
        route = {
            "route_budget": {
                "max_budget_units": 5,
                "estimated_route_cost_units": 8,
                "max_cli_subagent_calls": 1,
                "estimated_primary_calls": 2,
                "estimated_verification_calls": 1,
                "max_verification_passes": 0,
                "estimated_fallback_hops": 2,
                "max_fallback_hops": 1,
            }
        }

        blockers = self.dispatch.route_budget_blockers(route)

        self.assertIn("route_budget_cap_exceeded", blockers)
        self.assertIn("cli_subagent_call_cap_exceeded", blockers)
        self.assertIn("verification_pass_cap_exceeded", blockers)
        self.assertIn("fallback_hop_cap_exceeded", blockers)

    def test_dispatch_policy_violation_blocks_budget_overage(self) -> None:
        route = {
            "task_class": "implementation",
            "dispatch_required": "external_first_review",
            "external_first_required": "no",
            "analysis_plan": {
                "required": "no",
                "receipt_required": "no",
                "route_task_class": "",
            },
            "dispatch_policy": {
                "direct_internal_bypass_forbidden": "no",
                "internal_escalation_allowed": "no",
                "internal_route_authorized": "no",
            },
            "route_budget": {
                "max_budget_units": 5,
                "estimated_route_cost_units": 5,
                "max_cli_subagent_calls": 4,
                "estimated_primary_calls": 1,
                "estimated_verification_calls": 0,
                "max_verification_passes": 1,
                "estimated_fallback_hops": 0,
                "max_fallback_hops": 1,
            },
        }
        subagent_cfg = {
            "budget_cost_units": 10,
            "billing_tier": "internal",
        }

        violation = self.dispatch.dispatch_policy_violation(
            "unit-task",
            route,
            "codex_cli",
            "single",
            subagent_cfg,
        )

        self.assertIn("budget", violation)

    def test_run_verification_phase_blocks_when_pass_cap_is_zero(self) -> None:
        route = {
            "independent_verification_required": "yes",
            "verification_plan": {
                "route_task_class": "review_ensemble",
                "selected_subagent": "gemini_cli",
            },
            "route_budget": {
                "max_verification_passes": 0,
            },
        }

        with mock.patch.object(self.dispatch.subprocess, "run") as mocked_run:
            result, synthesis_ready = self.dispatch.run_verification_phase(
                task_id="unit-task",
                task_class="implementation",
                prompt_file=ROOT_DIR / "AGENTS.md",
                output_dir=ROOT_DIR / "_temp" / "budget-test",
                workdir=ROOT_DIR,
                route=route,
                merge_summary={},
                post_arbitration_merge_summary={},
                results=[],
            )

        mocked_run.assert_not_called()
        self.assertFalse(synthesis_ready)
        self.assertEqual(result["status"], "blocked")
        self.assertEqual(result["reason"], "verification_pass_cap_exceeded")

    def test_build_route_budget_uses_snapshot_subagent_configs_for_free_coaches(self) -> None:
        snapshot = {
            "subagents": {
                "qwen_cli": {
                    "budget_cost_units": 0,
                    "billing_tier": "free",
                    "orchestration_tier": "external_free",
                },
                "gemini_cli": {
                    "budget_cost_units": 0,
                    "billing_tier": "free",
                    "orchestration_tier": "external_free",
                },
                "codex_cli": {
                    "budget_cost_units": 1,
                    "billing_tier": "low",
                    "orchestration_tier": "bridge",
                },
            }
        }
        budget = self.system.build_route_budget(
            snapshot=snapshot,
            budget_limits={
                "budget_policy": "balanced",
                "max_budget_units": 5,
                "max_cli_subagent_calls": 4,
                "max_coach_passes": 2,
                "max_verification_passes": 1,
                "max_fallback_hops": 2,
                "max_total_runtime_seconds": 540,
                "max_budget_cost_class": "paid",
            },
            selected={"budget_cost_units": 1},
            fanout_subagents=[],
            coach_plan={
                "required": "yes",
                "selected_subagent": "qwen_cli",
                "selected_subagents": ["qwen_cli", "gemini_cli"],
            },
            verification_plan={"required": "no", "selected_subagent": None},
            bridge_fallback_subagent="codex_cli",
            internal_escalation_trigger="",
        )

        self.assertEqual(budget["estimated_coach_cost_units"], 0)
        self.assertEqual(budget["estimated_route_cost_units"], 1)


if __name__ == "__main__":
    unittest.main()
