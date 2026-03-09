## VIDA TODO Runtime — task step projection from beads execution log.
##
## Ports the core task-view surface from `todo-runtime.py`:
## ui-json, list, current, next, board, compact, tracks, sync.

import std/[algorithm, json, os, strutils, tables]
import ../core/[config, utils]
import ./beads

proc todoIndexDir*(): string = vidaRoot() / ".vida" / "logs" / "todo-index"
proc todoLogDir*(): string = vidaRoot() / ".vida" / "logs"
proc todoSyncStateDir*(): string = vidaRoot() / ".vida" / "logs" / "todo-sync-state"

type
  StepState = object
    blockId: string
    taskId: string
    goal: string
    trackId: string
    owner: string
    dependsOn: string
    nextStep: string
    tsStart: string
    tsEnd: string
    result: string
    actions: string
    evidenceRef: string
    mergeReady: string

proc stepStatus(step: StepState): string =
  case step.result
  of "done": "done"
  of "failed": "blocked"
  of "redirected": "superseded"
  of "partial": "partial"
  else:
    if step.tsStart.len > 0 and step.tsEnd.len == 0: "doing" else: "todo"

proc stepToJson(step: StepState): JsonNode =
  result = %*{
    "block_id": step.blockId,
    "task_id": step.taskId,
    "goal": step.goal,
    "track_id": (if step.trackId.len > 0: step.trackId else: "main"),
    "owner": (if step.owner.len > 0: step.owner else: "orchestrator"),
    "depends_on": step.dependsOn,
    "next_step": step.nextStep,
    "ts_start": step.tsStart,
    "ts_end": step.tsEnd,
    "result": step.result,
    "actions": step.actions,
    "evidence_ref": step.evidenceRef,
    "merge_ready": step.mergeReady,
    "status": stepStatus(step),
  }
  for key in @["goal", "depends_on", "next_step", "ts_start", "ts_end", "result", "actions", "evidence_ref", "merge_ready"]:
    if policyValue(result[key], "").len == 0:
      result.delete(key)

proc logSignature*(): string =
  let logPath = beadsLogPath()
  if not fileExists(logPath): return "missing"
  let logStat = getFileInfo(logPath)
  let runtimePath = currentSourcePath()
  let runtimeStat = getFileInfo(runtimePath)
  $logStat.lastWriteTime & ":" & $logStat.size & ":" &
    $runtimeStat.lastWriteTime & ":" & $runtimeStat.size

proc readTaskEvents(taskId: string): seq[JsonNode] =
  let logPath = beadsLogPath()
  if not fileExists(logPath): return @[]
  for line in lines(logPath):
    if line.strip().len == 0: continue
    try:
      let event = parseJson(line)
      let eventTaskId = dottedGetStr(event, "task_id")
      let eventType = dottedGetStr(event, "type")
      if eventTaskId == taskId and eventType in ["block_plan", "block_start", "block_end"]:
        result.add(event)
    except: discard

proc propagateSuperseded(byBlock: var OrderedTable[string, StepState]) =
  var changed = true
  while changed:
    changed = false
    var superseded: seq[string] = @[]
    for blockId, step in byBlock:
      if stepStatus(step) == "superseded":
        superseded.add(blockId)
    for blockId, step in mpairs(byBlock):
      if stepStatus(step) != "todo":
        continue
      if step.dependsOn.len > 0 and step.dependsOn in superseded:
        step.result = "redirected"
        changed = true

proc computeSteps*(taskId: string): JsonNode =
  let events = readTaskEvents(taskId)
  var byBlock = initOrderedTable[string, StepState]()
  for event in events:
    let blockId = policyValue(event["block_id"], "")
    if blockId.len == 0:
      continue
    if not byBlock.hasKey(blockId):
      byBlock[blockId] = StepState(blockId: blockId, taskId: taskId, trackId: "main", owner: "orchestrator")
    var step = byBlock[blockId]
    let eventType = dottedGetStr(event, "type")
    case eventType
    of "block_plan":
      step.goal = dottedGetStr(event, "goal", step.goal)
      step.trackId = dottedGetStr(event, "track_id", step.trackId)
      step.owner = dottedGetStr(event, "owner", step.owner)
      step.dependsOn = dottedGetStr(event, "depends_on", step.dependsOn)
      if event.hasKey("next_step"):
        step.nextStep = dottedGetStr(event, "next_step", step.nextStep)
    of "block_start":
      step.goal = dottedGetStr(event, "goal", step.goal)
      step.trackId = dottedGetStr(event, "track_id", step.trackId)
      step.owner = dottedGetStr(event, "owner", step.owner)
      step.dependsOn = dottedGetStr(event, "depends_on", step.dependsOn)
      if event.hasKey("next_step"):
        step.nextStep = dottedGetStr(event, "next_step", step.nextStep)
      step.tsStart = dottedGetStr(event, "ts", step.tsStart)
      step.tsEnd = ""
      step.result = ""
    of "block_end":
      step.result = dottedGetStr(event, "result", step.result)
      step.nextStep = dottedGetStr(event, "next_step", step.nextStep)
      step.actions = dottedGetStr(event, "actions", step.actions)
      step.evidenceRef = dottedGetStr(event, "evidence_ref", step.evidenceRef)
      step.mergeReady = dottedGetStr(event, "merge_ready", step.mergeReady)
      step.tsEnd = dottedGetStr(event, "ts_end", dottedGetStr(event, "ts", step.tsEnd))
    else:
      discard
    byBlock[blockId] = step

  propagateSuperseded(byBlock)
  var blockIds: seq[string] = @[]
  for blockId in byBlock.keys:
    blockIds.add(blockId)
  blockIds.sort(system.cmp[string])
  result = newJArray()
  for blockId in blockIds:
    result.add(stepToJson(byBlock[blockId]))

proc stepsJson*(taskId: string): JsonNode =
  createDir(todoIndexDir())
  let indexPath = todoIndexDir() / (safeName(taskId, "task") & ".json")
  let signature = logSignature()
  let cached = loadJson(indexPath)
  if cached.kind == JObject and cached.hasKey("log_signature") and policyValue(cached["log_signature"], "") == signature:
    return cached["steps"]

  let steps = computeSteps(taskId)
  saveJson(indexPath, %*{
    "task_id": taskId,
    "updated_at": nowUtc(),
    "log_signature": signature,
    "steps": steps,
  })
  return steps

proc groupIds(steps: JsonNode, status: string): string =
  var ids: seq[string] = @[]
  for step in steps:
    if policyValue(step["status"], "") == status:
      ids.add(policyValue(step["block_id"], ""))
  if ids.len == 0: return ""
  ids.join(", ")

proc shorten(text: string, limit: int = 72): string =
  if text.len <= limit: text else: text[0..<limit] & "..."

proc compactLine(steps: JsonNode, status: string, limit: int): string =
  var items: seq[string] = @[]
  var total = 0
  for step in steps:
    if policyValue(step["status"], "") != status:
      continue
    total += 1
    if items.len < limit:
      items.add(policyValue(step["block_id"], "") & ":" & shorten(policyValue(step["goal"], "-")))
  if total == 0:
    return status.toUpperAscii() & "(0): -"
  var body = items.join(" | ")
  if total > limit:
    body.add(" | +" & $(total - limit) & " more")
  status.toUpperAscii() & "(" & $total & "): " & body

proc writeSyncJson*(taskId: string): tuple[payload: JsonNode, jsonOut: string, stateOut: string] =
  createDir(todoLogDir())
  createDir(todoSyncStateDir())
  let payload = %*{"task_id": taskId, "steps": stepsJson(taskId)}
  let jsonOut = todoLogDir() / ("todo-sync-" & safeName(taskId, "task") & ".json")
  let stateOut = todoSyncStateDir() / (safeName(taskId, "task") & "-last-ui.json")
  saveJson(jsonOut, payload)
  return (payload, jsonOut, stateOut)

proc syncDelta*(prevPayload, payload: JsonNode): seq[string] =
  var prev = initTable[string, JsonNode]()
  var cur = initTable[string, JsonNode]()
  for step in prevPayload{"steps"}:
    prev[policyValue(step["block_id"], "")] = step
  for step in payload{"steps"}:
    cur[policyValue(step["block_id"], "")] = step
  var blockIds: seq[string] = @[]
  for blockId in prev.keys:
    if blockId.len > 0 and blockId notin blockIds:
      blockIds.add(blockId)
  for blockId in cur.keys:
    if blockId.len > 0 and blockId notin blockIds:
      blockIds.add(blockId)
  blockIds.sort(system.cmp[string])

  for blockId in blockIds:
    let hasPrev = prev.hasKey(blockId)
    let hasCur = cur.hasKey(blockId)
    if not hasPrev and hasCur:
      let step = cur[blockId]
      result.add("- [+] " & blockId & " — " & policyValue(step["goal"], "-") &
        " (status=" & policyValue(step["status"], "todo") & ")")
    elif hasPrev and not hasCur:
      let step = prev[blockId]
      result.add("- [-] " & blockId & " — " & policyValue(step["goal"], "-"))
    elif hasPrev and hasCur:
      let prevStep = prev[blockId]
      let curStep = cur[blockId]
      let prevStatus = policyValue(prevStep["status"], "")
      let curStatus = policyValue(curStep["status"], "")
      if prevStatus != curStatus:
        result.add("- [~] " & blockId & " — " &
          policyValue(curStep["goal"], policyValue(prevStep["goal"], "-")) &
          " (" & prevStatus & " -> " & curStatus & ")")
  if result.len == 0:
    result.add("- no status changes")

proc cmdTodo*(args: seq[string]): int =
  if args.len < 2:
    echo """Usage:
  vida-v0 todo ui-json <task_id>
  vida-v0 todo list <task_id>
  vida-v0 todo current <task_id>
  vida-v0 todo next <task_id>
  vida-v0 todo board <task_id>
  vida-v0 todo compact <task_id> [limit]
  vida-v0 todo tracks <task_id>
  vida-v0 todo sync <task_id> [--stdout-only] [--mode <full|json-only|delta|compact>] [--quiet] [--max-items <n>]"""
    return 1

  let cmd = args[0]
  let taskId = args[1]
  let steps = stepsJson(taskId)

  case cmd
  of "ui-json":
    echo $(%*{"task_id": taskId, "steps": steps})
    return 0
  of "list":
    for step in steps:
      let nextStep = policyValue(step["next_step"], "-")
      echo policyValue(step["block_id"], "") & " [" & policyValue(step["status"], "todo") & "] goal=" &
        policyValue(step["goal"], "-") & " next=" & nextStep & " track=" & policyValue(step["track_id"], "main")
    return 0
  of "current":
    var found = false
    for step in steps:
      if policyValue(step["status"], "") == "doing":
        echo policyValue(step["block_id"], "") & ": " & policyValue(step["goal"], "-")
        found = true
    if not found:
      echo "none"
    return 0
  of "next":
    var nextValue = "none"
    for step in steps:
      let candidate = policyValue(step["next_step"], "")
      if candidate.len > 0:
        nextValue = candidate
    echo nextValue
    return 0
  of "board":
    echo "TODO:    " & groupIds(steps, "todo")
    echo "DOING:   " & groupIds(steps, "doing")
    echo "PARTIAL: " & groupIds(steps, "partial")
    echo "DONE:    " & groupIds(steps, "done")
    echo "BLOCKED: " & groupIds(steps, "blocked")
    echo "SUPERSEDED: " & groupIds(steps, "superseded")
    return 0
  of "compact":
    let limit = if args.len > 2: parseInt(args[2]) else: 3
    for status in @["todo", "doing", "partial", "done", "blocked", "superseded"]:
      echo compactLine(steps, status, limit)
    return 0
  of "tracks":
    var grouped = initOrderedTable[string, seq[string]]()
    for step in steps:
      let trackId = policyValue(step["track_id"], "main")
      let nextStep = policyValue(step["next_step"], "")
      let suffix = if nextStep.len > 0: "->" & nextStep else: ""
      if not grouped.hasKey(trackId):
        grouped[trackId] = @[]
      grouped[trackId].add(policyValue(step["block_id"], "") & "[" & policyValue(step["status"], "todo") & "]" & suffix)
    for trackId, items in grouped:
      echo "TRACK " & trackId & ": " & items.join(", ")
    return 0
  of "sync":
    var stdoutOnly = false
    var mode = "full"
    var quiet = false
    var maxItems = 3
    var i = 2
    while i < args.len:
      case args[i]
      of "--stdout-only":
        stdoutOnly = true
        i += 1
      of "--mode":
        if i + 1 < args.len:
          mode = args[i + 1]
          i += 2
        else:
          return 1
      of "--quiet":
        quiet = true
        i += 1
      of "--max-items":
        if i + 1 < args.len:
          maxItems = parseInt(args[i + 1])
          i += 2
        else:
          return 1
      else:
        i += 1

    let (payload, jsonOut, stateOut) = writeSyncJson(taskId)
    if mode == "json-only":
      if stdoutOnly:
        echo $payload
      elif not quiet:
        echo "Snapshot JSON: " & jsonOut
      return 0

    if mode == "delta":
      var prevPayload = %*{"task_id": "", "steps": []}
      if fileExists(stateOut):
        prevPayload = loadJson(stateOut, prevPayload)
      if not quiet:
        echo "# TODO Delta Snapshot: " & taskId
        echo ""
        for line in syncDelta(prevPayload, payload):
          echo line
      saveJson(stateOut, payload)
      return 0

    if mode == "compact":
      if not quiet:
        echo "# TODO Compact Snapshot: " & taskId
        echo ""
        for status in @["todo", "doing", "partial", "done", "blocked", "superseded"]:
          echo compactLine(payload["steps"], status, maxItems)
        if not stdoutOnly:
          echo ""
          echo "Snapshot JSON: " & jsonOut
      saveJson(stateOut, payload)
      return 0

    if not quiet:
      echo "# TODO Sync Snapshot: " & taskId
      echo ""
      var blockIds: seq[string] = @[]
      for step in payload["steps"]:
        blockIds.add(policyValue(step["block_id"], ""))
      blockIds.sort(system.cmp[string])
      for blockId in blockIds:
        for step in payload["steps"]:
          if policyValue(step["block_id"], "") != blockId:
            continue
          let status = policyValue(step["status"], "todo")
          let mark = if status == "done": "x"
            elif status == "doing": ">"
            elif status in ["superseded", "partial"]: "~"
            elif status == "blocked": "!"
            else: " "
          echo "- [" & mark & "] " & blockId & " — " & policyValue(step["goal"], "-") &
            " (status=" & status & ")"
          break
      if not stdoutOnly:
        echo ""
        echo "Snapshot JSON: " & jsonOut
    saveJson(stateOut, payload)
    return 0
  else:
    echo "Unknown todo subcommand: " & cmd
    return 1
