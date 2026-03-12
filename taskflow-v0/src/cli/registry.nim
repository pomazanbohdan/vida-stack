import ../config/loader

const Version* = "0.2.0"

proc printHelp*() =
  echo """
VIDA v0 Runtime v""" & Version & """

Usage:
  taskflow-v0 <command> [args...]

Commands:
  config    Config validation and inspection
  kernel    Root config-law introspection and transition evaluation
  boot      Boot profile, packets, and snapshots
  run-graph Run graph ledger
  task      DB-backed task surface with JSONL ingest
  br        Legacy import/export compatibility over taskflow-v0 task store
  todo      TODO task views
  reconcile Task-state reconcile status
  draft-execution-spec Draft execution spec validator
  spec-intake Spec intake validator
  spec-delta Spec delta validator
  problem-party Party Chat / problem_party manifest and receipt helper
  system    Agent-backend system runtime
  registry  Capability registry
  route     Route resolution and receipts
  role-select Auto-role selection and compiled agent-extension bundle
  bundle    Compiled runtime bundle and bundle readiness
  lease     Resource lease management
  pool      Agent-backend pool (borrow/release)
  auth      Execution authorization gate
  worker    Worker packet validation
  coach     Coach review gate
  coach-decision Coach decision parse/merge helpers
  verification Executable verifier admissibility and merge
  verification-prompt Render verification prompt text
  recovery  Checkpoint, gateway, and resumability helpers
  consume   Direct runtime consumption and taskflow -> docflow final loop
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
Current VIDA_WORKSPACE: """ & vidaWorkspaceDir() & """
"""
