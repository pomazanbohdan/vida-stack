import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
WORKFLOW_SCRIPT = ROOT_DIR / "_vida" / "scripts" / "beads-workflow.sh"
LOG_SCRIPT = ROOT_DIR / "_vida" / "scripts" / "beads-log.sh"
VIDA_NIM_SRC = ROOT_DIR / "_vida" / "scripts-nim" / "src" / "vida.nim"


class BeadsRuntimeTest(unittest.TestCase):
    def _compile_vida_legacy(self, out_dir: Path) -> Path:
        binary = out_dir / "vida-legacy"
        subprocess.run(
            [
                "nim",
                "c",
                f"--nimcache:{out_dir / 'nimcache'}",
                f"-o:{binary}",
                str(VIDA_NIM_SRC),
            ],
            cwd=ROOT_DIR,
            check=True,
            capture_output=True,
            text=True,
        )
        return binary

    def test_context_capsule_bootstrap_wiring_enables_missing_capsule_probe(self) -> None:
        workflow_text = WORKFLOW_SCRIPT.read_text(encoding="utf-8")
        self.assertIn(
            'VIDA_CONTEXT_HYDRATE_ALLOW_MISSING=1 vida_legacy_context_capsule hydrate "$issue_id" --json',
            workflow_text,
        )
        self.assertIn(
            'vida_legacy_context_capsule hydrate "$issue_id" --json',
            workflow_text,
        )

    def test_beads_workflow_start_updates_status_via_vida_legacy_bridge(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            root = Path(tmp_dir)
            vida_legacy = self._compile_vida_legacy(root)
            (root / ".beads").mkdir()
            (root / ".vida" / "state").mkdir(parents=True)
            (root / ".vida" / "logs").mkdir(parents=True)
            (root / ".vida" / "locks").mkdir(parents=True)

            env = {
                "VIDA_ROOT": str(root),
                "VIDA_LEGACY_TURSO_PYTHON": str(ROOT_DIR / ".venv" / "bin" / "python3"),
            }
            subprocess.run(
                [
                    str(vida_legacy),
                    "task",
                    "create",
                    "unit-task",
                    "Unit Task",
                    "--json",
                ],
                cwd=ROOT_DIR,
                env=env,
                check=True,
                capture_output=True,
                text=True,
            )
            subprocess.run(
                [
                    "bash",
                    str(ROOT_DIR / "_vida" / "scripts" / "beads-workflow.sh"),
                    "start",
                    "unit-task",
                ],
                cwd=ROOT_DIR,
                env=env,
                check=True,
                capture_output=True,
                text=True,
            )
            completed = subprocess.run(
                [
                    str(vida_legacy),
                    "task",
                    "show",
                    "unit-task",
                    "--json",
                ],
                cwd=ROOT_DIR,
                env=env,
                check=True,
                capture_output=True,
                text=True,
            )
            payload = json.loads(completed.stdout)
            self.assertEqual(payload.get("status"), "in_progress")

    def test_context_capsule_missing_probe_bootstraps_minimal_capsule_when_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as tmp_dir:
            root = Path(tmp_dir)
            vida_legacy = self._compile_vida_legacy(root)
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
                        f'"{vida_legacy}" context-capsule hydrate "unit-task" --json; '
                    ),
                ],
                cwd=root,
                check=False,
                capture_output=True,
                text=True,
            )
            self.assertEqual(completed.returncode, 0)
            payload = json.loads(completed.stdout)
            self.assertEqual(payload["task_id"], "unit-task")
            self.assertEqual(payload["task_role_in_epic"], "runtime-bootstrap")
            self.assertEqual(payload["done"], "bootstrap")
            self.assertEqual(payload["next"], "planning")
            self.assertEqual(payload["constraints"], ["legacy-zero,vida-legacy-task-store"])
            self.assertEqual(completed.stderr, "")

    def test_quality_health_check_uses_dev_pack_execution_context_instead_of_receipt_or_block_id_heuristics(self) -> None:
        health_text = (ROOT_DIR / "_vida" / "scripts" / "quality-health-check.sh").read_text(encoding="utf-8")
        self.assertIn('task_has_open_pack() {', health_text)
        self.assertIn('if task_has_open_pack "$TASK_ID" "dev-pack"; then', health_text)
        self.assertIn('if [[ "$implementation_context_present" == "true" ]]; then', health_text)
        self.assertNotIn('implementation_route_receipt_path=".vida/logs/route-receipts/${TASK_ID}.implementation.route.json"', health_text)
        self.assertNotIn('(.block_id // "") == "P02"', health_text)
        self.assertNotIn('(.block_id // "") == "IEP04"', health_text)
        self.assertNotIn('(.block_id // "") == "CL4"', health_text)

    def test_beads_log_preserves_terminal_next_step_marker(self) -> None:
        log_text = LOG_SCRIPT.read_text(encoding="utf-8")
        self.assertIn('normalize_next_step() {', log_text)
        self.assertIn('next_step="$(normalize_next_step "${8:--}")"', log_text)
        self.assertIn('next_step="$(normalize_next_step "${6:--}")"', log_text)

    def test_workflow_finish_distinguishes_empty_review_payloads_from_completed_review(self) -> None:
        workflow_text = WORKFLOW_SCRIPT.read_text(encoding="utf-8")
        self.assertIn('subagent_review_completed', workflow_text)
        self.assertIn('subagent_review_empty', workflow_text)
        self.assertIn('review_processed="$(jq -r \'.subagent_runs_processed // 0\' "$review_path"', workflow_text)


if __name__ == "__main__":
    unittest.main()
