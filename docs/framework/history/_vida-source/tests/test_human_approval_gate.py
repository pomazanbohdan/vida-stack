import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
DISPATCH_PATH = ROOT_DIR / "_vida" / "scripts" / "subagent-dispatch.py"
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "human-approval-gate.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class HumanApprovalGateTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.dispatch = load_module("subagent_dispatch_human_gate_test", DISPATCH_PATH)

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.dispatch.ROUTE_RECEIPT_DIR = Path(self.temp_dir.name)
        self.original_run_graph_dir = self.dispatch.run_graph_runtime.STATE_DIR
        self.dispatch.run_graph_runtime.STATE_DIR = Path(self.temp_dir.name) / "run-graphs"

    def tearDown(self) -> None:
        self.dispatch.run_graph_runtime.STATE_DIR = self.original_run_graph_dir
        self.temp_dir.cleanup()

    def test_validate_approval_receipt_not_required_for_promotion_ready(self) -> None:
        route = {"task_class": "analysis"}

        ok, payload, reason = self.dispatch.validate_approval_receipt(
            "unit-task",
            "analysis",
            route,
            "promotion_ready",
        )

        self.assertTrue(ok)
        self.assertEqual(payload.get("status"), "not_required")
        self.assertEqual(reason, "")

    def test_validate_approval_receipt_requires_receipt_for_policy_gate(self) -> None:
        route = {"task_class": "analysis"}

        ok, payload, reason = self.dispatch.validate_approval_receipt(
            "unit-task",
            "analysis",
            route,
            "policy_gate_required",
        )

        self.assertFalse(ok)
        self.assertEqual(payload, {})
        self.assertEqual(reason, "missing_approval_receipt")

    def test_validate_approval_receipt_accepts_matching_approved_receipt(self) -> None:
        route = {"task_class": "analysis"}
        receipt = {
            "task_id": "unit-task",
            "task_class": "analysis",
            "review_state": "policy_gate_required",
            "decision": "approved",
            "approver_id": "operator",
            "notes": "Approved after bounded review.",
            "route_receipt_hash": self.dispatch.route_receipt_hash(route),
        }
        path = self.dispatch.approval_receipt_path("unit-task", "analysis")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(receipt), encoding="utf-8")

        ok, payload, reason = self.dispatch.validate_approval_receipt(
            "unit-task",
            "analysis",
            route,
            "policy_gate_required",
        )

        self.assertTrue(ok)
        self.assertEqual(payload.get("decision"), "approved")
        self.assertEqual(reason, "")

    def test_validate_approval_receipt_rejects_stale_or_rejected_receipt(self) -> None:
        route = {"task_class": "analysis"}
        receipt = {
            "task_id": "unit-task",
            "task_class": "analysis",
            "review_state": "senior_review_required",
            "decision": "rejected",
            "approver_id": "operator",
            "notes": "Needs stronger evidence.",
            "route_receipt_hash": self.dispatch.route_receipt_hash({"task_class": "other"}),
        }
        path = self.dispatch.approval_receipt_path("unit-task", "analysis")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(receipt), encoding="utf-8")

        ok, payload, reason = self.dispatch.validate_approval_receipt(
            "unit-task",
            "analysis",
            route,
            "senior_review_required",
        )

        self.assertFalse(ok)
        self.assertEqual(payload.get("decision"), "rejected")
        self.assertEqual(reason, "approval_rejected")

    def test_apply_manifest_approval_gate_blocks_synthesis_without_receipt(self) -> None:
        route = {"task_class": "analysis"}
        manifest = {
            "synthesis_ready": True,
            "status": "completed",
            "phase": "completed",
            "review_state": "policy_gate_required",
            "target_manifest_review_state": "policy_gate_required",
        }

        gated = self.dispatch.apply_manifest_approval_gate(
            "unit-task",
            "analysis",
            route,
            manifest,
        )

        self.assertFalse(gated["synthesis_ready"])
        self.assertEqual(gated["status"], "approval_pending")
        self.assertEqual(gated["approval"]["reason"], "missing_approval_receipt")
        run_graph = self.dispatch.run_graph_runtime.load_graph("unit-task")
        self.assertEqual(run_graph["nodes"]["approval"]["status"], "blocked")
        self.assertEqual(run_graph["nodes"]["synthesis"]["status"], "blocked")


class HumanApprovalScriptTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.script = load_module("human_approval_gate_script_test", SCRIPT_PATH)

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.script.dispatch.ROUTE_RECEIPT_DIR = Path(self.temp_dir.name)

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_write_receipt_and_validate_round_trip(self) -> None:
        self.script.dispatch.route_snapshot = lambda task_class, task_id: ({}, {"task_class": task_class})

        path = self.script.write_receipt(
            task_id="unit-task",
            task_class="analysis",
            review_state="policy_gate_required",
            decision="approved",
            approver_id="operator",
            notes="Approved after manual policy review.",
        )

        self.assertTrue(path.exists())
        ok, payload, reason = self.script.dispatch.validate_approval_receipt(
            "unit-task",
            "analysis",
            {"task_class": "analysis"},
            "policy_gate_required",
        )
        self.assertTrue(ok)
        self.assertEqual(payload.get("decision"), "approved")
        self.assertEqual(reason, "")
