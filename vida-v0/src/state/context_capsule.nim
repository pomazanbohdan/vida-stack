## VIDA Context Capsule — compact task+epic recovery capsule.

import std/[json, os, osproc, strutils]
import ../core/[config, toon, utils]
import ./task

proc capsuleDir*(): string =
  vidaRoot() / ".vida" / "logs" / "context-capsules"

proc capsulePath*(taskId: string): string =
  capsuleDir() / (safeName(taskId, "task") & ".json")

proc logEvent(taskId, eventName, meta: string) =
  discard execCmdEx("bash " & quoteShell(vidaRoot() / "_vida" / "scripts" / "beads-log.sh") &
    " op-event " & quoteShell(taskId) & " " & quoteShell(eventName) & " " & quoteShell(meta))

proc normalizeOptional(value: string): string =
  let trimmed = value.strip()
  if trimmed.len == 0 or trimmed == "-": "" else: trimmed

proc buildCapsulePayload(taskId, doneText, nextStep, risks, acceptanceSlice, constraints, taskRole: string): JsonNode =
  let issue = showIssue(taskId)
  let taskTitle = policyValue(issue{"title"}, "")
  let taskDesc = policyValue(issue{"description"}, "")
  var parentEpic = ""
  if (not issue{"dependencies"}.isNil) and issue{"dependencies"}.kind == JArray:
    for dep in issue["dependencies"]:
      if dep.kind == JObject and policyValue(dep{"type"}, "") == "parent-child":
        parentEpic = policyValue(dep{"depends_on_id"}, "")
        break

  var epicGoal = if taskTitle.len > 0: taskTitle else: taskId
  if parentEpic.len > 0:
    let epic = showIssue(parentEpic)
    epicGoal = policyValue(epic{"description"}, policyValue(epic{"title"}, epicGoal))

  %*{
    "updated_at": nowUtc(),
    "trace_id": ("ctx-" & nowUtc().multiReplace(("-",""),(":",""),("T",""),("Z",""))),
    "epic_id": parentEpic,
    "epic_goal": epicGoal,
    "task_id": taskId,
    "task_title": taskTitle,
    "task_role_in_epic": taskRole,
    "done": doneText,
    "next": nextStep,
    "constraints": (if constraints.len > 0: %*[constraints] else: %*["follow-L0-invariants", "legacy-zero", "vida-v0-task-store"]),
    "open_risks": (if risks.len > 0: %*[risks] else: newJArray()),
    "acceptance_slice": acceptanceSlice,
    "task_context": taskDesc,
  }

proc writeCapsule*(taskId, doneText, nextStep, risks, acceptanceSlice, constraints, taskRole: string): JsonNode =
  let payload = buildCapsulePayload(taskId, doneText, nextStep, risks, acceptanceSlice, constraints, taskRole)
  saveJson(capsulePath(taskId), payload)
  logEvent(taskId, "context_capsule_written",
    "capsule_path=" & capsulePath(taskId) & ";next=" & nextStep & ";slice=" & acceptanceSlice)
  payload

proc readCapsule*(taskId: string): JsonNode =
  loadJson(capsulePath(taskId), newJObject())

proc hydrateCapsule*(taskId: string): tuple[code: int, payload: JsonNode, reason: string] =
  let path = capsulePath(taskId)
  if not fileExists(path):
    if getEnv("VIDA_CONTEXT_HYDRATE_ALLOW_MISSING", "0") == "1":
      let payload = writeCapsule(
        taskId,
        "bootstrap",
        "planning",
        "",
        "runtime-bootstrap",
        "legacy-zero,vida-v0-task-store",
        "runtime-bootstrap",
      )
      return (0, payload, "")
    logEvent(taskId, "context_hydration_failed", "reason=missing_capsule")
    return (2, newJObject(), "missing_capsule")

  let payload = readCapsule(taskId)
  var missing: seq[string] = @[]
  for field in ["epic_goal", "task_id", "next"]:
    if policyValue(payload{field}, "").len == 0:
      missing.add(field)
  if missing.len > 0:
    let reason = "missing_fields:" & missing.join(",")
    logEvent(taskId, "context_hydration_failed", "reason=missing_fields;fields=" & missing.join(","))
    return (2, payload, reason)

  logEvent(taskId, "context_hydrated", "capsule_path=" & path & ";next=" & policyValue(payload{"next"}, ""))
  (0, payload, "")

proc cmdContextCapsule*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  vida-v0 context-capsule write <task_id> <done> <next> [risks] [acceptance_slice] [constraints] [task_role]
  vida-v0 context-capsule read <task_id> [--json]
  vida-v0 context-capsule hydrate <task_id> [--json]"""
    return 1

  case args[0]
  of "write":
    if args.len < 4:
      echo "Usage: vida-v0 context-capsule write <task_id> <done> <next> [risks] [acceptance_slice] [constraints] [task_role]"
      return 1
    let payload = writeCapsule(
      args[1],
      args[2],
      args[3],
      normalizeOptional(if args.len > 4: args[4] else: "-"),
      normalizeOptional(if args.len > 5: args[5] else: "-"),
      normalizeOptional(if args.len > 6: args[6] else: "-"),
      normalizeOptional(if args.len > 7: args[7] else: "-"),
    )
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0
  of "read":
    if args.len < 2:
      echo "Usage: vida-v0 context-capsule read <task_id> [--json]"
      return 1
    let payload = readCapsule(args[1])
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0
  of "hydrate":
    if args.len < 2:
      echo "Usage: vida-v0 context-capsule hydrate <task_id> [--json]"
      return 1
    let (code, payload, reason) = hydrateCapsule(args[1])
    if code != 0:
      if reason == "missing_capsule":
        stderr.writeLine("[context-capsule] BLK_CONTEXT_NOT_HYDRATED: missing capsule for " & args[1])
      elif reason.startsWith("missing_fields:"):
        stderr.writeLine("[context-capsule] BLK_CONTEXT_NOT_HYDRATED: missing fields (" &
          reason["missing_fields:".len .. ^1] & ") for " & args[1])
      return code
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0
  else:
    echo "Unknown context-capsule subcommand: " & args[0]
    return 1
