## VIDA Context Governance — source classes, freshness, provenance ledger.

import std/[json, os, strutils, tables]
import ../core/[config, toon, utils]

const ValidSourceClasses* = [
  "local_repo",
  "local_runtime",
  "overlay_declared",
  "web_validated",
  "external_connector",
]

proc statePath*(): string =
  vidaRoot() / ".vida" / "state" / "context-governance.json"

proc loadState*(): JsonNode =
  loadJson(statePath(), %*{
    "entries": [],
    "summary": {"by_source_class": {}, "task_count": 0, "web_validated_count": 0}
  })

proc saveState*(payload: JsonNode) =
  saveJson(statePath(), payload)

proc validSourceClass*(value: string): string =
  let normalized = value.strip().toLowerAscii()
  if normalized notin ValidSourceClasses:
    raise newException(ValueError, "invalid source class: " & value)
  normalized

proc normalizeSources*(items: JsonNode): JsonNode =
  result = newJArray()
  var seen = initTable[string, bool]()
  if items.kind != JArray:
    return
  for item in items:
    if item.kind != JObject:
      continue
    let sourceClass = validSourceClass(policyValue(item["source_class"], ""))
    let path = policyValue(item["path"], "")
    if path.len == 0:
      continue
    let dedupeKey = sourceClass & "::" & path
    if seen.hasKey(dedupeKey):
      continue
    seen[dedupeKey] = true
    let explicitFreshness = dottedGetStr(item, "freshness")
    let explicitProvenance = dottedGetStr(item, "provenance")
    let explicitRoleScope = dottedGetStr(item, "role_scope")
    result.add(%*{
      "source_class": sourceClass,
      "path": path,
      "freshness": (if explicitFreshness.len > 0: explicitFreshness elif sourceClass == "web_validated": "validated" else: "current"),
      "provenance": (if explicitProvenance.len > 0: explicitProvenance else: sourceClass),
      "role_scope": (if explicitRoleScope.len > 0: explicitRoleScope else: "orchestrator"),
      "notes": dottedGetStr(item, "notes"),
    })

proc summarizeSources*(items: JsonNode): JsonNode =
  var bySourceClass = newJObject()
  var roleScopes = newJObject()
  var freshness = newJObject()
  if items.kind == JArray:
    for item in items:
      let sourceClass = policyValue(item["source_class"], "")
      let roleScope = policyValue(item["role_scope"], "")
      let freshnessValue = policyValue(item["freshness"], "")
      if sourceClass.len > 0:
        bySourceClass[sourceClass] = %(policyInt(bySourceClass{sourceClass}, 0) + 1)
      if roleScope.len > 0:
        roleScopes[roleScope] = %(policyInt(roleScopes{roleScope}, 0) + 1)
      if freshnessValue.len > 0:
        freshness[freshnessValue] = %(policyInt(freshness{freshnessValue}, 0) + 1)
  %*{
    "by_source_class": bySourceClass,
    "role_scopes": roleScopes,
    "freshness": freshness,
    "web_validated_count": policyInt(bySourceClass{"web_validated"}, 0),
    "source_count": (if items.kind == JArray: items.len else: 0),
  }

proc validateSources*(items: JsonNode): JsonNode =
  try:
    let normalized = normalizeSources(items)
    if normalized.len == 0:
      return %*{"valid": false, "reason": "missing_context_sources", "sources": []}
    for item in normalized:
      if policyValue(item["source_class"], "") == "web_validated" and policyValue(item["freshness"], "") notin ["validated", "current"]:
        return %*{"valid": false, "reason": "invalid_web_validated_freshness", "sources": normalized}
    return %*{"valid": true, "reason": "", "sources": normalized, "summary": summarizeSources(normalized)}
  except ValueError as e:
    return %*{"valid": false, "reason": e.msg, "sources": []}

proc recordEntry*(taskId, phase: string, sources: JsonNode, notes: string = ""): JsonNode =
  var payload = loadState()
  let normalizedSources = normalizeSources(sources)
  let entry = %*{
    "ts": nowUtc(),
    "task_id": taskId.strip(),
    "phase": phase.strip(),
    "sources": normalizedSources,
    "summary": summarizeSources(normalizedSources),
    "notes": notes.strip(),
  }
  if payload{"entries"}.isNil or payload{"entries"}.kind != JArray:
    payload["entries"] = newJArray()
  payload["entries"].add(entry)

  var aggregateCounts = newJObject()
  var webValidatedCount = 0
  var tasks = initTable[string, bool]()
  for row in payload["entries"]:
    let rowTaskId = policyValue(row["task_id"], "")
    if rowTaskId.len > 0:
      tasks[rowTaskId] = true
    if row{"sources"}.kind == JArray:
      for source in row["sources"]:
        let sourceClass = policyValue(source["source_class"], "")
        if sourceClass.len == 0:
          continue
        aggregateCounts[sourceClass] = %(policyInt(aggregateCounts{sourceClass}, 0) + 1)
        if sourceClass == "web_validated":
          webValidatedCount += 1

  payload["summary"] = %*{
    "by_source_class": aggregateCounts,
    "task_count": tasks.len,
    "web_validated_count": webValidatedCount,
    "last_recorded_at": entry["ts"],
  }
  saveState(payload)
  return entry

proc cmdContext*(args: seq[string]): int =
  if args.len == 0:
    echo "Usage: taskflow-v0 context <status|validate|record>"
    return 1
  case args[0]
  of "status":
    let payload = normalizeJson(loadState())
    if "--json" in args:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0
  of "validate":
    var sourcesJson = ""
    var i = 1
    while i < args.len:
      if args[i] == "--sources-json" and i + 1 < args.len:
        sourcesJson = args[i + 1]
        i += 2
      else:
        i += 1
    if sourcesJson.len == 0:
      echo "Missing --sources-json"
      return 1
    let payload = try: parseJson(sourcesJson) except: newJArray()
    let resultPayload = validateSources(payload)
    let normalized = normalizeJson(resultPayload)
    if "--json" in args:
      echo pretty(normalized)
    else:
      echo renderToon(normalized)
    return if dottedGetBool(resultPayload, "valid", false): 0 else: 2
  of "record":
    var taskId = ""
    var phase = ""
    var sourcesJson = ""
    var notes = ""
    var i = 1
    while i < args.len:
      case args[i]
      of "--task-id":
        if i + 1 < args.len: taskId = args[i + 1]
        i += 2
      of "--phase":
        if i + 1 < args.len: phase = args[i + 1]
        i += 2
      of "--sources-json":
        if i + 1 < args.len: sourcesJson = args[i + 1]
        i += 2
      of "--notes":
        if i + 1 < args.len: notes = args[i + 1]
        i += 2
      else:
        i += 1
    if taskId.len == 0 or phase.len == 0 or sourcesJson.len == 0:
      echo "Missing required args"
      return 1
    let payload = try: parseJson(sourcesJson) except: newJArray()
    try:
      echo pretty(recordEntry(taskId, phase, payload, notes))
      return 0
    except ValueError as e:
      echo e.msg
      return 1
  else:
    echo "Unknown context subcommand: " & args[0]
    return 1
