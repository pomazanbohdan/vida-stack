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
        self.module.ROUTE_RECEIPT_DIR = tmp_path / "route-receipts"
        self.module.ROUTE_RECEIPT_DIR.mkdir(parents=True, exist_ok=True)

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
        (self.module.ROUTE_RECEIPT_DIR / "mobile-2wy.1.analysis.approval.json").write_text(
            json.dumps({"decision": "approved", "review_state": "policy_gate_required"}),
            encoding="utf-8",
        )

        payload = self.module.build_status_payload()

        self.assertEqual(payload["framework_memory"]["anomaly_count"], 3)
        self.assertEqual(payload["approval_summary"]["approved_count"], 1)


if __name__ == "__main__":
    unittest.main()
