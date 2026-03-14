# Instruction Activation Runtime Capsule

Purpose: provide a compact runtime-facing projection of instruction activation law for routine orchestrator and worker boot.

Boundary rule:

1. this file is a compact projection, not the owner of activation law,
2. the canonical owner remains `instruction-contracts/bridge.instruction-activation-protocol`,
3. use the owner file when introducing new protocol surfaces, resolving edge-case activation disputes, or auditing protocol coverage.

## Activation Classes

Each instruction surface belongs to one of:

1. `always_on`
2. `lane_entry`
3. `triggered_domain`
4. `closure_reflection`

If the class is unclear, fail closed and consult the owner protocol.

## Runtime Rule

1. bootstrap reads only always-on and lane-entry surfaces,
2. load triggered-domain surfaces only when route, risk, artifact flow, or runtime gate requires them,
3. load closure/reflection surfaces only near checkpoint, handoff, finish, or diagnosis reflection,
4. do not bulk-read protocols merely because they exist.

## High-Frequency Triggers

1. repository or runtime mutation required:
   - `work.taskflow-protocol.md`
   - `runtime.task-state-telemetry-protocol.md`
2. request-intent classification, worker-first coordination, or lawful-next selection beyond `answer_only`:
   - `core.orchestration-runtime-capsule.md`
   - owner file `core.orchestration-protocol.md` for edge cases
3. worker-first routing, mode posture, backend fallback, or saturation-recovery posture:
   - `core.agent-system-runtime-capsule.md`
   - owner file `core.agent-system-protocol.md` for edge cases
4. external facts can change the decision:
   - `work.web-validation-protocol.md`
5. visible skill catalog or skill-bound work:
   - `core.skill-activation-protocol.md`
6. packet shaping / leaf-depth selection:
   - `core.packet-decomposition-protocol.md`
7. worker packet / handoff / next-agent prompt work:
   - `lane.agent-handoff-context-protocol.md`
8. tracked execution with multiple lawful next items:
   - `work.execution-priority-protocol.md`
9. silent diagnosis enabled:
   - `analysis.silent-framework-diagnosis-protocol.md`
10. restart / checkpoint / replay / resumability:
   - `recovery.checkpoint-replay-recovery-protocol.md`
   - `core.run-graph-protocol.md`
11. settled plan/spec/task pool should continue to completion:
   - `overlay.autonomous-execution-runtime-capsule.md`
   - owner file `overlay.autonomous-execution-protocol.md` for edge cases
12. separated authorship, verifier independence, or closure proof semantics are active:
   - `work.verification-lane-runtime-capsule.md`
   - owner file `work.verification-lane-protocol.md` for edge cases

## Bootstrap Defaults

1. `AGENTS.md` owns global bootstrap invariants,
2. lane entry files own boot-profile selection and next-read routing,
3. use compact runtime capsules first where available,
4. consult owner protocols only when the capsule does not settle the current activation question.

## Escalate To Owner Protocol When

1. a new protocol-bearing file is being added or renamed,
2. activation class is disputed,
3. trigger conditions conflict,
4. protocol-coverage/index consistency is being audited,
5. a broad refactor changes ownership boundaries.

-----
artifact_path: config/instructions/instruction-contracts/bridge.instruction-activation-runtime-capsule
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/bridge.instruction-activation-runtime-capsule.md
created_at: '2026-03-13T22:25:00+02:00'
updated_at: '2026-03-14T00:35:00+02:00'
changelog_ref: bridge.instruction-activation-runtime-capsule.changelog.jsonl
