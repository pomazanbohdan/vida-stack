## VIDA Coach Review Gate — post-write coach review validation.
##
## Replaces `coach-review-gate.py` (213 lines).
## Validates coach review receipts, handles override receipts,
## and checks rework handoff status.

import std/[json, os, strutils, sequtils, sets]
import ../core/[utils, config, toon]

# ─────────────────────────── Constants ───────────────────────────

const AllowedCoachOverrideReasons* = ["no_eligible_coach", "runtime_blocker"].toHashSet

# ─────────────────────────── Paths ───────────────────────────

proc coachOverrideDir*(): string = vidaRoot() / ".vida" / "logs" / "coach-review-overrides"

proc coachOverrideReceiptPath*(taskId: string): string =
  coachOverrideDir() / (safeName(taskId) & ".json")

proc coachReceiptPath*(taskId, taskClass: string): string =
  vidaRoot() / ".vida" / "logs" / "route-receipts" /
    (safeName(taskId) & "." & safeName(taskClass) & ".coach.json")

proc coachBlockerPath*(taskId, taskClass: string): string =
  vidaRoot() / ".vida" / "logs" / "route-receipts" /
    (safeName(taskId) & "." & safeName(taskClass) & ".coach-blocker.json")

proc reworkHandoffPath*(taskId, taskClass: string): string =
  vidaRoot() / ".vida" / "logs" / "route-receipts" /
    (safeName(taskId) & "." & safeName(taskClass) & ".rework-handoff.json")

# ─────────────────────────── Helpers ───────────────────────────

proc loadReceipt(path: string): JsonNode =
  if not fileExists(path): return newJObject()
  loadJson(path)

# ─────────────────────────── Override Receipt ───────────────────────────

proc writeCoachOverrideReceipt*(taskId, reason, notes: string,
                                evidence: string = "", actor: string = ""): string =
  let normReason = reason.strip()
  if normReason notin AllowedCoachOverrideReasons:
    raise newException(ValueError, "unsupported override reason: " & normReason)
  let payload = %*{
    "task_id": taskId, "reason": normReason, "notes": notes.strip(),
    "evidence": evidence.strip(), "actor": actor.strip(), "ts": nowUtc(),
  }
  let path = coachOverrideReceiptPath(taskId)
  createDir(path.parentDir())
  saveJson(path, payload)
  return path

# ─────────────────────────── Validation ───────────────────────────

proc validateCoachArtifact*(taskId, taskClass: string): JsonNode =
  ## Validate coach review receipt or blocker for a task.
  let receipt = loadReceipt(coachReceiptPath(taskId, taskClass))
  if receipt.len > 0:
    let status = policyValue(receipt{"status"}, "")
    if status == "coach_approved":
      return %*{
        "task_class": taskClass, "status": "ok",
        "artifact": "coach_receipt",
        "path": coachReceiptPath(taskId, taskClass),
      }
    return %*{
      "task_class": taskClass, "status": "blocked",
      "artifact": "coach_receipt",
      "path": coachReceiptPath(taskId, taskClass),
      "reason": "stale_or_invalid_coach_receipt",
    }

  let blocker = loadReceipt(coachBlockerPath(taskId, taskClass))
  if blocker.len > 0:
    let blockerReason = policyValue(blocker{"reason"}, "")
    let blockerStatus = policyValue(blocker{"status"}, "coach_blocked")
    var reworkPath = policyValue(blocker{"rework_handoff_path"}, "")
    var reworkStatus = policyValue(blocker{"rework_handoff_status"}, "")

    if blockerStatus == "return_for_rework":
      let handoff = loadReceipt(reworkHandoffPath(taskId, taskClass))
      if handoff.len > 0:
        reworkPath = reworkHandoffPath(taskId, taskClass)
        reworkStatus = "writer_rework_ready"

    return %*{
      "task_class": taskClass, "status": "blocked",
      "artifact": "coach_blocker",
      "path": coachBlockerPath(taskId, taskClass),
      "reason": (if blockerReason.len > 0: blockerReason else: blockerStatus),
      "rework_handoff_path": reworkPath,
      "rework_handoff_status": reworkStatus,
    }

  return %*{
    "task_class": taskClass, "status": "blocked",
    "artifact": "missing", "path": "",
    "reason": "missing_coach_review_artifact",
  }

# ─────────────────────────── Check Gate ───────────────────────────

proc checkCoachGate*(taskId: string): tuple[exitCode: int, payload: JsonNode] =
  let receiptDir = vidaRoot() / ".vida" / "logs" / "route-receipts"
  var evaluations: seq[JsonNode] = @[]

  # Find route receipts for this task
  let prefix = safeName(taskId)
  if dirExists(receiptDir):
    for entry in walkDir(receiptDir):
      if entry.kind != pcFile: continue
      let name = extractFilename(entry.path)
      if not name.startsWith(prefix) or not name.endsWith(".route.json"): continue
      let receipt = loadReceipt(entry.path)
      let routeReceipt = receipt{"route_receipt"}
      if routeReceipt.isNil or routeReceipt.kind != JObject: continue
      if policyValue(routeReceipt{"coach_required"}, "no").toLowerAscii() != "yes": continue
      let taskClass = policyValue(routeReceipt{"task_class"}, "")
      if taskClass.len == 0: continue
      var eval = validateCoachArtifact(taskId, taskClass)
      eval["route_receipt_path"] = %entry.path
      evaluations.add(eval)

  let blockers = evaluations.filterIt(policyValue(it{"status"}, "") != "ok")

  let overridePath = coachOverrideReceiptPath(taskId)
  let overridePayload = loadReceipt(overridePath)
  let overrideReason = policyValue(overridePayload{"reason"}, "")
  let overrideValid = overridePayload.len > 0 and overrideReason in AllowedCoachOverrideReasons

  let ok = blockers.len == 0 or overrideValid
  let authorizedVia = if blockers.len > 0 and overrideValid: "structured_override" else: ""

  let payload = %*{
    "task_id": taskId,
    "status": (if ok: "ok" else: "blocked"),
    "authorized_via": authorizedVia,
    "required_routes": evaluations,
    "blockers": blockers,
    "override_receipt_path": overridePath,
    "override_receipt_present": overridePayload.len > 0,
    "override_receipt": overridePayload,
  }
  let exitCode = if ok: 0 else: 2
  return (exitCode, payload)

# ─────────────────────────── CLI ───────────────────────────

proc cmdCoachGate*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  taskflow-v0 coach check <task_id>
  taskflow-v0 coach authorize-skip <task_id> <reason> <notes> [evidence] [actor]"""
    return 1

  case args[0]
  of "check":
    if args.len < 2: echo "Usage: taskflow-v0 coach check <task_id>"; return 1
    let (exitCode, rawPayload) = checkCoachGate(args[1])
    let payload = normalizeJson(rawPayload)
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return exitCode

  of "authorize-skip":
    if args.len < 4:
      echo "Usage: taskflow-v0 coach authorize-skip <task_id> <reason> <notes>"; return 1
    let evidence = if args.len > 4: args[4] else: ""
    let actor = if args.len > 5: args[5] else: ""
    try:
      let path = writeCoachOverrideReceipt(args[1], args[2], args[3], evidence, actor)
      echo path; return 0
    except ValueError as e:
      echo e.msg; return 1

  else:
    echo "Unknown coach subcommand: " & args[0]; return 1
