import std/[json, os, unittest]
import ../src/state/[context_capsule, task]

suite "context capsule":
  let root = "/tmp/vida_scripts_nim_context_capsule"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)
  putEnv("VIDA_V0_TURSO_PYTHON", getCurrentDir() / ".venv" / "bin" / "python3")

  proc seedIssues(lines: seq[JsonNode]) =
    let dbPath = root / ".vida" / "state" / "vida-v0.db"
    let issuesPath = root / ".beads" / "issues.jsonl"
    if fileExists(dbPath):
      removeFile(dbPath)
    createDir(parentDir(issuesPath))
    writeFile(issuesPath, "")
    for line in lines:
      writeFile(issuesPath, readFile(issuesPath) & $line & "\n")
    discard importIssuesJsonl(issuesPath)

  test "hydrate allow missing bootstraps capsule":
    seedIssues(@[])
    putEnv("VIDA_CONTEXT_HYDRATE_ALLOW_MISSING", "1")
    let (code, payload, reason) = hydrateCapsule("unit-task")
    delEnv("VIDA_CONTEXT_HYDRATE_ALLOW_MISSING")
    check code == 0
    check reason == ""
    check payload["task_id"].getStr() == "unit-task"
    check payload["done"].getStr() == "bootstrap"
    check payload["next"].getStr() == "planning"
    check payload["task_role_in_epic"].getStr() == "runtime-bootstrap"

  test "write captures parent epic context":
    seedIssues(@[
      %*{"id": "vida-root", "title": "Root", "description": "Epic goal", "status": "open", "priority": 1, "issue_type": "epic"},
      %*{"id": "vida-task", "title": "Task", "description": "Task context", "status": "open", "priority": 2, "issue_type": "task",
         "dependencies": [%*{"issue_id": "vida-task", "depends_on_id": "vida-root", "type": "parent-child"}]},
    ])
    let payload = writeCapsule("vida-task", "done", "next", "risk", "slice", "constraint", "role")
    check payload["epic_id"].getStr() == "vida-root"
    check payload["epic_goal"].getStr() == "Epic goal"
    check payload["task_context"].getStr() == "Task context"
    check payload["open_risks"][0].getStr() == "risk"
