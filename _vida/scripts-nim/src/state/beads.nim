## VIDA Beads Runtime — JSONL-based task state management.
##
## Replaces `beads-runtime.sh` (204 lines) + `beads-verify-runtime.py` (266 lines)
## + `beads-log.sh` + `beads-compact.sh` + `beads-workflow.sh`.
## Single-writer rule: read-only ops use br directly,
## mutations route through queue-backed writer.

import std/[json, os, strutils, times, algorithm, sequtils, options]
import ../core/[utils, config, toon]

# ─────────────────────────── Paths ───────────────────────────

proc beadsDir*(): string = vidaRoot() / ".beads"
proc issuesJsonlPath*(): string = beadsDir() / "issues.jsonl"
proc backupDir*(): string = beadsDir() / "backups"
proc modeFilePath*(): string = beadsDir() / "runtime-mode.json"
proc beadsLogPath*(): string = vidaRoot() / ".vida" / "logs" / "beads-execution.jsonl"

const DefaultMode* = "jsonl_safe"

# ─────────────────────────── Mode Management ───────────────────────────

proc ensureModeFile() =
  createDir(beadsDir())
  if not fileExists(modeFilePath()):
    saveJson(modeFilePath(), %*{
      "mode": DefaultMode,
      "updated_at": nowUtc(),
      "reason": "auto-init",
    })

proc beadsMode*(): string =
  ensureModeFile()
  let payload = loadJson(modeFilePath())
  policyValue(payload{"mode"}, DefaultMode)

proc setBeadsMode*(mode, reason: string) =
  ensureModeFile()
  # Force jsonl_safe — direct mode is denied
  var effectiveMode = DefaultMode
  var effectiveReason = reason
  case mode
  of "jsonl_safe", "": discard
  of "direct":
    effectiveReason = "direct-request-denied: " & reason
  else:
    effectiveReason = "unknown-mode-" & mode & ": " & reason

  let payload = %*{
    "mode": effectiveMode,
    "updated_at": nowUtc(),
    "reason": effectiveReason,
  }
  let tmpPath = modeFilePath() & ".tmp"
  saveJson(tmpPath, payload)
  moveFile(tmpPath, modeFilePath())

# ─────────────────────────── JSONL Snapshot ───────────────────────────

proc snapshotJsonl*(reason: string = "manual"): string =
  ## Create timestamped backup of issues.jsonl.
  let issuesPath = issuesJsonlPath()
  if not fileExists(issuesPath):
    raise newException(IOError, "Missing JSONL source: " & issuesPath)

  createDir(backupDir())
  let stamp = nowUtcDt().format("yyyyMMdd'-'HHmmss")
  let dest = backupDir() / ("issues-" & stamp & ".jsonl")
  let latest = backupDir() / "latest.jsonl"

  copyFile(issuesPath, dest)
  copyFile(dest, latest)

  saveJson(backupDir() / "latest.meta.json", %*{
    "mode": beadsMode(),
    "reason": reason,
    "snapshot": dest,
    "updated_at": nowUtc(),
  })
  return dest

proc latestSnapshotPath*(): string = backupDir() / "latest.jsonl"

proc snapshotAgeSeconds*(): int =
  let latest = latestSnapshotPath()
  if not fileExists(latest): return -1
  let info = getFileInfo(latest)
  let age = getTime() - info.lastWriteTime
  return int(age.inSeconds)

# ─────────────────────────── JSONL Stats ───────────────────────────

proc jsonlStats*(): JsonNode =
  let issuesPath = issuesJsonlPath()
  if not fileExists(issuesPath):
    return %*{"path": "", "total": 0, "unique": 0, "duplicates": 0}

  var total = 0
  var ids: seq[string] = @[]
  for line in lines(issuesPath):
    if line.strip().len == 0: continue
    total += 1
    try:
      let payload = parseJson(line)
      let id = policyValue(payload{"id"}, "")
      if id.len > 0: ids.add(id)
    except: discard

  ids.sort()
  var unique = 0
  var prev = ""
  for id in ids:
    if id != prev: unique += 1
    prev = id

  %*{
    "path": issuesPath,
    "total": total,
    "unique": unique,
    "duplicates": total - unique,
  }

# ─────────────────────────── Log Verification ───────────────────────────

type VerifyResult* = object
  entryCount*: int
  criticalCount*: int
  warnCount*: int

proc readBeadsLog(taskId: string): seq[JsonNode] =
  let logPath = beadsLogPath()
  if not fileExists(logPath): return @[]
  for line in lines(logPath):
    if line.strip().len == 0: continue
    try:
      let event = parseJson(line)
      if policyValue(event{"task_id"}, "") == taskId or
         policyValue(event{"task_before"}, "") == taskId or
         policyValue(event{"task_after"}, "") == taskId:
        result.add(event)
    except: discard

proc verifyTaskLog*(taskId: string, strict: bool = false,
                    assumptionHours: int = 8): VerifyResult =
  let logs = readBeadsLog(taskId)
  result.entryCount = logs.len

  if logs.len == 0:
    if strict: result.criticalCount += 1
    return

  # Check block_end(done) missing next_step
  for event in logs:
    if policyValue(event{"type"}, "") == "block_end" and
       policyValue(event{"result"}, "") == "done" and
       policyValue(event{"next_step"}, "").len == 0:
      result.criticalCount += 1

  # Check compact_post task switch without recovery_action
  for event in logs:
    if policyValue(event{"type"}, "") == "compact_post" and
       policyValue(event{"task_before"}, "") != policyValue(event{"task_after"}, "") and
       policyValue(event{"recovery_action"}, "").len == 0:
      result.criticalCount += 1

  # Check stale assumptions
  let now = getTime()
  let threshold = initDuration(hours = assumptionHours)
  for event in logs:
    if policyValue(event{"type"}, "") != "block_end": continue
    if policyValue(event{"assumptions"}, "").len == 0: continue
    let tsRaw = policyValue(event{"ts_end"}, policyValue(event{"ts"}, ""))
    let ts = parseUtcTimestamp(tsRaw)
    if ts.isSome and now - ts.get.toTime > threshold:
      result.warnCount += 1

  # Check missing evidence
  for event in logs:
    if policyValue(event{"type"}, "") == "block_end" and
       policyValue(event{"actions"}, "").len > 0 and
       policyValue(event{"artifacts"}, "").len == 0 and
       policyValue(event{"evidence_ref"}, "").len == 0:
      result.warnCount += 1

  if strict:
    let blockEnds = logs.filterIt(policyValue(it{"type"}, "") == "block_end").len
    if blockEnds == 0: result.criticalCount += 1
    let reflections = logs.filterIt(policyValue(it{"type"}, "") == "self_reflection").len
    if reflections == 0: result.criticalCount += 1

# ─────────────────────────── CLI ───────────────────────────

proc cmdBeads*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  vida-legacy beads mode
  vida-legacy beads set-mode <mode> [reason]
  vida-legacy beads snapshot [reason]
  vida-legacy beads snapshot-age
  vida-legacy beads jsonl-stats
  vida-legacy beads verify --task <id> [--strict] [--assumption-hours N]"""
    return 1

  case args[0]
  of "mode":
    echo beadsMode(); return 0

  of "set-mode":
    if args.len < 2: echo "Usage: vida-legacy beads set-mode <mode> [reason]"; return 1
    let reason = if args.len > 2: args[2] else: "manual"
    setBeadsMode(args[1], reason); return 0

  of "snapshot":
    let reason = if args.len > 1: args[1] else: "manual"
    try:
      let path = snapshotJsonl(reason); echo path; return 0
    except IOError as e:
      echo e.msg; return 2

  of "snapshot-age":
    echo $snapshotAgeSeconds(); return 0

  of "jsonl-stats":
    let payload = normalizeJson(jsonlStats())
    if "--json" in args: echo pretty(payload) else: echo renderToon(payload)
    return 0

  of "verify":
    var taskId = ""
    var strict = false
    var assumptionHours = 8
    var i = 1
    while i < args.len:
      case args[i]
      of "--task": taskId = args[i+1]; i += 2
      of "--strict": strict = true; i += 1
      of "--assumption-hours": assumptionHours = parseInt(args[i+1]); i += 2
      else: i += 1
    if taskId.len == 0: echo "Missing --task"; return 1
    let res = verifyTaskLog(taskId, strict, assumptionHours)
    let icon = if res.criticalCount > 0: "❌" elif res.warnCount > 0: "⚠️" else: "✅"
    echo icon & " task=" & taskId & " entries=" & $res.entryCount &
         " critical=" & $res.criticalCount & " warnings=" & $res.warnCount
    return if res.criticalCount > 0: 2 else: 0

  else:
    echo "Unknown beads subcommand: " & args[0]; return 1
