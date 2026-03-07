import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT_DIR / "_vida" / "scripts" / "problem-party.py"


def load_module(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class ProblemPartyTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_module("problem_party_test", SCRIPT_PATH)

    def test_render_board_uses_small_preset(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            out = self.module.render_board(
                "mobile-1hv.13",
                "framework conflict",
                board="small",
                rounds=None,
                problem_payload={
                    "problem_frame": "Need bounded discussion for protocol conflicts",
                    "constraints": ["quality", "token efficiency"],
                },
                output_dir=Path(tmpdir),
            )
            payload = json.loads(out.read_text(encoding="utf-8"))

        self.assertEqual(payload["board_size"], "small")
        self.assertEqual(payload["round_count"], 1)
        self.assertEqual(len(payload["roles"]), 4)
        self.assertEqual(payload["problem_frame"], "Need bounded discussion for protocol conflicts")

    def test_synthesize_board_writes_decision_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            manifest_path = tmp_path / "board.json"
            notes_path = tmp_path / "notes.json"
            manifest_path.write_text(
                json.dumps(
                    {
                        "task_id": "mobile-1hv.13",
                        "topic": "problem-party",
                        "board_size": "large",
                        "round_count": 2,
                        "roles": [{"role": "architect"}, {"role": "runtime_systems"}],
                        "problem_frame": "Need a bounded escalation protocol",
                        "constraints": ["quality", "token efficiency"],
                        "budget_summary": {"board_size": "large"},
                    }
                ),
                encoding="utf-8",
            )
            notes_path.write_text(
                json.dumps(
                    [
                        {
                            "role": "architect",
                            "options": ["small board first", "always large board"],
                            "conflict_points": ["large board is expensive"],
                            "decision": "small board first",
                            "why_not_others": ["always large board wastes tokens"],
                            "next_execution_step": "implement protocol helper",
                        },
                        {
                            "role": "runtime_systems",
                            "options": ["small board first"],
                            "conflict_points": ["must persist artifact"],
                            "decision": "small board first",
                            "why_not_others": ["free-form chat leaves no artifact"],
                            "next_execution_step": "implement protocol helper",
                        },
                    ]
                ),
                encoding="utf-8",
            )

            output_path = self.module.synthesize_board(manifest_path, notes_path, None)
            payload = json.loads(output_path.read_text(encoding="utf-8"))

        self.assertEqual(payload["decision"], "small board first")
        self.assertIn("large board is expensive", payload["conflict_points"])
        self.assertEqual(payload["next_execution_step"], "implement protocol helper")

    def test_write_decision_receipt_writes_route_visible_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp_path = Path(tmpdir)
            self.module.ROUTE_RECEIPT_DIR = tmp_path / "route-receipts"
            decision_path = tmp_path / "decision.json"
            decision_path.write_text(
                json.dumps(
                    {
                        "task_id": "mobile-2wy.4",
                        "topic": "problem-party",
                        "decision": "small board first",
                        "next_execution_step": "record bounded receipt",
                    }
                ),
                encoding="utf-8",
            )

            receipt_path = self.module.write_decision_receipt(
                task_id="mobile-2wy.4",
                task_class="architecture",
                topic="problem-party",
                decision_artifact_path=decision_path,
            )
            payload = json.loads(receipt_path.read_text(encoding="utf-8"))

        self.assertEqual(payload["task_class"], "architecture")
        self.assertEqual(payload["decision"], "small board first")
        self.assertEqual(payload["status"], "problem_party_ready")


if __name__ == "__main__":
    unittest.main()
