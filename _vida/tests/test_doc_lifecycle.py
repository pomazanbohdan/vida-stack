import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "doc-lifecycle.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class DocLifecycleTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("doc_lifecycle_test", SCRIPT_PATH)

    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.module.STATE_PATH = Path(self.temp_dir.name) / "doc-lifecycle.json"

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_record_document_state(self) -> None:
        entry = self.module.record_doc_state(
            path="_vida/docs/protocol-index.md",
            state="current",
            owner="framework",
            notes="Canonical map kept current",
        )
        payload = json.loads(self.module.STATE_PATH.read_text(encoding="utf-8"))

        self.assertEqual(entry["state"], "current")
        self.assertEqual(payload["entries"]["_vida/docs/protocol-index.md"]["owner"], "framework")

    def test_validate_marks_stale_document(self) -> None:
        self.module.record_doc_state(
            path="_vida/docs/legacy.md",
            state="current",
            owner="framework",
            notes="Old doc",
        )
        payload = json.loads(self.module.STATE_PATH.read_text(encoding="utf-8"))
        payload["entries"]["_vida/docs/legacy.md"]["last_reviewed_at"] = "2025-01-01T00:00:00Z"
        self.module.STATE_PATH.write_text(json.dumps(payload), encoding="utf-8")

        result = self.module.validate_doc_state("_vida/docs/legacy.md", max_age_days=30)

        self.assertFalse(result["valid"])
        self.assertEqual(result["reason"], "stale_document")


if __name__ == "__main__":
    unittest.main()
