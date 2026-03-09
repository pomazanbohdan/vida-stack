## Tests for state/task module

import std/[json, os, osproc, sequtils, strtabs, strutils, unittest]
import ../src/state/task

suite "task":
  let root = "/tmp/vida_scripts_nim_task"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)
  putEnv("VIDA_LEGACY_TURSO_PYTHON", getCurrentDir() / ".venv" / "bin" / "python3")

  proc seedIssues(lines: seq[JsonNode]) =
    let dbPath = root / ".vida" / "state" / "vida-legacy.db"
    let issuesPath = root / ".beads" / "issues.jsonl"
    if fileExists(dbPath):
      removeFile(dbPath)
    createDir(parentDir(issuesPath))
    writeFile(issuesPath, "")
    for line in lines:
      writeFile(issuesPath, readFile(issuesPath) & $line & "\n")
    discard importIssuesJsonl(issuesPath)

  test "list excludes closed by default":
    seedIssues(@[
      %*{"id": "vida-open", "title": "Open", "status": "open", "priority": 2, "issue_type": "task"},
      %*{"id": "vida-closed", "title": "Closed", "status": "closed", "priority": 1, "issue_type": "task"},
    ])
    let rows = listIssues()
    check rows.len == 1
    check rows[0]["id"].getStr() == "vida-open"

  test "show returns payload for existing task":
    seedIssues(@[
      %*{"id": "vida-show", "display_id": "vida-2d9.1", "title": "Show me", "status": "open", "priority": 1, "issue_type": "task"}
    ])
    let row = showIssue("vida-show")
    check row["title"].getStr() == "Show me"
    check row["display_id"].getStr() == "vida-2d9.1"

  test "show resolves display id fallback":
    seedIssues(@[
      %*{"id": "vida-show", "display_id": "vida-2d9.1", "title": "Show me", "status": "open", "priority": 1, "issue_type": "task"}
    ])
    let row = showIssue("vida-2d9.1")
    check row["id"].getStr() == "vida-show"
    check row["title"].getStr() == "Show me"

  test "display_id falls back to id and sorts before raw id":
    seedIssues(@[
      %*{"id": "vida-c", "display_id": "vida-2d9.10", "title": "Task C", "status": "open", "priority": 2, "issue_type": "task"},
      %*{"id": "vida-b", "display_id": "vida-2d9.2", "title": "Task B", "status": "open", "priority": 2, "issue_type": "task"},
      %*{"id": "vida-a", "display_id": "vida-2d9.1", "title": "Task A", "status": "open", "priority": 2, "issue_type": "task"},
      %*{"id": "vida-d", "title": "Task D", "status": "open", "priority": 2, "issue_type": "task"},
    ])
    let rows = listIssues()
    check rows[0]["display_id"].getStr() == "vida-2d9.1"
    check rows[1]["display_id"].getStr() == "vida-2d9.2"
    check rows[2]["display_id"].getStr() == "vida-2d9.10"
    check rows[3]["display_id"].getStr() == "vida-d"

  test "ready ignores epics and blocked dependencies":
    seedIssues(@[
      %*{"id": "vida-root", "title": "Root", "status": "open", "priority": 1, "issue_type": "epic"},
      %*{"id": "vida-a", "title": "Task A", "status": "open", "priority": 2, "issue_type": "task",
         "dependencies": [%*{"issue_id": "vida-a", "depends_on_id": "vida-root", "type": "parent-child"}]},
      %*{"id": "vida-b", "title": "Task B", "status": "open", "priority": 3, "issue_type": "task",
         "dependencies": [%*{"issue_id": "vida-b", "depends_on_id": "vida-a", "type": "blocks"}]},
      %*{"id": "vida-c", "title": "Task C", "status": "in_progress", "priority": 4, "issue_type": "task"}
    ])
    let rows = readyIssues()
    check rows.len == 2
    check rows[0]["id"].getStr() == "vida-c"
    check rows[1]["id"].getStr() == "vida-a"

  test "next display id advances one level under parent":
    seedIssues(@[
      %*{"id": "vida-root", "display_id": "vida-2d9", "title": "Root", "status": "open", "priority": 1, "issue_type": "epic"},
      %*{"id": "vida-a", "display_id": "vida-2d9.1", "title": "Task A", "status": "open", "priority": 2, "issue_type": "task"},
      %*{"id": "vida-b", "display_id": "vida-2d9.3", "title": "Task B", "status": "open", "priority": 2, "issue_type": "task"},
      %*{"id": "vida-c", "display_id": "vida-2d9.1.1", "title": "Subtask C", "status": "open", "priority": 2, "issue_type": "task"},
    ])
    let payload = nextDisplayId("vida-2d9")
    check payload["valid"].getBool() == true
    check payload["next_display_id"].getStr() == "vida-2d9.4"

  test "next display id advances subtask sequence under task":
    seedIssues(@[
      %*{"id": "vida-a", "display_id": "vida-2d9.1", "title": "Task A", "status": "open", "priority": 2, "issue_type": "task"},
      %*{"id": "vida-b", "display_id": "vida-2d9.1.1", "title": "Subtask 1", "status": "open", "priority": 2, "issue_type": "task"},
      %*{"id": "vida-c", "display_id": "vida-2d9.1.3", "title": "Subtask 3", "status": "open", "priority": 2, "issue_type": "task"},
    ])
    let payload = nextDisplayId("vida-2d9.1")
    check payload["valid"].getBool() == true
    check payload["next_display_id"].getStr() == "vida-2d9.1.4"

  test "create stores display id and parent-child dependency":
    seedIssues(@[
      %*{"id": "vida-root", "display_id": "vida-2d9", "title": "Root", "status": "open", "priority": 1, "issue_type": "epic"}
    ])
    let created = createIssue(
      "vida-child",
      "Child task",
      displayId = "vida-2d9.1",
      parentId = "vida-root",
    )
    check created["status"].getStr() == "ok"
    let row = showIssue("vida-child")
    check row["display_id"].getStr() == "vida-2d9.1"
    check row["dependencies"][0]["type"].getStr() == "parent-child"
    check row["dependencies"][0]["depends_on_id"].getStr() == "vida-root"
    let described = createIssue(
      "vida-described",
      "Described task",
      issueType = "bug",
      description = "details",
      labels = @["framework", "mode:autonomous"],
    )
    check described["status"].getStr() == "ok"
    let describedRow = showIssue("vida-described")
    check describedRow["description"].getStr() == "details"
    check describedRow["labels"][0].getStr() == "framework"
    check describedRow["labels"][1].getStr() == "mode:autonomous"

  test "resolve task id by display id finds parent":
    seedIssues(@[
      %*{"id": "vida-root", "display_id": "vida-2d9", "title": "Root", "status": "open", "priority": 1, "issue_type": "epic"}
    ])
    let payload = resolveTaskIdByDisplayId("vida-2d9")
    check payload["found"].getBool() == true
    check payload["task_id"].getStr() == "vida-root"

  test "create flow can use parent display id and auto display numbering":
    seedIssues(@[
      %*{"id": "vida-root", "display_id": "vida-2d9", "title": "Root", "status": "open", "priority": 1, "issue_type": "epic"},
      %*{"id": "vida-existing", "display_id": "vida-2d9.1", "title": "Existing task", "status": "open", "priority": 2, "issue_type": "task",
         "dependencies": [%*{"issue_id": "vida-existing", "depends_on_id": "vida-root", "type": "parent-child"}]},
    ])
    let nextDisplay = nextDisplayId("vida-2d9")
    let parentLookup = resolveTaskIdByDisplayId("vida-2d9")
    let created = createIssue(
      "vida-new",
      "New task",
      displayId = nextDisplay["next_display_id"].getStr(),
      parentId = parentLookup["task_id"].getStr(),
    )
    check created["status"].getStr() == "ok"
    let row = showIssue("vida-2d9.2")
    check row["id"].getStr() == "vida-new"
    check row["display_id"].getStr() == "vida-2d9.2"

  test "update changes status notes and labels":
    seedIssues(@[
      %*{"id": "vida-update", "display_id": "vida-2d9.1", "title": "Update me", "status": "open", "priority": 2, "issue_type": "task"}
    ])
    let updated = updateIssue(
      "vida-update",
      status = "in_progress",
      notes = "working",
      addLabels = @["mode:autonomous"],
    )
    check updated["status"].getStr() == "ok"
    let row = showIssue("vida-update")
    check row["status"].getStr() == "in_progress"
    check row["notes"].getStr() == "working"
    check row["labels"][0].getStr() == "mode:autonomous"

  test "close records close reason":
    seedIssues(@[
      %*{"id": "vida-close", "display_id": "vida-2d9.2", "title": "Close me", "status": "in_progress", "priority": 2, "issue_type": "task"}
    ])
    let closed = closeIssue("vida-close", "done")
    check closed["status"].getStr() == "ok"
    let row = showIssue("vida-close")
    check row["status"].getStr() == "closed"
    check row["close_reason"].getStr() == "done"

  test "import derives simplified display id from legacy stack id":
    seedIssues(@[
      %*{"id": "vida-stack-2d9.1.3", "title": "Legacy task", "status": "open", "priority": 2, "issue_type": "task"}
    ])
    let row = showIssue("vida-stack-2d9.1.3")
    check row["display_id"].getStr() == "vida-2d9.1.3"

  test "export writes normalized jsonl":
    seedIssues(@[
      %*{"id": "vida-stack-2d9.1.3", "title": "Legacy task", "status": "open", "priority": 2, "issue_type": "task"},
      %*{"id": "vida-child", "display_id": "vida-2d9.2", "title": "Child task", "status": "in_progress", "priority": 1, "issue_type": "task"}
    ])
    let target = root / ".beads" / "exported.jsonl"
    let payload = exportIssuesJsonl(target)
    check payload["status"].getStr() == "ok"
    check payload["exported_count"].getInt() == 2
    let lines = readFile(target).splitLines().filterIt(it.len > 0)
    check lines.len == 2
    let first = parseJson(lines[0])
    let second = parseJson(lines[1])
    check first["id"].getStr() == "vida-child"
    check first["display_id"].getStr() == "vida-2d9.2"
    check second["id"].getStr() == "vida-stack-2d9.1.3"
    check second["display_id"].getStr() == "vida-2d9.1.3"

  test "vida legacy br import export aliases work":
    let rootDir = getCurrentDir()
    let binPath = root / "vida-legacy-test-bin"
    let compile = execCmdEx("nim c --nimcache:/tmp/vida-nimcache-task-br -o:" & binPath.quoteShell &
      " " & (rootDir / "_vida" / "scripts-nim" / "src" / "vida.nim").quoteShell)
    check compile.exitCode == 0

    let source = root / ".beads" / "alias-import.jsonl"
    writeFile(source, $(%*{
      "id": "vida-stack-2d9.4",
      "title": "Alias imported",
      "status": "open",
      "priority": 2,
      "issue_type": "task"
    }) & "\n")

    let importResult = execCmdEx(
      binPath.quoteShell & " br import " & source.quoteShell & " --json",
      env = newStringTable({
        "VIDA_ROOT": root,
        "VIDA_LEGACY_TURSO_PYTHON": rootDir / ".venv" / "bin" / "python3",
      })
    )
    check importResult.exitCode == 0
    let imported = parseJson(importResult.output)
    check imported["status"].getStr() == "ok"

    let exportPath = root / ".beads" / "alias-export.jsonl"
    let exportResult = execCmdEx(
      binPath.quoteShell & " br export " & exportPath.quoteShell & " --json",
      env = newStringTable({
        "VIDA_ROOT": root,
        "VIDA_LEGACY_TURSO_PYTHON": rootDir / ".venv" / "bin" / "python3",
      })
    )
    check exportResult.exitCode == 0
    let exported = parseJson(exportResult.output)
    check exported["status"].getStr() == "ok"
    let rows = readFile(exportPath).splitLines().filterIt(it.len > 0)
    check rows.len >= 1
    let aliasRow = parseJson(rows[^1])
    check aliasRow["display_id"].getStr() == "vida-2d9.4"
