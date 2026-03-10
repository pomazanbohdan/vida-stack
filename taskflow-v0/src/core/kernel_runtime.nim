## VIDA Kernel Runtime CLI — root config introspection, assignment, and transition evaluation.

import std/json
import ./[agent_inventory, assignment_engine, instruction_engine, kernel_config, transition_engine, toon, utils]
import ../agents/route

proc printHelp() =
  echo """
Usage:
  taskflow-v0 kernel summary
  taskflow-v0 kernel machine <name> [--json]
  taskflow-v0 kernel route <name> [--json]
  taskflow-v0 kernel agents [--json]
  taskflow-v0 kernel instruction <role> [machine] [route] [--json]
  taskflow-v0 kernel policy <name> [--json]
  taskflow-v0 kernel assign lane <lane> [context_json] [--json]
  taskflow-v0 kernel assign task-class <task_class> [context_json] [--json]
  taskflow-v0 kernel transition <machine> <current_state> <command> [context_json]
  taskflow-v0 kernel project transition <machine> <current_state> <command> [context_json] [--json]
  taskflow-v0 kernel project route <task_class> [task_id] [--json]
"""

proc emitPayload(payload: JsonNode, asJson: bool) =
  if asJson:
    echo pretty(normalizeJson(payload))
  else:
    echo renderToon(normalizeJson(payload))

proc wantsJson(args: seq[string]): bool =
  for arg in args:
    if arg == "--json":
      return true
  false

proc cmdKernel*(args: seq[string]): int =
  if args.len == 0:
    printHelp()
    return 1

  case args[0]
  of "summary":
    emitPayload(kernelSummary(), wantsJson(args))
    return 0

  of "machine":
    if args.len < 2:
      echo "Usage: taskflow-v0 kernel machine <name> [--json]"
      return 1
    let payload = loadMachineSpec(args[1])
    if payload.kind != JObject or payload.len == 0:
      echo "unknown machine: " & args[1]
      return 1
    emitPayload(payload, wantsJson(args))
    return 0

  of "route":
    if args.len < 2:
      echo "Usage: taskflow-v0 kernel route <name> [--json]"
      return 1
    let catalog = loadRouteCatalog()
    let payload = dottedGet(catalog, "routes." & args[1], newJObject())
    if payload.kind != JObject or payload.len == 0:
      echo "unknown route: " & args[1]
      return 1
    emitPayload(%*{"route_name": args[1], "route": payload}, wantsJson(args))
    return 0

  of "agents":
    emitPayload(buildRuntimeAgentInventory(), wantsJson(args))
    return 0

  of "policy":
    if args.len < 2:
      echo "Usage: taskflow-v0 kernel policy <name> [--json]"
      return 1
    let payload = loadPolicySpec(args[1])
    if payload.kind != JObject or payload.len == 0:
      echo "unknown policy: " & args[1]
      return 1
    emitPayload(payload, wantsJson(args))
    return 0

  of "instruction":
    if args.len < 2:
      echo "Usage: taskflow-v0 kernel instruction <role> [machine] [route] [--json]"
      return 1
    let machineName =
      if args.len >= 3 and args[2] != "--json": args[2]
      else: ""
    let routeName =
      if args.len >= 4 and args[3] != "--json": args[3]
      else: ""
    let payload = composeInstructionSurface(args[1], machineName, routeName)
    emitPayload(payload, wantsJson(args))
    return (if dottedGetBool(payload, "ok", false): 0 else: 1)

  of "assign":
    if args.len < 3:
      echo "Usage: taskflow-v0 kernel assign <lane|task-class> <name> [context_json] [--json]"
      return 1
    let ctx =
      if args.len >= 4 and args[3] != "--json":
        try:
          parseJson(args[3])
        except:
          echo "invalid context_json"
          return 1
      else:
        newJObject()
    let payload =
      case args[1]
      of "lane":
        resolveAssignmentForLane(args[2], ctx)
      of "task-class":
        resolveAssignmentForTaskClass(args[2], ctx)
      else:
        echo "Usage: taskflow-v0 kernel assign <lane|task-class> <name> [context_json] [--json]"
        return 1
    emitPayload(payload, wantsJson(args))
    return (if dottedGetBool(payload, "ok", false): 0 else: 1)

  of "transition":
    if args.len < 4:
      echo "Usage: taskflow-v0 kernel transition <machine> <current_state> <command> [context_json]"
      return 1
    let machine = loadMachineSpec(args[1])
    if machine.kind != JObject or machine.len == 0:
      echo "unknown machine: " & args[1]
      return 1
    let ctx =
      if args.len >= 5:
        try:
          parseJson(args[4])
        except:
          echo "invalid context_json"
          return 1
      else:
        newJObject()
    let payload = applyTransition(machine, args[2], args[3], ctx)
    emitPayload(payload, wantsJson(args))
    return (if dottedGetBool(payload, "ok", false): 0 else: 1)

  of "project":
    if args.len < 2:
      echo "Usage: taskflow-v0 kernel project <transition|route> ..."
      return 1
    case args[1]
    of "transition":
      if args.len < 5:
        echo "Usage: taskflow-v0 kernel project transition <machine> <current_state> <command> [context_json] [--json]"
        return 1
      let machine = loadMachineSpec(args[2])
      if machine.kind != JObject or machine.len == 0:
        echo "unknown machine: " & args[2]
        return 1
      let ctx =
        if args.len >= 6 and args[5] != "--json":
          try:
            parseJson(args[5])
          except:
            echo "invalid context_json"
            return 1
        elif args.len >= 6 and args[5] == "--json":
          newJObject()
        else:
          newJObject()
      let payload = projectTransitionOutcome(machine, args[3], args[4], ctx)
      emitPayload(payload, wantsJson(args))
      return (if dottedGetBool(payload, "ok", false): 0 else: 1)
    of "route":
      if args.len < 3:
        echo "Usage: taskflow-v0 kernel project route <task_class> [task_id] [--json]"
        return 1
      let taskId =
        if args.len >= 4 and args[3] != "--json": args[3]
        else: ""
      let payload = projectRouteSnapshot(args[2], taskId)
      emitPayload(payload, wantsJson(args))
      return (if dottedGetBool(payload, "ok", false): 0 else: 1)
    else:
      echo "Usage: taskflow-v0 kernel project <transition|route> ..."
      return 1

  else:
    echo "Unknown kernel subcommand: " & args[0]
    return 1
