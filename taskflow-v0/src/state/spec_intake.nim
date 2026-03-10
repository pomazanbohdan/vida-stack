## VIDA Spec Intake — normalization and validation helper.

import std/[json, os, strutils]
import ../core/[toon, utils]
import ../agents/route

const AllowedClasses* = ["research", "issue", "release_signal", "user_negotiation", "mixed"]
const AllowedStatus* = ["ready_for_scp", "ready_for_issue_contract", "needs_user_negotiation", "needs_spec_delta", "insufficient_intake"]
const AllowedPaths* = ["scp", "issue_contract", "spec_delta", "user_negotiation", "gather_evidence"]

proc artifactPath*(taskId: string): string =
  route.specIntakePath(taskId)

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

proc issueLike(path: string): bool = path == "issue_contract"

proc normalizePayload*(taskId: string, payload: JsonNode): JsonNode =
  var intakeClass = dottedGetStr(payload, "intake_class").toLowerAscii()
  if intakeClass.len == 0 or intakeClass notin AllowedClasses:
    intakeClass = "mixed"
  var status = dottedGetStr(payload, "status").toLowerAscii()
  if status.len == 0 or status notin AllowedStatus:
    status = "insufficient_intake"
  var contractPath = dottedGetStr(payload, "recommended_contract_path").toLowerAscii()
  if contractPath.len == 0 or contractPath notin AllowedPaths:
    contractPath = "gather_evidence"
  %*{
    "task_id": (let value = dottedGetStr(payload, "task_id"); if value.len > 0: value else: taskId),
    "intake_class": intakeClass,
    "source_inputs": textList(payload{"source_inputs"}),
    "problem_statement": dottedGetStr(payload, "problem_statement"),
    "requested_outcome": dottedGetStr(payload, "requested_outcome"),
    "research_findings": textList(payload{"research_findings"}),
    "issue_signals": textList(payload{"issue_signals"}),
    "release_signals": textList(payload{"release_signals"}),
    "assumptions": textList(payload{"assumptions"}),
    "proposed_scope_in": textList(payload{"proposed_scope_in"}),
    "proposed_scope_out": textList(payload{"proposed_scope_out"}),
    "open_decisions": textList(payload{"open_decisions"}),
    "acceptance_checks": textList(payload{"acceptance_checks"}),
    "recommended_contract_path": contractPath,
    "status": status,
  }

proc validatePayload*(payload: JsonNode, taskId: string): tuple[ok: bool, reason: string] =
  if dottedGetStr(payload, "task_id") != taskId:
    return (false, "task_id_mismatch")
  if dottedGetStr(payload, "intake_class") notin AllowedClasses:
    return (false, "invalid_intake_class")
  if dottedGetStr(payload, "status") notin AllowedStatus:
    return (false, "invalid_status")
  if dottedGetStr(payload, "recommended_contract_path") notin AllowedPaths:
    return (false, "invalid_recommended_contract_path")
  if dottedGetStr(payload, "problem_statement").len == 0:
    return (false, "missing_problem_statement")
  if dottedGetStr(payload, "requested_outcome").len == 0:
    return (false, "missing_requested_outcome")
  if dottedGetStr(payload, "status") in ["ready_for_scp", "ready_for_issue_contract", "needs_spec_delta"] and payload{"proposed_scope_in"}.len == 0:
    return (false, "missing_proposed_scope_in")
  if dottedGetStr(payload, "status") == "needs_user_negotiation" and payload{"open_decisions"}.len == 0:
    return (false, "missing_open_decisions")
  if dottedGetStr(payload, "status") == "ready_for_issue_contract" and not issueLike(dottedGetStr(payload, "recommended_contract_path")):
    return (false, "issue_contract_path_required")
  if dottedGetStr(payload, "status") == "needs_spec_delta" and dottedGetStr(payload, "recommended_contract_path") != "spec_delta":
    return (false, "spec_delta_path_required")
  if dottedGetStr(payload, "status") == "insufficient_intake" and dottedGetStr(payload, "recommended_contract_path") != "gather_evidence":
    return (false, "gather_evidence_path_required")
  (true, "ok")

proc cmdSpecIntake*(args: seq[string]): int =
  if args.len < 2:
    echo "Usage: taskflow-v0 spec-intake <validate|status> <task_id> [--path PATH]"
    return 2
  let command = args[0]
  let taskId = args[1]
  var path = ""
  if args.len > 3 and args[2] == "--path":
    path = args[3]
  let selected = if path.len > 0: path else: artifactPath(taskId)
  if not fileExists(selected):
    echo "[spec-intake] missing file: " & selected
    return 1
  let payload = normalizePayload(taskId, loadJson(selected))
  let (ok, reason) = validatePayload(payload, taskId)
  if command == "validate":
    if not ok:
      echo "[spec-intake] " & reason
      return 2
    echo "OK " & selected
    return 0
  if command == "status":
    let payloadOut = normalizeJson(%*{
      "path": selected,
      "valid": ok,
      "reason": reason,
      "status": payload{"status"},
      "recommended_contract_path": payload{"recommended_contract_path"},
      "open_decisions": payload{"open_decisions"},
    })
    if "--json" in args: echo pretty(payloadOut) else: echo renderToon(payloadOut)
    return if ok: 0 else: 2
  echo "Unknown spec-intake subcommand: " & command
  return 2
