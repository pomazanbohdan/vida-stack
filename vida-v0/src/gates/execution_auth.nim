## VIDA Execution Auth Gate — authorization checks for writer execution.
##
## Replaces `execution-auth-gate.py` (584 lines).
## Validates analysis prereqs, verification state, spec intake/delta,
## local execution receipts, and override receipts.

import std/[json, os, strutils, hashes, sets]
import ../core/[utils, config, toon]
import ../agents/route as routeMod
import ../state/[draft_execution_spec, spec_delta, spec_intake]

# ─────────────────────────── Constants ───────────────────────────

const FrameworkOverrideLabels* = [
  "framework", "agent-system", "fsap", "vida-stack",
  "local-platform-alignment", "registry", "evals",
  "context", "operator-surface", "durability",
].toHashSet

const AllowedOverrideReasons* = ["no_eligible_analysis_lane"].toHashSet

# ─────────────────────────── Paths ───────────────────────────

proc routeReceiptDir*(): string = routeMod.routeReceiptDir()
proc overrideDir*(): string = vidaRoot() / ".vida" / "logs" / "execution-auth-overrides"

proc localExecReceiptPath*(taskId, taskClass: string): string =
  routeReceiptDir() / (safeName(taskId) & "." & safeName(taskClass) & ".local-exec.json")

proc execAuthReceiptPath*(taskId, taskClass: string): string =
  routeReceiptDir() / (safeName(taskId) & "." & safeName(taskClass) & ".execution-auth.json")

proc overrideReceiptPath*(taskId, taskClass: string): string =
  overrideDir() / (safeName(taskId) & "." & safeName(taskClass) & ".json")

proc analysisReceiptPath*(taskId, taskClass: string): string =
  routeReceiptDir() / (safeName(taskId) & "." & safeName(taskClass) & ".analysis.json")

proc analysisBlockerPath*(taskId, taskClass: string): string =
  routeReceiptDir() / (safeName(taskId) & "." & safeName(taskClass) & ".analysis-blocker.json")

proc specIntakePath*(taskId: string): string = routeMod.specIntakePath(taskId)
proc specDeltaPath*(taskId: string): string = routeMod.specDeltaPath(taskId)
proc draftExecSpecPath*(taskId: string): string = routeMod.draftExecSpecPath(taskId)
proc issueContractPath*(taskId: string): string = routeMod.issueContractPath(taskId)
proc internalEscalationReceiptPath*(taskId, taskClass: string): string =
  routeMod.internalEscalationReceiptPath(taskId, taskClass)

# ─────────────────────────── Helpers ───────────────────────────

proc jsonHash*(payload: JsonNode): string =
  let encoded = $payload
  var h: Hash = 0
  for ch in encoded: h = h !& hash(ch)
  h = !$h
  return toHex(h).toLowerAscii()

proc loadIssueMetadata*(taskId: string): JsonNode =
  let issuesPath = vidaRoot() / ".beads" / "issues.jsonl"
  if not fileExists(issuesPath): return newJObject()
  for line in lines(issuesPath):
    if line.strip().len == 0: continue
    try:
      let payload = parseJson(line)
      if payload.kind == JObject and policyValue(payload{"id"}, "") == taskId:
        return payload
    except: discard
  return newJObject()

proc taskAllowsOverride*(taskId: string): bool =
  let payload = loadIssueMetadata(taskId)
  let labels = payload{"labels"}
  if labels.isNil or labels.kind != JArray: return false
  for label in labels:
    if label.getStr().strip().toLowerAscii() in FrameworkOverrideLabels:
      return true
  return false

proc loadReceipt(path: string): JsonNode =
  if not fileExists(path): return newJObject()
  loadJson(path)

proc shouldRequireIssueContract*(taskId, taskClass: string, draftExecutionSpecPresent: bool): bool =
  if taskClass != "implementation":
    return false
  if draftExecutionSpecPresent:
    return false
  let payload = loadIssueMetadata(taskId)
  if payload.len == 0:
    return true
  let issueType = policyValue(payload{"issue_type"}, "").toLowerAscii()
  let labels = payload{"labels"}
  if issueType == "bug":
    return true
  if not labels.isNil and labels.kind == JArray:
    for label in labels:
      if label.getStr().strip().toLowerAscii() == "bug":
        return true
  return false

# ─────────────────────────── Validation Helpers ───────────────────────────

proc validateAnalysisBlocker*(taskId, taskClass: string, route: JsonNode): tuple[ok: bool, receipt: JsonNode, error: string] =
  let receipt = loadReceipt(analysisBlockerPath(taskId, taskClass))
  if receipt.len == 0: return (false, newJObject(), "missing_analysis_receipt")
  let reason = policyValue(receipt{"reason"}, "")
  if reason.len == 0: return (false, receipt, "missing_analysis_blocker_reason")
  let status = policyValue(receipt{"status"}, "")
  if status == "blocked_missing_analysis_route":
    return (false, receipt, "analysis_route_not_ready")
  if status != "analysis_failed":
    return (false, receipt, "invalid_analysis_blocker_status")
  if policyValue(receipt{"route_receipt_hash"}, "") != routeMod.routeReceiptHash(route):
    return (false, receipt, "stale_analysis_blocker")
  return (true, receipt, "")

proc validateDraftExecutionSpecReceipt*(taskId: string): tuple[ok: bool, receipt: JsonNode, error: string] =
  let path = draftExecSpecPath(taskId)
  if not fileExists(path):
    return (true, newJObject(), "")
  let payload = draft_execution_spec.normalizePayload(taskId, loadJson(path))
  let (ok, reason) = draft_execution_spec.validatePayload(payload, taskId)
  if not ok:
    return (false, payload, reason)
  return (true, payload, "")

proc validateSpecIntakeReceipt*(taskId: string): tuple[ok: bool, receipt: JsonNode, error: string] =
  let path = specIntakePath(taskId)
  if not fileExists(path):
    return (true, newJObject(), "")
  let payload = spec_intake.normalizePayload(taskId, loadJson(path))
  let (ok, reason) = spec_intake.validatePayload(payload, taskId)
  if not ok:
    return (false, payload, reason)
  return (true, payload, "")

proc validateSpecDeltaReceipt*(taskId: string): tuple[ok: bool, receipt: JsonNode, error: string] =
  let path = specDeltaPath(taskId)
  if not fileExists(path):
    return (true, newJObject(), "")
  let payload = spec_delta.normalizePayload(taskId, loadJson(path))
  let (ok, reason) = spec_delta.validatePayload(payload, taskId)
  if not ok:
    return (false, payload, reason)
  return (true, payload, "")

proc verificationPrereqState*(verificationPlan: JsonNode): tuple[ok: bool, via: string] =
  if policyValue(verificationPlan{"required"}, "no") != "yes":
    return (true, "not_required")
  if policyValue(verificationPlan{"selected_subagent"}, "").len > 0:
    return (true, "verifier_selected")
  if policyValue(verificationPlan{"reason"}, "") == "no_eligible_verifier":
    return (true, "no_eligible_verifier")
  return (false, "missing_verifier_plan")

proc validateLocalExecutionReceipt*(taskId, taskClass: string, route: JsonNode): tuple[ok: bool, receipt: JsonNode, error: string] =
  let receipt = loadReceipt(localExecReceiptPath(taskId, taskClass))
  if receipt.len == 0: return (false, newJObject(), "missing_local_execution_receipt")
  if policyValue(receipt{"reason"}, "") != "emergency_override":
    return (false, receipt, "invalid_local_execution_reason")
  if policyValue(receipt{"scope"}, "").len == 0:
    return (false, receipt, "missing_local_execution_scope")
  if policyValue(receipt{"notes"}, "").len == 0:
    return (false, receipt, "missing_local_execution_notes")
  if policyValue(receipt{"route_receipt_hash"}, "") != routeMod.routeReceiptHash(route):
    return (false, receipt, "stale_local_execution_receipt")
  return (true, receipt, "")

proc validateStructuredOverrideReceipt*(taskId, taskClass: string, route: JsonNode, expectedReason: string): tuple[ok: bool, receipt: JsonNode, error: string] =
  let receipt = loadReceipt(overrideReceiptPath(taskId, taskClass))
  if receipt.len == 0: return (false, newJObject(), "missing_execution_auth_override")
  if not taskAllowsOverride(taskId):
    return (false, receipt, "execution_auth_override_not_allowed")
  let reason = policyValue(receipt{"reason"}, "")
  if reason notin AllowedOverrideReasons:
    return (false, receipt, "invalid_execution_auth_override_reason")
  if reason != expectedReason:
    return (false, receipt, "mismatched_execution_auth_override_reason")
  if policyValue(receipt{"notes"}, "").len == 0:
    return (false, receipt, "missing_execution_auth_override_notes")
  if policyValue(receipt{"route_receipt_hash"}, "") != routeMod.routeReceiptHash(route):
    return (false, receipt, "stale_execution_auth_override")
  return (true, receipt, "")

# ─────────────────────────── Override Receipt ───────────────────────────

proc writeOverrideReceipt*(taskId, taskClass, reason, notes: string,
                           evidence: string = "", actor: string = ""): string =
  let normReason = reason.strip()
  if normReason notin AllowedOverrideReasons:
    raise newException(ValueError, "unsupported override reason: " & normReason)
  let payload = %*{
    "ts": nowUtc(), "task_id": taskId, "task_class": taskClass,
    "reason": normReason, "notes": notes.strip(),
    "evidence": evidence.strip(), "actor": actor.strip(),
  }
  let path = overrideReceiptPath(taskId, taskClass)
  createDir(path.parentDir())
  saveJson(path, payload)
  return path

# ─────────────────────────── Authorize Local ───────────────────────────

proc authorizeLocal*(taskId, taskClass, reason, scope, notes: string,
                     evidence: string = "", actor: string = "orchestrator"): int =
  if reason != "emergency_override":
    echo "[execution-auth-gate] only the explicit `emergency_override` reason is allowed"
    return 1
  let (_, route) = routeMod.routeSnapshot(taskClass, taskId)
  let routeReceiptPath = routeMod.writeRouteReceipt(taskId, taskClass, route)
  let receiptPayload = %*{
    "ts": nowUtc(), "task_id": taskId, "task_class": taskClass,
    "reason": reason, "scope": scope, "notes": notes,
    "evidence": evidence, "actor": actor,
    "route_receipt_path": routeReceiptPath,
    "route_receipt_hash": routeMod.routeReceiptHash(route),
    "analysis_receipt_present": loadReceipt(analysisReceiptPath(taskId, taskClass)).len > 0,
  }
  let path = localExecReceiptPath(taskId, taskClass)
  createDir(path.parentDir())
  saveJson(path, receiptPayload)
  echo path
  return 0

proc authorizeSkip*(taskId, taskClass, reason, notes: string,
                    evidence: string = "", actor: string = "orchestrator"): int =
  if not taskAllowsOverride(taskId):
    echo "[execution-auth-gate] structured execution-auth override is allowed only for framework-labeled tasks"
    return 1
  let (_, route) = routeMod.routeSnapshot(taskClass, taskId)
  let routeReceiptPath = routeMod.writeRouteReceipt(taskId, taskClass, route)
  try:
    let path = writeOverrideReceipt(taskId, taskClass, reason, notes, evidence, actor)
    var payload = loadReceipt(path)
    payload["route_receipt_path"] = %routeReceiptPath
    payload["route_receipt_hash"] = %routeMod.routeReceiptHash(route)
    saveJson(path, payload)
    echo path
    return 0
  except ValueError as e:
    echo e.msg
    return 1

# ─────────────────────────── Authorize Internal ───────────────────────────

proc authorizeInternal*(taskId, taskClass, reason, scope, notes: string,
                        evidence: string = "", actor: string = "orchestrator"): int =
  let (_, route) = routeMod.routeSnapshot(taskClass, taskId)
  let routeReceiptPath = routeMod.writeRouteReceipt(taskId, taskClass, route)
  let receiptPayload = %*{
    "ts": nowUtc(), "task_id": taskId, "task_class": taskClass,
    "reason": reason, "scope": scope, "notes": notes,
    "evidence": evidence, "actor": actor,
    "route_receipt_path": routeReceiptPath,
    "route_receipt_hash": routeMod.routeReceiptHash(route),
  }
  let path = internalEscalationReceiptPath(taskId, taskClass)
  createDir(path.parentDir())
  saveJson(path, receiptPayload)
  echo path
  return 0

# ─────────────────────────── Check Gate ───────────────────────────

proc checkGate*(taskId, taskClass: string, localWrite: bool = false,
                blockId: string = ""): tuple[exitCode: int, payload: JsonNode] =
  let (_, route) = routeMod.routeSnapshot(taskClass, taskId)
  let routeReceiptPath = routeMod.writeRouteReceipt(taskId, taskClass, route)
  let routePayload = routeMod.routeReceiptPayload(route)
  let analysisPlan = route{"analysis_plan"}
  let verificationPlan = route{"verification_plan"}
  let dispatchPolicy = route{"dispatch_policy"}
  var blockers: seq[string] = @[]
  var analysisPrereqVia = "not_required"
  var verificationPrereqVia = "not_required"
  let localAllowedByRoute = policyValue(dispatchPolicy{"local_execution_allowed"}, "no") == "yes"

  # Analysis receipt check
  let analysisReceipt = loadReceipt(analysisReceiptPath(taskId, taskClass))
  let hasAnalysisReceipt = analysisReceipt.len > 0
  var analysisBlocker = newJObject()
  var analysisBlockerOk = false
  var analysisBlockerError = ""

  if policyValue(analysisPlan{"required"}, "no") == "yes" and policyValue(analysisPlan{"receipt_required"}, "no") == "yes":
    if hasAnalysisReceipt:
      analysisPrereqVia = "analysis_receipt"
    else:
      let (abOk, abReceipt, abError) = validateAnalysisBlocker(taskId, taskClass, route)
      analysisBlocker = abReceipt
      analysisBlockerOk = abOk
      analysisBlockerError = abError
      if abOk:
        analysisPrereqVia = "analysis_blocker"
      elif abError.len > 0:
        blockers.add(abError)

  let (verificationOk, verificationVia) = verificationPrereqState(verificationPlan)
  if verificationOk:
    verificationPrereqVia = verificationVia
  else:
    blockers.add(verificationVia)

  let (specIntakeOk, specIntake, specIntakeError) = validateSpecIntakeReceipt(taskId)
  let (specDeltaOk, specDelta, specDeltaError) = validateSpecDeltaReceipt(taskId)
  let (draftExecSpecOk, draftExecSpec, draftExecSpecError) = validateDraftExecutionSpecReceipt(taskId)
  if not specIntakeOk:
    blockers.add(specIntakeError)
  if not specDeltaOk:
    blockers.add(specDeltaError)
  if not draftExecSpecOk:
    blockers.add(draftExecSpecError)
  let issueContractRequired = shouldRequireIssueContract(taskId, taskClass, draftExecSpec.len > 0 and draftExecSpecOk)
  let issueContractPresent = loadReceipt(issueContractPath(taskId)).len > 0
  if issueContractRequired and not issueContractPresent:
    blockers.add("missing_issue_contract")

  var localReceiptOk = false
  var localReceipt = newJObject()
  var overrideReceiptOk = false
  var overrideReceipt = newJObject()
  var authorizedVia = ""

  if localWrite and not localAllowedByRoute:
    var expectedOverrideReason = ""
    if analysisBlockerOk:
      let blockerReason = policyValue(analysisBlocker{"reason"}, "")
      if blockerReason in AllowedOverrideReasons:
        expectedOverrideReason = blockerReason
    if expectedOverrideReason.len > 0:
      let (ok, receipt, err) = validateStructuredOverrideReceipt(taskId, taskClass, route, expectedOverrideReason)
      overrideReceiptOk = ok
      overrideReceipt = receipt
      if not ok:
        blockers.add(err)
    else:
      let (ok, receipt, err) = validateLocalExecutionReceipt(taskId, taskClass, route)
      localReceiptOk = ok
      localReceipt = receipt
      if not ok:
        blockers.add(err)

  if localAllowedByRoute:
    authorizedVia = "route_local_execution"
  elif overrideReceiptOk:
    authorizedVia = "structured_unavailability_override"
  elif localReceiptOk:
    authorizedVia = "local_emergency_override"

  let payload = %*{
    "ts": nowUtc(),
    "task_id": taskId, "task_class": taskClass,
    "block_id": (if blockId.len > 0: %blockId else: newJNull()),
    "status": (if blockers.len == 0: "ok" else: "blocked"),
    "local_write": localWrite,
    "route_receipt_path": routeReceiptPath,
    "analysis_receipt_present": hasAnalysisReceipt,
    "analysis_blocker_present": analysisBlocker.len > 0,
    "analysis_prereq_via": analysisPrereqVia,
    "verification_prereq_via": verificationPrereqVia,
    "issue_contract_path": issueContractPath(taskId),
    "issue_contract_present": issueContractPresent,
    "issue_contract_required": issueContractRequired,
    "spec_intake_present": specIntake.len > 0,
    "spec_intake": specIntake,
    "spec_delta_present": specDelta.len > 0,
    "spec_delta": specDelta,
    "draft_execution_spec_present": draftExecSpec.len > 0,
    "draft_execution_spec": draftExecSpec,
    "local_execution_allowed": localAllowedByRoute,
    "local_execution_authorized": localAllowedByRoute or localReceiptOk or overrideReceiptOk,
    "authorized_via": authorizedVia,
    "required_dispatch_path": dispatchPolicy{"required_dispatch_path"},
    "route_receipt": routePayload,
    "analysis_blocker": analysisBlocker,
    "local_execution_receipt": localReceipt,
    "execution_auth_override_receipt": overrideReceipt,
    "blockers": blockers,
  }

  let receiptPath = execAuthReceiptPath(taskId, taskClass)
  createDir(receiptPath.parentDir())
  saveJson(receiptPath, payload)
  let exitCode = if blockers.len == 0: 0 else: 2
  return (exitCode, payload)

# ─────────────────────────── CLI ───────────────────────────

proc cmdAuthGate*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  vida-v0 auth check <task_id> <task_class> [--local-write] [--block-id <id>]
  vida-v0 auth authorize-local <task_id> <task_class> <reason> <scope> <notes> [evidence] [actor]
  vida-v0 auth authorize-internal <task_id> <task_class> <reason> <scope> <notes> [evidence] [actor]
  vida-v0 auth authorize-skip <task_id> <task_class> <reason> <notes> [evidence] [actor]"""
    return 1

  case args[0]
  of "check":
    if args.len < 3:
      echo "Usage: vida-v0 auth check <task_id> <task_class>"; return 1
    var localWrite = false; var blockId = ""
    var i = 3
    while i < args.len:
      if args[i] == "--local-write": localWrite = true; i += 1
      elif args[i] == "--block-id" and i + 1 < args.len: blockId = args[i+1]; i += 2
      else: i += 1
    let (exitCode, rawPayload) = checkGate(args[1], args[2], localWrite, blockId)
    let payload = normalizeJson(rawPayload)
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return exitCode

  of "authorize-local":
    if args.len < 6: echo "Missing args"; return 1
    let evidence = if args.len > 6: args[6] else: ""
    let actor = if args.len > 7: args[7] else: "orchestrator"
    return authorizeLocal(args[1], args[2], args[3], args[4], args[5], evidence, actor)

  of "authorize-internal":
    if args.len < 6: echo "Missing args"; return 1
    let evidence = if args.len > 6: args[6] else: ""
    let actor = if args.len > 7: args[7] else: "orchestrator"
    return authorizeInternal(args[1], args[2], args[3], args[4], args[5], evidence, actor)

  of "authorize-skip":
    if args.len < 5: echo "Missing args"; return 1
    let evidence = if args.len > 5: args[5] else: ""
    let actor = if args.len > 6: args[6] else: "orchestrator"
    return authorizeSkip(args[1], args[2], args[3], args[4], evidence, actor)

  else:
    echo "Unknown auth subcommand: " & args[0]; return 1
