import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "framework-task-sync.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class FrameworkTaskSyncTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("framework_task_sync_test", SCRIPT_PATH)

    def test_sync_manifest_closes_open_task(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            manifest_path = Path(tmpdir) / "sync.json"
            manifest_path.write_text(
                json.dumps(
                    {
                        "wave_id": "mobile-1hv",
                        "tasks": [
                            {
                                "task_id": "mobile-1hv.5",
                                "status": "closed",
                                "reason": "done",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            with mock.patch.object(self.module, "run_task_read", return_value={"id": "mobile-1hv.5", "status": "open"}), \
                mock.patch.object(self.module, "run_task_mutation", return_value={"id": "mobile-1hv.5", "status": "closed"}) as mocked_mutation:
                result = self.module.sync_manifest(manifest_path)

        mocked_mutation.assert_called_once_with(["close", "mobile-1hv.5", "--reason", "done"])
        self.assertEqual(result["results"][0]["status"], "closed")


if __name__ == "__main__":
    unittest.main()
