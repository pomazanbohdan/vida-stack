## VIDA Boot Snapshot — task state snapshot for boot paths.
##
## Replaces `vida-boot-snapshot.py` (379 lines).
## Queries the vida-v0 task store for current task state, enriches with run-graph
## and reconciliation status, renders as JSON or compact text.

import std/[json, os, strutils, algorithm, sequtils, times, options]
import ../core/[utils, config, toon]
import ../state/task

# ─────────────────────────── Issue Helpers ───────────────────────────

proc issueMode*(issue: JsonNode): string =
  let labels = issue{"labels"}
  if not labels.isNil and labels.kind == JArray:
    for label in labels:
      if label.getStr() == "mode:decision_required":
        return "decision_required"
      if label.getStr() == "mode:autonomous":
        return "autonomous"
  return "auto"

proc issuePriority*(issue: JsonNode): int =
  policyInt(issue{"priority"}, 999)

proc parseIssueTimestamp(value: JsonNode): float =
  let text = policyValue(value, "").strip()
  if text.len == 0:
    return 0.0
  let dt = parseUtcTimestamp(text)
  if dt.isSome:
    return dt.get.toTime.toUnixFloat()
  return 0.0

type IssueSortKey = tuple[priority: int, negUpdated: float, negCreated: float, id: string]

proc issueSortKey*(issue: JsonNode): IssueSortKey =
  (
    issuePriority(issue),
    -parseIssueTimestamp(issue{"updated_at"}),
    -parseIssueTimestamp(issue{"created_at"}),
    policyValue(issue{"id"}, ""),
  )

proc hasParentIssue(issue: JsonNode): bool =
  let deps = issue{"dependencies"}
  if deps.isNil or deps.kind != JArray:
    return policyValue(issue{"parent"}, "").len > 0
  for dep in deps:
    if policyValue(dep{"type"}, "") == "parent-child":
      return true
  return policyValue(issue{"parent"}, "").len > 0

proc topLevel(rows: seq[JsonNode]): seq[JsonNode] =
  rows.filterIt(not hasParentIssue(it))

proc uniqueById(rows: seq[JsonNode]): seq[JsonNode] =
  var seen: seq[string] = @[]
  for row in rows:
    let id = policyValue(row{"id"}, "")
    if id.len == 0 or id in seen:
      continue
    seen.add(id)
    result.add(row)

# ─────────────────────────── Run Graph / Reconciliation ───────────────────────────

proc runGraphStatus(taskId: string, stateDir: string): JsonNode =
  ## Load run-graph status for a task (reads from state files).
  let graphDir = stateDir / "run-graphs"
  let path = graphDir / (taskId & ".json")
  if not fileExists(path):
    return %*{"present": false}
  let payload = loadJson(path)
  if payload.kind != JObject or not payload.hasKey("nodes"):
    return %*{"present": false}

  # Build resume hint
  let resumePriority = @["analysis", "writer", "coach", "problem_party",
                         "verifier", "approval", "synthesis"]
  var hint = %*{"next_node": "", "status": "completed", "reason": "no_resumable_node"}
  let nodes = payload["nodes"]
  for node in resumePriority:
    if nodes.hasKey(node):
      let status = policyValue(nodes[node]{"status"}, "")
      if status in ["blocked", "failed", "running", "ready"]:
        let reason = policyValue(nodes[node]{"meta"}{"reason"}, "")
        hint = %*{"next_node": node, "status": status, "reason": reason}
        break

  return %*{"present": true, "resume_hint": hint}

proc reconciliationStatus(taskId: string, stateDir: string): JsonNode =
  let path = stateDir / "reconciliation" / (taskId & ".json")
  if not fileExists(path):
    return %*{"classification": "", "allowed_actions": []}
  let payload = loadJson(path)
  return %*{
    "classification": policyValue(payload{"classification"}, ""),
    "allowed_actions": payload{"allowed_actions"},
  }

# ─────────────────────────── Issue Entry Builder ───────────────────────────

proc childEntriesFor(issueId: string): seq[JsonNode] =
  ## Get open/in-progress children of an issue via task-store dependencies.
  for dep in listIssues(includeAll = true):
    let status = policyValue(dep{"status"}, "")
    if status notin ["open", "in_progress"]:
      continue
    let deps = dep{"dependencies"}
    if deps.isNil or deps.kind != JArray:
      continue
    for edge in deps:
      if policyValue(edge{"type"}, "") != "parent-child":
        continue
      if policyValue(edge{"depends_on_id"}, "") != issueId:
        continue
      result.add(%*{
        "id": dep{"id"},
        "title": dep{"title"},
        "status": dep{"status"},
        "priority": dep{"priority"},
        "updated_at": dep{"updated_at"},
        "created_at": dep{"created_at"},
      })
      break
  result.sort(proc(a, b: JsonNode): int =
    let aInProg = if policyValue(a{"status"}, "") == "in_progress": 0 else: 1
    let bInProg = if policyValue(b{"status"}, "") == "in_progress": 0 else: 1
    if aInProg != bInProg: return cmp(aInProg, bInProg)
    return cmp(issueSortKey(a), issueSortKey(b))
  )

proc issueEntry(issue: JsonNode, subtasksLimit: int, stateDir: string): JsonNode =
  let issueId = policyValue(issue{"id"}, "")
  let subtasks = childEntriesFor(issueId)
  let shownSubtasks = subtasks[0 ..< min(subtasksLimit, subtasks.len)]
  let rg = runGraphStatus(issueId, stateDir)
  let recon = reconciliationStatus(issueId, stateDir)
  result = %*{
    "id": issue{"id"},
    "title": cleanText(policyValue(issue{"title"}, "")),
    "status": issue{"status"},
    "mode": issueMode(issue),
    "priority": issuePriority(issue),
    "updated_at": issue{"updated_at"},
    "run_graph": rg,
    "reconciliation": recon,
    "subtasks": shownSubtasks,
    "hidden_subtasks": max(0, subtasks.len - shownSubtasks.len),
  }

# ─────────────────────────── Snapshot Builder ───────────────────────────

proc buildSnapshot*(topLimit: int = 5, readyLimit: int = 3,
                    subtasksLimit: int = 5): JsonNode =
  let stateDir = vidaWorkspacePath("state")

  let openRows = listIssues(status = "open")
  let doingRows = listIssues(status = "in_progress")
  let blockedRows = listIssues(status = "blocked")
  let readyRows = readyIssues()

  var topOpen = topLevel(openRows)
  var topDoing = topLevel(doingRows)
  var topBlocked = topLevel(blockedRows)
  var topReady = uniqueById(topLevel(readyRows))

  topOpen.sort(proc(a, b: JsonNode): int = cmp(issueSortKey(a), issueSortKey(b)))
  topDoing.sort(proc(a, b: JsonNode): int = cmp(issueSortKey(a), issueSortKey(b)))
  topBlocked.sort(proc(a, b: JsonNode): int = cmp(issueSortKey(a), issueSortKey(b)))
  topReady.sort(proc(a, b: JsonNode): int = cmp(issueSortKey(a), issueSortKey(b)))

  let topReadyOpen = topReady.filterIt(policyValue(it{"status"}, "") == "open")
  let topReadyInProgress = topReady.filterIt(policyValue(it{"status"}, "") == "in_progress")

  var inProgress: seq[JsonNode] = @[]
  for row in topDoing[0 ..< min(topLimit, topDoing.len)]:
    inProgress.add(issueEntry(row, subtasksLimit, stateDir))

  var readyHead: seq[JsonNode] = @[]
  for row in topReadyOpen[0 ..< min(readyLimit, topReadyOpen.len)]:
    readyHead.add(issueEntry(row, subtasksLimit, stateDir))

  var decisionRequired: seq[JsonNode] = @[]
  for row in uniqueById(topDoing & topOpen & topBlocked):
    if issueMode(row) == "decision_required":
      decisionRequired.add(%*{
        "id": row{"id"},
        "title": cleanText(policyValue(row{"title"}, "")),
        "status": row{"status"},
        "priority": issuePriority(row),
        "updated_at": row{"updated_at"},
      })

  let cfg = loadRawConfig()
  let fsd = getFrameworkSelfDiagnosis(cfg)
  var activeRunGraphs = 0
  for item in inProgress:
    if dottedGetBool(item, "run_graph.present", false):
      activeRunGraphs += 1

  result = %*{
    "generated_at": nowUtc(),
    "execution_continue_default": {
      "mode": "route_then_external_analysis",
      "summary": "For write-producing continuation work in hybrid mode, stop after compact task snapshot, build the route receipt, and obtain the external analysis receipt before writer dispatch.",
      "selection_policy": "prefer explicit priority first, then most recently updated work within the same priority band",
      "compact_assumption": "compact_can_happen_any_time",
    },
    "summary": {
      "top_level_in_progress": topDoing.len,
      "top_level_open": topOpen.len,
      "top_level_blocked": topBlocked.len,
      "ready_total": topReady.len,
      "ready_open": topReadyOpen.len,
      "ready_in_progress": topReadyInProgress.len,
      "active_run_graphs": activeRunGraphs,
    },
    "framework_self_diagnosis": fsd,
    "in_progress": inProgress,
    "ready_head": readyHead,
    "decision_required": decisionRequired,
    "limits": {
      "top_level": topLimit,
      "ready_head": readyLimit,
      "subtasks": subtasksLimit,
    },
  }

# ─────────────────────────── Text Renderer ───────────────────────────

proc renderSection(lines: var seq[string], name: string, items: seq[JsonNode]) =
  if items.len == 0:
    return
  lines.add("")
  lines.add(name & ":")
  for item in items:
    var suffix = ""
    if policyValue(item{"mode"}, "") == "decision_required":
      suffix = "  [decision_required]"
    lines.add("- " & policyValue(item{"id"}, "?") & "  " &
              policyValue(item{"title"}, "-") & suffix)

    let rg = item{"run_graph"}
    if not rg.isNil and dottedGetBool(rg, "present", false):
      let hint = rg{"resume_hint"}
      if not hint.isNil and hint.kind == JObject:
        let nextNode = policyValue(hint{"next_node"}, "-")
        let status = policyValue(hint{"status"}, "-")
        let reason = policyValue(hint{"reason"}, "")
        var detail = "  - run_graph: next=" & nextNode & " status=" & status
        if reason.len > 0:
          detail &= " reason=" & reason
        lines.add(detail)

    let subtasks = item{"subtasks"}
    if not subtasks.isNil and subtasks.kind == JArray:
      for child in subtasks:
        lines.add("  - [" & policyValue(child{"status"}, "?") & "] " &
                  policyValue(child{"id"}, "?") & "  " &
                  policyValue(child{"title"}, "-"))
    let hidden = policyInt(item{"hidden_subtasks"}, 0)
    if hidden > 0:
      lines.add("  - +" & $hidden & " more")

proc renderText*(snapshot: JsonNode): string =
  let summary = snapshot["summary"]
  let fsd = snapshot{"framework_self_diagnosis"}
  var lines = @[
    "VIDA BOOT SNAPSHOT",
    "execution_continue_default: " & policyValue(snapshot{"execution_continue_default"}{"summary"}, ""),
    "task_selection_policy: " & policyValue(snapshot{"execution_continue_default"}{"selection_policy"}, ""),
    "compact_assumption: " & policyValue(snapshot{"execution_continue_default"}{"compact_assumption"}, ""),
    "summary: in_progress=" & $summary["top_level_in_progress"].getInt() &
      " open=" & $summary["top_level_open"].getInt() &
      " blocked=" & $summary["top_level_blocked"].getInt() &
      " ready_total=" & $summary["ready_total"].getInt() &
      " ready_open=" & $summary["ready_open"].getInt() &
      " active_run_graphs=" & $summary["active_run_graphs"].getInt(),
  ]

  if not fsd.isNil and dottedGetBool(fsd, "enabled", false):
    lines.add("framework_self_diagnosis: " &
      "silent_mode=" & $dottedGetBool(fsd, "silent_mode", false) &
      " auto_capture_bugs=" & $dottedGetBool(fsd, "auto_capture_bugs", false) &
      " defer_fix_until_task_boundary=" & $dottedGetBool(fsd, "defer_fix_until_task_boundary", false) &
      " platform_direction=" & dottedGetStr(fsd, "platform_direction", "-") &
      " quality_token_efficiency=" & dottedGetStr(fsd, "quality_token_efficiency", "-"))

  let ipItems = snapshot{"in_progress"}
  if not ipItems.isNil and ipItems.kind == JArray:
    var items: seq[JsonNode] = @[]
    for item in ipItems: items.add(item)
    renderSection(lines, "in_progress", items)

  let rhItems = snapshot{"ready_head"}
  if not rhItems.isNil and rhItems.kind == JArray:
    var items: seq[JsonNode] = @[]
    for item in rhItems: items.add(item)
    renderSection(lines, "ready_head", items)

  let drItems = snapshot{"decision_required"}
  if not drItems.isNil and drItems.kind == JArray and drItems.len > 0:
    lines.add("")
    lines.add("decision_required:")
    for item in drItems:
      lines.add("- [" & policyValue(item{"status"}, "?") & "] " &
                policyValue(item{"id"}, "?") & "  " &
                policyValue(item{"title"}, "-"))

  return lines.join("\n")

# ─────────────────────────── CLI Command ───────────────────────────

proc cmdSnapshot*(args: seq[string]): int =
  var jsonMode = false
  var textMode = false
  var topLimit = 5
  var readyLimit = 3
  var subtasksLimit = 5

  var i = 0
  while i < args.len:
    case args[i]
    of "--json": jsonMode = true
    of "--text": textMode = true
    of "--top-limit":
      if i + 1 < args.len:
        topLimit = parseInt(args[i + 1])
        i += 1
    of "--ready-limit":
      if i + 1 < args.len:
        readyLimit = parseInt(args[i + 1])
        i += 1
    of "--subtasks-limit":
      if i + 1 < args.len:
        subtasksLimit = parseInt(args[i + 1])
        i += 1
    else: discard
    i += 1

  let snapshot = buildSnapshot(topLimit, readyLimit, subtasksLimit)
  if jsonMode:
    echo pretty(snapshot)
  elif textMode:
    echo renderText(snapshot)
  else:
    echo renderToon(snapshot)
  return 0
