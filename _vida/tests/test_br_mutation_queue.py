import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
QUEUE_PATH = ROOT_DIR / "_vida" / "scripts" / "br-mutation-queue.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class BrMutationQueueTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.queue = load_module("br_mutation_queue_test", QUEUE_PATH)

    def test_backend_command_builds_expected_runner(self) -> None:
        command = self.queue.backend_command("mutator", ["update", "mobile-1", "--status", "in_progress"])
        self.assertEqual(command[0], self.queue.sys.executable)
        self.assertEqual(command[1], str(self.queue.JSONL_MUTATOR))
        self.assertEqual(command[2:], ["update", "mobile-1", "--status", "in_progress"])

    def test_run_request_writes_queue_journal(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            beads_dir = tmp_path / ".beads"
            queue_log = beads_dir / "mutation-queue.jsonl"
            queue_lock = beads_dir / "mutation-queue.lock"
            fake_runner = tmp_path / "fake-runner.sh"
            fake_runner.write_text("#!/usr/bin/env bash\nprintf 'OK:%s\\n' \"$1\"\n", encoding="utf-8")
            fake_runner.chmod(0o755)

            original_root = self.queue.ROOT
            original_beads_dir = self.queue.BEADS_DIR
            original_queue_log = self.queue.QUEUE_LOG
            original_queue_lock = self.queue.QUEUE_LOCK
            original_br_safe = self.queue.BR_SAFE
            try:
                self.queue.ROOT = tmp_path
                self.queue.BEADS_DIR = beads_dir
                self.queue.QUEUE_LOG = queue_log
                self.queue.QUEUE_LOCK = queue_lock
                self.queue.BR_SAFE = fake_runner

                rc = self.queue.run_request("br", ["show"])
                self.assertEqual(rc, 0)

                entries = [
                    json.loads(line)
                    for line in queue_log.read_text(encoding="utf-8").splitlines()
                    if line.strip()
                ]
                self.assertEqual([item["event"] for item in entries], ["queued", "completed"])
                self.assertEqual(entries[0]["backend"], "br")
                self.assertEqual(entries[0]["args"], ["show"])
                self.assertEqual(entries[1]["return_code"], 0)
            finally:
                self.queue.ROOT = original_root
                self.queue.BEADS_DIR = original_beads_dir
                self.queue.QUEUE_LOG = original_queue_log
                self.queue.QUEUE_LOCK = original_queue_lock
                self.queue.BR_SAFE = original_br_safe


if __name__ == "__main__":
    unittest.main()
