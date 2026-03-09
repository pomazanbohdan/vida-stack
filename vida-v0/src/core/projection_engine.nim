## VIDA Projection Engine — derive projection/listener/checkpoint metadata
## from lawful machine transitions without mutating canonical state.

import std/[json, strutils]
import ./utils

proc addUnique(target: var seq[string], value: string) =
  let item = value.strip()
  if item.len > 0 and item notin target:
    target.add(item)

proc machineName(machine: JsonNode): string =
  policyValue(machine{"artifact_name"}, "")

proc routeNextAction(route: JsonNode): string =
  let requiredPath = route{"dispatch_policy"}{"required_dispatch_path"}
  if requiredPath.isNil or requiredPath.kind == JNull:
    return ""
  if requiredPath.kind == JArray:
    let parts = splitCsv(requiredPath)
    if parts.len == 0:
      return ""
    return parts.join("_then_")
  policyValue(requiredPath, "")

proc checkpointHint(machineName, newState, command: string): JsonNode =
  var required = false
  var kind = "none"
  var reason = ""

  case machineName
  of "execution_plan":
    required = newState in ["doing", "blocked", "done"]
    kind = "execution_cursor"
    reason = "execution_plan_transition"
  of "route_progression":
    required = true
    kind = "route_cursor"
    reason = "route_stage_transition"
  of "verification_lifecycle":
    required = true
    kind = "verification_snapshot"
    reason = "verification_state_transition"
  of "approval_lifecycle":
    required = true
    kind = "approval_wait"
    reason = "approval_state_transition"
  of "boot_migration_gate":
    required = true
    kind = "boot_gate"
    reason = "boot_migration_transition"
  of "coach_lifecycle":
    required = newState in ["feedback_issued", "rework_required", "accepted", "dismissed", "closed"]
    kind = "coach_review"
    reason = "coach_state_transition"
  of "task_lifecycle":
    required = newState in ["in_progress", "deferred", "closed"]
    kind = "task_snapshot"
    reason = "task_lifecycle_transition"
  else:
    discard

  if command == "route.escalate":
    required = true
    kind = "manual_gateway"
    reason = "route_escalation"

  %*{
    "required": required,
    "kind": kind,
    "reason": reason,
  }

proc gatewayPosture(machineName, newState, command: string): JsonNode =
  var active = false
  var kind = "none"
  var reason = ""

  case machineName
  of "route_progression":
    if newState == "blocked" or command == "route.escalate":
      active = true
      kind = "manual_intervention"
      reason = "route_blocked_or_escalated"
  of "approval_lifecycle":
    active = true
    kind = "approval_gate"
    reason = "approval_machine_transition"
  of "coach_lifecycle":
    if newState == "rework_required":
      active = true
      kind = "coach_rework"
      reason = "coach_requested_rework"
  of "verification_lifecycle":
    if newState in ["failed", "inconclusive", "aggregation_pending"]:
      active = true
      kind = "verification_followup"
      reason = "verification_requires_followup"
  else:
    discard

  %*{
    "active": active,
    "kind": kind,
    "reason": reason,
  }

proc topicSet(machineName, currentState, newState, command, route: string, ctx: JsonNode): seq[string] =
  addUnique(result, "machine." & machineName)
  addUnique(result, "event." & command)
  addUnique(result, "state." & machineName & "." & currentState & "." & newState)

  let workflowRole = policyValue(ctx{"workflow_role"}, "")
  if workflowRole.len > 0:
    addUnique(result, "role." & workflowRole)
  if route.len > 0:
    addUnique(result, "route." & route)

  case machineName
  of "task_lifecycle":
    addUnique(result, "projection.status")
    addUnique(result, "projection.closure")
  of "execution_plan":
    addUnique(result, "projection.execution")
    addUnique(result, "projection.resume")
    addUnique(result, "checkpoint.execution")
  of "route_progression":
    addUnique(result, "projection.route")
    addUnique(result, "projection.readiness")
    addUnique(result, "checkpoint.route")
  of "coach_lifecycle":
    addUnique(result, "projection.coach")
  of "verification_lifecycle":
    addUnique(result, "projection.verification")
    addUnique(result, "projection.proof_coverage")
  of "approval_lifecycle":
    addUnique(result, "projection.approval")
    addUnique(result, "gateway.approval")
  of "boot_migration_gate":
    addUnique(result, "projection.boot")
    addUnique(result, "checkpoint.boot")
  else:
    addUnique(result, "projection.generic")

proc deriveRuntimeSurface*(
  machine: JsonNode,
  currentState, newState, command, route: string,
  ctx: JsonNode = newJObject(),
): JsonNode =
  let name = machineName(machine)
  let topics = topicSet(name, currentState, newState, command, route, ctx)
  let checkpoint = checkpointHint(name, newState, command)
  let gateway = gatewayPosture(name, newState, command)

  result = %*{
    "projection": {
      "machine": name,
      "current_state": currentState,
      "new_state": newState,
      "topics": topics,
      "gateway": gateway,
    },
    "listener_topics": topics,
    "checkpoint": checkpoint,
  }

proc projectRoutePayload*(route: JsonNode): JsonNode =
  let routeLane = policyValue(route{"route_lane"}, "")
  let nextAction = routeNextAction(route)
  let selectedAgentBackend = policyValue(route{"selected_agent_backend"}, "")

  var intents: seq[string] = @["projection.refresh", "subscription.notify"]
  if selectedAgentBackend.len > 0:
    addUnique(intents, "dispatch.eligible")
  if nextAction.len > 0:
    addUnique(intents, "checkpoint.write")

  %*{
    "ok": true,
    "projection": {
      "projection_type": "route_projection",
      "task_class": route{"task_class"},
      "task_id": route{"task_id"},
      "route_lane": route{"route_lane"},
      "selected_agent_backend": route{"selected_agent_backend"},
      "risk_class": route{"risk_class"},
    },
    "checkpoint": {
      "checkpoint_type": "route_checkpoint",
      "required": true,
      "next_action": nextAction,
      "route_lane": routeLane,
    },
    "listener_intents": intents,
    "route": route,
  }
