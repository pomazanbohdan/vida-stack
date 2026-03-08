import importlib.util
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "task-state-reconcile.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class TaskStateReconcileTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("task_state_reconcile_test", SCRIPT_PATH)

    def test_active_when_doing_block_exists(self) -> None:
        classification, reasons, actions = self.module.classify_state(
            issue_status="in_progress",
            steps=[{"status": "doing"}],
            boot_receipt_ok=True,
            verify_ok=False,
            run_graph={},
        )
        self.assertEqual(classification, "active")
        self.assertEqual(reasons, [])
        self.assertEqual(actions, ["continue_current_block"])

    def test_done_ready_to_close_when_in_progress_has_only_terminal_steps_and_verify(self) -> None:
        classification, _, actions = self.module.classify_state(
            issue_status="in_progress",
            steps=[{"status": "done"}, {"status": "superseded"}],
            boot_receipt_ok=True,
            verify_ok=True,
            run_graph={},
        )
        self.assertEqual(classification, "done_ready_to_close")
        self.assertEqual(actions, ["close_now"])

    def test_stale_in_progress_when_backlog_exists_without_active_block(self) -> None:
        classification, reasons, actions = self.module.classify_state(
            issue_status="in_progress",
            steps=[{"status": "done"}, {"status": "todo"}],
            boot_receipt_ok=True,
            verify_ok=False,
            run_graph={},
        )
        self.assertEqual(classification, "stale_in_progress")
        self.assertIn("task is in_progress but no active block is running", reasons)
        self.assertEqual(actions, ["resume_next_block", "or_reconcile_br"])

    def test_open_but_satisfied_when_open_has_only_terminal_steps(self) -> None:
        classification, _, actions = self.module.classify_state(
            issue_status="open",
            steps=[{"status": "done"}],
            boot_receipt_ok=False,
            verify_ok=True,
            run_graph={},
        )
        self.assertEqual(classification, "open_but_satisfied")
        self.assertEqual(actions, ["close_now_if_scope_satisfied", "or_mark_in_progress_before_resume"])

    def test_drift_detected_when_terminal_done_exists_but_todo_backlog_remains(self) -> None:
        classification, reasons, actions = self.module.classify_state(
            issue_status="in_progress",
            steps=[
                {"status": "done", "next_step": "-"},
                {"status": "todo"},
            ],
            boot_receipt_ok=True,
            verify_ok=True,
            run_graph={},
        )
        self.assertEqual(classification, "drift_detected")
        self.assertIn("terminal done block exists but TODO backlog still remains", reasons)
        self.assertEqual(actions, ["reconcile_todo_then_close_or_manual_review"])

    def test_open_with_planned_backlog_stays_open(self) -> None:
        classification, reasons, actions = self.module.classify_state(
            issue_status="open",
            steps=[
                {"status": "done", "next_step": "P02"},
                {"status": "todo"},
            ],
            boot_receipt_ok=True,
            verify_ok=False,
            run_graph={},
        )
        self.assertEqual(classification, "open")
        self.assertEqual(reasons, [])
        self.assertEqual(actions, ["continue"])

    def test_invalid_state_when_closed_has_active_backlog(self) -> None:
        classification, reasons, actions = self.module.classify_state(
            issue_status="closed",
            steps=[{"status": "todo"}],
            boot_receipt_ok=True,
            verify_ok=True,
            run_graph={},
        )
        self.assertEqual(classification, "invalid_state")
        self.assertIn("closed task still has active TODO backlog", reasons)
        self.assertEqual(actions, ["manual_review"])


if __name__ == "__main__":
    unittest.main()
