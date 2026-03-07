import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
MEMORY_PATH = ROOT_DIR / "_vida" / "scripts" / "framework-memory.py"
SILENT_PATH = ROOT_DIR / "_vida" / "scripts" / "vida-silent-diagnosis.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class FrameworkMemoryTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.memory = load_module("framework_memory_test", MEMORY_PATH)
        cls.silent = load_module("vida_silent_diag_memory_test", SILENT_PATH)

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.memory.STATE_PATH = Path(self.temp_dir.name) / "framework-memory.json"
        self.silent.STATE_PATH = Path(self.temp_dir.name) / "silent-diagnosis.json"
        self.silent.FRAMEWORK_MEMORY_MODULE = self.memory

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_record_entry_groups_by_kind(self) -> None:
        entry = self.memory.record_entry(
            kind="lesson",
            summary="Approval gate prevented unsafe closure",
            source_task="mobile-2wy.1",
            details={"pattern": "fail_closed"},
        )
        payload = json.loads(self.memory.STATE_PATH.read_text(encoding="utf-8"))

        self.assertEqual(entry["kind"], "lesson")
        self.assertEqual(payload["summary"]["lesson_count"], 1)
        self.assertEqual(payload["entries"][0]["source_task"], "mobile-2wy.1")

    def test_session_reflection_records_anomalies_in_memory(self) -> None:
        reflection = self.silent.record_session_reflection(
            "mobile-2wy.2",
            ["architecture_cleanliness"],
            ["operator surface missing correction history"],
        )
        memory_payload = json.loads(self.memory.STATE_PATH.read_text(encoding="utf-8"))

        self.assertEqual(reflection["current_task"], "mobile-2wy.2")
        self.assertEqual(memory_payload["summary"]["anomaly_count"], 1)
        self.assertEqual(memory_payload["entries"][0]["source_task"], "mobile-2wy.2")
        self.assertEqual(memory_payload["entries"][0]["kind"], "anomaly")


if __name__ == "__main__":
    unittest.main()
