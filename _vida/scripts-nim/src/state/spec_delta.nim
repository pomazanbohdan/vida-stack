## VIDA Spec Delta — normalization and validation helper.

import std/[json, os, strutils]
import ../core/[toon, utils]
import ../agents/route

const AllowedSources* = ["issue_contract", "spec_intake", "release_signal", "research_findings", "coach_reopen"]
const AllowedStatus* = ["delta_ready", "needs_user_confirmation", "needs_scp_reconciliation", "not_required", "insufficient_delta"]

proc artifactPath*(taskId: string): string =
  route.specDeltaPath(taskId)

proc textList(value: JsonNode): JsonNode =
  result = newJArray()
  if value.isNil or value.kind == JNull:
    return
  if value.kind == JArray:
    for item in value:
      let text = policyValue(item, "")
      if text.len > 0:
        result.add(%text)
  else:
    let text = policyValue(value, "")
    if text.len > 0:
      result.add(%text)

proc normalizePayload*(taskId: string, payload: JsonNode): JsonNode =
  var source = dottedGetStr(payload, "delta_source").toLowerAscii()
  if source.len == 0 or source notin AllowedSources:
    source = "issue_contract"
  var status = dottedGetStr(payload, "status").toLowerAscii()
  if status.len == 0 or status notin AllowedStatus:
    status = "insufficient_delta"
  %*{
    "task_id": (let taskValue = dottedGetStr(payload, "task_id"); if taskValue.len > 0: taskValue else: taskId),
    "delta_source": source,
    "trigger_status": dottedGetStr(payload, "trigger_status"),
    "current_contract": dottedGetStr(payload, "current_contract"),
    "proposed_contract": dottedGetStr(payload, "proposed_contract"),
    "delta_summary": dottedGetStr(payload, "delta_summary"),
    "behavior_change": dottedGetStr(payload, "behavior_change"),
    "scope_impact": textList(payload{"scope_impact"}),
    "user_confirmation_required": (let confirmValue = dottedGetStr(payload, "user_confirmation_required").toLowerAscii(); if confirmValue.len > 0: confirmValue else: "no"),
    "reconciliation_targets": textList(payload{"reconciliation_targets"}),
    "status": status,
  }

proc validatePayload*(payload: JsonNode, taskId: string): tuple[ok: bool, reason: string] =
  if dottedGetStr(payload, "task_id") != taskId:
    return (false, "task_id_mismatch")
  if dottedGetStr(payload, "delta_source") notin AllowedSources:
    return (false, "invalid_delta_source")
  if dottedGetStr(payload, "status") notin AllowedStatus:
    return (false, "invalid_status")
  if dottedGetStr(payload, "status") != "not_required":
    for field in @["current_contract", "proposed_contract", "delta_summary", "behavior_change"]:
      if dottedGetStr(payload, field).len == 0:
        return (false, "missing_" & field)
  if dottedGetStr(payload, "status") in ["delta_ready", "needs_scp_reconciliation"] and payload{"reconciliation_targets"}.len == 0:
    return (false, "missing_reconciliation_targets")
  if dottedGetStr(payload, "status") == "needs_user_confirmation" and dottedGetStr(payload, "user_confirmation_required") != "yes":
    return (false, "user_confirmation_required_yes_expected")
  if dottedGetStr(payload, "status") == "not_required" and dottedGetStr(payload, "delta_summary").len > 0:
    return (false, "not_required_should_not_describe_delta")
  (true, "ok")

proc cmdSpecDelta*(args: seq[string]): int =
  if args.len < 2:
    echo "Usage: vida-legacy spec-delta <validate|status> <task_id> [--path PATH]"
    return 2
  let command = args[0]
  let taskId = args[1]
  var path = ""
  if args.len > 3 and args[2] == "--path":
    path = args[3]
  let selected = if path.len > 0: path else: artifactPath(taskId)
  if not fileExists(selected):
    echo "[spec-delta] missing file: " & selected
    return 1
  let payload = normalizePayload(taskId, loadJson(selected))
  let (ok, reason) = validatePayload(payload, taskId)
  if command == "validate":
    if not ok:
      echo "[spec-delta] " & reason
      return 2
    echo "OK " & selected
    return 0
  if command == "status":
    let payloadOut = normalizeJson(%*{
      "path": selected,
      "valid": ok,
      "reason": reason,
      "status": payload{"status"},
      "user_confirmation_required": payload{"user_confirmation_required"},
    })
    if "--json" in args: echo pretty(payloadOut) else: echo renderToon(payloadOut)
    return if ok: 0 else: 2
  echo "Unknown spec-delta subcommand: " & command
  return 2
