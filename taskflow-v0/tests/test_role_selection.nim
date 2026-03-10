## Tests for core/role_selection runtime.

import std/[json, sequtils, unittest]
import ../src/core/role_selection

suite "role selection runtime":
  test "compiled bundle exposes enabled framework roles and role-selection config":
    let cfg = parseJson("""{
      "agent_extensions": {
        "enabled": true,
        "enabled_framework_roles": ["orchestrator", "business_analyst", "pm"],
        "enabled_standard_flow_sets": ["minimal"],
        "default_flow_set": "minimal",
        "role_selection": {
          "mode": "auto",
          "fallback_role": "orchestrator",
          "conversation_modes": {
            "scope_discussion": {
              "enabled": true,
              "role": "business_analyst",
              "tracked_flow_entry": "spec-pack"
            }
          }
        }
      }
    }""")
    let bundle = buildCompiledAgentExtensionBundle(cfg)
    check "business_analyst" in bundle["enabled_framework_roles"].getElems().mapIt(it.getStr())
    check bundle["role_selection"]["mode"].getStr() == "auto"

  test "auto role selection chooses business analyst for scope talk":
    let cfg = parseJson("""{
      "pack_router_keywords": {"spec": "spec,scope"},
      "agent_extensions": {
        "enabled": false,
        "enabled_framework_roles": ["orchestrator", "business_analyst", "pm"],
        "enabled_standard_flow_sets": ["minimal"],
        "default_flow_set": "minimal",
        "role_selection": {
          "mode": "auto",
          "fallback_role": "orchestrator",
          "conversation_modes": {
            "scope_discussion": {
              "enabled": true,
              "role": "business_analyst",
              "single_task_only": true,
              "tracked_flow_entry": "spec-pack",
              "allow_freeform_chat": true
            },
            "pbi_discussion": {
              "enabled": true,
              "role": "pm",
              "single_task_only": true,
              "tracked_flow_entry": "work-pool-pack",
              "allow_freeform_chat": true
            }
          }
        }
      }
    }""")
    let payload = selectAgentRoleForRequest("Need to clarify scope and acceptance constraints for one feature", cfg)
    check payload["selected_role"].getStr() == "business_analyst"
    check payload["conversational_mode"].getStr() == "scope_discussion"
    check payload["tracked_flow_entry"].getStr() == "spec-pack"

  test "auto role selection chooses pm for pbi discussion":
    let cfg = parseJson("""{
      "pack_router_keywords": {"pool": "backlog,pbi", "pool_strong": "form-task"},
      "agent_extensions": {
        "enabled": false,
        "enabled_framework_roles": ["orchestrator", "business_analyst", "pm"],
        "enabled_standard_flow_sets": ["minimal"],
        "default_flow_set": "minimal",
        "role_selection": {
          "mode": "auto",
          "fallback_role": "orchestrator",
          "conversation_modes": {
            "scope_discussion": {
              "enabled": true,
              "role": "business_analyst",
              "tracked_flow_entry": "spec-pack"
            },
            "pbi_discussion": {
              "enabled": true,
              "role": "pm",
              "single_task_only": true,
              "tracked_flow_entry": "work-pool-pack"
            }
          }
        }
      }
    }""")
    let payload = selectAgentRoleForRequest("Let's discuss one PBI in the backlog and prioritize the delivery cut", cfg)
    check payload["selected_role"].getStr() == "pm"
    check payload["conversational_mode"].getStr() == "pbi_discussion"
    check payload["tracked_flow_entry"].getStr() == "work-pool-pack"

  test "fixed mode falls back to orchestrator":
    let cfg = parseJson("""{
      "agent_extensions": {
        "enabled": false,
        "enabled_framework_roles": ["orchestrator", "business_analyst", "pm"],
        "enabled_standard_flow_sets": ["minimal"],
        "default_flow_set": "minimal",
        "role_selection": {
          "mode": "fixed",
          "fallback_role": "orchestrator"
        }
      }
    }""")
    let payload = selectAgentRoleForRequest("Need a scope review", cfg)
    check payload["selected_role"].getStr() == "orchestrator"
    check payload["reason"].getStr() == "fixed_mode"
