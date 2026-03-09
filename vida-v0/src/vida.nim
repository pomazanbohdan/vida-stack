## VIDA v0 — Nim CLI entry point.
##
## Experimental Nim runtime for selected VIDA script surfaces.
## Usage: vida-v0 <subcommand> [args...]

import std/os
import core/config
import core/kernel_runtime
import boot/profile as bootProfile
import state/run_graph as runGraph
import state/todo as todoRuntime
import state/task as taskRuntime
import state/reconcile as reconcileRuntime
import state/memory as memoryRuntime
import state/context as contextRuntime
import state/context_capsule as contextCapsuleRuntime
import state/beads as beadsRuntime
import state/draft_execution_spec as draftExecutionSpecRuntime
import state/spec_intake as specIntakeRuntime
import state/spec_delta as specDeltaRuntime
import agents/[registry, leases, system, pool, route]
import agents/prepare_execution as prepareExecutionRuntime
import gates/[execution_auth, worker_packet, coach_review, coach_decision, verification_prompt]

const Version = "0.2.0"

proc printHelp() =
  echo """
VIDA v0 Runtime v""" & Version & """

Usage:
  vida-v0 <command> [args...]

Commands:
  config    Config validation and inspection
  kernel    Root config-law introspection and transition evaluation
  boot      Boot profile, packets, and snapshots
  run-graph Run graph ledger
  task      DB-backed task surface with JSONL ingest
  br        Legacy import/export compatibility over vida-v0 task store
  todo      TODO task views
  reconcile Task-state reconcile status
  draft-execution-spec Draft execution spec validator
  spec-intake Spec intake validator
  spec-delta Spec delta validator
  system    Subagent system runtime
  registry  Capability registry
  route     Route resolution and receipts
  lease     Resource lease management
  pool      Subagent pool (borrow/release)
  auth      Execution authorization gate
  worker    Worker packet validation
  coach     Coach review gate
  coach-decision Coach decision parse/merge helpers
  verification-prompt Render verification prompt text
  memory    Framework memory ledger
  context   Context governance
  context-capsule Compact task recovery capsule
  beads     Beads verification/runtime helpers
  prepare-execution Local artifact-based writer readiness bridge
  status    System status overview

Global Flags:
  --help      Show this help
  --version   Show version

Environment:
  VIDA_ROOT               Override project root (or set in .env)
  VIDA_RUN_GRAPH_STATE_DIR Override run-graph state directory

Current VIDA_ROOT: """ & vidaRoot() & """
"""

# ─────────────────────────── Config ───────────────────────────

proc cmdConfig(args: seq[string]): int =
  if args.len == 0: echo "Usage: vida-v0 config <validate|dump|protocol-active>"; return 1
  case args[0]
  of "validate": config.cmdValidate()
  of "dump": config.cmdDump()
  of "protocol-active":
    if args.len < 2: echo "Usage: vida-v0 config protocol-active <protocol>"; return 1
    config.cmdProtocolActive(args[1])
  else: echo "Unknown config subcommand: " & args[0]; 1

# ─────────────────────────── Status Overview ───────────────────────────

proc cmdStatus(args: seq[string]): int =
  echo "VIDA v0 Runtime v" & Version
  echo "VIDA_ROOT: " & vidaRoot()
  echo "Config: " & configPath()
  return 0

# ─────────────────────────── Main ───────────────────────────

proc main() =
  let args = commandLineParams()
  if args.len == 0: printHelp(); quit(0)
  if args[0] in ["--help", "-h"]: printHelp(); quit(0)
  if args[0] in ["--version", "-v"]: echo "vida-v0 " & Version; quit(0)

  let command = args[0]
  let subArgs = if args.len > 1: args[1..^1] else: @[]

  let exitCode = case command
    of "config": cmdConfig(subArgs)
    of "kernel": kernel_runtime.cmdKernel(subArgs)
    of "boot": bootProfile.cmdProfile(subArgs)
    of "snapshot": bootProfile.cmdProfile(@["snapshot"] & subArgs)
    of "run-graph": runGraph.cmdRunGraph(subArgs)
    of "task": taskRuntime.cmdTask(subArgs)
    of "br": taskRuntime.cmdBrCompat(subArgs)
    of "todo": todoRuntime.cmdTodo(subArgs)
    of "reconcile": reconcileRuntime.cmdReconcile(subArgs)
    of "system": system.cmdSystem(subArgs)
    of "registry": registry.cmdRegistry(subArgs)
    of "route": route.cmdRoute(subArgs)
    of "lease": leases.cmdLease(subArgs)
    of "pool": pool.cmdPool(subArgs)
    of "prepare-execution": prepareExecutionRuntime.cmdPrepareExecution(subArgs)
    of "auth": execution_auth.cmdAuthGate(subArgs)
    of "worker": worker_packet.cmdWorkerPacket(subArgs)
    of "coach": coach_review.cmdCoachGate(subArgs)
    of "coach-decision": coach_decision.cmdCoachDecision(subArgs)
    of "verification-prompt": verification_prompt.cmdVerificationPrompt(subArgs)
    of "memory": memoryRuntime.cmdMemory(subArgs)
    of "context": contextRuntime.cmdContext(subArgs)
    of "context-capsule": contextCapsuleRuntime.cmdContextCapsule(subArgs)
    of "beads": beadsRuntime.cmdBeads(subArgs)
    of "draft-execution-spec": draftExecutionSpecRuntime.cmdDraftExecutionSpec(subArgs)
    of "spec-intake": specIntakeRuntime.cmdSpecIntake(subArgs)
    of "spec-delta": specDeltaRuntime.cmdSpecDelta(subArgs)
    of "status": cmdStatus(subArgs)
    else:
      echo "Unknown command: " & command
      echo "Run `vida-v0 --help` to see available commands."
      1

  quit(exitCode)

when isMainModule:
  main()
