import unittest
import importlib.util
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parents[2]
GATE_PATH = ROOT_DIR / "_vida" / "scripts" / "worker-packet-gate.py"

def load_gate():
    spec = importlib.util.spec_from_file_location("worker_packet_gate", GATE_PATH)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

class TestReproImpactBug(unittest.TestCase):
    def test_stc_omitting_impact_analysis_fails_gate(self):
        gate = load_gate()
        prompt = "Return the machine-readable summary below."
        # Output omits impact_analysis because worker used STC
        output = """
```json
{
  "status": "done",
  "question_answered": "yes",
  "answer": "fixed bug",
  "evidence_refs": [],
  "changed_files": ["file.py"],
  "verification_commands": [],
  "verification_results": [],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "",
  "recommended_next_action": ""
}
```
"""
        errors = gate.validate_output_text(prompt, output)
        print(f"Errors found: {errors}")
        self.assertIn("machine-readable output missing key: impact_analysis", errors)

if __name__ == "__main__":
    unittest.main()
