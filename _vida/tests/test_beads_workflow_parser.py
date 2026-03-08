import json
import subprocess
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT = ROOT_DIR / "_vida" / "scripts" / "beads-workflow.sh"


class BeadsWorkflowParserTest(unittest.TestCase):
    def test_block_end_named_tail_parser_maps_fields_without_shift(self) -> None:
        completed = subprocess.run(
            [
                "bash",
                str(SCRIPT),
                "parse-block-tail",
                "block-end",
                "--artifacts",
                "a.md,b.md",
                "--risks",
                "low",
                "--assumptions",
                "none",
                "--evidence-ref",
                "proof.txt",
                "--track-id",
                "main",
                "--owner",
                "orchestrator",
                "--merge-ready",
                "yes",
            ],
            cwd=ROOT_DIR,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(
            json.loads(completed.stdout),
            {
                "mode": "block-end",
                "artifacts": "a.md,b.md",
                "risks": "low",
                "assumptions": "none",
                "evidence_ref": "proof.txt",
                "confidence": "",
                "track_id": "main",
                "owner": "orchestrator",
                "merge_ready": "yes",
            },
        )

    def test_block_finish_positional_tail_still_supported(self) -> None:
        completed = subprocess.run(
            [
                "bash",
                str(SCRIPT),
                "parse-block-tail",
                "block-finish",
                "art",
                "risk",
                "assume",
                "proof",
                "91",
                "main",
                "orchestrator",
                "yes",
            ],
            cwd=ROOT_DIR,
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(
            json.loads(completed.stdout),
            {
                "mode": "block-finish",
                "artifacts": "art",
                "risks": "risk",
                "assumptions": "assume",
                "evidence_ref": "proof",
                "confidence": "91",
                "track_id": "main",
                "owner": "orchestrator",
                "merge_ready": "yes",
            },
        )


if __name__ == "__main__":
    unittest.main()
