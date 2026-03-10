## VIDA Transition Engine — evaluates config-driven machine transitions.

import std/json
import ./[guard_engine, instruction_engine, projection_engine, utils]

proc transitionCommandMatches(transition: JsonNode, command: string): bool =
  let direct = dottedGetStr(transition, "command")
  if direct.len > 0 and direct == command:
    return true
  let eventId = dottedGetStr(transition, "event_id")
  if eventId.len > 0 and eventId == command:
    return true
  for alias in splitCsv(transition{"event_aliases"}):
    if alias == command:
      return true
  for alias in splitCsv(transition{"commands"}):
    if alias == command:
      return true
  false

proc transitionFromMatches(transition: JsonNode, currentState: string): bool =
  if dottedGetBool(transition, "from_any", false):
    return true
  for key in ["from", "from_status"]:
    let value = transition{key}
    if value.isNil or value.kind == JNull:
      continue
    if value.kind == JArray:
      for item in value:
        let source = policyValue(item, "")
        if source == "*" or source == currentState:
          return true
    elif policyValue(value, "") in ["*", currentState]:
      return true
  false

proc transitionTargetState(transition: JsonNode): string =
  let direct = dottedGetStr(transition, "to")
  if direct.len > 0:
    return direct
  dottedGetStr(transition, "to_status")

proc transitionRoute(transition, machine, ctx: JsonNode): string =
  let direct = dottedGetStr(transition, "route")
  if direct.len > 0:
    return direct
  let ctxStage = policyValue(ctx{"current_stage"}, "")
  if ctxStage.len > 0 and machine.kind == JObject and machine.hasKey("stage_routes"):
    let fromCtx = dottedGetStr(machine, "stage_routes." & ctxStage)
    if fromCtx.len > 0:
      return fromCtx
  if machine.kind == JObject and machine.hasKey("stage_routes"):
    let stage = dottedGetStr(machine, "current_stage")
    if stage.len > 0:
      return dottedGetStr(machine, "stage_routes." & stage)
  ""

proc enrichInstructionContext(machine, transition, ctx: JsonNode): JsonNode =
  result = normalizeJson(ctx)
  if result.kind != JObject:
    result = newJObject()
  if dottedGet(result, "instruction_bundle_composed").kind != JNull and
      dottedGet(result, "instruction_revision_supported").kind != JNull:
    return result

  let role = policyValue(result{"workflow_role"}, "")
  if role.len == 0:
    return result

  let machineName = dottedGetStr(machine, "artifact_name")
  let routeName = transitionRoute(transition, machine, result)
  let composed = composeInstructionSurface(role, machineName, routeName)
  if dottedGetBool(composed, "ok", false):
    if dottedGet(result, "instruction_bundle_composed").kind == JNull:
      result["instruction_bundle_composed"] = %true
    if dottedGet(result, "instruction_revision_supported").kind == JNull:
      result["instruction_revision_supported"] = %true
    result["instruction_surface"] = composed
  elif dottedGet(result, "instruction_bundle_composed").kind == JNull:
    result["instruction_bundle_composed"] = %false

proc applyTransition*(machine: JsonNode, currentState, command: string, ctx: JsonNode = newJObject()): JsonNode =
  if machine.kind != JObject:
    return %*{
      "ok": false,
      "error": "invalid_machine_spec",
    }

  let transitions = machine{"transitions"}
  if transitions.isNil or transitions.kind != JArray:
    return %*{
      "ok": false,
      "error": "missing_transitions",
    }

  for transition in transitions:
    if not transitionCommandMatches(transition, command):
      continue
    if not transitionFromMatches(transition, currentState):
      continue
    let effectiveCtx = enrichInstructionContext(machine, transition, ctx)
    let guards = dottedGet(transition, "guards")
    if not evalGuardExpr(guards, effectiveCtx):
      return %*{
        "ok": false,
        "error": "guard_failed",
        "transition": dottedGetStr(transition, "name"),
        "command": command,
        "current_state": currentState,
      }
    var payload = %*{
      "ok": true,
      "transition": dottedGetStr(transition, "name"),
      "command": command,
      "current_state": currentState,
      "new_state": transitionTargetState(transition),
      "route": transitionRoute(transition, machine, effectiveCtx),
      "receipts": dottedGet(transition, "receipts", newJArray()),
      "instruction_bundle_composed": dottedGet(effectiveCtx, "instruction_bundle_composed", %false),
    }
    let runtimeSurface = deriveRuntimeSurface(
      machine,
      currentState,
      transitionTargetState(transition),
      command,
      transitionRoute(transition, machine, effectiveCtx),
      effectiveCtx,
    )
    payload["projection"] = runtimeSurface{"projection"}
    payload["listener_topics"] = runtimeSurface{"listener_topics"}
    payload["checkpoint"] = runtimeSurface{"checkpoint"}
    return payload

  %*{
    "ok": false,
    "error": "transition_not_allowed",
    "command": command,
    "current_state": currentState,
  }

proc projectTransitionOutcome*(machine: JsonNode, currentState, command: string,
    ctx: JsonNode = newJObject()): JsonNode =
  let transitionResult = applyTransition(machine, currentState, command, ctx)
  if not dottedGetBool(transitionResult, "ok", false):
    return transitionResult

  let routeLane = policyValue(transitionResult{"route"}, "")
  let checkpointRequired = dottedGetBool(transitionResult, "checkpoint.required", false)
  let checkpointKind = policyValue(transitionResult{"checkpoint"}{"kind"}, "none")

  var intents: seq[string] = @["projection.refresh"]
  if routeLane.len > 0:
    intents.add("dispatch.eligible")
  if checkpointRequired:
    intents.add("checkpoint.write")
  if dottedGetBool(transitionResult, "projection.gateway.active", false):
    intents.add("gateway.notify")

  result = %*{
    "ok": true,
    "projection": {
      "projection_type": "transition_projection",
      "machine": transitionResult{"projection"}{"machine"},
      "current_state": transitionResult{"current_state"},
      "new_state": transitionResult{"new_state"},
      "route_lane": transitionResult{"route"},
      "topics": transitionResult{"projection"}{"topics"},
      "gateway": transitionResult{"projection"}{"gateway"},
    },
    "checkpoint": {
      "checkpoint_type": "transition_checkpoint",
      "required": transitionResult{"checkpoint"}{"required"},
      "checkpoint_kind": transitionResult{"checkpoint"}{"kind"},
      "next_action": (if routeLane.len > 0: %("dispatch." & routeLane) else: %("checkpoint." & checkpointKind)),
    },
    "listener_intents": intents,
    "transition_result": transitionResult,
  }
