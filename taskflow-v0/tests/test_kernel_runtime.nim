import std/[json, os, sequtils, unittest]
import ../src/core/[agent_inventory, assignment_engine, kernel_config, guard_engine, projection_engine, transition_engine]

suite "kernel config loader":
  let root = getCurrentDir()
  putEnv("VIDA_ROOT", root)

  test "finds root config tree":
    check kernelFamilyExists("machines")
    check kernelFamilyExists("routes")
    check kernelFamilyExists("policies")

  test "loads task_lifecycle machine":
    let machine = loadMachineSpec("task_lifecycle")
    check machine["artifact_name"].getStr() == "task_lifecycle"
    check machine["states"].len == 4
    check machine["states"][0].getStr() == "open"

  test "loads route catalog":
    let catalog = loadRouteCatalog()
    check catalog["routes"]["writer_lane"]["required_roles"][0].getStr() == "writer"
    check catalog["routes"]["approval_lane"]["selection_strategy"].getStr() == "human_required"
    check catalog["task_class_bindings"]["implementation"]["primary_lane"].getStr() == "writer_lane"

  test "loads assignment policy converted from overlay":
    let policy = loadPolicySpec("assignment_policy")
    check policy["runtime"]["mode"].getStr() == "hybrid"
    check policy["scoring"]["promotion_score"].getInt() == 75

  test "builds runtime agent inventory from overlay":
    let inventory = buildRuntimeAgentInventory()
    check inventory["artifact_type"].getStr() == "runtime_agent_inventory"
    let agents = inventory["agents"]
    check agents.len > 0
    var foundInternal = false
    for agent in agents:
      let agentId = agent["id"].getStr()
      if agentId == "internal_subagents":
        foundInternal = true
        check "writer" in agent["workflow_roles"].getElems().mapIt(it.getStr())
        check "coach" in agent["workflow_roles"].getElems().mapIt(it.getStr())
        check "verifier" in agent["workflow_roles"].getElems().mapIt(it.getStr())
    check foundInternal

suite "assignment engine":
  test "resolves writer lane for implementation task class":
    let payload = resolveAssignmentForTaskClass(
      "implementation",
      %*{"effective_mode": "hybrid", "external_first_required": true},
    )
    check payload["ok"].getBool() == true
    check payload["lane"].getStr() == "writer_lane"
    check payload["selected_agent_backend"].getStr() == "internal_subagents"
    check payload["inventory_source"].getStr() == "overlay_runtime"

  test "resolves verification lane under internal-only posture":
    let payload = resolveAssignmentForTaskClass(
      "review_ensemble",
      %*{"effective_mode": "hybrid"},
    )
    check payload["ok"].getBool() == true
    check payload["lane"].getStr() == "verification_lane"
    check payload["selected_agent_backend"].getStr() == "internal_subagents"

  test "fails for unknown task class":
    let payload = resolveAssignmentForTaskClass("unknown_task_class")
    check payload["ok"].getBool() == false
    check payload["reason"].getStr() == "unbound_task_class"

  test "manual lane requires manual assignment":
    let payload = resolveAssignmentForLane("manual_intervention_lane")
    check payload["ok"].getBool() == false
    check payload["reason"].getStr() == "manual_assignment_required"

  test "disabled mode blocks assignment":
    let payload = resolveAssignmentForTaskClass(
      "implementation",
      %*{"effective_mode": "disabled"},
    )
    check payload["ok"].getBool() == false
    check payload["reason"].getStr() == "assignment_runtime_disabled"

  test "budget filtering can exhaust writer candidates":
    let payload = resolveAssignmentForTaskClass(
      "implementation",
      %*{"effective_mode": "hybrid", "external_first_required": true, "max_budget_units": 0},
    )
    check payload["ok"].getBool() == false
    check payload["reason"].getStr() == "no_eligible_agent"

suite "guard engine":
  test "evaluates all_of and any_of":
    let ctx = %*{
      "execution_plan_present": true,
      "route_authorized_for_start": true,
      "fallback_allowed": false
    }
    check evalGuardExpr(%*{"all_of": ["execution_plan_present", "route_authorized_for_start"]}, ctx)
    check not evalGuardExpr(%*{"all_of": ["execution_plan_present", "fallback_allowed"]}, ctx)
    check evalGuardExpr(%*{"any_of": ["fallback_allowed", "route_authorized_for_start"]}, ctx)

  test "evaluates not":
    let ctx = %*{"blocked": false}
    check evalGuardExpr(%*{"not": "blocked"}, ctx)

suite "transition engine":
  test "applies task start transition":
    let machine = loadMachineSpec("task_lifecycle")
    let ctx = %*{
      "execution_plan_present": true,
      "route_authorized_for_start": true
    }
    let result = applyTransition(machine, "open", "task.start", ctx)
    check result["ok"].getBool() == true
    check result["new_state"].getStr() == "in_progress"
    check result["receipts"][0].getStr() == "task_state_changed_receipt"

  test "fails when guard is false":
    let machine = loadMachineSpec("task_lifecycle")
    let ctx = %*{
      "execution_plan_present": true,
      "route_authorized_for_start": false
    }
    let result = applyTransition(machine, "open", "task.start", ctx)
    check result["ok"].getBool() == false
    check result["error"].getStr() == "guard_failed"

  test "applies route progression status transition":
    let machine = loadMachineSpec("route_progression")
    let ctx = %*{
      "route_metadata_present": true,
      "instruction_bundle_composed": true
    }
    let result = applyTransition(machine, "pending", "route.resolve", ctx)
    check result["ok"].getBool() == true
    check result["new_state"].getStr() == "ready"
    check "projection.route" in result["listener_topics"].getElems().mapIt(it.getStr())
    check result["checkpoint"]["required"].getBool() == true
    check result["checkpoint"]["kind"].getStr() == "route_cursor"

  test "supports event_id and event aliases":
    let machine = %*{
      "artifact_name": "event_machine",
      "transitions": [
        {
          "name": "advance",
          "event_id": "evt.advance",
          "event_aliases": ["alias.advance"],
          "from": ["open"],
          "to": "closed",
          "receipts": ["task_state_changed_receipt"]
        }
      ]
    }
    let byEvent = applyTransition(machine, "open", "evt.advance")
    check byEvent["ok"].getBool() == true
    check byEvent["new_state"].getStr() == "closed"
    let byAlias = applyTransition(machine, "open", "alias.advance")
    check byAlias["ok"].getBool() == true
    check byAlias["new_state"].getStr() == "closed"

  test "supports from_any transitions":
    let machine = %*{
      "artifact_name": "global_machine",
      "transitions": [
        {
          "name": "cancel",
          "command": "task.cancel",
          "from_any": true,
          "to": "cancelled",
          "receipts": ["task_state_changed_receipt"]
        }
      ]
    }
    let result = applyTransition(machine, "open", "task.cancel")
    check result["ok"].getBool() == true
    check result["new_state"].getStr() == "cancelled"

  test "auto-composes instruction bundle for route progression":
    let machine = loadMachineSpec("route_progression")
    let ctx = %*{
      "workflow_role": "writer",
      "route_metadata_present": true,
      "current_stage": "writer"
    }
    let result = applyTransition(machine, "pending", "route.resolve", ctx)
    check result["ok"].getBool() == true
    check result["new_state"].getStr() == "ready"
    check result["instruction_bundle_composed"].getBool() == true
    check result["route"].getStr() == "writer_lane"

  test "boot machine respects auto instruction revision support":
    let machine = loadMachineSpec("boot_migration_gate")
    let ctx = %*{
      "workflow_role": "orchestrator",
      "compatibility_class_supported": true
    }
    let result = applyTransition(machine, "boot_unchecked", "boot.check", ctx)
    check result["ok"].getBool() == true
    check result["new_state"].getStr() == "compat_checked"
    check "checkpoint.boot" in result["listener_topics"].getElems().mapIt(it.getStr())
    check result["checkpoint"]["kind"].getStr() == "boot_gate"

  test "execution plan transition emits resume projection and checkpoint hint":
    let machine = loadMachineSpec("execution_plan")
    let ctx = %*{
      "dependencies_satisfied": true,
      "next_step_known": true,
      "blocker_absent": true
    }
    let result = applyTransition(machine, "todo", "run.step.start", ctx)
    check result["ok"].getBool() == true
    check "projection.resume" in result["projection"]["topics"].getElems().mapIt(it.getStr())
    check result["checkpoint"]["required"].getBool() == true
    check result["checkpoint"]["kind"].getStr() == "execution_cursor"

  test "route escalation emits manual gateway posture":
    let machine = loadMachineSpec("route_progression")
    let ctx = %*{"escalation_lawful": true}
    let result = applyTransition(machine, "failed", "route.escalate", ctx)
    check result["ok"].getBool() == true
    check result["projection"]["gateway"]["active"].getBool() == true
    check result["projection"]["gateway"]["kind"].getStr() == "manual_intervention"
    check result["checkpoint"]["kind"].getStr() == "manual_gateway"

  test "projects transition into checkpoint and listener intents":
    let machine = loadMachineSpec("route_progression")
    let ctx = %*{
      "workflow_role": "writer",
      "route_metadata_present": true,
      "current_stage": "writer"
    }
    let payload = projectTransitionOutcome(machine, "pending", "route.resolve", ctx)
    check payload["ok"].getBool() == true
    check payload["projection"]["projection_type"].getStr() == "transition_projection"
    check payload["projection"]["route_lane"].getStr() == "writer_lane"
    check payload["checkpoint"]["checkpoint_type"].getStr() == "transition_checkpoint"
    check payload["checkpoint"]["next_action"].getStr() == "dispatch.writer_lane"
    check "projection.refresh" in payload["listener_intents"].getElems().mapIt(it.getStr())
    check "dispatch.eligible" in payload["listener_intents"].getElems().mapIt(it.getStr())
