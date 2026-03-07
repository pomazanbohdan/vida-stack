import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT_DIR = Path(__file__).resolve().parents[2]
DISPATCH_PATH = ROOT_DIR / "_vida" / "scripts" / "subagent-dispatch.py"
SYSTEM_PATH = ROOT_DIR / "_vida" / "scripts" / "subagent-system.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class InternalEscalationPolicyTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.dispatch = load_module("subagent_dispatch_runtime_test", DISPATCH_PATH)
        cls.system = load_module("subagent_system_runtime_test", SYSTEM_PATH)

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.dispatch.ROUTE_RECEIPT_DIR = Path(self.temp_dir.name)

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_route_policy_payload_does_not_auto_authorize_internal_fallback(self) -> None:
        task_id = "unit-task"
        route = {
            "task_class": "analysis",
            "bridge_fallback_subagent": "codex_cli",
            "internal_escalation_trigger": "semantic_conflict_without_majority",
            "dispatch_policy": {
                "direct_internal_bypass_forbidden": "yes",
                "internal_route_authorized": "no",
                "internal_escalation_allowed": "yes",
                "allowed_internal_reasons": ["semantic_conflict_without_majority"],
            },
            "route_budget": {
                "max_budget_units": 10,
                "budget_policy": "balanced",
                "max_budget_cost_class": "expensive",
                "estimated_route_cost_class": "paid",
            },
            "analysis_plan": {
                "required": "no",
                "receipt_required": "no",
            },
        }
        subagent_cfg = {
            "budget_cost_units": 10,
            "billing_tier": "internal",
        }

        payload = self.dispatch.route_policy_payload(task_id, route, "internal_subagents", subagent_cfg, "fallback")

        self.assertFalse(payload["internal_route_authorized"])
        self.assertEqual(payload["internal_escalation_receipt"], {})
        self.assertTrue(payload["policy_bypass"])

    def test_dispatch_policy_violation_requires_explicit_internal_receipt(self) -> None:
        task_id = "unit-task"
        route = {
            "task_class": "analysis",
            "dispatch_required": "fanout_then_synthesize",
            "external_first_required": "yes",
            "bridge_fallback_subagent": "codex_cli",
            "internal_escalation_trigger": "semantic_conflict_without_majority",
            "fanout_subagents": ["qwen_cli", "gemini_cli"],
            "dispatch_policy": {
                "direct_internal_bypass_forbidden": "yes",
                "internal_route_authorized": "no",
                "internal_escalation_allowed": "yes",
                "allowed_internal_reasons": ["semantic_conflict_without_majority"],
            },
            "analysis_plan": {
                "required": "no",
                "receipt_required": "no",
                "route_task_class": "",
            },
        }

        violation = self.dispatch.dispatch_policy_violation(task_id, route, "internal_subagents", "fallback")

        self.assertIn("receipt", violation)

    def test_route_policy_payload_accepts_matching_internal_receipt(self) -> None:
        task_id = "unit-task"
        route = {
            "task_class": "analysis",
            "bridge_fallback_subagent": "codex_cli",
            "internal_escalation_trigger": "semantic_conflict_without_majority",
            "dispatch_policy": {
                "direct_internal_bypass_forbidden": "yes",
                "internal_route_authorized": "no",
                "internal_escalation_allowed": "yes",
                "allowed_internal_reasons": ["semantic_conflict_without_majority"],
            },
            "route_budget": {
                "max_budget_units": 10,
                "budget_policy": "balanced",
                "max_budget_cost_class": "expensive",
                "estimated_route_cost_class": "paid",
            },
            "analysis_plan": {
                "required": "no",
                "receipt_required": "no",
            },
        }
        subagent_cfg = {
            "budget_cost_units": 10,
            "billing_tier": "internal",
        }
        receipt = {
            "reason": "semantic_conflict_without_majority",
            "scope": "arbitration-escalation",
            "notes": "external fanout ended without majority consensus",
            "route_receipt_hash": self.dispatch.route_receipt_hash(route),
        }
        receipt_path = self.dispatch.internal_escalation_receipt_path(task_id, "analysis")
        receipt_path.parent.mkdir(parents=True, exist_ok=True)
        receipt_path.write_text(json.dumps(receipt), encoding="utf-8")

        payload = self.dispatch.route_policy_payload(task_id, route, "internal_subagents", subagent_cfg, "fallback")
        violation = self.dispatch.dispatch_policy_violation(task_id, route, "internal_subagents", "fallback")

        self.assertTrue(payload["internal_route_authorized"])
        self.assertEqual(payload["internal_escalation_receipt"]["reason"], "semantic_conflict_without_majority")
        self.assertEqual(violation, "")

    def test_route_subagent_keeps_internal_route_unauthorized_in_hybrid_without_receipt(self) -> None:
        candidate = {
            "subagent": "internal_subagents",
            "state": "preferred",
            "lifecycle_stage": "active",
            "lane_stage": "active",
            "effective_score": 100,
            "task_fit_score": 90,
            "global_score": 90,
            "task_class_fit_bonus": 10,
            "task_class_fit_reasons": ["internal_only_candidate"],
            "memory_adjustment": 0,
            "budget_adjustment": 0,
            "budget_cost_units": 10,
            "selected_model": None,
            "selected_model_source": "none",
            "selected_profile": None,
            "selected_profile_source": "none",
            "subagent_backend_class": "internal",
            "subagent_state": "active",
            "capability_band": ["implementation_safe"],
            "subagent_write_scope": "orchestrator_native",
            "orchestration_tier": "senior",
            "cost_priority": "premium",
            "max_runtime_seconds": 120,
            "startup_timeout_seconds": 30,
            "no_output_timeout_seconds": 90,
            "progress_idle_timeout_seconds": 60,
            "success_count": 1,
        }
        context = {
            "task_route_cfg": {},
            "candidates": [candidate],
            "suppressed_subagents": [],
            "write_scope": "bounded",
            "verification_gate": "subagent_return_contract",
            "risk_class": "R1",
            "max_runtime_seconds": 120,
            "min_output_bytes": 180,
            "merge_policy": "single_subagent",
            "fanout_order": [],
            "dispatch_required": "external_readonly_then_senior_writer",
            "external_first_required": "no",
            "analysis_required": "no",
            "analysis_route_task_class": "",
            "analysis_fanout_order": [],
            "analysis_fanout_min_results": 0,
            "analysis_merge_policy": "single_subagent",
            "analysis_external_first_required": "no",
            "analysis_receipt_required": "no",
            "analysis_zero_budget_required": "no",
            "analysis_default_in_boot": "no",
            "bridge_fallback_subagent": "codex_cli",
            "internal_escalation_trigger": "cross_module_or_architecture_shift",
            "graph_strategy": "deterministic_then_escalate",
            "deterministic_first": "no",
            "max_parallel_agents": 1,
            "state_owner": "orchestrator_only",
            "budget_limits": {
                "budget_policy": "balanced",
                "max_budget_units": 10,
                "max_budget_cost_class": "expensive",
            },
            "verification_route_task_class": "review_ensemble",
            "independent_verification_required": "no",
            "local_execution_allowed": "no",
            "local_execution_preferred": "no",
            "direct_internal_bypass_forbidden": "no",
            "internal_escalation_allowed": "yes",
            "allowed_internal_reasons": ["cross_module_or_architecture_shift"],
            "cli_dispatch_required_if_delegating": "no",
            "required_dispatch_path": ["route_selected", "bridge_fallback", "internal_escalation"],
        }

        with mock.patch.object(self.system, "runtime_snapshot", return_value={"agent_system": {"effective_mode": "hybrid"}}), \
            mock.patch.object(self.system.vida_config, "load_validated_config", return_value={}), \
            mock.patch.object(self.system, "load_strategy_memory", return_value={}), \
            mock.patch.object(self.system, "route_candidate_context", return_value=context), \
            mock.patch.object(self.system, "apply_bridge_fallback_priority", side_effect=lambda items, _: items), \
            mock.patch.object(
                self.system,
                "build_analysis_plan",
                return_value={
                    "required": "no",
                    "route_task_class": "",
                    "selected_subagent": None,
                    "fanout_subagents": [],
                    "fanout_min_results": 0,
                    "merge_policy": "single_subagent",
                    "external_first_required": "no",
                    "zero_budget_required": "no",
                    "receipt_required": "no",
                    "default_in_boot": "no",
                    "reason": "analysis_not_required",
                },
            ), \
            mock.patch.object(
                self.system,
                "build_independent_verification_plan",
                return_value={
                    "required": "no",
                    "route_task_class": "review_ensemble",
                    "selected_subagent": None,
                    "fallback_subagents": [],
                    "independent": False,
                    "reason": "verification_not_required",
                },
            ), \
            mock.patch.object(
                self.system,
                "build_route_budget",
                return_value={
                    "budget_policy": "balanced",
                    "max_budget_units": 10,
                    "max_budget_cost_class": "expensive",
                    "estimated_route_cost_class": "paid",
                },
            ), \
            mock.patch.object(
                self.system,
                "build_route_graph",
                return_value={"graph_strategy": "deterministic_then_escalate", "deterministic_first": "no", "primary_mode": "single", "nodes": [], "edges": [], "planned_path": []},
            ), \
            mock.patch.object(self.system, "target_review_state_for", return_value="review_required"), \
            mock.patch.object(self.system, "target_manifest_review_state_for", return_value="promotion_ready"):
            route = self.system.route_subagent("implementation")

        self.assertEqual(route["selected_subagent"], "internal_subagents")
        self.assertEqual(route["dispatch_policy"]["internal_route_authorized"], "no")
        self.assertEqual(route["internal_route_authorized"], "no")

    def test_build_coach_plan_selects_two_independent_coaches(self) -> None:
        coach_candidates = [
            {
                "subagent": "qwen_cli",
                "effective_score": 140,
                "selected_model": None,
                "selected_profile": None,
                "selected_model_source": "none",
                "selected_profile_source": "none",
                "subagent_backend_class": "external_cli",
            },
            {
                "subagent": "gemini_cli",
                "effective_score": 136,
                "selected_model": None,
                "selected_profile": None,
                "selected_model_source": "none",
                "selected_profile_source": "none",
                "subagent_backend_class": "external_cli",
            },
            {
                "subagent": "kilo_cli",
                "effective_score": 120,
                "selected_model": None,
                "selected_profile": None,
                "selected_model_source": "none",
                "selected_profile_source": "none",
                "subagent_backend_class": "external_cli",
            },
        ]
        coach_context = {
            "candidates": coach_candidates,
            "bridge_fallback_subagent": "codex_cli",
            "fanout_order": ["qwen_cli", "gemini_cli", "kilo_cli"],
            "merge_policy": "unanimous_approve_rework_bias",
            "task_route_cfg": {"fanout_min_results": 2},
            "risk_class": "R1",
            "verification_gate": "coach_review",
        }

        with mock.patch.object(self.system, "route_candidate_context", return_value=coach_context), \
            mock.patch.object(self.system, "apply_bridge_fallback_priority", side_effect=lambda items, _: items):
            plan = self.system.build_coach_plan(
                "implementation",
                snapshot={},
                config={},
                strategy={},
                excluded_subagents={"codex_cli"},
                coach_task_class="coach",
                required="yes",
                max_passes=2,
            )

        self.assertEqual(plan["selected_subagent"], "qwen_cli")
        self.assertEqual(plan["selected_subagents"], ["qwen_cli", "gemini_cli"])
        self.assertEqual(plan["min_results"], 2)
        self.assertEqual(plan["merge_policy"], "unanimous_approve_rework_bias")
        self.assertTrue(plan["independent"])
        self.assertEqual(plan["fallback_subagents"][0]["subagent"], "kilo_cli")

    def test_route_subagent_raises_default_coach_pass_cap_to_quorum(self) -> None:
        candidate = {
            "subagent": "codex_cli",
            "state": "preferred",
            "lifecycle_stage": "active",
            "lane_stage": "active",
            "effective_score": 100,
            "task_fit_score": 90,
            "global_score": 90,
            "task_class_fit_bonus": 10,
            "task_class_fit_reasons": ["writer"],
            "memory_adjustment": 0,
            "budget_adjustment": 0,
            "budget_cost_units": 1,
            "selected_model": None,
            "selected_model_source": "none",
            "selected_profile": None,
            "selected_profile_source": "none",
            "subagent_backend_class": "external_cli",
            "subagent_state": "active",
            "capability_band": ["bounded_write_safe"],
            "subagent_write_scope": "scoped_only",
            "orchestration_tier": "bridge",
            "cost_priority": "fallback",
            "max_runtime_seconds": 120,
            "startup_timeout_seconds": 30,
            "no_output_timeout_seconds": 90,
            "progress_idle_timeout_seconds": 60,
            "success_count": 1,
        }
        context = {
            "task_route_cfg": {},
            "candidates": [candidate],
            "suppressed_subagents": [],
            "write_scope": "scoped_only",
            "verification_gate": "targeted_verification",
            "risk_class": "R1",
            "max_runtime_seconds": 120,
            "min_output_bytes": 180,
            "merge_policy": "single_subagent",
            "fanout_order": [],
            "dispatch_required": "external_readonly_then_senior_writer",
            "external_first_required": "no",
            "analysis_required": "no",
            "analysis_route_task_class": "",
            "analysis_fanout_order": [],
            "analysis_fanout_min_results": 0,
            "analysis_merge_policy": "single_subagent",
            "analysis_external_first_required": "no",
            "analysis_receipt_required": "no",
            "analysis_zero_budget_required": "no",
            "analysis_default_in_boot": "no",
            "coach_required": "yes",
            "coach_route_task_class": "coach",
            "bridge_fallback_subagent": "codex_cli",
            "internal_escalation_trigger": "",
            "graph_strategy": "deterministic_then_escalate",
            "deterministic_first": "no",
            "max_parallel_agents": 1,
            "state_owner": "orchestrator_only",
            "budget_limits": {
                "budget_policy": "balanced",
                "max_budget_units": 5,
                "max_cli_subagent_calls": 4,
                "max_coach_passes": 1,
                "max_verification_passes": 1,
                "max_fallback_hops": 2,
                "max_total_runtime_seconds": 540,
                "max_budget_cost_class": "paid",
            },
            "max_coach_passes": 1,
            "verification_route_task_class": "review_ensemble",
            "independent_verification_required": "yes",
            "local_execution_allowed": "no",
            "local_execution_preferred": "no",
            "direct_internal_bypass_forbidden": "no",
            "internal_escalation_allowed": "no",
            "allowed_internal_reasons": [],
            "cli_dispatch_required_if_delegating": "no",
            "required_dispatch_path": ["route_selected"],
        }

        with mock.patch.object(self.system, "runtime_snapshot", return_value={"agent_system": {"effective_mode": "hybrid"}}), \
            mock.patch.object(self.system.vida_config, "load_validated_config", return_value={}), \
            mock.patch.object(self.system, "load_strategy_memory", return_value={}), \
            mock.patch.object(self.system, "route_candidate_context", return_value=context), \
            mock.patch.object(self.system, "apply_bridge_fallback_priority", side_effect=lambda items, _: items), \
            mock.patch.object(
                self.system,
                "build_analysis_plan",
                return_value={
                    "required": "no",
                    "route_task_class": "",
                    "selected_subagent": None,
                    "fanout_subagents": [],
                    "fanout_min_results": 0,
                    "merge_policy": "single_subagent",
                    "external_first_required": "no",
                    "zero_budget_required": "no",
                    "receipt_required": "no",
                    "default_in_boot": "no",
                    "reason": "analysis_not_required",
                },
            ), \
            mock.patch.object(
                self.system,
                "build_coach_plan",
                return_value={
                    "required": "yes",
                    "route_task_class": "coach",
                    "selected_subagent": "qwen_cli",
                    "selected_subagents": ["qwen_cli", "gemini_cli"],
                    "independent": True,
                    "min_results": 2,
                    "merge_policy": "unanimous_approve_rework_bias",
                    "max_passes": 1,
                    "fallback_subagents": [],
                    "reason": "independent_coach_ensemble_selected",
                },
            ), \
            mock.patch.object(
                self.system,
                "build_independent_verification_plan",
                return_value={
                    "required": "yes",
                    "route_task_class": "review_ensemble",
                    "selected_subagent": "qwen_cli",
                    "fallback_subagents": [],
                    "independent": True,
                    "reason": "independent_verifier_selected",
                },
            ), \
            mock.patch.object(
                self.system,
                "build_route_budget",
                side_effect=lambda **kwargs: kwargs["budget_limits"],
            ), \
            mock.patch.object(
                self.system,
                "build_route_graph",
                return_value={"graph_strategy": "deterministic_then_escalate", "deterministic_first": "no", "primary_mode": "single", "nodes": [], "edges": [], "planned_path": []},
            ), \
            mock.patch.object(self.system, "target_review_state_for", return_value="review_required"), \
            mock.patch.object(self.system, "target_manifest_review_state_for", return_value="promotion_ready"):
            route = self.system.route_subagent("implementation")

        self.assertEqual(route["route_budget"]["max_coach_passes"], 2)


if __name__ == "__main__":
    unittest.main()
