import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT_DIR / "_vida" / "scripts" / "fsap-verification-gate.py"


def load_gate_module():
    spec = importlib.util.spec_from_file_location("fsap_verification_gate", MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class FsapVerificationGateTest(unittest.TestCase):
    def setUp(self) -> None:
        self.module = load_gate_module()
        self.temp_dir = tempfile.TemporaryDirectory()
        self.root = Path(self.temp_dir.name)
        self.module.LOG_FILE = self.root / "beads-execution.jsonl"
        self.module.REVIEW_DIR = self.root
        self.module.RECEIPT_DIR = self.root / "fsap-verification"
        self.task_id = "mobile-fsap-test"

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def _write_logs(self, events: list[dict]) -> None:
        payload = "\n".join(json.dumps(item) for item in events)
        self.module.LOG_FILE.write_text(f"{payload}\n" if payload else "")

    def _tracked_fsap_events(self) -> list[dict]:
        return [
            {"task_id": self.task_id, "type": "pack_start", "pack_id": "reflection-pack"},
            {"task_id": self.task_id, "type": "block_plan", "block_id": "FSAP01"},
        ]

    def _write_review(self, payload: dict) -> None:
        (self.module.REVIEW_DIR / f"subagent-review-{self.task_id}.json").write_text(json.dumps(payload))

    def test_non_fsap_task_does_not_require_gate(self) -> None:
        self._write_logs([{"task_id": self.task_id, "type": "pack_start", "pack_id": "dev-pack"}])

        exit_code, payload = self.module.check_gate(self.task_id)

        self.assertEqual(exit_code, 0)
        self.assertEqual(payload["status"], "not_required")

    def test_tracked_fsap_without_review_or_override_is_blocked(self) -> None:
        self._write_logs(self._tracked_fsap_events())

        exit_code, payload = self.module.check_gate(self.task_id)

        self.assertEqual(exit_code, 2)
        self.assertIn("missing_delegated_fsap_verification", payload["blockers"])

    def test_review_file_with_zero_runs_is_not_sufficient(self) -> None:
        self._write_logs(self._tracked_fsap_events())
        self._write_review({"subagent_runs_seen": 0, "subagent_runs_processed": 0})

        exit_code, payload = self.module.check_gate(self.task_id)

        self.assertEqual(exit_code, 2)
        self.assertIn("missing_delegated_fsap_verification", payload["blockers"])

    def test_review_file_without_processed_runs_is_not_sufficient(self) -> None:
        self._write_logs(self._tracked_fsap_events())
        self._write_review({"subagent_runs_seen": 2, "subagent_runs_processed": 0})

        exit_code, payload = self.module.check_gate(self.task_id)

        self.assertEqual(exit_code, 2)
        self.assertIn("missing_delegated_fsap_verification", payload["blockers"])

    def test_review_file_with_subagent_runs_satisfies_gate(self) -> None:
        self._write_logs(self._tracked_fsap_events())
        self._write_review({"subagent_runs_seen": 2, "subagent_runs_processed": 1})

        exit_code, payload = self.module.check_gate(self.task_id)

        self.assertEqual(exit_code, 0)
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["authorized_via"], "delegated_review")

    def test_structured_override_receipt_satisfies_gate(self) -> None:
        self._write_logs(self._tracked_fsap_events())
        self.module.write_override_receipt(
            self.task_id,
            "no_available_verifier",
            "all delegated verification lanes were unavailable",
            evidence="probe failures",
            actor="codex",
        )

        exit_code, payload = self.module.check_gate(self.task_id)

        self.assertEqual(exit_code, 0)
        self.assertEqual(payload["status"], "ok")
        self.assertEqual(payload["authorized_via"], "structured_override")


if __name__ == "__main__":
    unittest.main()
