## Tests for gates/coach_decision module

import std/[json, strutils, unittest]
import ../src/gates/coach_decision

suite "coach decision":
  test "parse detects return for rework":
    let output = """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "implementation misses the close gate",
  "evidence_refs": ["_vida/scripts/quality-health-check.sh:280"],
  "changed_files": [],
  "verification_commands": ["bash _vida/scripts/quality-health-check.sh --mode quick unit-task"],
  "verification_results": ["coach review indicates missing gate wiring"],
  "merge_ready": "no",
  "blockers": ["coach gate missing from quality health check"],
  "notes": "return to writer",
  "recommended_next_action": "wire coach gate into close checks",
  "impact_analysis": {
    "affected_scope": ["_vida/scripts/quality-health-check.sh"],
    "contract_impact": ["close gate incomplete"],
    "follow_up_actions": ["rerun coach review after patch"],
    "residual_risks": ["writer may still bypass coach if not enforced"]
  },
  "coach_decision": "return_for_rework",
  "rework_required": "yes",
  "coach_feedback": "wire the mandatory coach gate before close"
}
"""
    let decision = parseCoachDecision(output)
    check decision["approved"].getBool() == false
    check decision["coach_decision"].getStr() == "return_for_rework"
    check decision["rework_required"].getStr() == "yes"

  test "parse fails closed without payload":
    let decision = parseCoachDecision("")
    check decision["approved"].getBool() == false
    check decision["coach_decision"].getStr() == "coach_failed"
    check decision["payload_state"].getStr() == "missing_payload"
    check decision["reason"].getStr() == "missing_coach_decision_payload"

  test "parse marks approved merge conflict invalid":
    let output = """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "looks fine",
  "evidence_refs": ["_vida/scripts/subagent-dispatch.py:2442"],
  "changed_files": [],
  "verification_commands": [],
  "verification_results": [],
  "merge_ready": "no",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "",
  "impact_analysis": {
    "affected_scope": [],
    "contract_impact": [],
    "follow_up_actions": [],
    "residual_risks": []
  },
  "coach_decision": "approved",
  "rework_required": "no",
  "coach_feedback": "ready for verification"
}
"""
    let decision = parseCoachDecision(output)
    check decision["approved"].getBool() == false
    check decision["coach_decision"].getStr() == "invalid_coach_payload.approved_conflict"

  test "parse accepts valid approved payload":
    let output = """
{
  "status": "done",
  "question_answered": "yes",
  "answer": "implementation is ready",
  "evidence_refs": ["_vida/scripts/subagent-dispatch.py:2442"],
  "changed_files": [],
  "verification_commands": ["python3 -m unittest"],
  "verification_results": ["python3 -m unittest -> pass"],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "",
  "recommended_next_action": "approve_for_independent_verification",
  "impact_analysis": {
    "affected_scope": ["_vida/scripts/subagent-dispatch.py"],
    "contract_impact": [],
    "follow_up_actions": [],
    "residual_risks": []
  },
  "coach_decision": "approved",
  "rework_required": "no",
  "coach_feedback": "ready for verification"
}
"""
    let decision = parseCoachDecision(output)
    check decision["approved"].getBool() == true
    check decision["coach_decision"].getStr() == "approved"
    check decision["invalid_reasons"].len == 0

  test "merge approves only when quorum approves":
    let decisions = @[
      %*{
        "approved": true,
        "coach_decision": "approved",
        "parsed_json": true,
        "coach_feedback": "ready",
        "recommended_next_action": "proceed_to_independent_verification",
        "evidence_refs": [],
        "verification_results": [],
        "impact_analysis": {},
        "blockers": []
      },
      %*{
        "approved": true,
        "coach_decision": "approved",
        "parsed_json": true,
        "coach_feedback": "looks good",
        "recommended_next_action": "proceed_to_independent_verification",
        "evidence_refs": [],
        "verification_results": [],
        "impact_analysis": {},
        "blockers": []
      }
    ]
    let merged = mergeCoachDecisions(decisions, 2, "unanimous_approve_rework_bias")
    check merged["approved"].getBool() == true
    check merged["coach_decision"].getStr() == "approved"

  test "merge returns rework when any valid coach requests it":
    let decisions = @[
      %*{
        "approved": true,
        "coach_decision": "approved",
        "parsed_json": true,
        "coach_feedback": "ready",
        "recommended_next_action": "proceed_to_independent_verification",
        "evidence_refs": [],
        "verification_results": [],
        "impact_analysis": {},
        "blockers": []
      },
      %*{
        "approved": false,
        "coach_decision": "return_for_rework",
        "parsed_json": true,
        "coach_feedback": "missing gate",
        "recommended_next_action": "return_to_writer",
        "evidence_refs": [],
        "verification_results": ["gate missing"],
        "impact_analysis": {},
        "blockers": ["missing gate"]
      }
    ]
    let merged = mergeCoachDecisions(decisions, 2, "unanimous_approve_rework_bias")
    check merged["approved"].getBool() == false
    check merged["coach_decision"].getStr() == "return_for_rework"
    check "missing gate" in merged["reason"].getStr()

  test "merge fails closed without valid quorum":
    let decisions = @[
      %*{
        "approved": true,
        "coach_decision": "approved",
        "parsed_json": true,
        "coach_feedback": "ready",
        "recommended_next_action": "proceed_to_independent_verification",
        "evidence_refs": [],
        "verification_results": [],
        "impact_analysis": {},
        "blockers": []
      },
      %*{
        "approved": false,
        "coach_decision": "coach_failed",
        "parsed_json": false,
        "invalid_reasons": ["missing_coach_decision_payload"],
        "coach_feedback": "",
        "recommended_next_action": "",
        "evidence_refs": [],
        "verification_results": [],
        "impact_analysis": {},
        "blockers": []
      }
    ]
    let merged = mergeCoachDecisions(decisions, 2, "unanimous_approve_rework_bias")
    check merged["approved"].getBool() == false
    check merged["coach_decision"].getStr() == "coach_failed"
