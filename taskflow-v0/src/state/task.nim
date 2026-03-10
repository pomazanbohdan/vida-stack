## VIDA v0 task surface — DB-backed replacement for basic br reads.

import std/[json, strutils]
import ../core/[toon, turso_task_store, utils]

proc listIssues*(status = ""; includeAll = false): seq[JsonNode] =
  let payload = runTaskStore(@["list"] &
    (if status.len > 0: @["--status", status] else: @[]) &
    (if includeAll: @["--all"] else: @[]))
  if payload.kind == JArray:
    for item in payload:
      result.add(item)

proc resolveTaskIdByDisplayId*(displayId: string): JsonNode

proc showIssue*(taskId: string): JsonNode =
  let payload = runTaskStore(@["show", taskId])
  if payload.kind == JObject and policyValue(payload{"status"}, "") == "missing" and taskId.startsWith("vida-"):
    let resolved = resolveTaskIdByDisplayId(taskId)
    if resolved{"found"}.kind == JBool and resolved["found"].getBool():
      return runTaskStore(@["show", policyValue(resolved{"task_id"}, "")])
  payload

proc readyIssues*(): seq[JsonNode] =
  let payload = runTaskStore(@["ready"])
  if payload.kind == JArray:
    for item in payload:
      result.add(item)

proc importIssuesJsonl*(path: string): JsonNode =
  runTaskStore(@["import-jsonl", path])

proc exportIssuesJsonl*(path: string): JsonNode =
  runTaskStore(@["export-jsonl", path])

proc createIssue*(
  taskId: string,
  title: string,
  issueType = "task",
  status = "open",
  priority = 2,
  displayId = "",
  parentId = "",
  description = "",
  labels: seq[string] = @[],
): JsonNode =
  var args = @[
    "create",
    taskId,
    title,
    "--type", issueType,
    "--status", status,
    "--priority", $priority,
    "--display-id", displayId,
    "--parent-id", parentId,
  ]
  if description.len > 0:
    args.add(@["--description", description])
  for label in labels:
    args.add(@["--labels", label])
  runTaskStore(args)

proc updateIssue*(
  taskId: string,
  status = "",
  notes = "",
  description = "",
  addLabels: seq[string] = @[],
  removeLabels: seq[string] = @[],
  setLabels: seq[string] = @[],
): JsonNode =
  var args = @["update", taskId]
  if status.len > 0:
    args.add(@["--status", status])
  if notes.len > 0:
    args.add(@["--notes", notes])
  if description.len > 0:
    args.add(@["--description", description])
  for label in addLabels:
    args.add(@["--add-label", label])
  for label in removeLabels:
    args.add(@["--remove-label", label])
  for label in setLabels:
    args.add(@["--set-labels", label])
  runTaskStore(args)

proc closeIssue*(taskId: string, reason: string): JsonNode =
  runTaskStore(@["close", taskId, "--reason", reason])

proc printIssueJsonl(issue: JsonNode) =
  echo $issue

proc printIssueToon(issue: JsonNode) =
  echo renderToon(issue)

proc parseDisplayPath(displayId: string): tuple[valid: bool, root: string, levels: seq[int]] =
  let trimmed = displayId.strip()
  if not trimmed.startsWith("vida-"):
    return (false, "", @[])
  let parts = trimmed.split('.')
  if parts.len == 0:
    return (false, "", @[])
  let root = parts[0]
  if root.len <= 5:
    return (false, "", @[])
  var levels: seq[int] = @[]
  for idx in 1 ..< parts.len:
    if parts[idx].len == 0:
      return (false, "", @[])
    try:
      levels.add(parseInt(parts[idx]))
    except ValueError:
      return (false, "", @[])
  (true, root, levels)

proc nextDisplayId*(parentDisplayId: string): JsonNode =
  let parsed = parseDisplayPath(parentDisplayId)
  if not parsed.valid:
    return %*{
      "valid": false,
      "reason": "invalid_parent_display_id",
      "parent_display_id": parentDisplayId,
    }
  var maxChild = 0
  for issue in listIssues(includeAll = true):
    let displayId = policyValue(issue{"display_id"}, policyValue(issue{"id"}, ""))
    let child = parseDisplayPath(displayId)
    if not child.valid or child.root != parsed.root:
      continue
    if child.levels.len != parsed.levels.len + 1:
      continue
    if parsed.levels.len > 0 and child.levels[0 ..< parsed.levels.len] != parsed.levels:
      continue
    maxChild = max(maxChild, child.levels[^1])
  let nextIndex = maxChild + 1
  let nextDisplay = parentDisplayId & "." & $nextIndex
  %*{
    "valid": true,
    "parent_display_id": parentDisplayId,
    "next_display_id": nextDisplay,
    "next_index": nextIndex,
  }

proc resolveTaskIdByDisplayId*(displayId: string): JsonNode =
  for issue in listIssues(includeAll = true):
    if policyValue(issue{"display_id"}, policyValue(issue{"id"}, "")) == displayId:
      return %*{
        "found": true,
        "display_id": displayId,
        "task_id": policyValue(issue{"id"}, ""),
      }
  %*{
    "found": false,
    "display_id": displayId,
    "reason": "parent_display_id_not_found",
  }

proc cmdTask*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  taskflow-v0 task create <task_id> <title> [--type <issue_type>] [--status <status>] [--priority <n>] [--display-id <display_id>] [--parent-id <task_id>] [--parent-display-id <display_id>] [--auto-display-from <parent_display_id>] [--description <description>] [--labels <label>] [--json]
  taskflow-v0 task update <task_id> [--status <status>] [--notes <notes>] [--description <description>] [--add-label <label>] [--remove-label <label>] [--set-labels <label>] [--json]
  taskflow-v0 task close <task_id> --reason <reason> [--json]
  taskflow-v0 task import-jsonl <path> [--json]
  taskflow-v0 task export-jsonl <path> [--json]
  taskflow-v0 task list [--status <status>] [--all] [--json]
  taskflow-v0 task show <task_id> [--json|--jsonl]
  taskflow-v0 task next-display-id <parent_display_id> [--json]
  taskflow-v0 task ready [--json]"""
    return 1

  case args[0]
  of "create":
    if args.len < 3:
      echo "Usage: taskflow-v0 task create <task_id> <title> [--type <issue_type>] [--status <status>] [--priority <n>] [--display-id <display_id>] [--parent-id <task_id>] [--parent-display-id <display_id>] [--auto-display-from <parent_display_id>] [--description <description>] [--labels <label>] [--json]"
      return 1
    var issueType = "task"
    var status = "open"
    var priority = 2
    var displayId = ""
    var parentId = ""
    var parentDisplayId = ""
    var autoDisplayFrom = ""
    var description = ""
    var labels: seq[string] = @[]
    var asJson = false
    var i = 3
    while i < args.len:
      case args[i]
      of "--type":
        issueType = args[i + 1]
        i += 2
      of "--status":
        status = args[i + 1]
        i += 2
      of "--priority":
        priority = parseInt(args[i + 1])
        i += 2
      of "--display-id":
        displayId = args[i + 1]
        i += 2
      of "--parent-id":
        parentId = args[i + 1]
        i += 2
      of "--parent-display-id":
        parentDisplayId = args[i + 1]
        i += 2
      of "--auto-display-from":
        autoDisplayFrom = args[i + 1]
        i += 2
      of "--description":
        description = args[i + 1]
        i += 2
      of "--labels":
        labels.add(args[i + 1])
        i += 2
      of "--json":
        asJson = true
        i += 1
      else:
        i += 1
    if displayId.len == 0 and autoDisplayFrom.len > 0:
      let displayPayload = nextDisplayId(autoDisplayFrom)
      if displayPayload{"valid"}.kind == JBool and not displayPayload["valid"].getBool():
        if asJson:
          echo pretty(displayPayload)
        else:
          echo policyValue(displayPayload{"reason"}, "invalid_parent_display_id")
        return 1
      displayId = policyValue(displayPayload{"next_display_id"}, "")
    if parentId.len == 0 and parentDisplayId.len > 0:
      let parentPayload = resolveTaskIdByDisplayId(parentDisplayId)
      if parentPayload{"found"}.kind == JBool and not parentPayload["found"].getBool():
        if asJson:
          echo pretty(parentPayload)
        else:
          echo policyValue(parentPayload{"reason"}, "parent_display_id_not_found")
        return 1
      parentId = policyValue(parentPayload{"task_id"}, "")
    let payload = createIssue(
      args[1],
      args[2],
      issueType = issueType,
      status = status,
      priority = priority,
      displayId = displayId,
      parentId = parentId,
      description = description,
      labels = labels,
    )
    if asJson:
      echo pretty(payload)
    else:
      if policyValue(payload{"status"}, "") == "ok":
        echo renderToon(payload)
      else:
        echo policyValue(payload{"reason"}, "task_create_failed")
    return if policyValue(payload{"status"}, "") == "ok": 0 else: 1

  of "update":
    if args.len < 2:
      echo "Usage: taskflow-v0 task update <task_id> [--status <status>] [--notes <notes>] [--description <description>] [--add-label <label>] [--remove-label <label>] [--set-labels <label>] [--json]"
      return 1
    var status = ""
    var notes = ""
    var description = ""
    var addLabels: seq[string] = @[]
    var removeLabels: seq[string] = @[]
    var setLabels: seq[string] = @[]
    var asJson = false
    var i = 2
    while i < args.len:
      case args[i]
      of "--status":
        status = args[i + 1]
        i += 2
      of "--notes":
        notes = args[i + 1]
        i += 2
      of "--description":
        description = args[i + 1]
        i += 2
      of "--add-label":
        addLabels.add(args[i + 1])
        i += 2
      of "--remove-label":
        removeLabels.add(args[i + 1])
        i += 2
      of "--set-labels":
        setLabels.add(args[i + 1])
        i += 2
      of "--json":
        asJson = true
        i += 1
      else:
        i += 1
    let payload = updateIssue(
      args[1],
      status = status,
      notes = notes,
      description = description,
      addLabels = addLabels,
      removeLabels = removeLabels,
      setLabels = setLabels,
    )
    if policyValue(payload{"status"}, "") == "missing":
      if asJson:
        echo pretty(payload)
      else:
        echo "Missing task: " & args[1]
      return 1
    if asJson:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0

  of "close":
    if args.len < 4 or args[2] != "--reason":
      echo "Usage: taskflow-v0 task close <task_id> --reason <reason> [--json]"
      return 1
    let payload = closeIssue(args[1], args[3])
    let asJson = "--json" in args
    if policyValue(payload{"status"}, "") == "missing":
      if asJson:
        echo pretty(payload)
      else:
        echo "Missing task: " & args[1]
      return 1
    if asJson:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0

  of "import-jsonl":
    if args.len < 2:
      echo "Usage: taskflow-v0 task import-jsonl <path> [--json]"
      return 1
    let payload = importIssuesJsonl(args[1])
    let asJson = "--json" in args
    if asJson:
      echo pretty(payload)
    else:
      echo policyValue(payload{"status"}, "error") & ": imported=" &
           $policyInt(payload{"imported_count"}, 0) & " unchanged=" &
           $policyInt(payload{"unchanged_count"}, 0) & " updated=" &
           $policyInt(payload{"updated_count"}, 0)
    return if policyValue(payload{"status"}, "") == "ok": 0 else: 1

  of "export-jsonl":
    if args.len < 2:
      echo "Usage: taskflow-v0 task export-jsonl <path> [--json]"
      return 1
    let payload = exportIssuesJsonl(args[1])
    let asJson = "--json" in args
    if asJson:
      echo pretty(payload)
    else:
      echo policyValue(payload{"status"}, "error") & ": exported=" &
           $policyInt(payload{"exported_count"}, 0) & " target=" &
           policyValue(payload{"target_path"}, args[1])
    return if policyValue(payload{"status"}, "") == "ok": 0 else: 1

  of "list":
    var status = ""
    var includeAll = false
    var asJson = false
    var i = 1
    while i < args.len:
      case args[i]
      of "--status":
        status = args[i + 1]
        i += 2
      of "--all":
        includeAll = true
        i += 1
      of "--json":
        asJson = true
        i += 1
      else:
        i += 1
    let rows = listIssues(status, includeAll)
    if asJson:
      echo pretty(%rows)
    else:
      for issue in rows:
        printIssueJsonl(issue)
    return 0

  of "show":
    if args.len < 2:
      echo "Usage: taskflow-v0 task show <task_id> [--json|--jsonl]"
      return 1
    let payload = showIssue(args[1])
    let asJson = "--json" in args
    let asJsonl = "--jsonl" in args
    if payload.kind == JObject and policyValue(payload{"status"}, "") == "missing":
      if asJson:
        echo pretty(payload)
      else:
        echo "Missing task: " & args[1]
      return 1
    if asJson:
      echo pretty(payload)
    elif asJsonl:
      printIssueJsonl(payload)
    else:
      printIssueToon(payload)
    return 0

  of "ready":
    let asJson = "--json" in args
    let rows = readyIssues()
    if asJson:
      echo pretty(%rows)
    else:
      for issue in rows:
        printIssueJsonl(issue)
    return 0

  of "next-display-id":
    if args.len < 2:
      echo "Usage: taskflow-v0 task next-display-id <parent_display_id> [--json]"
      return 1
    let payload = nextDisplayId(args[1])
    let asJson = "--json" in args
    if payload{"valid"}.kind == JBool and not payload["valid"].getBool():
      if asJson:
        echo pretty(payload)
      else:
        echo policyValue(payload{"reason"}, "invalid_parent_display_id")
      return 1
    if asJson:
      echo pretty(payload)
    else:
      echo renderToon(payload)
    return 0

  else:
    echo "Unknown task subcommand: " & args[0]
    return 1

proc cmdBrCompat*(args: seq[string]): int =
  if args.len == 0:
    echo """Usage:
  taskflow-v0 br import [path] [--json]
  taskflow-v0 br export [path] [--json]

Defaults:
  import -> .beads/issues.jsonl
  export -> .beads/issues.jsonl"""
    return 1

  case args[0]
  of "import":
    let path = if args.len > 1 and not args[1].startsWith("--"): args[1] else: ".beads/issues.jsonl"
    let asJson = "--json" in args
    let payload = importIssuesJsonl(path)
    if asJson:
      echo pretty(payload)
    else:
      echo policyValue(payload{"status"}, "error") & ": imported=" &
           $policyInt(payload{"imported_count"}, 0) & " unchanged=" &
           $policyInt(payload{"unchanged_count"}, 0) & " updated=" &
           $policyInt(payload{"updated_count"}, 0)
    return if policyValue(payload{"status"}, "") == "ok": 0 else: 1

  of "export":
    let path = if args.len > 1 and not args[1].startsWith("--"): args[1] else: ".beads/issues.jsonl"
    let asJson = "--json" in args
    let payload = exportIssuesJsonl(path)
    if asJson:
      echo pretty(payload)
    else:
      echo policyValue(payload{"status"}, "error") & ": exported=" &
           $policyInt(payload{"exported_count"}, 0) & " target=" &
           policyValue(payload{"target_path"}, path)
    return if policyValue(payload{"status"}, "") == "ok": 0 else: 1

  else:
    echo "Usage: taskflow-v0 br <import|export> [path] [--json]"
    return 1
