import importlib.util
import unittest
from pathlib import Path
from unittest import mock


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "vida-boot-snapshot.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class VidaBootSnapshotTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("vida_boot_snapshot_test", SCRIPT_PATH)

    def test_build_snapshot_surfaces_run_graph_resume_hint(self) -> None:
        issue = {
            "id": "mobile-q5b",
            "title": "Add durable run graph",
            "status": "in_progress",
            "priority": 1,
            "updated_at": "2026-03-07T21:00:00Z",
            "created_at": "2026-03-07T20:00:00Z",
            "labels": [],
        }
        with mock.patch.object(
            self.module,
            "run_br_json",
            side_effect=[[issue], [issue], [], [issue], [{"id": "mobile-q5b"}]],
        ), mock.patch.object(self.module, "child_entries_for", return_value=[]), mock.patch.object(
            self.module, "framework_self_diagnosis_config", return_value={}
        ), mock.patch.object(
            self.module,
            "run_graph_status",
            return_value={
                "present": True,
                "resume_hint": {"next_node": "writer", "status": "ready", "reason": "analysis_complete"},
            },
        ):
            snapshot = self.module.build_snapshot(5, 3, 5)

        self.assertEqual(snapshot["summary"]["active_run_graphs"], 1)
        self.assertEqual(snapshot["in_progress"][0]["run_graph"]["resume_hint"]["next_node"], "writer")

    def test_render_text_includes_run_graph_detail(self) -> None:
        snapshot = {
            "execution_continue_default": {
                "summary": "route then external analysis",
                "selection_policy": "priority first",
                "compact_assumption": "compact_can_happen_any_time",
            },
            "summary": {
                "top_level_in_progress": 1,
                "top_level_open": 0,
                "top_level_blocked": 0,
                "ready_total": 0,
                "ready_open": 0,
                "active_run_graphs": 1,
            },
            "framework_self_diagnosis": {},
            "in_progress": [
                {
                    "id": "mobile-q5b",
                    "title": "Add durable run graph",
                    "mode": "auto",
                    "subtasks": [],
                    "hidden_subtasks": 0,
                    "run_graph": {
                        "present": True,
                        "resume_hint": {"next_node": "writer", "status": "blocked", "reason": "missing_issue_contract"},
                    },
                }
            ],
            "ready_head": [],
            "decision_required": [],
        }

        rendered = self.module.render_text(snapshot)

        self.assertIn("active_run_graphs=1", rendered)
        self.assertIn("run_graph: next=writer status=blocked reason=missing_issue_contract", rendered)


if __name__ == "__main__":
    unittest.main()
