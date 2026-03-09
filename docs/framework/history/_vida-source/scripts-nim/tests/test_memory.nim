## Tests for state/memory module

import std/[json, os, unittest]
import ../src/state/memory

suite "framework memory":
  let root = "/tmp/vida_scripts_nim_memory"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)

  setup:
    let statePath = root / ".vida" / "state" / "framework-memory.json"
    if fileExists(statePath):
      removeFile(statePath)

  test "record updates summary counters":
    let entry = recordEntry("lesson", "Remember parity", "vida-mem-1", %*{"scope": "nim"})
    check entry["kind"].getStr() == "lesson"
    let state = loadState()
    check state["summary"]["lesson_count"].getInt() == 1

  test "invalid kind is rejected":
    expect(ValueError):
      discard recordEntry("bad_kind", "oops")
