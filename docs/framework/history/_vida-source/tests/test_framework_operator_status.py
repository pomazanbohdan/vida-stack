import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "framework-operator-status.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class FrameworkOperatorStatusTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("framework_operator_status_test", SCRIPT_PATH)

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        tmp_path = Path(self.temp_dir.name)
        self.module.FRAMEWORK_MEMORY_PATH = tmp_path / "framework-memory.json"
        self.module.CONTEXT_GOVERNANCE_PATH = tmp_path / "context-governance.json"
        self.module.SILENT_DIAGNOSIS_PATH = tmp_path / "silent-framework-diagnosis.json"
        self.module.ISSUES_JSONL_PATH = tmp_path / "issues.jsonl"
        self.module.ROUTE_RECEIPT_DIR = tmp_path / "route-receipts"
        self.module.ROUTE_RECEIPT_DIR.mkdir(parents=True, exist_ok=True)
        self.module.RUN_GRAPH_DIR = tmp_path / "run-graphs"
        self.module.RUN_GRAPH_DIR.mkdir(parents=True, exist_ok=True)
        self.module.TASK_STATE_RECONCILE = ROOT_DIR / "_vida" / "scripts" / "task-state-reconcile.py"

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_build_status_summarizes_memory_and_approvals(self) -> None:
        self.module.FRAMEWORK_MEMORY_PATH.write_text(
            json.dumps(
                {
                    "summary": {"lesson_count": 1, "correction_count": 2, "anomaly_count": 3},
                    "entries": [],
                }
            ),
            encoding="utf-8",
        )
        self.module.CONTEXT_GOVERNANCE_PATH.write_text(
            json.dumps(
                {
                    "summary": {
                        "by_source_class": {"local_runtime": 2, "web_validated": 1},
                        "task_count": 1,
                        "web_validated_count": 1,
                    }
                }
            ),
            encoding="utf-8",
        )
        (self.module.ROUTE_RECEIPT_DIR / "mobile-2wy.1.analysis.approval.json").write_text(
            json.dumps({"decision": "approved", "review_state": "policy_gate_required"}),
            encoding="utf-8",
        )

        payload = self.module.build_status_payload()

        self.assertEqual(payload["framework_memory"]["anomaly_count"], 3)
        self.assertEqual(payload["context_governance"]["web_validated_count"], 1)
        self.assertEqual(payload["approval_summary"]["approved_count"], 1)

    def test_build_status_includes_silent_diagnosis_backlog_and_recent_reflections(self) -> None:
        self.module.SILENT_DIAGNOSIS_PATH.write_text(
            json.dumps(
                {
                    "pending_framework_bugs": [
                        {
                            "bug_id": "mobile-1hv.18",
                            "summary": "run-graph pollution",
                            "current_task": "mobile-10y",
                            "created_at": "2026-03-07T22:12:14Z",
                            "workaround": "ignore unit-task",
                        }
                    ],
                    "session_reflections": [
                        {
                            "current_task": "mobile-10y",
                            "criteria": ["instruction_clarity"],
                            "gaps": ["operator surface too thin"],
                            "ts": "2026-03-07T22:13:00Z",
                        }
                    ],
                }
            ),
            encoding="utf-8",
        )

        payload = self.module.build_status_payload()

        self.assertEqual(payload["silent_diagnosis"]["pending_bug_count"], 1)
        self.assertEqual(payload["silent_diagnosis"]["pending_bug_ids"], ["mobile-1hv.18"])
        self.assertEqual(payload["silent_diagnosis"]["recent_pending"][0]["current_task"], "mobile-10y")
        self.assertEqual(payload["silent_diagnosis"]["session_reflection_count"], 1)
        self.assertEqual(payload["silent_diagnosis"]["recent_reflections"][0]["gaps"], ["operator surface too thin"])

    def test_build_status_filters_closed_pending_framework_bugs(self) -> None:
        self.module.SILENT_DIAGNOSIS_PATH.write_text(
            json.dumps(
                {
                    "pending_framework_bugs": [
                        {
                            "bug_id": "mobile-1hv.18",
                            "summary": "run-graph pollution",
                            "current_task": "mobile-10y",
                            "created_at": "2026-03-07T22:12:14Z",
                            "workaround": "ignore unit-task",
                        },
                        {
                            "bug_id": "mobile-1hv.21",
                            "summary": "operator backlog drift",
                            "current_task": "mobile-1hv.17",
                            "created_at": "2026-03-07T22:57:31Z",
                            "workaround": "use silent diagnosis status directly",
                        },
                    ]
                }
            ),
            encoding="utf-8",
        )
        self.module.ISSUES_JSONL_PATH.write_text(
            "\n".join(
                [
                    json.dumps({"id": "mobile-1hv.18", "status": "closed"}),
                    json.dumps({"id": "mobile-1hv.21", "status": "open"}),
                ]
            )
            + "\n",
            encoding="utf-8",
        )

        payload = self.module.build_status_payload()

        self.assertEqual(payload["silent_diagnosis"]["pending_bug_count"], 1)
        self.assertEqual(payload["silent_diagnosis"]["pending_bug_ids"], ["mobile-1hv.21"])

    def test_build_status_summarizes_route_rationale_and_flags_suspicious_run_graphs(self) -> None:
        (self.module.ROUTE_RECEIPT_DIR / "mobile-10y.implementation.route.json").write_text(
            json.dumps(
                {
                    "route_receipt": {
                        "dispatch_required": "external_readonly_then_senior_writer",
                        "analysis_plan": {"selected_subagent": "gemini_cli"},
                        "verification_plan": {"selected_subagent": "qwen_cli"},
                        "coach_plan": {"selected_subagents": ["gemini_cli", "kilo_cli"]},
                        "route_budget": {
                            "estimated_route_cost_class": "paid",
                            "estimated_route_cost_units": 8,
                            "max_budget_units": 5,
                        },
                        "route_graph": {
                            "nodes": [
                                {"id": "writer", "selected_subagent": "internal_subagents"},
                            ]
                        },
                        "web_search_required": "yes",
                    }
                }
            ),
            encoding="utf-8",
        )
        (self.module.RUN_GRAPH_DIR / "unit-task.json").write_text(
            json.dumps(
                {
                    "task_id": "unit-task",
                    "nodes": {"writer": {"status": "blocked", "meta": {"reason": "test_only"}}},
                }
            ),
            encoding="utf-8",
        )

        payload = self.module.build_status_payload()

        self.assertEqual(
            payload["route_rationale"]["dispatch_required"]["external_readonly_then_senior_writer"],
            1,
        )
        self.assertEqual(payload["route_rationale"]["analysis_selected"]["gemini_cli"], 1)
        self.assertEqual(payload["route_rationale"]["verification_selected"]["qwen_cli"], 1)
        self.assertEqual(payload["route_rationale"]["coach_selected"]["gemini_cli"], 1)
        self.assertEqual(payload["route_rationale"]["coach_selected"]["kilo_cli"], 1)
        self.assertEqual(payload["route_rationale"]["writer_selected"]["internal_subagents"], 1)
        self.assertEqual(payload["route_rationale"]["budget_over_cap_count"], 1)
        self.assertEqual(payload["route_rationale"]["internal_writer_count"], 1)
        self.assertEqual(payload["route_rationale"]["web_search_required_count"], 1)
        self.assertEqual(payload["run_graphs"]["active_run_graphs"], 1)
        self.assertEqual(payload["run_graphs"]["real_active_run_graphs"], 0)
        self.assertEqual(payload["run_graphs"]["suspicious_artifacts"], ["unit-task.json"])

    def test_build_status_clusters_anomalies(self) -> None:
        self.module.FRAMEWORK_MEMORY_PATH.write_text(
            json.dumps(
                {
                    "summary": {"lesson_count": 0, "correction_count": 0, "anomaly_count": 3},
                    "entries": [
                        {"kind": "anomaly", "summary": "cheap lane stalled", "source_task": "mobile-1ic.2"},
                        {"kind": "anomaly", "summary": "cheap lane stalled", "source_task": "mobile-1ic.3"},
                        {"kind": "anomaly", "summary": "queue path incorrect", "source_task": "mobile-eab"},
                        {"kind": "lesson", "summary": "ignore"},
                    ],
                }
            ),
            encoding="utf-8",
        )

        payload = self.module.build_status_payload()

        self.assertEqual(payload["anomaly_clusters"]["unique_anomaly_count"], 2)
        self.assertEqual(payload["anomaly_clusters"]["top_summaries"][0]["summary"], "cheap lane stalled")
        self.assertEqual(payload["anomaly_clusters"]["top_summaries"][0]["count"], 2)
        self.assertEqual(payload["anomaly_clusters"]["anomaly_tasks"]["mobile-1ic.2"], 1)

    def test_build_status_summarizes_task_reconciliation_classes(self) -> None:
        self.module.ISSUES_JSONL_PATH.write_text(
            "\n".join(
                [
                    json.dumps({"id": "mobile-3rf.1", "status": "in_progress", "labels": ["framework"]}),
                    json.dumps({"id": "mobile-3rf.2", "status": "open", "labels": ["framework"]}),
                ]
            )
            + "\n",
            encoding="utf-8",
        )

        original = self.module.load_module

        class FakeModule:
            @staticmethod
            def build_status_payload(task_id: str) -> dict:
                mapping = {
                    "mobile-3rf.1": {"classification": "active"},
                    "mobile-3rf.2": {"classification": "open_but_satisfied"},
                }
                return mapping[task_id]

        self.module.load_module = lambda _name, _path: FakeModule
        try:
            payload = self.module.build_status_payload()
        finally:
            self.module.load_module = original

        self.assertEqual(payload["task_reconciliation"]["counts"]["active"], 1)
        self.assertEqual(payload["task_reconciliation"]["counts"]["open_but_satisfied"], 1)
        self.assertEqual(payload["task_reconciliation"]["sample_ids"]["active"], ["mobile-3rf.1"])


if __name__ == "__main__":
    unittest.main()
