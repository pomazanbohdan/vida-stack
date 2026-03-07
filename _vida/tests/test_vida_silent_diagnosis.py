import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "vida-silent-diagnosis.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class VidaSilentDiagnosisTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("vida_silent_diagnosis_test", SCRIPT_PATH)

    def test_build_status_payload_reads_defaults(self) -> None:
        with mock.patch.object(self.module, "diagnosis_config", return_value={"enabled": True, "silent_mode": True}), \
             mock.patch.object(self.module, "load_state", return_value={"pending_framework_bugs": [], "session_reflections": []}):
            payload = self.module.build_status_payload()

        self.assertTrue(payload["enabled"])
        self.assertTrue(payload["silent_mode"])
        self.assertEqual(payload["pending_framework_bugs"], [])

    def test_capture_bug_creates_entry_and_dedupes_by_summary(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            state_path = tmp_path / "silent-framework-diagnosis.json"
            original_state_path = self.module.STATE_PATH
            self.module.STATE_PATH = state_path
            try:
                completed = mock.Mock(returncode=0, stdout=json.dumps({"id": "mobile-1hv.99"}), stderr="")
                with mock.patch.object(self.module.subprocess, "run", return_value=completed):
                    first = self.module.capture_bug(
                        summary="cheap lane stalled",
                        details="analysis lane returned only preamble",
                        current_task="mobile-1ic.2",
                        workaround="reroute to fallback lane",
                        parent_issue="mobile-1hv",
                    )
                    second = self.module.capture_bug(
                        summary="cheap lane stalled",
                        details="duplicate",
                        current_task="mobile-1ic.2",
                        workaround="duplicate workaround",
                        parent_issue="mobile-1hv",
                    )

                self.assertEqual(first["bug_id"], "mobile-1hv.99")
                self.assertEqual(second["bug_id"], "mobile-1hv.99")
                state = json.loads(state_path.read_text(encoding="utf-8"))
                self.assertEqual(len(state["pending_framework_bugs"]), 1)
            finally:
                self.module.STATE_PATH = original_state_path


if __name__ == "__main__":
    unittest.main()
