## VIDA Draft Execution Spec — normalization and validation helper.

import std/[json, os]
import ../core/[toon, utils]
import ../agents/route

const AllowedPaths* = ["/vida-form-task", "/vida-bug-fix"]

proc artifactPath*(taskId: string): string =
  route.draftExecSpecPath(taskId)

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
  let recommendedNextPath =
    (let raw = dottedGetStr(payload, "recommended_next_path");
     if raw.len > 0: raw else: "/vida-form-task")
  %*{
    "task_id": (let value = dottedGetStr(payload, "task_id"); if value.len > 0: value else: taskId),
    "scope_in": textList(payload{"scope_in"}),
    "scope_out": textList(payload{"scope_out"}),
    "acceptance_checks": textList(payload{"acceptance_checks"}),
    "assumptions": textList(payload{"assumptions"}),
    "open_decisions": textList(payload{"open_decisions"}),
    "recommended_next_path": recommendedNextPath,
  }

proc validatePayload*(payload: JsonNode, taskId: string): tuple[ok: bool, reason: string] =
  if dottedGetStr(payload, "task_id") != taskId:
    return (false, "task_id_mismatch")
  if payload{"scope_in"}.kind != JArray or payload{"scope_in"}.len == 0:
    return (false, "missing_scope_in")
  if payload{"acceptance_checks"}.kind != JArray or payload{"acceptance_checks"}.len == 0:
    return (false, "missing_acceptance_checks")
  if dottedGetStr(payload, "recommended_next_path") notin AllowedPaths:
    return (false, "invalid_recommended_next_path")
  (true, "ok")

proc cmdDraftExecutionSpec*(args: seq[string]): int =
  if args.len < 2:
    echo "Usage: vida-legacy draft-execution-spec <validate|status> <task_id> [--path PATH]"
    return 2
  let command = args[0]
  let taskId = args[1]
  var path = ""
  if args.len > 3 and args[2] == "--path":
    path = args[3]
  let selected = if path.len > 0: path else: artifactPath(taskId)
  if not fileExists(selected):
    echo "[draft-execution-spec] missing file: " & selected
    return 1
  let payload = normalizePayload(taskId, loadJson(selected))
  let (ok, reason) = validatePayload(payload, taskId)
  if command == "validate":
    if not ok:
      echo "[draft-execution-spec] " & reason
      return 2
    echo "OK " & selected
    return 0
  if command == "status":
    let payloadOut = normalizeJson(%*{
      "path": selected,
      "valid": ok,
      "reason": reason,
      "recommended_next_path": payload{"recommended_next_path"},
      "open_decisions": payload{"open_decisions"},
    })
    if "--json" in args: echo pretty(payloadOut) else: echo renderToon(payloadOut)
    return if ok: 0 else: 2
  echo "Unknown draft-execution-spec subcommand: " & command
  return 2
