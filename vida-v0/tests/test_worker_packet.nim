## Tests for gates/worker_packet module

import std/[json, unittest]
import ../src/gates/worker_packet

const ValidPrompt = """
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- worker_entry: docs/framework/SUBAGENT-ENTRY.MD
- worker_thinking: docs/framework/SUBAGENT-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Task: implement
Scope: docs/framework/history/_vida-source/scripts
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

suite "worker packet":
  test "missing worker markers fail validation":
    let text = "Task: review something\nScope: app/core\nVerification:\n- pytest\nDeliverable:\n- findings\n"
    let errors = validatePacketText(text)
    check "missing worker_lane_confirmed marker" in errors
    check "missing worker_role marker" in errors

  test "machine readable contract requires merge ready and verification results":
    let text = """
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- worker_entry: docs/framework/SUBAGENT-ENTRY.MD
- worker_thinking: docs/framework/SUBAGENT-THINKING.MD
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
    let errors = validatePacketText(text)
    check "machine-readable contract missing key: merge_ready" in errors
    check "machine-readable contract missing key: verification_results" in errors

  test "valid machine readable output passes":
    let output = """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "implemented validator",
  "evidence_refs": ["docs/framework/history/_vida-source/scripts/file.py:10"],
  "changed_files": ["docs/framework/history/_vida-source/scripts/file.py"],
  "verification_commands": ["python3 -m unittest"],
  "verification_results": ["python3 -m unittest -> pass"],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "integrate",
  "impact_analysis": {
    "affected_scope": ["docs/framework/history/_vida-source/scripts/file.py"],
    "contract_impact": ["worker packet gate"],
    "follow_up_actions": [],
    "residual_risks": []
  }
}
"""
    check validateOutputText(ValidPrompt, output) == newSeq[string]()

  test "output validation rejects wrong field types":
    let invalidOutput = """
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
    let errors = validateOutputText(ValidPrompt, invalidOutput)
    check "machine-readable output answer must be a string" in errors
    check "machine-readable output evidence_refs must be a list of strings" in errors
    check "machine-readable output impact_analysis affected_scope must be a list of strings" in errors

  test "extract json payload accepts balanced nested json after prose":
    let output = """
Draft notes before the final payload.

{
  "status": "done",
  "question_answered": "yes",
  "answer": "implemented validator",
  "evidence_refs": ["docs/framework/history/_vida-source/scripts/file.py:10"],
  "changed_files": ["docs/framework/history/_vida-source/scripts/file.py"],
  "verification_commands": ["python3 -m unittest"],
  "verification_results": ["python3 -m unittest -> pass"],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "integrate",
  "impact_analysis": {
    "affected_scope": ["docs/framework/history/_vida-source/scripts/file.py"],
    "contract_impact": ["worker packet gate"],
    "follow_up_actions": ["rerun the coach gate"],
    "residual_risks": []
  }
}

Trailing text after the payload.
"""
    let payload = extractJsonPayload(output)
    check not payload.isNil
    check payload["answer"].getStr() == "implemented validator"
    check payload["impact_analysis"]["follow_up_actions"].len == 1

  test "packet validation requires real section headers":
    let text = """
Runtime Role Packet:
- worker_lane_confirmed: true
- worker_role: subagent
- worker_entry: docs/framework/SUBAGENT-ENTRY.MD
- worker_thinking: docs/framework/SUBAGENT-THINKING.MD
- impact_tail_policy: required_for_non_stc
- impact_analysis_scope: bounded_to_assigned_scope
Task: mentions Scope: inline only
Blocking Question: what changed?
Notes: Verification: is mentioned inline too.
Deliverable text without header mention.
"""
    let errors = validatePacketText(text)
    check "missing Scope section" in errors
    check "missing Verification section" in errors
    check "missing Deliverable section" in errors
