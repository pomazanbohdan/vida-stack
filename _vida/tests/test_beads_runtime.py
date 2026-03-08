import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT = ROOT_DIR / "_vida" / "scripts" / "beads-runtime.sh"
WORKFLOW_SCRIPT = ROOT_DIR / "_vida" / "scripts" / "beads-workflow.sh"


class BeadsRuntimeTest(unittest.TestCase):
    def test_sourced_runtime_resolves_mutation_queue_inside_vida_scripts(self) -> None:
        completed = subprocess.run(
            [
                "bash",
                "-lc",
                f'source "{SCRIPT}"; printf "%s\\n" "$BEADS_RUNTIME_MUTATION_QUEUE"',
            ],
            cwd=ROOT_DIR,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(
            completed.stdout.strip(),
            str(ROOT_DIR / "_vida" / "scripts" / "br-mutation-queue.py"),
        )

    def test_sourced_runtime_beads_mutate_uses_valid_queue_path(self) -> None:
        completed = subprocess.run(
            [
                "bash",
                "-lc",
                f'source "{SCRIPT}"; beads_mutate --help >/dev/null 2>&1; echo $?',
            ],
            cwd=ROOT_DIR,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(completed.stdout.strip(), "0")

    def test_context_capsule_bootstrap_wiring_enables_missing_capsule_probe(self) -> None:
        workflow_text = WORKFLOW_SCRIPT.read_text(encoding="utf-8")
        self.assertIn(
            'VIDA_CONTEXT_HYDRATE_ALLOW_MISSING=1 \\',
            workflow_text,
        )
        self.assertIn(
            'bash "$CONTEXT_CAPSULE_SCRIPT" hydrate "$issue_id"',
            workflow_text,
        )

    def test_context_capsule_missing_probe_logs_pending_when_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            root = Path(tmp_dir)
            (root / ".beads").mkdir()
            (root / ".vida" / "logs").mkdir(parents=True)
            (root / ".vida" / "locks").mkdir(parents=True)
            (root / ".vida" / "state").mkdir(parents=True)
            (root / ".vida" / "logs" / "context-capsules").mkdir(parents=True)
            (root / ".beads" / "issues.jsonl").write_text("", encoding="utf-8")

            completed = subprocess.run(
                [
                    "bash",
                    "-lc",
                    (
                        'VIDA_CONTEXT_HYDRATE_ALLOW_MISSING=1 '
                        f'bash "{ROOT_DIR / "_vida" / "scripts" / "context-capsule.sh"}" hydrate "unit-task"; '
                        'python3 - <<\'PY\'\n'
                        'from pathlib import Path\n'
                        'import json\n'
                        'log_path = Path(".vida/logs/beads-execution.jsonl")\n'
                        'events = []\n'
                        'if log_path.exists():\n'
                        '    for line in log_path.read_text(encoding="utf-8").splitlines():\n'
                        '        if not line.strip():\n'
                        '            continue\n'
                        '        obj = json.loads(line)\n'
                        '        if obj.get("issue_id") == "unit-task":\n'
                        '            events.append(obj.get("event"))\n'
                        'print("\\n".join(events))\n'
                        'PY'
                    ),
                ],
                cwd=root,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertIn("CONTEXT_HYDRATION_PENDING", completed.stderr)
            self.assertNotIn("BLK_CONTEXT_NOT_HYDRATED", completed.stderr)

    def test_quality_health_check_uses_dev_pack_execution_context_instead_of_receipt_or_block_id_heuristics(self) -> None:
        health_text = (ROOT_DIR / "_vida" / "scripts" / "quality-health-check.sh").read_text(encoding="utf-8")
        self.assertIn('task_has_open_pack() {', health_text)
        self.assertIn('if task_has_open_pack "$TASK_ID" "dev-pack"; then', health_text)
        self.assertIn('if [[ "$implementation_context_present" == "true" ]]; then', health_text)
        self.assertNotIn('implementation_route_receipt_path=".vida/logs/route-receipts/${TASK_ID}.implementation.route.json"', health_text)
        self.assertNotIn('(.block_id // "") == "P02"', health_text)
        self.assertNotIn('(.block_id // "") == "IEP04"', health_text)
        self.assertNotIn('(.block_id // "") == "CL4"', health_text)


if __name__ == "__main__":
    unittest.main()
