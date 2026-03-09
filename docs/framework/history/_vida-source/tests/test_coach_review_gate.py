import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
GATE_PATH = ROOT_DIR / "_vida" / "scripts" / "coach-review-gate.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class CoachReviewGateTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.gate = load_module("coach_review_gate_test", GATE_PATH)

    def test_gate_accepts_valid_coach_receipt(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            self.gate.ROUTE_RECEIPT_DIR = tmp_path
            self.gate.dispatch_runtime.ROUTE_RECEIPT_DIR = tmp_path
            self.gate.OVERRIDE_DIR = tmp_path / "coach-review-overrides"
            route_receipt = {
                "task_class": "implementation",
                "coach_required": "yes",
                "coach_plan": {"required": "yes", "route_task_class": "coach"},
            }
            route_file = tmp_path / "unit-task.implementation.route.json"
            route_file.write_text(json.dumps({"route_receipt": route_receipt}), encoding="utf-8")
            coach_file = tmp_path / "unit-task.implementation.coach.json"
            coach_file.write_text(
                json.dumps(
                    {
                        "status": "coach_approved",
                        "route_receipt_hash": self.gate.dispatch_runtime.route_receipt_hash(route_receipt),
                    }
                ),
                encoding="utf-8",
            )

            exit_code, payload = self.gate.check_gate("unit-task")

        self.assertEqual(exit_code, 0)
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(len(payload["blockers"]), 0)

    def test_gate_blocks_when_coach_artifact_is_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            self.gate.ROUTE_RECEIPT_DIR = tmp_path
            self.gate.dispatch_runtime.ROUTE_RECEIPT_DIR = tmp_path
            self.gate.OVERRIDE_DIR = tmp_path / "coach-review-overrides"
            route_receipt = {
                "task_class": "implementation",
                "coach_required": "yes",
                "coach_plan": {"required": "yes", "route_task_class": "coach"},
            }
            route_file = tmp_path / "unit-task.implementation.route.json"
            route_file.write_text(json.dumps({"route_receipt": route_receipt}), encoding="utf-8")

            exit_code, payload = self.gate.check_gate("unit-task")

        self.assertEqual(exit_code, 2)
        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["blockers"][0]["reason"], "missing_coach_review_artifact")

    def test_gate_accepts_structured_override_when_no_eligible_coach_is_recorded(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            self.gate.ROUTE_RECEIPT_DIR = tmp_path
            self.gate.dispatch_runtime.ROUTE_RECEIPT_DIR = tmp_path
            self.gate.OVERRIDE_DIR = tmp_path / "coach-review-overrides"
            route_receipt = {
                "task_class": "implementation",
                "coach_required": "yes",
                "coach_plan": {"required": "yes", "route_task_class": "coach"},
            }
            route_file = tmp_path / "unit-task.implementation.route.json"
            route_file.write_text(json.dumps({"route_receipt": route_receipt}), encoding="utf-8")
            coach_blocker = tmp_path / "unit-task.implementation.coach-blocker.json"
            coach_blocker.write_text(
                json.dumps(
                    {
                        "status": "coach_pass_cap_exceeded",
                        "reason": "no_eligible_coach",
                        "route_receipt_hash": self.gate.dispatch_runtime.route_receipt_hash(route_receipt),
                    }
                ),
                encoding="utf-8",
            )
            self.gate.write_override_receipt(
                "unit-task",
                "no_eligible_coach",
                "No eligible coach lane produced a lawful review artifact",
                evidence="coach-review.json",
                actor="orchestrator",
            )

            exit_code, payload = self.gate.check_gate("unit-task")

        self.assertEqual(exit_code, 0)
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["authorized_via"], "structured_override")
        self.assertTrue(payload["override_receipt_present"])

    def test_gate_reports_missing_structured_rework_handoff(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            self.gate.ROUTE_RECEIPT_DIR = tmp_path
            self.gate.dispatch_runtime.ROUTE_RECEIPT_DIR = tmp_path
            self.gate.OVERRIDE_DIR = tmp_path / "coach-review-overrides"
            route_receipt = {
                "task_class": "implementation",
                "coach_required": "yes",
                "coach_plan": {"required": "yes", "route_task_class": "coach"},
            }
            route_file = tmp_path / "unit-task.implementation.route.json"
            route_file.write_text(json.dumps({"route_receipt": route_receipt}), encoding="utf-8")
            coach_blocker = tmp_path / "unit-task.implementation.coach-blocker.json"
            coach_blocker.write_text(
                json.dumps(
                    {
                        "status": "return_for_rework",
                        "reason": "writer must start over",
                        "route_receipt_hash": self.gate.dispatch_runtime.route_receipt_hash(route_receipt),
                    }
                ),
                encoding="utf-8",
            )

            exit_code, payload = self.gate.check_gate("unit-task")

        self.assertEqual(exit_code, 2)
        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["blockers"][0]["reason"], "missing_rework_handoff")

    def test_gate_surfaces_valid_rework_handoff_for_return_to_writer(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            self.gate.ROUTE_RECEIPT_DIR = tmp_path
            self.gate.dispatch_runtime.ROUTE_RECEIPT_DIR = tmp_path
            self.gate.OVERRIDE_DIR = tmp_path / "coach-review-overrides"
            route_receipt = {
                "task_class": "implementation",
                "coach_required": "yes",
                "coach_plan": {"required": "yes", "route_task_class": "coach"},
                "analysis_plan": {"required": "no", "receipt_required": "no", "route_task_class": "", "fanout_subagents": []},
                "dispatch_policy": {},
                "route_budget": {},
                "fallback_subagents": [],
            }
            route_file = tmp_path / "unit-task.implementation.route.json"
            route_file.write_text(json.dumps({"route_receipt": route_receipt}), encoding="utf-8")
            handoff_payload = {
                "status": "writer_rework_ready",
                "fresh_start_required": True,
                "route_receipt_hash": self.gate.dispatch_runtime.route_receipt_hash(route_receipt),
                "original_prompt_text": "Implement the feature from spec.",
                "fresh_prompt_text": "Implement the feature from spec.\n\nFresh Rework Handoff:\n- Summary: missing gate\n",
                "coach_delta": {
                    "coach_feedback": "missing gate",
                    "feedback_source": "output_json_payload",
                    "feedback_sources": ["output_json_payload"],
                },
            }
            (tmp_path / "unit-task.implementation.rework-handoff.json").write_text(
                json.dumps(handoff_payload),
                encoding="utf-8",
            )
            coach_blocker = tmp_path / "unit-task.implementation.coach-blocker.json"
            coach_blocker.write_text(
                json.dumps(
                    {
                        "status": "return_for_rework",
                        "reason": "writer must start over",
                        "route_receipt_hash": self.gate.dispatch_runtime.route_receipt_hash(route_receipt),
                    }
                ),
                encoding="utf-8",
            )

            exit_code, payload = self.gate.check_gate("unit-task")

        self.assertEqual(exit_code, 2)
        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["blockers"][0]["reason"], "writer must start over")
        self.assertTrue(payload["blockers"][0]["rework_handoff_path"].endswith(".rework-handoff.json"))
        self.assertEqual(payload["blockers"][0]["rework_handoff_status"], "writer_rework_ready")

    def test_gate_blocks_rework_handoff_without_feedback_provenance(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            self.gate.ROUTE_RECEIPT_DIR = tmp_path
            self.gate.dispatch_runtime.ROUTE_RECEIPT_DIR = tmp_path
            self.gate.OVERRIDE_DIR = tmp_path / "coach-review-overrides"
            route_receipt = {
                "task_class": "implementation",
                "coach_required": "yes",
                "coach_plan": {"required": "yes", "route_task_class": "coach"},
                "analysis_plan": {"required": "no", "receipt_required": "no", "route_task_class": "", "fanout_subagents": []},
                "dispatch_policy": {},
                "route_budget": {},
                "fallback_subagents": [],
            }
            route_file = tmp_path / "unit-task.implementation.route.json"
            route_file.write_text(json.dumps({"route_receipt": route_receipt}), encoding="utf-8")
            handoff_payload = {
                "status": "writer_rework_ready",
                "fresh_start_required": True,
                "route_receipt_hash": self.gate.dispatch_runtime.route_receipt_hash(route_receipt),
                "original_prompt_text": "Implement the feature from spec.",
                "fresh_prompt_text": "Implement the feature from spec.\n\nFresh Rework Handoff:\n- Summary: missing gate\n",
                "coach_delta": {"coach_feedback": "missing gate"},
            }
            (tmp_path / "unit-task.implementation.rework-handoff.json").write_text(
                json.dumps(handoff_payload),
                encoding="utf-8",
            )
            coach_blocker = tmp_path / "unit-task.implementation.coach-blocker.json"
            coach_blocker.write_text(
                json.dumps(
                    {
                        "status": "return_for_rework",
                        "reason": "writer must start over",
                        "route_receipt_hash": self.gate.dispatch_runtime.route_receipt_hash(route_receipt),
                    }
                ),
                encoding="utf-8",
            )

            exit_code, payload = self.gate.check_gate("unit-task")

        self.assertEqual(exit_code, 2)
        self.assertEqual(payload["status"], "blocked")
        self.assertEqual(payload["blockers"][0]["reason"], "missing_feedback_provenance")
