import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "todo-runtime.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


class TodoRuntimeTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("todo_runtime_test", SCRIPT_PATH)

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        tmp = Path(self.temp_dir.name)
        self.module.LOG_FILE = tmp / "beads-execution.jsonl"
        self.module.TODO_INDEX_DIR = tmp / "todo-index"
        self.module.TODO_LOG_DIR = tmp / "logs"
        self.module.TODO_SYNC_STATE_DIR = tmp / "todo-sync-state"
        self.module.LOG_FILE.write_text("", encoding="utf-8")

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_partial_block_is_not_projected_as_todo(self) -> None:
        with self.module.LOG_FILE.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps({"task_id": "unit-task", "type": "block_start", "block_id": "B01", "goal": "Slice 1", "ts": "2026-03-07T10:00:00Z"}) + "\n")
            handle.write(json.dumps({"task_id": "unit-task", "type": "block_end", "block_id": "B01", "result": "partial", "next_step": "B02", "actions": "continue", "evidence_ref": "proof", "merge_ready": "", "ts_end": "2026-03-07T10:10:00Z"}) + "\n")

        steps = self.module.steps_json("unit-task")

        self.assertEqual(steps[0]["status"], "partial")

    def test_redirected_ancestor_marks_unstarted_descendants_as_superseded(self) -> None:
        with self.module.LOG_FILE.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps({"task_id": "unit-task", "type": "block_plan", "block_id": "FT09", "goal": "Old branch", "next_step": "FT10"}) + "\n")
            handle.write(json.dumps({"task_id": "unit-task", "type": "block_end", "block_id": "FT09", "result": "redirected", "next_step": "FT12", "actions": "redirected", "evidence_ref": "proof", "ts_end": "2026-03-07T10:10:00Z"}) + "\n")
            handle.write(json.dumps({"task_id": "unit-task", "type": "block_plan", "block_id": "FT10", "goal": "Unreached child", "depends_on": "FT09", "next_step": "FT11"}) + "\n")
            handle.write(json.dumps({"task_id": "unit-task", "type": "block_plan", "block_id": "FT11", "goal": "Unreached grandchild", "depends_on": "FT10"}) + "\n")

        steps = {item["block_id"]: item for item in self.module.steps_json("unit-task")}

        self.assertEqual(steps["FT09"]["status"], "superseded")
        self.assertEqual(steps["FT10"]["status"], "superseded")
        self.assertEqual(steps["FT11"]["status"], "superseded")

    def test_log_signature_changes_when_runtime_file_mtime_changes(self) -> None:
        first = self.module.log_signature()
        original_file = self.module.__file__
        try:
            self.module.__file__ = str(Path(self.temp_dir.name) / "todo-runtime-alt.py")
            Path(self.module.__file__).write_text("# alt runtime\n", encoding="utf-8")
            second = self.module.log_signature()
        finally:
            self.module.__file__ = original_file

        self.assertNotEqual(first, second)


if __name__ == "__main__":
    unittest.main()
