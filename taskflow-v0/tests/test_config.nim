## Tests for core/config module

import std/[json, unittest]
import ../src/core/config

suite "YAML subset parser":
  test "parses simple key-value":
    let result = parseYamlSubset("name: test\nversion: 1")
    check result["name"].getStr() == "test"
    check result["version"].getInt() == 1

  test "parses nested mapping":
    let yaml = """
parent:
  child: value
  number: 42
"""
    let result = parseYamlSubset(yaml)
    check result["parent"]["child"].getStr() == "value"
    check result["parent"]["number"].getInt() == 42

  test "parses list of strings":
    let yaml = """
items:
  - alpha
  - beta
  - gamma
"""
    let result = parseYamlSubset(yaml)
    check result["items"].len == 3
    check result["items"][0].getStr() == "alpha"
    check result["items"][2].getStr() == "gamma"

  test "parses list of mappings with nested fields":
    let yaml = """
transitions:
  - name: start_task
    command: task.start
    from:
      - open
      - deferred
    to: in_progress
  - name: close_task
    command: task.close
    from:
      - in_progress
    to: closed
"""
    let result = parseYamlSubset(yaml)
    check result["transitions"].kind == JArray
    check result["transitions"].len == 2
    check result["transitions"][0]["name"].getStr() == "start_task"
    check result["transitions"][0]["command"].getStr() == "task.start"
    check result["transitions"][0]["from"][0].getStr() == "open"
    check result["transitions"][1]["name"].getStr() == "close_task"

  test "parses booleans":
    let yaml = """
enabled: true
disabled: false
yes_val: yes
no_val: no
"""
    let result = parseYamlSubset(yaml)
    check result["enabled"].getBool() == true
    check result["disabled"].getBool() == false
    check result["yes_val"].getBool() == true
    check result["no_val"].getBool() == false

  test "parses quoted strings":
    let yaml = """
quoted: "hello world"
single: 'test value'
"""
    let result = parseYamlSubset(yaml)
    check result["quoted"].getStr() == "hello world"
    check result["single"].getStr() == "test value"

  test "skips comments":
    let yaml = """
# This is a comment
key: value
# Another comment
other: 123
"""
    let result = parseYamlSubset(yaml)
    check result["key"].getStr() == "value"
    check result["other"].getInt() == 123

  test "handles deep nesting":
    let yaml = """
level1:
  level2:
    level3:
      value: deep
"""
    let result = parseYamlSubset(yaml)
    check result["level1"]["level2"]["level3"]["value"].getStr() == "deep"

suite "validation":
  test "validates empty config":
    let config = newJObject()
    let result = validateConfig(config)
    check result.valid == true

  test "rejects non-object config":
    let config = newJArray()
    let result = validateConfig(config)
    check result.valid == false

  test "validates agent_system mode":
    let config = parseJson("""{"agent_system": {"mode": "invalid_mode"}}""")
    let result = validateConfig(config)
    check result.valid == false

  test "accepts valid agent_system mode":
    let config = parseJson("""{"agent_system": {"mode": "hybrid"}}""")
    let result = validateConfig(config)
    check result.valid == true

suite "config access helpers":
  test "isProtocolActive":
    let config = parseJson("""{"protocol_activation": {"agent_system": true}}""")
    check isProtocolActive(config, "agent_system") == true
    check isProtocolActive(config, "nonexistent") == false

  test "getAgentBackends":
    let config = parseJson("""{"agent_system": {"subagents": {"codex": {"enabled": true}}}}""")
    let subagents = getAgentBackends(config)
    check subagents["codex"]["enabled"].getBool() == true
