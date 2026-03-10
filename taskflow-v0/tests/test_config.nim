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

  test "parses inline empty collections":
    let yaml = """
empty_list: []
empty_map: {}
"""
    let result = parseYamlSubset(yaml)
    check result["empty_list"].kind == JArray
    check result["empty_map"].kind == JObject

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

  test "rejects unknown enabled framework role in agent_extensions":
    let config = parseJson("""{
      "agent_extensions": {
        "enabled": true,
        "enabled_framework_roles": ["unknown_role"],
        "enabled_standard_flow_sets": ["minimal"],
        "default_flow_set": "minimal",
        "registries": {}
      }
    }""")
    let result = validateConfig(config)
    check result.valid == false

  test "rejects unknown standard flow set in agent_extensions":
    let config = parseJson("""{
      "agent_extensions": {
        "enabled": true,
        "enabled_framework_roles": ["worker"],
        "enabled_standard_flow_sets": ["unknown_flow"],
        "default_flow_set": "unknown_flow",
        "registries": {}
      }
    }""")
    let result = validateConfig(config)
    check result.valid == false

  test "accepts disabled agent_extensions without registries":
    let config = parseJson("""{
      "agent_extensions": {
        "enabled": false,
        "enabled_framework_roles": ["worker"],
        "enabled_standard_flow_sets": ["minimal"],
        "default_flow_set": "minimal"
      }
    }""")
    let result = validateConfig(config)
    check result.valid == true

  test "accepts business analyst and pm as known framework roles":
    let config = parseJson("""{
      "agent_extensions": {
        "enabled": false,
        "enabled_framework_roles": ["business_analyst", "pm"],
        "enabled_standard_flow_sets": ["minimal"],
        "default_flow_set": "minimal"
      }
    }""")
    let result = validateConfig(config)
    check result.valid == true

  test "accepts auto role selection for standard conversational modes":
    let config = parseJson("""{
      "agent_extensions": {
        "enabled": false,
        "enabled_framework_roles": ["business_analyst", "pm"],
        "enabled_standard_flow_sets": ["minimal"],
        "default_flow_set": "minimal",
        "role_selection": {
          "mode": "auto",
          "fallback_role": "orchestrator",
          "conversation_modes": {
            "scope_discussion": {
              "role": "business_analyst",
              "tracked_flow_entry": "spec-pack"
            },
            "pbi_discussion": {
              "role": "pm",
              "tracked_flow_entry": "work-pool-pack"
            }
          }
        }
      }
    }""")
    let result = validateConfig(config)
    check result.valid == true

  test "accepts valid party_chat config":
    let config = parseJson("""{
      "party_chat": {
        "enabled": true,
        "execution_mode": "multi_agent",
        "model_routing_strategy": "by_role",
        "default_board_size": "small",
        "min_experts": 2,
        "max_experts": 6,
        "hard_cap_agents": 8,
        "single_agent": {
          "backend": "qwen_cli",
          "model": "qwen-max"
        },
        "role_model_bindings": {
          "party_chat_facilitator": {"backend": "qwen_cli", "model": "qwen-max"},
          "party_chat_architect": {"backend": "qwen_cli", "model": "qwen-max"}
        }
      }
    }""")
    let result = validateConfig(config)
    check result.valid == true

  test "rejects impossible party_chat capacity":
    let config = parseJson("""{
      "party_chat": {
        "execution_mode": "multi_agent",
        "model_routing_strategy": "by_role",
        "default_board_size": "small",
        "min_experts": 4,
        "max_experts": 6,
        "hard_cap_agents": 2,
        "role_model_bindings": {
          "party_chat_facilitator": {"backend": "qwen_cli", "model": "qwen-max"}
        }
      }
    }""")
    let result = validateConfig(config)
    check result.valid == false

  test "rejects missing single_agent backend and model":
    let config = parseJson("""{
      "party_chat": {
        "execution_mode": "single_agent",
        "model_routing_strategy": "uniform",
        "default_board_size": "small",
        "min_experts": 1,
        "max_experts": 1,
        "hard_cap_agents": 2,
        "single_agent": {},
        "role_model_bindings": {
          "party_chat_facilitator": {"backend": "qwen_cli", "model": "qwen-max"}
        }
      }
    }""")
    let result = validateConfig(config)
    check result.valid == false

  test "rejects unknown party_chat role binding":
    let config = parseJson("""{
      "party_chat": {
        "execution_mode": "multi_agent",
        "model_routing_strategy": "by_role",
        "default_board_size": "small",
        "min_experts": 2,
        "max_experts": 2,
        "hard_cap_agents": 3,
        "single_agent": {"backend": "qwen_cli", "model": "qwen-max"},
        "role_model_bindings": {
          "unknown_role": {"backend": "qwen_cli", "model": "qwen-max"}
        }
      }
    }""")
    let result = validateConfig(config)
    check result.valid == false

suite "autonomous execution getters":
  test "reads extended autonomous execution flags":
    let config = parseJson("""{
      "autonomous_execution": {
        "next_task_boundary_analysis": true,
        "next_task_boundary_report": "brief_plan",
        "continue_after_reports": true,
        "spec_ready_auto_development": true,
        "validation_report_required_before_implementation": true,
        "resume_after_validation_gate": true
      }
    }""")
    check isNextTaskBoundaryAnalysisEnabled(config) == true
    check nextTaskBoundaryReport(config) == "brief_plan"
    check continueAfterReportsEnabled(config) == true
    check specReadyAutoDevelopmentEnabled(config) == true
    check validationReportRequiredBeforeImplementation(config) == true
    check resumeAfterValidationGateEnabled(config) == true

  test "rejects unknown tracked flow entry in conversational mode":
    let config = parseJson("""{
      "agent_extensions": {
        "enabled": false,
        "enabled_framework_roles": ["business_analyst"],
        "enabled_standard_flow_sets": ["minimal"],
        "default_flow_set": "minimal",
        "role_selection": {
          "mode": "auto",
          "fallback_role": "orchestrator",
          "conversation_modes": {
            "scope_discussion": {
              "role": "business_analyst",
              "tracked_flow_entry": "unknown-pack"
            }
          }
        }
      }
    }""")
    let result = validateConfig(config)
    check result.valid == false

suite "config access helpers":
  test "isProtocolActive":
    let config = parseJson("""{"protocol_activation": {"agent_system": true}}""")
    check isProtocolActive(config, "agent_system") == true
    check isProtocolActive(config, "nonexistent") == false

  test "getAgentBackends":
    let config = parseJson("""{"agent_system": {"subagents": {"codex": {"enabled": true}}}}""")
    let subagents = getAgentBackends(config)
    check subagents["codex"]["enabled"].getBool() == true
