## Tests for protocol-binding runtime bridge.

import std/[json, os, osproc, strtabs, strutils, times, unittest]
import ../src/state/protocol_binding
import ../src/core/utils

proc detectRepoRoot(): string =
  let cwd = getCurrentDir()
  if dirExists(cwd / "taskflow-v0"):
    return cwd
  cwd.parentDir()

proc freshRoot(repoRoot: string): string =
  let root = getTempDir() / ("vida-protocol-binding-" & safeName($epochTime(), "tmp").replace(".", "-"))
  createDir(root)
  for relPath in @[
    "taskflow-v0/config/protocol_binding.seed.json",
    "taskflow-v0/helpers/turso_task_store.py",
    "vida/config/instructions/system-maps/protocol.index.md",
    "vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md",
    "vida/config/instructions/runtime-instructions/work.taskflow-protocol.md",
    "vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md",
    "vida/config/instructions/runtime-instructions/work.execution-health-check-protocol.md",
    "vida/config/instructions/runtime-instructions/work.task-state-reconciliation-protocol.md",
  ]:
    let src = repoRoot / relPath
    let dst = root / relPath
    createDir(parentDir(dst))
    copyFile(src, dst)
  root

suite "protocol binding":
  let repoRoot = detectRepoRoot()
  let pythonPath = repoRoot / ".venv" / "bin" / "python3"

  test "sync persists compiled payload and DB receipt":
    let root = freshRoot(repoRoot)
    putEnv("VIDA_ROOT", root)
    putEnv("VIDA_V0_TURSO_PYTHON", pythonPath)

    let syncPayload = protocolBindingSyncPayload()
    check dottedGetBool(syncPayload, "ok", false)
    check dottedGetInt(syncPayload, "db_sync.receipt.total_bindings", 0) == 5
    check fileExists(root / "taskflow-v0" / "generated" / "protocol_binding.compiled.json")
    check fileExists(root / ".vida" / "state" / "taskflow-state.db")

    let checkPayload = protocolBindingCheckPayload()
    check dottedGetBool(checkPayload, "ok", false)
    check dottedGetStr(checkPayload, "required_primary_state_authority") == "taskflow_v0_state_db"

  test "cli gate blocks runtime work until protocol binding sync":
    let root = freshRoot(repoRoot)
    let binPath = root / "taskflow-v0-test-bin"
    let compile = execCmdEx(
      "nim c --nimcache:" & quoteShell(root / "nimcache") &
      " -o:" & quoteShell(binPath) &
      " " & quoteShell(repoRoot / "taskflow-v0" / "src" / "vida.nim"),
      workingDir = repoRoot,
    )
    check compile.exitCode == 0

    let env = newStringTable({
      "VIDA_ROOT": root,
      "VIDA_V0_TURSO_PYTHON": pythonPath,
    })

    let blocked = execCmdEx(
      quoteShell(binPath) & " task list --json",
      env = env,
      workingDir = repoRoot,
    )
    check blocked.exitCode == 1
    check blocked.output.contains("Protocol-binding runtime state is missing or invalid.")

    let sync = execCmdEx(
      quoteShell(binPath) & " protocol-binding sync --json",
      env = env,
      workingDir = repoRoot,
    )
    check sync.exitCode == 0
    let syncJson = parseJson(sync.output)
    check syncJson["ok"].getBool() == true

    let listed = execCmdEx(
      quoteShell(binPath) & " task list --json",
      env = env,
      workingDir = repoRoot,
    )
    check listed.exitCode == 0
    check parseJson(listed.output).kind == JArray

    let status = execCmdEx(
      quoteShell(binPath) & " status --json",
      env = env,
      workingDir = repoRoot,
    )
    check status.exitCode == 0
    let statusJson = parseJson(status.output)
    check statusJson["protocol_binding"]["ok"].getBool() == true
