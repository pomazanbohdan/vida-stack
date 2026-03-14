# Orchestrator Runtime Boot Capsule

Purpose: provide a compact runtime-facing projection of the orchestrator boot path for routine startup and resume.

Boundary rule:

1. this file is a compact projection, not the owner of orchestrator boot law,
2. the canonical owners remain:
   - `AGENTS.md`
   - `agent-definitions/entry.orchestrator-entry`
   - `system-maps/bootstrap.orchestrator-boot-flow`
3. when boot profile selection, trigger scope, or an edge-case escalation is unclear, consult those owner surfaces directly.

## Shared Bootstrap

Always read first:

1. `AGENTS.md`
2. `AGENTS.sidecar.md`
3. bounded framework instruction-home surfaces only when the sidecar or active bootstrap route points to them

## Lean Boot Runtime View

Use for routine continuation and token-efficient startup.

1. enter orchestrator lane through `entry.orchestrator-entry.md`,
2. capture compact task-state snapshot first when the request is development-related,
3. hydrate active context capsule when task context exists,
4. read compact reasoning/control surfaces:
   - `overlay.step-thinking-runtime-capsule.md`
   - `overlay.session-context-continuity-protocol.md`
   - `core.orchestration-runtime-capsule.md` when execution work is active
5. read runtime/overlay surfaces:
   - `work.web-validation-protocol.md`
   - `bridge.project-overlay-runtime-capsule.md`
   - root `vida.config.yaml` when present
6. add only trigger-required surfaces:
   - `core.agent-system-runtime-capsule.md` when agent system is active
   - `overlay.autonomous-execution-runtime-capsule.md` when overlay-driven continuation/reporting behavior is active
   - `runtime.task-state-telemetry-protocol.md` when compact snapshot is insufficient
   - `analysis.silent-framework-diagnosis-protocol.md` when silent diagnosis is enabled
7. consult the owner file `core.agent-system-protocol.md` when the capsule does not settle routing, saturation, or verification-posture edge cases.
8. consult the owner file `overlay.autonomous-execution-protocol.md` when the capsule does not settle boundary, approval, or stop-condition edge cases.
9. use `bridge.instruction-activation-protocol.md` before expanding beyond the lean set.

## Standard / Full Expansion

1. `Standard`
   - add only route-triggered protocol owners such as TaskFlow, implement, run-graph, recovery, verification, or packet/prompt-stack surfaces.
2. `Full`
   - add full owner surfaces for architecture, unknown-root-cause, high-risk policy, or explicit meta-analysis work.

## Pre-Execution Gate

Before any write-producing action, be able to name:

1. active bounded unit,
2. lawful next slice,
3. lane sequence,
4. proof target,
5. why the root session remains `orchestrator`,
6. why bounded-unit binding is explicit rather than inferred.

If any item is missing, fail closed before implementation.

-----
artifact_path: config/system-maps/orchestrator-runtime-boot-capsule
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/bootstrap.orchestrator-runtime-capsule.md
created_at: '2026-03-13T22:10:00+02:00'
updated_at: '2026-03-13T23:20:00+02:00'
changelog_ref: bootstrap.orchestrator-runtime-capsule.changelog.jsonl
