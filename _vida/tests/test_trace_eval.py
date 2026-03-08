import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "trace-eval.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class TraceEvalTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("trace_eval_test", SCRIPT_PATH)

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        tmp = Path(self.temp_dir.name)
        self.module.ROUTE_RECEIPT_DIR = tmp / "route-receipts"
        self.module.RUN_GRAPH_DIR = tmp / "run-graphs"
        self.module.TRACE_EVAL_DIR = tmp / "trace-evals"
        self.module.TRACE_DATASET_DIR = tmp / "trace-datasets"
        self.module.ROUTE_RECEIPT_DIR.mkdir(parents=True, exist_ok=True)
        self.module.RUN_GRAPH_DIR.mkdir(parents=True, exist_ok=True)

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def write_route_receipt(self, name: str, payload: dict) -> None:
        (self.module.ROUTE_RECEIPT_DIR / name).write_text(json.dumps(payload), encoding="utf-8")

    def write_run_graph(self, payload: dict) -> None:
        (self.module.RUN_GRAPH_DIR / "unit-task.json").write_text(json.dumps(payload), encoding="utf-8")

    def test_build_trace_eval_passes_for_consistent_trace(self) -> None:
        self.write_route_receipt(
            "unit-task.implementation.route.json",
            {
                "route_receipt": {
                    "dispatch_required": "external_readonly_then_senior_writer",
                    "analysis_required": "yes",
                    "coach_required": "yes",
                    "independent_verification_required": "yes",
                    "dispatch_policy": {"direct_internal_bypass_forbidden": "yes"},
                    "route_graph": {"planned_path": ["analysis", "writer", "coach", "verification", "synthesis"]},
                },
                "status": "route_selected",
                "task_class": "implementation",
            },
        )
        self.write_route_receipt(
            "unit-task.implementation.approval.json",
            {"review_state": "policy_gate_required", "decision": "approved"},
        )
        self.write_run_graph(
            {
                "resume_hint": {"next_node": "", "status": "completed"},
                "nodes": {
                    "analysis": {"status": "completed", "meta": {}},
                    "writer": {"status": "ready", "meta": {}},
                    "coach": {"status": "completed", "meta": {}},
                    "verifier": {"status": "completed", "meta": {}},
                    "approval": {"status": "completed", "meta": {}},
                    "synthesis": {"status": "completed", "meta": {}},
                },
            }
        )
        with mock.patch.object(
            self.module,
            "run_eval_pack",
            return_value={"task_status": "closed", "block_total": 1, "block_success_rate": 100, "compact_recovery_scenario": "pass"},
        ):
            payload = self.module.build_trace_eval("unit-task")

        self.assertEqual(payload["overall_grade"], "pass")
        self.assertEqual(payload["grades"]["route_correctness"]["grade"], "pass")

    def test_build_trace_eval_fails_on_budget_violation(self) -> None:
        self.write_route_receipt(
            "unit-task.implementation.route.json",
            {
                "route_receipt": {
                    "dispatch_required": "external_readonly_then_senior_writer",
                    "analysis_required": "yes",
                    "coach_required": "yes",
                    "independent_verification_required": "yes",
                    "dispatch_policy": {"direct_internal_bypass_forbidden": "yes"},
                    "route_graph": {"planned_path": ["analysis", "writer"]},
                },
                "status": "route_selected",
                "task_class": "implementation",
                "budget_violation": True,
            },
        )
        self.write_run_graph({"nodes": {"analysis": {"status": "completed"}, "writer": {"status": "blocked"}}})
        with mock.patch.object(self.module, "run_eval_pack", return_value={}):
            payload = self.module.build_trace_eval("unit-task")

        self.assertEqual(payload["overall_grade"], "fail")
        self.assertEqual(payload["grades"]["budget_correctness"]["grade"], "fail")

    def test_build_trace_eval_marks_non_routed_framework_trace_partial(self) -> None:
        self.write_run_graph({"nodes": {"analysis": {"status": "completed"}, "writer": {"status": "ready"}}})
        with mock.patch.object(self.module, "run_eval_pack", return_value={}):
            payload = self.module.build_trace_eval("unit-task")

        self.assertEqual(payload["overall_grade"], "partial")
        self.assertEqual(payload["grades"]["route_correctness"]["grade"], "partial")

    def test_build_trace_eval_marks_missing_trace_artifacts_partial_for_non_routed_task(self) -> None:
        with mock.patch.object(self.module, "run_eval_pack", return_value={}):
            payload = self.module.build_trace_eval("unit-task")

        self.assertEqual(payload["overall_grade"], "partial")
        self.assertEqual(payload["grades"]["route_correctness"]["reason"], "non_routed_task_without_trace_artifacts")

    def test_dataset_export_references_trace_eval(self) -> None:
        trace_eval = {
            "overall_grade": "partial",
            "grades": {
                "route_correctness": {"grade": "pass"},
                "fallback_correctness": {"grade": "pass"},
                "budget_correctness": {"grade": "pass"},
                "approval_correctness": {"grade": "partial"},
            },
            "route_receipt_count": 1,
            "run_graph_resume_hint": {"next_node": "approval", "status": "blocked"},
            "eval_pack": {"task_status": "in_progress"},
            "run_graph_path": "/tmp/run-graph.json",
            "eval_pack_path": "/tmp/eval-pack.json",
            "route_receipts": [{"path": "/tmp/route.json"}],
        }

        payload = self.module.build_trace_dataset("unit-task", trace_eval)

        self.assertEqual(payload["labels"]["overall_grade"], "partial")
        self.assertEqual(payload["summary"]["resume_hint"]["next_node"], "approval")


if __name__ == "__main__":
    unittest.main()
