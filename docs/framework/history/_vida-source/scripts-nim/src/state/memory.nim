## VIDA Framework Memory — persistent lessons, corrections, anomalies ledger.

import std/[json, os, strutils]
import ../core/[config, toon, utils]

const ValidKinds* = ["lesson", "correction", "anomaly"]

proc statePath*(): string =
  vidaRoot() / ".vida" / "state" / "framework-memory.json"

proc loadState*(): JsonNode =
  loadJson(statePath(), %*{
    "entries": [],
    "summary": {"lesson_count": 0, "correction_count": 0, "anomaly_count": 0}
  })

proc saveState*(payload: JsonNode) =
  saveJson(statePath(), payload)

proc recordEntry*(kind: string; summary: string; sourceTask: string = ""; details: JsonNode = newJObject()): JsonNode =
  let normalized = kind.strip().toLowerAscii()
  if normalized notin ValidKinds:
    raise newException(ValueError, "invalid framework memory kind: " & kind)
  var payload = loadState()
  let entry = %*{
    "ts": nowUtc(),
    "kind": normalized,
    "summary": summary.strip(),
    "source_task": sourceTask.strip(),
    "details": details,
  }
  if payload{"entries"}.isNil or payload{"entries"}.kind != JArray:
    payload["entries"] = newJArray()
  payload["entries"].add(entry)
  if payload{"summary"}.isNil or payload{"summary"}.kind != JObject:
    payload["summary"] = newJObject()
  let summaryKey = normalized & "_count"
  payload["summary"][summaryKey] = %(policyInt(payload["summary"]{summaryKey}, 0) + 1)
  saveState(payload)
  return entry

proc cmdMemory*(args: seq[string]): int =
  if args.len == 0:
    echo "Usage: vida-legacy memory <record|status>"
    return 1
  case args[0]
  of "status":
    let payload = normalizeJson(loadState())
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0
  of "record":
    if args.len < 2:
      echo "Usage: vida-legacy memory record <kind> --summary <text>"
      return 1
    let kind = args[1]
    var summary = ""
    var sourceTask = ""
    var details = newJObject()
    var i = 2
    while i < args.len:
      case args[i]
      of "--summary":
        if i + 1 < args.len:
          summary = args[i + 1]
          i += 2
        else:
          i += 1
      of "--source-task":
        if i + 1 < args.len:
          sourceTask = args[i + 1]
          i += 2
        else:
          i += 1
      of "--details-json":
        if i + 1 < args.len:
          try:
            details = parseJson(args[i + 1])
          except:
            details = newJObject()
          i += 2
        else:
          i += 1
      else:
        i += 1
    if summary.len == 0:
      echo "Missing --summary"
      return 1
    try:
      echo pretty(recordEntry(kind, summary, sourceTask, details))
      return 0
    except ValueError as e:
      echo e.msg
      return 1
  else:
    echo "Unknown memory subcommand: " & args[0]
    return 1
