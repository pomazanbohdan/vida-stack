## VIDA Task State Reconcile — unify issue, TODO, verify, and run-graph signals.
##
## Ports the core `status` payload from `task-state-reconcile.py`.

import std/[json, os, strutils]
import ../core/[config, toon, utils]
import ./[beads, run_graph, task, todo]

proc issuesJsonlPath*(): string = vidaRoot() / ".beads" / "issues.jsonl"
proc bootReceiptPath*(taskId: string): string =
  vidaRoot() / ".vida" / "logs" / "boot-receipts" / (safeName(taskId, "task") & ".json")

proc loadIssue*(taskId: string): JsonNode =
  let payload = showIssue(taskId)
  if payload.kind == JObject and
     policyValue(payload{"status"}, "") != "missing" and
     (policyValue(payload{"id"}, "") == taskId or policyValue(payload{"display_id"}, "") == taskId):
    return payload
  let issuesPath = issuesJsonlPath()
  if not fileExists(issuesPath):
    return newJObject()
  for rawLine in lines(issuesPath):
    let line = rawLine.strip()
    if line.len == 0:
      continue
    try:
      let item = parseJson(line)
      if item.kind == JObject and policyValue(item{"id"}, "") == taskId:
        return item
    except:
      discard
  return newJObject()

proc verifyBootReceipt*(taskId: string): bool =
  fileExists(bootReceiptPath(taskId))

proc verifyLogOk*(taskId: string): bool =
  let verifySummary = beads.verifyTaskLog(taskId, false, 8)
  verifySummary.criticalCount == 0

proc stepCounts*(steps: JsonNode): JsonNode =
  result = %*{
    "todo": 0,
    "doing": 0,
    "done": 0,
    "blocked": 0,
    "superseded": 0,
    "partial": 0,
  }
  if steps.kind != JArray:
    return
  for step in steps:
    let status = policyValue(step["status"], "")
    if result.hasKey(status):
      result[status] = %(result[status].getInt() + 1)

proc classifyState*(issueStatus: string, steps: JsonNode, bootReceiptOk, verifyOk: bool,
                    runGraph: JsonNode): tuple[classification: string, reasons: seq[string], allowedActions: seq[string]] =
  let counts = stepCounts(steps)
  let hasSteps = steps.kind == JArray and steps.len > 0
  let resumeHint = dottedGet(runGraph, "resume_hint", newJObject())
  let activeRunGraph = dottedGetBool(runGraph, "present", false) and
    policyValue(resumeHint["status"], "") in ["running", "ready", "blocked"]
  var terminalDoneExists = false
  if steps.kind == JArray:
    for step in steps:
      if policyValue(step["status"], "") == "done" and policyValue(step["next_step"], "") == "-":
        terminalDoneExists = true
        break

  if issueStatus == "closed":
    if counts["doing"].getInt() > 0 or counts["todo"].getInt() > 0 or counts["blocked"].getInt() > 0:
      return ("invalid_state", @["closed task still has active TODO backlog"], @["manual_review"])
    return ("closed", @[], @["none"])

  if counts["blocked"].getInt() > 0:
    return ("blocked", @["TODO block is blocked"], @["unblock_or_escalate"])

  if counts["doing"].getInt() > 0 or activeRunGraph:
    return ("active", @[], @["continue_current_block"])

  if issueStatus == "in_progress" and terminalDoneExists and counts["todo"].getInt() > 0:
    return ("drift_detected", @["terminal done block exists but TODO backlog still remains"], @["reconcile_todo_then_close_or_manual_review"])

  if issueStatus == "in_progress" and counts["todo"].getInt() > 0:
    return ("stale_in_progress", @["task is in_progress but no active block is running"], @["resume_next_block", "or_reconcile_br"])

  if hasSteps and counts["todo"].getInt() == 0 and counts["doing"].getInt() == 0 and counts["blocked"].getInt() == 0:
    if issueStatus == "in_progress":
      if verifyOk:
        return ( "done_ready_to_close",
          (if bootReceiptOk: @[] else: @["boot receipt missing"]),
          @["close_now"] )
      return ("drift_detected", @["all TODO blocks ended but verify evidence is missing"], @["verify_then_close_or_manual_review"])
    if issueStatus == "open":
      return ("open_but_satisfied", (if verifyOk: @[] else: @["verify evidence missing"]), @["close_now_if_scope_satisfied", "or_mark_in_progress_before_resume"])

  if issueStatus == "in_progress" and not hasSteps:
    return ("stale_in_progress", @["in_progress task has no TODO execution trace"], @["start_or_reconcile"])

  if issueStatus == "open" and terminalDoneExists and counts["todo"].getInt() > 0:
    return ("drift_detected", @["terminal done block exists but open backlog still remains"], @["reconcile_todo_or_scope"])

  return ((if issueStatus == "in_progress": "active" else: "open"), @[], @["continue"])

proc buildStatusPayload*(taskId: string): JsonNode =
  let issue = loadIssue(taskId)
  let issueStatus = policyValue(issue{"status"}, "")
  let title = policyValue(issue{"title"}, "")
  let steps = todo.stepsJson(taskId)
  let bootOk = verifyBootReceipt(taskId)
  let verifyOk = verifyLogOk(taskId)
  let graph = run_graph.statusPayload(taskId)
  let (classification, reasons, allowedActions) = classifyState(issueStatus, steps, bootOk, verifyOk, graph)
  let counts = stepCounts(steps)
  var current = newJNull()
  if steps.kind == JArray:
    for step in steps:
      if policyValue(step["status"], "") == "doing":
        current = %*{
          "block_id": policyValue(step["block_id"], ""),
          "goal": policyValue(step["goal"], ""),
        }
        break

  %*{
    "task_id": taskId,
    "title": title,
    "issue_status": issueStatus,
    "classification": classification,
    "reasons": reasons,
    "allowed_actions": allowedActions,
    "boot_receipt_ok": bootOk,
    "verify_ok": verifyOk,
    "todo_counts": counts,
    "current_block": current,
    "run_graph": {
      "present": dottedGetBool(graph, "present", false),
      "resume_hint": dottedGet(graph, "resume_hint", newJObject()),
    },
  }

proc cmdReconcile*(args: seq[string]): int =
  if args.len < 2:
    echo """Usage:
  taskflow-v0 reconcile status <task_id> [--json]"""
    return 1
  case args[0]
  of "status":
    let payload = buildStatusPayload(args[1])
    let rest = if args.len > 2: args[2..^1] else: @[]
    if "--json" in rest:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0
  else:
    echo "Unknown reconcile subcommand: " & args[0]
    return 1
