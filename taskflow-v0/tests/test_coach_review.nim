## Tests for gates/coach_review module

import std/[json, os, strutils, unittest]
import ../src/gates/coach_review
import ../src/core/utils

proc writeJsonFile(path: string, payload: JsonNode) =
  let dir = parentDir(path)
  if not dirExists(dir):
    createDir(dir)
  writeFile(path, pretty(payload) & "\n")

suite "coach review":
  let root = "/tmp/vida_scripts_nim_coach_review"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)

  test "gate accepts valid coach receipt":
    let taskId = "unit-task"
    let taskClass = "implementation"
    let routeReceiptPath = root / ".vida" / "logs" / "route-receipts" / (safeName(taskId) & "." & safeName(taskClass) & ".route.json")
    let routeReceipt = %*{
      "task_class": taskClass,
      "coach_required": "yes",
      "coach_plan": {"required": "yes", "route_task_class": "coach"},
    }
    writeJsonFile(routeReceiptPath, %*{"route_receipt": routeReceipt})
    writeJsonFile(coachReceiptPath(taskId, taskClass), %*{
      "status": "coach_approved",
      "route_receipt_hash": "ignored-by-current-gate"
    })
    let (exitCode, payload) = checkCoachGate(taskId)
    check exitCode == 0
    check payload["status"].getStr() == "ok"
    check payload["blockers"].len == 0

  test "gate blocks when coach artifact is missing":
    let taskId = "unit-task-missing"
    let taskClass = "implementation"
    let routeReceiptPath = root / ".vida" / "logs" / "route-receipts" / (safeName(taskId) & "." & safeName(taskClass) & ".route.json")
    let routeReceipt = %*{
      "task_class": taskClass,
      "coach_required": "yes",
      "coach_plan": {"required": "yes", "route_task_class": "coach"},
    }
    writeJsonFile(routeReceiptPath, %*{"route_receipt": routeReceipt})
    let (exitCode, payload) = checkCoachGate(taskId)
    check exitCode == 2
    check payload["status"].getStr() == "blocked"
    check payload["blockers"].len == 1
    check payload["blockers"][0]["reason"].getStr() == "missing_coach_review_artifact"

  test "gate accepts structured override when no eligible coach is recorded":
    let taskId = "unit-task-override"
    let taskClass = "implementation"
    let routeReceiptPath = root / ".vida" / "logs" / "route-receipts" / (safeName(taskId) & "." & safeName(taskClass) & ".route.json")
    let routeReceipt = %*{
      "task_class": taskClass,
      "coach_required": "yes",
      "coach_plan": {"required": "yes", "route_task_class": "coach"},
    }
    writeJsonFile(routeReceiptPath, %*{"route_receipt": routeReceipt})
    writeJsonFile(coachBlockerPath(taskId, taskClass), %*{
      "status": "coach_pass_cap_exceeded",
      "reason": "no_eligible_coach"
    })
    discard writeCoachOverrideReceipt(taskId, "no_eligible_coach", "No eligible coach lane produced a lawful review artifact")
    let (exitCode, payload) = checkCoachGate(taskId)
    check exitCode == 0
    check payload["status"].getStr() == "ok"
    check payload["authorized_via"].getStr() == "structured_override"
    check payload["override_receipt_present"].getBool() == true

  test "gate reports missing structured rework handoff":
    let taskId = "unit-task-rework-missing"
    let taskClass = "implementation"
    let routeReceiptPath = root / ".vida" / "logs" / "route-receipts" / (safeName(taskId) & "." & safeName(taskClass) & ".route.json")
    let routeReceipt = %*{
      "task_class": taskClass,
      "coach_required": "yes",
      "coach_plan": {"required": "yes", "route_task_class": "coach"},
    }
    writeJsonFile(routeReceiptPath, %*{"route_receipt": routeReceipt})
    writeJsonFile(coachBlockerPath(taskId, taskClass), %*{
      "status": "return_for_rework",
      "reason": "writer must start over"
    })
    let (exitCode, payload) = checkCoachGate(taskId)
    check exitCode == 2
    check payload["status"].getStr() == "blocked"
    check payload["blockers"][0]["reason"].getStr() == "writer must start over"

  test "gate surfaces valid rework handoff for return to writer":
    let taskId = "unit-task-rework-ready"
    let taskClass = "implementation"
    let routeReceiptPath = root / ".vida" / "logs" / "route-receipts" / (safeName(taskId) & "." & safeName(taskClass) & ".route.json")
    let routeReceipt = %*{
      "task_class": taskClass,
      "coach_required": "yes",
      "coach_plan": {"required": "yes", "route_task_class": "coach"},
    }
    writeJsonFile(routeReceiptPath, %*{"route_receipt": routeReceipt})
    writeJsonFile(coachBlockerPath(taskId, taskClass), %*{
      "status": "return_for_rework",
      "reason": "writer must start over"
    })
    writeJsonFile(reworkHandoffPath(taskId, taskClass), %*{
      "status": "writer_rework_ready",
      "fresh_start_required": true,
      "original_prompt_text": "Implement the feature from spec.",
      "fresh_prompt_text": "Implement the feature from spec.\n\nFresh Rework Handoff:\n- Summary: missing gate\n",
      "coach_delta": {
        "coach_feedback": "missing gate",
        "feedback_source": "output_json_payload",
        "feedback_sources": ["output_json_payload"]
      }
    })
    let (exitCode, payload) = checkCoachGate(taskId)
    check exitCode == 2
    check payload["status"].getStr() == "blocked"
    check payload["blockers"][0]["reason"].getStr() == "writer must start over"
    check payload["blockers"][0]["rework_handoff_path"].getStr().endsWith(".rework-handoff.json")
    check payload["blockers"][0]["rework_handoff_status"].getStr() == "writer_rework_ready"
