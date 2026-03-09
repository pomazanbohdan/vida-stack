## Tests for state/todo module

import std/[json, os, strutils, unittest]
import ../src/state/todo

proc appendLine(path: string, payload: JsonNode) =
  let dir = parentDir(path)
  if not dirExists(dir):
    createDir(dir)
  let fh = open(path, fmAppend)
  defer: fh.close()
  fh.writeLine($payload)

suite "todo runtime":
  let root = "/tmp/vida_scripts_nim_todo"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)
  let logPath = root / ".vida" / "logs" / "beads-execution.jsonl"

  test "compute steps derives statuses and next-step views":
    if fileExists(logPath):
      removeFile(logPath)
    appendLine(logPath, %*{"type": "block_plan", "task_id": "vida-todo-1", "block_id": "P01", "goal": "Plan", "track_id": "main", "owner": "orchestrator", "next_step": "P02"})
    appendLine(logPath, %*{"type": "block_end", "task_id": "vida-todo-1", "block_id": "P01", "result": "done", "next_step": "P02"})
    appendLine(logPath, %*{"type": "block_plan", "task_id": "vida-todo-1", "block_id": "P02", "goal": "Implement", "track_id": "main", "owner": "orchestrator"})
    appendLine(logPath, %*{"type": "block_start", "task_id": "vida-todo-1", "block_id": "P02", "goal": "Implement", "track_id": "main", "owner": "orchestrator", "ts": "2026-03-08T21:00:00Z"})
    appendLine(logPath, %*{"type": "block_plan", "task_id": "vida-todo-1", "block_id": "P03", "goal": "Verify", "track_id": "main", "owner": "orchestrator", "depends_on": "P02"})

    let steps = stepsJson("vida-todo-1")
    check steps.len == 3
    check steps[0]["status"].getStr() == "done"
    check steps[1]["status"].getStr() == "doing"
    check steps[2]["status"].getStr() == "todo"

  test "superseded dependency cascades to waiting step":
    if fileExists(logPath):
      removeFile(logPath)
    appendLine(logPath, %*{"type": "block_plan", "task_id": "vida-todo-2", "block_id": "P01", "goal": "Old path", "track_id": "main"})
    appendLine(logPath, %*{"type": "block_end", "task_id": "vida-todo-2", "block_id": "P01", "result": "redirected"})
    appendLine(logPath, %*{"type": "block_plan", "task_id": "vida-todo-2", "block_id": "P02", "goal": "Dependent", "track_id": "main", "depends_on": "P01"})

    let steps = stepsJson("vida-todo-2")
    check steps[1]["status"].getStr() == "superseded"

  test "sync delta reports status transitions":
    let prevPayload = %*{
      "task_id": "vida-todo-3",
      "steps": [
        {"block_id": "P01", "goal": "Plan", "status": "todo"}
      ]
    }
    let payload = %*{
      "task_id": "vida-todo-3",
      "steps": [
        {"block_id": "P01", "goal": "Plan", "status": "done"},
        {"block_id": "P02", "goal": "Implement", "status": "todo"}
      ]
    }
    let lines = syncDelta(prevPayload, payload)
    check lines.len == 2
    check lines[0].contains("P01") or lines[1].contains("P01")
    check lines[0].contains("P02") or lines[1].contains("P02")
