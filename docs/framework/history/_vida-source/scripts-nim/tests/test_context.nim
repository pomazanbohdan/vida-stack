## Tests for state/context module

import std/[json, os, unittest]
import ../src/state/context

suite "context governance":
  let root = "/tmp/vida_scripts_nim_context"
  discard existsOrCreateDir(root)
  putEnv("VIDA_ROOT", root)

  setup:
    let statePath = root / ".vida" / "state" / "context-governance.json"
    if fileExists(statePath):
      removeFile(statePath)

  test "validate accepts normalized web_validated sources":
    let payload = %*[
      {
        "source_class": "web_validated",
        "path": "https://example.com",
        "freshness": "validated"
      }
    ]
    let result = validateSources(payload)
    check result["valid"].getBool() == true
    check result["summary"]["web_validated_count"].getInt() == 1

  test "record updates aggregate summary":
    let payload = %*[
      {
        "source_class": "local_repo",
        "path": "src/main.rs"
      },
      {
        "source_class": "web_validated",
        "path": "https://example.com/spec"
      }
    ]
    let entry = recordEntry("vida-ctx-1", "analysis", payload, "notes")
    check entry["task_id"].getStr() == "vida-ctx-1"
    let state = loadState()
    check state["summary"]["task_count"].getInt() == 1
    check state["summary"]["web_validated_count"].getInt() == 1
