# Orchestrator Boot Flow Map

Purpose: provide the explicit step-by-step boot flow for the orchestrator lane so orchestrator startup does not need to be reconstructed from `AGENTS.md`, lane-entry prose, and activation law by inference.

## Activation Trigger

Read this map when:

1. worker-lane confirmation is absent,
2. the runtime explicitly places the agent in orchestrator lane,
3. bootstrap or compact recovery needs the exact orchestrator boot path,
4. framework work asks for the canonical orchestrator startup flow.

## Step 0. Shared Bootstrap

The orchestrator boot always begins with the mandatory shared bootstrap:

1. `AGENTS.md`
2. `AGENTS.sidecar.md`
3. `vida/root-map.md`

Rules:

1. lane resolution happens only after the shared bootstrap carriers and maps are read,
2. `AGENTS.sidecar.md` remains the project-doc bootstrap map,
3. `vida/root-map.md` remains the framework root map.

## Step 1. Enter The Orchestrator Lane

1. Read `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`.
2. Apply `vida/config/instructions/instruction-contracts/bridge.instruction-activation-runtime-capsule.md`, consulting `bridge.instruction-activation-protocol.md` when activation semantics are unclear.
3. If the task is documentation-shaped, activate `vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md` immediately.

## Step 2. Classify Request Intent

Before boot-profile branching, classify the current request into:

1. `answer_only`
2. `artifact_flow`
3. `execution_flow`
4. `mixed`

Intent authority:

1. `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`
2. `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md` when orchestration route selection beyond `answer_only` is required

Clean-session execution rule:

1. if a fresh or resumed session starts with execution intent such as `continue development`, the next required result is an explicit orchestrator-first route receipt, not local implementation,
2. if that receipt is not yet visible, bootstrap is incomplete and write-producing execution must not start,
3. generic execution intent must not be interpreted as permission to skip orchestration-first routing on a clean session.

## Step 3. Select Boot Profile

Use the orchestrator entry contract to choose one boot profile:

1. `Lean`
2. `Standard`
3. `Full`

Profile selection authority:

1. `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`

## Step 4. Lean Boot Minimum Read Set

For the compact runtime-facing Lean view, use:

1. `vida/config/instructions/system-maps/bootstrap.orchestrator-runtime-capsule.md`

Owner notes:

1. `entry.orchestrator-entry.md` remains the owner of boot-profile selection and lean-boot semantics,
2. this map remains the owner of the explicit end-to-end orchestrator boot route,
3. when a Lean edge case is unclear, fall back to the owner surfaces rather than improvising.

## Step 5. Standard Boot Expansion

`Standard` boot executes `Lean` boot first, then adds only route-triggered surfaces:

1. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md` when tracked execution is required
2. `vida/config/instructions/command-instructions/routing.use-case-packs-protocol.md` when a pack path is required
3. `vida/config/instructions/command-instructions/execution.implement-execution-protocol.md` when implementation flow is in scope
4. `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md` when request intent classification, orchestration route selection, or worker-first coordination is required beyond `answer_only`
5. `vida/config/instructions/instruction-contracts/core.skill-activation-protocol.md` when a visible skill catalog exists or skill-bound work is active
6. `vida/config/instructions/instruction-contracts/core.packet-decomposition-protocol.md` when bounded packet shaping or leaf-depth selection is active
7. `vida/config/instructions/instruction-contracts/core.agent-prompt-stack-protocol.md` when packet/routing work depends on explicit prompt-layer precedence
8. `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md` when node-level resumability, route control limits, or checkpoint-visible continuation is active
9. `vida/config/instructions/runtime-instructions/recovery.checkpoint-replay-recovery-protocol.md` when restart, resumability, checkpoint, replay, or duplicate-delivery safety is active
10. `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md` when separated authorship, verifier-independence, or closure-proof semantics are active
11. `vida/config/instructions/runtime-instructions/work.verification-merge-protocol.md` when review-pool or merged verification admissibility is active

## Step 6. Full Boot Expansion

`Full` boot executes `Standard` boot first, then adds:

1. `vida/config/instructions/command-instructions/operator.runtime-pipeline-guide.md`

Use `Full` only for:

1. architecture/topology refactor,
2. unknown root cause,
3. cross-module integration,
4. security/data-safety decisions,
5. explicit meta-analysis,
6. confidence below `80%` after `Standard`.

## Step 7. Route To Domain Protocols

After intent classification:

1. use `vida/config/instructions/system-maps/protocol.index.md` for domain protocol lookup,
2. use `vida/config/instructions/system-maps/runtime-family.index.md` for runtime-family routing,
3. keep additional reads trigger-bound under `vida/config/instructions/instruction-contracts/bridge.instruction-activation-runtime-capsule.md`, consulting the owner activation protocol for edge cases,
4. do not assume that implementation, recovery, verification, or resumability owners are active merely because execution flow exists; load them only when their trigger conditions are satisfied.

Pre-execution gate:

1. before any write-producing action on a clean session, the orchestrator must be able to name:
   - the active bounded unit,
   - the lawful next slice,
   - the lane sequence,
   - the proof target,
   - and the reason the root session is still acting as orchestrator
2. if any of those are missing, fail closed before implementation.

## Boundary Rule

1. This map describes orchestrator startup only.
2. It does not replace the orchestrator entry contract.
3. It must remain a thin explicit boot route, not a second full orchestrator manual.

-----
artifact_path: config/system-maps/orchestrator-boot-flow
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/bootstrap.orchestrator-boot-flow.md
created_at: '2026-03-10T15:05:00+02:00'
updated_at: '2026-03-13T07:44:24+02:00'
changelog_ref: bootstrap.orchestrator-boot-flow.changelog.jsonl
