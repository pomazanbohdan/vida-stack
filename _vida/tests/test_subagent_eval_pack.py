import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "subagent-eval-pack.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class SubagentEvalPackTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("subagent_eval_pack_test", SCRIPT_PATH)

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        tmp = Path(self.temp_dir.name)
        self.module.LOG_DIR = tmp / "logs"
        self.module.STATE_DIR = tmp / "state"
        self.module.RUN_LOG_PATH = self.module.LOG_DIR / "subagent-runs.jsonl"
        self.module.PROCESSED_PATH = self.module.STATE_DIR / "subagent-eval-processed.json"
        self.module.STRATEGY_PATH = self.module.STATE_DIR / "subagent-strategy.json"
        self.module.LOG_DIR.mkdir(parents=True, exist_ok=True)
        self.module.STATE_DIR.mkdir(parents=True, exist_ok=True)

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_run_writes_trace_eval_and_dataset_summary(self) -> None:
        self.module.RUN_LOG_PATH.write_text("", encoding="utf-8")
        with mock.patch.object(self.module, "ensure_eval_pack", return_value={"task_status": "closed"}), \
             mock.patch.object(
                 self.module,
                 "ensure_trace_eval",
                 return_value={"overall_grade": "pass", "grades": {"route_correctness": {"grade": "pass"}}},
             ), \
             mock.patch.object(
                 self.module.trace_eval,
                 "build_trace_dataset",
                 return_value={"labels": {"overall_grade": "pass"}},
             ), \
             mock.patch.object(
                 self.module.trace_eval,
                 "save_json",
                 side_effect=lambda path, payload: Path(path).write_text(json.dumps(payload), encoding="utf-8") or Path(path),
             ), \
             mock.patch.object(self.module, "task_closed", return_value=True), \
             mock.patch.object(self.module, "refresh_strategy", return_value={"subagents": {}}):
            code = self.module.run("unit-task")

        self.assertEqual(code, 0)
        payload = json.loads((self.module.LOG_DIR / "subagent-review-unit-task.json").read_text(encoding="utf-8"))
        self.assertEqual(payload["trace_eval"]["overall_grade"], "pass")
        self.assertEqual(payload["trace_dataset"]["labels"]["overall_grade"], "pass")


if __name__ == "__main__":
    unittest.main()
