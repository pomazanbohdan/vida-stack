import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT_DIR = Path(__file__).resolve().parents[2]
MODULE_PATH = ROOT_DIR / "_vida" / "scripts" / "worker-packet-gate.py"


def load_gate_module():
    spec = importlib.util.spec_from_file_location("worker_packet_gate", MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class WorkerPacketGateTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = load_gate_module()

    def test_missing_worker_markers_fail_validation(self) -> None:
        text = """Task: review something\nScope: app/core\nVerification:\n- pytest\nDeliverable:\n- findings\n"""

        errors = self.module.validate_packet_text(text)

        self.assertIn("missing worker_lane_confirmed marker", errors)
        self.assertIn("missing worker_role marker", errors)

    def test_machine_readable_contract_requires_merge_ready_and_verification_fields(self) -> None:
        text = """
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Task: something
Scope: app/core
Blocking Question: what changed?
Verification:
- pytest
Deliverable:
- Return the machine-readable summary below.
```json
{
  "status": "done",
  "question_answered": "yes"
}
```
"""

        errors = self.module.validate_packet_text(text)

        self.assertIn("machine-readable contract missing key: merge_ready", errors)
        self.assertIn("machine-readable contract missing key: verification_results", errors)

    def test_output_contract_validation_requires_machine_readable_keys_when_prompt_demands_it(self) -> None:
        prompt = """
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Task: implement
Scope: _vida/scripts
Blocking Question: what changed?
Verification:
- python3 -m unittest
Deliverable:
- Return the machine-readable summary below.
```json
{
  "status": "done",
  "question_answered": "yes",
  "answer": "x",
  "evidence_refs": [],
  "changed_files": [],
  "verification_commands": [],
  "verification_results": [],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "",
  "impact_analysis": {
    "affected_scope": [],
    "contract_impact": [],
    "follow_up_actions": [],
    "residual_risks": []
  }
}
```
"""
        invalid_output = '{"status":"done","question_answered":"yes"}'
        valid_output = """
        {
          "status": "done",
          "question_answered": "yes",
          "answer": "implemented validator",
          "evidence_refs": ["_vida/scripts/file.py:10"],
          "changed_files": ["_vida/scripts/file.py"],
          "verification_commands": ["python3 -m unittest"],
          "verification_results": ["python3 -m unittest -> pass"],
          "merge_ready": "yes",
          "blockers": [],
          "notes": "",
          "recommended_next_action": "integrate",
          "impact_analysis": {
            "affected_scope": ["_vida/scripts/file.py"],
            "contract_impact": ["worker packet gate"],
            "follow_up_actions": [],
            "residual_risks": []
          }
        }
        """

        invalid_errors = self.module.validate_output_text(prompt, invalid_output)
        valid_errors = self.module.validate_output_text(prompt, valid_output)

        self.assertIn("machine-readable output missing key: merge_ready", invalid_errors)
        self.assertEqual(valid_errors, [])

    def test_output_contract_validation_prefers_last_json_block(self) -> None:
        prompt = """
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Task: implement
Scope: _vida/scripts
Blocking Question: what changed?
Verification:
- python3 -m unittest
Deliverable:
- Return the machine-readable summary below.
```json
{
  "status": "done",
  "question_answered": "yes",
  "answer": "x",
  "evidence_refs": [],
  "changed_files": [],
  "verification_commands": [],
  "verification_results": [],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "",
  "impact_analysis": {
    "affected_scope": [],
    "contract_impact": [],
    "follow_up_actions": [],
    "residual_risks": []
  }
}
```
"""
        output = """
Some draft JSON:
```json
{"status":"done","question_answered":"yes"}
```

Final answer:
```json
{
  "status": "done",
  "question_answered": "yes",
  "answer": "implemented validator",
  "evidence_refs": ["_vida/scripts/file.py:10"],
  "changed_files": ["_vida/scripts/file.py"],
  "verification_commands": ["python3 -m unittest"],
  "verification_results": ["python3 -m unittest -> pass"],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "integrate",
  "impact_analysis": {
    "affected_scope": ["_vida/scripts/file.py"],
    "contract_impact": ["worker packet gate"],
    "follow_up_actions": [],
    "residual_risks": []
  }
}
```
"""

        self.assertEqual(self.module.validate_output_text(prompt, output), [])

    def test_extract_json_payload_accepts_balanced_nested_json_after_prose(self) -> None:
        output = """
Draft notes before the final payload.

{
  "status": "done",
  "question_answered": "yes",
  "answer": "implemented validator",
  "evidence_refs": ["_vida/scripts/file.py:10"],
  "changed_files": ["_vida/scripts/file.py"],
  "verification_commands": ["python3 -m unittest"],
  "verification_results": ["python3 -m unittest -> pass"],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "integrate",
  "impact_analysis": {
    "affected_scope": ["_vida/scripts/file.py"],
    "contract_impact": ["worker packet gate"],
    "follow_up_actions": ["rerun the coach gate"],
    "residual_risks": []
  }
}

Trailing text after the payload.
"""

        payload = self.module.extract_json_payload(output)

        self.assertIsNotNone(payload)
        assert payload is not None
        self.assertEqual(payload["answer"], "implemented validator")
        self.assertEqual(payload["impact_analysis"]["follow_up_actions"], ["rerun the coach gate"])

    def test_packet_validation_requires_real_section_headers(self) -> None:
        text = """
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Task: mentions Scope: inline only
Blocking Question: what changed?
Notes: Verification: is mentioned inline too.
Deliverable text without header mention.
"""

        errors = self.module.validate_packet_text(text)

        self.assertIn("missing Scope section", errors)
        self.assertIn("missing Verification section", errors)
        self.assertIn("missing Deliverable section", errors)

    def test_output_contract_validation_rejects_wrong_field_types(self) -> None:
        prompt = """
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- worker_entry: _vida/docs/SUBAGENT-ENTRY.MD
- worker_thinking: _vida/docs/SUBAGENT-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Task: implement
Scope: _vida/scripts
Blocking Question: what changed?
Verification:
- python3 -m unittest
Deliverable:
- Return the machine-readable summary below.
```json
{
  "status": "done",
  "question_answered": "yes",
  "answer": "x",
  "evidence_refs": [],
  "changed_files": [],
  "verification_commands": [],
  "verification_results": [],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "",
  "impact_analysis": {
    "affected_scope": [],
    "contract_impact": [],
    "follow_up_actions": [],
    "residual_risks": []
  }
}
```
"""
        invalid_output = """
{
  "status": "done",
  "question_answered": "yes",
  "answer": ["not-a-string"],
  "evidence_refs": "not-a-list",
  "changed_files": [1, 2],
  "verification_commands": "not-a-list",
  "verification_results": [true],
  "merge_ready": "yes",
  "blockers": "not-a-list",
  "notes": [],
  "recommended_next_action": {},
  "impact_analysis": {
    "affected_scope": "not-a-list",
    "contract_impact": [],
    "follow_up_actions": [1],
    "residual_risks": []
  }
}
"""

        errors = self.module.validate_output_text(prompt, invalid_output)

        self.assertIn("machine-readable output answer must be a string", errors)
        self.assertIn("machine-readable output evidence_refs must be a list of strings", errors)
        self.assertIn("machine-readable output impact_analysis affected_scope must be a list of strings", errors)


if __name__ == "__main__":
    unittest.main()
