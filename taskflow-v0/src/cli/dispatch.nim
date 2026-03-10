import ../core/[direct_consumption, kernel_runtime, role_selection, runtime_bundle]
import ../boot/profile as bootProfile
import ../state/[run_graph, todo, task, reconcile, memory, context, context_capsule,
  beads, recovery, draft_execution_spec, spec_intake, spec_delta, problem_party]
import ../agents/[registry as agentRegistry, leases, system, pool, route]
import ../agents/prepare_execution as prepareExecutionRuntime
import ../gates/[execution_auth, worker_packet, coach_review, coach_decision, verification_prompt, verification_merge]
import ./[registry]
import ./commands/[config_cmd, status_cmd]

proc runCli*(args: seq[string]): int =
  if args.len == 0:
    printHelp()
    return 0
  if args[0] in ["--help", "-h"]:
    printHelp()
    return 0
  if args[0] in ["--version", "-v"]:
    echo "taskflow-v0 " & Version
    return 0

  let command = args[0]
  let subArgs = if args.len > 1: args[1..^1] else: @[]

  case command
  of "config": cmdConfig(subArgs)
  of "kernel": kernel_runtime.cmdKernel(subArgs)
  of "boot": bootProfile.cmdProfile(subArgs)
  of "snapshot": bootProfile.cmdProfile(@["snapshot"] & subArgs)
  of "run-graph": run_graph.cmdRunGraph(subArgs)
  of "task": task.cmdTask(subArgs)
  of "br": task.cmdBrCompat(subArgs)
  of "todo": todo.cmdTodo(subArgs)
  of "reconcile": reconcile.cmdReconcile(subArgs)
  of "system": system.cmdSystem(subArgs)
  of "registry": agentRegistry.cmdRegistry(subArgs)
  of "route": route.cmdRoute(subArgs)
  of "role-select": role_selection.cmdRoleSelection(subArgs)
  of "bundle": runtime_bundle.cmdRuntimeBundle(subArgs)
  of "lease": leases.cmdLease(subArgs)
  of "pool": pool.cmdPool(subArgs)
  of "prepare-execution": prepareExecutionRuntime.cmdPrepareExecution(subArgs)
  of "auth": execution_auth.cmdAuthGate(subArgs)
  of "worker": worker_packet.cmdWorkerPacket(subArgs)
  of "coach": coach_review.cmdCoachGate(subArgs)
  of "coach-decision": coach_decision.cmdCoachDecision(subArgs)
  of "verification": verification_merge.cmdVerificationMerge(subArgs)
  of "verification-prompt": verification_prompt.cmdVerificationPrompt(subArgs)
  of "recovery": recovery.cmdRecovery(subArgs)
  of "consume": direct_consumption.cmdDirectConsumption(subArgs)
  of "memory": memory.cmdMemory(subArgs)
  of "context": context.cmdContext(subArgs)
  of "context-capsule": context_capsule.cmdContextCapsule(subArgs)
  of "beads": beads.cmdBeads(subArgs)
  of "draft-execution-spec": draft_execution_spec.cmdDraftExecutionSpec(subArgs)
  of "spec-intake": spec_intake.cmdSpecIntake(subArgs)
  of "spec-delta": spec_delta.cmdSpecDelta(subArgs)
  of "problem-party": problem_party.cmdProblemParty(subArgs)
  of "status": status_cmd.cmdStatus(subArgs)
  else:
    echo "Unknown command: " & command
    echo "Run `taskflow-v0 --help` to see available commands."
    1
