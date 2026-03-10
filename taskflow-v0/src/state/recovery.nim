## VIDA Recovery Runtime — checkpoint commits, gateway handles, and resumable recovery.

import std/[json, os, strutils]
import ../core/[config, toon, utils]
import ./[context, run_graph]

proc normalizedStringList(value: JsonNode): seq[string] =
  result = @[]
  if value.isNil or value.kind != JArray:
    return
  for item in value:
    let text = policyValue(item, "")
    if text.len > 0 and text notin result:
      result.add(text)

proc checkpointDir*(): string = vidaWorkspacePath("state", "checkpoints")
proc gatewayDir*(): string = vidaWorkspacePath("state", "gateway-handles")
proc gatewayIndexPath*(): string = vidaWorkspacePath("state", "gateway-trigger-index.json")

proc checkpointPath*(taskId: string): string =
  checkpointDir() / (safeName(taskId, "task") & ".json")

proc gatewayPath*(taskId: string): string =
  gatewayDir() / (safeName(taskId, "task") & ".json")

proc loadGatewayIndex*(): JsonNode =
  loadJson(gatewayIndexPath(), %*{"handles_by_trigger": {}})

proc writeGatewayIndex*(payload: JsonNode): string =
  saveJson(gatewayIndexPath(), payload)
  gatewayIndexPath()

proc checkpointCommitPayload*(taskId, checkpointKind, cursorOrPosition, resumeTarget: string,
                              checkpointGroup: string = "", meta: JsonNode = newJObject()): JsonNode =
  %*{
    "task_id": taskId,
    "checkpoint_kind": checkpointKind,
    "cursor_or_position": cursorOrPosition,
    "resume_target": resumeTarget,
    "checkpoint_group": checkpointGroup,
    "lineage_kind": "live",
    "committed_at": nowUtc(),
    "meta": meta,
  }

proc writeCheckpointCommit*(taskId, checkpointKind, cursorOrPosition, resumeTarget: string,
                            checkpointGroup: string = "", meta: JsonNode = newJObject()): string =
  let path = checkpointPath(taskId)
  saveJson(path, checkpointCommitPayload(taskId, checkpointKind, cursorOrPosition, resumeTarget, checkpointGroup, meta))
  path

proc openGatewayHandle*(taskId, gatewayKind, triggerKey, routeStage, resumeNode: string,
                        singleUse: bool = true, meta: JsonNode = newJObject()): string =
  let path = gatewayPath(taskId)
  var payload = loadJson(path, %*{"task_id": taskId, "handles": []})
  if payload{"handles"}.kind != JArray:
    payload["handles"] = newJArray()

  let handleId = safeName(taskId & "-" & gatewayKind & "-" & triggerKey & "-" & nowUtc(), "gateway")
  let handle = %*{
    "handle_id": handleId,
    "gateway_kind": gatewayKind,
    "trigger_key": triggerKey,
    "route_stage": routeStage,
    "resume_node": resumeNode,
    "single_use": singleUse,
    "state": "open",
    "opened_at": nowUtc(),
    "meta": meta,
  }
  payload["handles"].add(handle)
  saveJson(path, payload)

  var indexPayload = loadGatewayIndex()
  if indexPayload{"handles_by_trigger"}.kind != JObject:
    indexPayload["handles_by_trigger"] = newJObject()
  let existing = normalizedStringList(indexPayload{"handles_by_trigger"}{triggerKey})
  var updated = existing
  if handleId notin updated:
    updated.add(handleId)
  indexPayload["handles_by_trigger"][triggerKey] = %updated
  discard writeGatewayIndex(indexPayload)

  path

proc resolveGatewayTrigger*(triggerKey: string): JsonNode =
  let indexPayload = loadGatewayIndex()
  let handleIds = normalizedStringList(indexPayload{"handles_by_trigger"}{triggerKey})
  var matches: seq[JsonNode] = @[]
  if dirExists(gatewayDir()):
    for entry in walkDir(gatewayDir()):
      if entry.kind != pcFile:
        continue
      let payload = loadJson(entry.path)
      for handle in payload{"handles"}:
        if handle.kind != JObject:
          continue
        if policyValue(handle{"trigger_key"}, "") != triggerKey:
          continue
        if policyValue(handle{"state"}, "open") != "open":
          continue
        matches.add(%*{
          "task_id": payload{"task_id"},
          "handle_id": handle{"handle_id"},
          "gateway_kind": handle{"gateway_kind"},
          "route_stage": handle{"route_stage"},
          "resume_node": handle{"resume_node"},
          "single_use": handle{"single_use"},
          "path": entry.path,
        })

  %*{
    "trigger_key": triggerKey,
    "indexed_handle_ids": handleIds,
    "matches": matches,
    "match_count": matches.len,
  }

proc recoveryStatus*(taskId: string): JsonNode =
  let graph = statusPayload(taskId)
  let checkpoint = loadJson(checkpointPath(taskId))
  let contextPayload = loadState()
  let gateways = loadJson(gatewayPath(taskId), %*{"task_id": taskId, "handles": []})
  let resumeHint = dottedGet(graph, "resume_hint", %*{"next_node": "", "status": "missing"})

  var blockers: seq[string] = @[]
  if not dottedGetBool(graph, "present", false):
    blockers.add("missing_run_graph")
  if policyValue(resumeHint{"next_node"}, "").len == 0:
    blockers.add("missing_resume_hint")
  if checkpoint.kind != JObject or checkpoint.len == 0:
    blockers.add("missing_checkpoint_commit")

  %*{
    "task_id": taskId,
    "run_graph": graph,
    "checkpoint": checkpoint,
    "context_governance": contextPayload,
    "gateways": gateways,
    "resume_hint": resumeHint,
    "recovery_ready": blockers.len == 0,
    "blockers": blockers,
  }

proc resumePayload*(taskId: string, triggerKey: string = ""): JsonNode =
  let status = recoveryStatus(taskId)
  if not dottedGetBool(status, "recovery_ready", false):
    return %*{
      "ok": false,
      "task_id": taskId,
      "reason": normalizedStringList(status{"blockers"}).join("; "),
      "status": status,
    }

  let nextNode = dottedGetStr(status, "resume_hint.next_node")
  var gatewayResolution = newJNull()
  if triggerKey.len > 0:
    gatewayResolution = resolveGatewayTrigger(triggerKey)
    let matches = gatewayResolution{"matches"}
    var matchedTask = false
    if matches.kind == JArray:
      for item in matches:
        if policyValue(item{"task_id"}, "") == taskId:
          matchedTask = true
          break
    if not matchedTask:
      return %*{
        "ok": false,
        "task_id": taskId,
        "reason": "gateway_trigger_not_resolved_for_task",
        "gateway_resolution": gatewayResolution,
      }

  %*{
    "ok": true,
    "task_id": taskId,
    "resume_node": nextNode,
    "resume_target": dottedGet(status, "checkpoint.resume_target", newJNull()),
    "checkpoint_kind": dottedGet(status, "checkpoint.checkpoint_kind", newJNull()),
    "gateway_resolution": gatewayResolution,
    "closure_authority": "taskflow",
  }

proc parseJsonArg(arg: string, default: JsonNode = newJObject()): JsonNode =
  if arg.len == 0:
    return default
  try:
    parseJson(arg)
  except:
    default

proc cmdRecovery*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  taskflow-v0 recovery checkpoint-commit <task_id> <checkpoint_kind> <cursor> <resume_target> [checkpoint_group] [meta_json]
  taskflow-v0 recovery gateway-open <task_id> <gateway_kind> <trigger_key> <route_stage> <resume_node> [single_use] [meta_json]
  taskflow-v0 recovery gateway-resolve <trigger_key> [--json]
  taskflow-v0 recovery status <task_id> [--json]
  taskflow-v0 recovery resume <task_id> [trigger_key] [--json]"""
    return 1

  let asJson = "--json" in args
  case args[0]
  of "checkpoint-commit":
    if args.len < 5:
      echo "Usage: taskflow-v0 recovery checkpoint-commit <task_id> <checkpoint_kind> <cursor> <resume_target> [checkpoint_group] [meta_json]"
      return 1
    let checkpointGroup = if args.len > 5 and args[5] != "--json": args[5] else: ""
    let meta =
      if args.len > 6 and args[6] != "--json": parseJsonArg(args[6])
      else: newJObject()
    echo writeCheckpointCommit(args[1], args[2], args[3], args[4], checkpointGroup, meta)
    return 0
  of "gateway-open":
    if args.len < 6:
      echo "Usage: taskflow-v0 recovery gateway-open <task_id> <gateway_kind> <trigger_key> <route_stage> <resume_node> [single_use] [meta_json]"
      return 1
    let singleUse =
      if args.len > 6 and args[6] != "--json":
        args[6].strip().toLowerAscii() notin ["false", "no", "0"]
      else:
        true
    let meta =
      if args.len > 7 and args[7] != "--json": parseJsonArg(args[7])
      else: newJObject()
    echo openGatewayHandle(args[1], args[2], args[3], args[4], args[5], singleUse, meta)
    return 0
  of "gateway-resolve":
    if args.len < 2:
      echo "Usage: taskflow-v0 recovery gateway-resolve <trigger_key> [--json]"
      return 1
    let payload = normalizeJson(resolveGatewayTrigger(args[1]))
    if asJson: echo pretty(payload) else: echo renderToon(payload)
    return (if dottedGetInt(payload, "match_count", 0) > 0: 0 else: 2)
  of "status":
    if args.len < 2:
      echo "Usage: taskflow-v0 recovery status <task_id> [--json]"
      return 1
    let payload = normalizeJson(recoveryStatus(args[1]))
    if asJson: echo pretty(payload) else: echo renderToon(payload)
    return (if dottedGetBool(payload, "recovery_ready", false): 0 else: 2)
  of "resume":
    if args.len < 2:
      echo "Usage: taskflow-v0 recovery resume <task_id> [trigger_key] [--json]"
      return 1
    let triggerKey = if args.len > 2 and args[2] != "--json": args[2] else: ""
    let payload = normalizeJson(resumePayload(args[1], triggerKey))
    if asJson: echo pretty(payload) else: echo renderToon(payload)
    return (if dottedGetBool(payload, "ok", false): 0 else: 2)
  else:
    echo "Unknown recovery subcommand: " & args[0]
    return 1
