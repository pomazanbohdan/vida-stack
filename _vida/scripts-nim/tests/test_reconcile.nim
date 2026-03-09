## Tests for state/reconcile module

import std/[json, os, unittest]
import ../src/state/reconcile

proc appendLine(path: string, payload: JsonNode) =
  let dir = parentDir(path)
  if not dirExists(dir):
    createDir(dir)
  let fh = open(path, fmAppend)
  defer: fh.close()
  fh.writeLine($payload)

suite "reconcile":
  let root = "/tmp/vida_scripts_nim_reconcile"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)

  test "done_ready_to_close when in_progress task has only done steps and verify ok":
    let issuesPath = root / ".beads" / "issues.jsonl"
    createDir(parentDir(issuesPath))
    writeFile(issuesPath, $(%*{"id": "vida-rec-1", "title": "Reconcile test", "status": "in_progress"}) & "\n")

    let logPath = root / ".vida" / "logs" / "beads-execution.jsonl"
    if fileExists(logPath):
      removeFile(logPath)
    appendLine(logPath, %*{"type": "block_plan", "task_id": "vida-rec-1", "block_id": "P01", "goal": "Finish", "track_id": "main", "owner": "orchestrator", "next_step": "-"})
    appendLine(logPath, %*{"type": "block_end", "task_id": "vida-rec-1", "block_id": "P01", "result": "done", "next_step": "-", "ts_end": "2026-03-08T21:00:00Z", "evidence_ref": "test"})

    let payload = buildStatusPayload("vida-rec-1")
    check payload["classification"].getStr() == "done_ready_to_close"
    check payload["verify_ok"].getBool() == true
    check payload["todo_counts"]["done"].getInt() == 1
    check payload["allowed_actions"].len == 1
    check payload["allowed_actions"][0].getStr() == "close_now"

  test "blocked when todo has blocked step":
    let issuesPath = root / ".beads" / "issues.jsonl"
    writeFile(issuesPath, $(%*{"id": "vida-rec-2", "title": "Blocked test", "status": "in_progress"}) & "\n")

    let logPath = root / ".vida" / "logs" / "beads-execution.jsonl"
    if fileExists(logPath):
      removeFile(logPath)
    appendLine(logPath, %*{"type": "block_plan", "task_id": "vida-rec-2", "block_id": "P01", "goal": "Blocked", "track_id": "main"})
    appendLine(logPath, %*{"type": "block_end", "task_id": "vida-rec-2", "block_id": "P01", "result": "failed"})

    let payload = buildStatusPayload("vida-rec-2")
    check payload["classification"].getStr() == "blocked"
    check payload["allowed_actions"].len == 1
    check payload["allowed_actions"][0].getStr() == "unblock_or_escalate"
