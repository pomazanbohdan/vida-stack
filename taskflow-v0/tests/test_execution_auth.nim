## Tests for gates/execution_auth module

import std/[json, os, unittest]
import ../src/gates/execution_auth
import ../src/core/utils
import ../src/agents/route as routeMod

proc writeJsonFile(path: string, payload: JsonNode) =
  let dir = parentDir(path)
  if not dirExists(dir):
    createDir(dir)
  writeFile(path, pretty(payload) & "\n")

suite "execution auth":
  let root = "/tmp/vida_scripts_nim_execution_auth"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)
  writeFile(
    root / "vida.config.yaml",
    """
agent_system:
  mode: hybrid
  subagents:
    codex_cli:
      enabled: true
      write_scope: scoped_only
      capability_band:
        - bounded_write_safe
      billing_tier: low
      speed_tier: medium
      quality_tier: high
"""
  )

  test "authorize-local writes route metadata":
    let receiptPath = localExecReceiptPath("vida-test-local", "implementation")
    if fileExists(receiptPath):
      removeFile(receiptPath)
    check authorizeLocal("vida-test-local", "implementation", "emergency_override", "task-only", "bounded notes") == 0
    let receipt = loadJson(receiptPath)
    check receipt["route_receipt_path"].getStr().len > 0
    check receipt["route_receipt_hash"].getStr().len > 0

  test "authorize-skip requires framework labels":
    let taskId = "vida-test-skip-denied"
    let issuesPath = root / ".beads" / "issues.jsonl"
    createDir(parentDir(issuesPath))
    writeFile(issuesPath, $(%*{"id": taskId, "labels": ["bug"]}) & "\n")
    check authorizeSkip(taskId, "implementation", "no_eligible_analysis_lane", "notes") == 1

  test "check-gate accepts structured override for no eligible analysis lane":
    let taskId = "vida-test-structured"
    let taskClass = "implementation"
    let issuesPath = root / ".beads" / "issues.jsonl"
    writeFile(issuesPath, $(%*{"id": taskId, "labels": ["framework", "bug"]}) & "\n")
    saveJson(routeMod.issueContractPath(taskId), %*{
      "task_id": taskId,
      "status": "writer_ready",
      "proven_scope": ["src/kernel"]
    })

    let (_, route) = routeMod.routeSnapshot(taskClass, taskId)
    let routeHash = loadJson(routeMod.writeRouteReceipt(taskId, taskClass, route))["route_receipt_hash"].getStr()
    writeJsonFile(
      execution_auth.analysisBlockerPath(taskId, taskClass),
      %*{
        "task_id": taskId,
        "task_class": taskClass,
        "status": "analysis_failed",
        "reason": "no_eligible_analysis_lane",
        "route_receipt_hash": routeHash,
      }
    )
    check authorizeSkip(taskId, taskClass, "no_eligible_analysis_lane", "framework-only path") == 0
    let (exitCode, payload) = checkGate(taskId, taskClass, true, "P01")
    check exitCode == 0
    check payload["status"].getStr() == "ok"
    check payload["authorized_via"].getStr() == "structured_unavailability_override"
    check payload["local_execution_authorized"].getBool() == true

  test "check-gate blocks on invalid draft execution spec":
    let taskId = "vida-test-invalid-draft"
    let taskClass = "implementation"
    let draftPath = execution_auth.draftExecSpecPath(taskId)
    saveJson(draftPath, %*{
      "task_id": taskId,
      "scope_in": [],
      "acceptance_checks": [],
      "recommended_next_path": "/bad-path"
    })
    let (exitCode, payload) = checkGate(taskId, taskClass, false, "P02")
    check exitCode == 2
    check payload["status"].getStr() == "blocked"
    var blockers: seq[string] = @[]
    for item in payload["blockers"]:
      blockers.add(item.getStr())
    check "invalid_recommended_next_path" in blockers or "missing_scope_in" in blockers

  test "check-gate blocks stale structured override receipt":
    let taskId = "vida-test-stale-override"
    let taskClass = "implementation"
    let issuesPath = root / ".beads" / "issues.jsonl"
    writeFile(issuesPath, $(%*{"id": taskId, "labels": ["framework", "vida-stack"]}) & "\n")
    saveJson(routeMod.issueContractPath(taskId), %*{
      "task_id": taskId,
      "status": "writer_ready",
      "proven_scope": ["src/kernel"]
    })

    let (_, route) = routeMod.routeSnapshot(taskClass, taskId)
    let routeHash = loadJson(routeMod.writeRouteReceipt(taskId, taskClass, route))["route_receipt_hash"].getStr()
    writeJsonFile(
      execution_auth.analysisBlockerPath(taskId, taskClass),
      %*{
        "task_id": taskId,
        "task_class": taskClass,
        "status": "analysis_failed",
        "reason": "no_eligible_analysis_lane",
        "route_receipt_hash": routeHash,
      }
    )
    saveJson(execution_auth.overrideReceiptPath(taskId, taskClass), %*{
      "task_id": taskId,
      "task_class": taskClass,
      "reason": "no_eligible_analysis_lane",
      "notes": "framework-only path",
      "route_receipt_hash": "stale-hash"
    })

    let (exitCode, payload) = checkGate(taskId, taskClass, true, "P02")
    check exitCode == 2
    check payload["status"].getStr() == "blocked"
    var blockers: seq[string] = @[]
    for item in payload["blockers"]:
      blockers.add(item.getStr())
    check "stale_execution_auth_override" in blockers

  test "check-gate allows valid draft execution spec without issue contract":
    let taskId = "vida-test-spec-driven"
    let taskClass = "implementation"
    let (_, route) = routeMod.routeSnapshot(taskClass, taskId)
    let routeHash = loadJson(routeMod.writeRouteReceipt(taskId, taskClass, route))["route_receipt_hash"].getStr()
    writeJsonFile(
      execution_auth.analysisBlockerPath(taskId, taskClass),
      %*{
        "task_id": taskId,
        "task_class": taskClass,
        "status": "analysis_failed",
        "reason": "fanout_min_results_not_met",
        "route_receipt_hash": routeHash,
      }
    )
    check authorizeLocal(taskId, taskClass, "emergency_override", "task-only", "bounded notes") == 0
    saveJson(execution_auth.draftExecSpecPath(taskId), %*{
      "task_id": taskId,
      "scope_in": ["settings flow"],
      "acceptance_checks": ["settings render correctly"],
      "recommended_next_path": "/vida-form-task"
    })

    let (exitCode, payload) = checkGate(taskId, taskClass, true, "P03")
    check exitCode == 0
    check payload["status"].getStr() == "ok"
    check payload["issue_contract_required"].getBool() == false
    check payload["draft_execution_spec_present"].getBool() == true

  test "check-gate accepts explicit no eligible verifier":
    let taskId = "vida-test-no-verifier"
    let taskClass = "implementation"
    let issuesPath = root / ".beads" / "issues.jsonl"
    writeFile(issuesPath, $(%*{"id": taskId, "labels": ["bug"]}) & "\n")
    saveJson(routeMod.issueContractPath(taskId), %*{
      "task_id": taskId,
      "status": "writer_ready",
      "proven_scope": ["src/no-verifier"]
    })

    let (routeCtx, route) = routeMod.routeSnapshot(taskClass, taskId)
    check routeCtx.len >= 0
    writeJsonFile(
      execution_auth.analysisBlockerPath(taskId, taskClass),
      %*{
        "task_id": taskId,
        "task_class": taskClass,
        "status": "analysis_failed",
        "reason": "fanout_min_results_not_met",
        "route_receipt_hash": routeMod.routeReceiptHash(route),
      }
    )
    check authorizeLocal(taskId, taskClass, "emergency_override", "task-only", "bounded notes") == 0
    let (exitCode, payload) = checkGate(taskId, taskClass, true, "P03")
    check exitCode == 0
    check payload["status"].getStr() == "ok"
    check payload["verification_prereq_via"].getStr() == "no_eligible_verifier"
    check payload["authorized_via"].getStr() == "local_emergency_override"

  test "check-gate blocks when issue contract is missing":
    let taskId = "vida-test-missing-issue-contract"
    let taskClass = "implementation"
    let issuesPath = root / ".beads" / "issues.jsonl"
    writeFile(issuesPath, $(%*{"id": taskId, "labels": ["bug"]}) & "\n")

    let (_, route) = routeMod.routeSnapshot(taskClass, taskId)
    writeJsonFile(
      execution_auth.analysisBlockerPath(taskId, taskClass),
      %*{
        "task_id": taskId,
        "task_class": taskClass,
        "status": "analysis_failed",
        "reason": "fanout_min_results_not_met",
        "route_receipt_hash": routeMod.routeReceiptHash(route),
      }
    )
    check authorizeLocal(taskId, taskClass, "emergency_override", "task-only", "bounded notes") == 0
    let (exitCode, payload) = checkGate(taskId, taskClass, true, "P04")
    check exitCode == 2
    check payload["status"].getStr() == "blocked"
    var blockers: seq[string] = @[]
    for item in payload["blockers"]:
      blockers.add(item.getStr())
    check "missing_issue_contract" in blockers
