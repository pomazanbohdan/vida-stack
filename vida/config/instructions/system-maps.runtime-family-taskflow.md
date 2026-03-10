# Runtime Family Map: TaskFlow

Purpose: define the bounded `taskflow` runtime family surface used for tracked execution, boot, task routing, run-graph state, and transitional implementation runtime behavior.

## Runtime Identity

1. runtime family: `taskflow`
2. root surface: `taskflow-v0/`
3. current role: transitional execution/runtime substrate for VIDA `0.2.0`
4. framework relationship: independently usable runtime family under the unified VIDA framework map

## Canonical Surfaces

1. runtime readme:
   - `taskflow-v0/README.md`
2. runtime source/workspace:
   - `taskflow-v0/src/**`
3. runtime tests:
   - `taskflow-v0/tests/**`
4. governing framework protocols:
   - `vida/config/instructions/runtime-instructions.taskflow-protocol.md`
   - `vida/config/instructions/system-maps.runtime-transition-map.md`
   - `vida/config/instructions/system-maps.script-runtime-architecture.md`
   - `vida/config/instructions/runtime-instructions.agent-handoff-context-protocol.md`
   - `vida/config/instructions/runtime-instructions.checkpoint-replay-recovery-protocol.md`
   - `vida/config/instructions/runtime-instructions.verification-lane-protocol.md`
   - `vida/config/instructions/runtime-instructions.runtime-kernel-bundle-protocol.md`
   - `vida/config/instructions/runtime-instructions.verification-merge-protocol.md`
   - `vida/config/instructions/runtime-instructions.direct-runtime-consumption-protocol.md`
   - `vida/config/instructions/system-maps.observability-map.md`

## Activation Triggers

Read this map when:

1. tracked execution, boot, route, run-graph, task, or system runtime behavior is active,
2. the task is about `taskflow-v0` commands or transitional runtime ownership,
3. runtime-family discoverability or runtime-transition questions are active,
4. a task asks about the `taskflow` runtime family directly.
5. handoff/context shaping, checkpoint/recovery, replay safety, verification-lane routing, or runtime observability is active.
6. the final `taskflow` runtime layer is being discussed, especially direct runtime consumption, runtime closure trust, or final consumption wiring into canonical documentation/readiness surfaces.
7. project role/skill/profile/flow extension activation or validation is active.

## Routing

1. Transitional runtime behavior and execution law:
   - continue to `vida/config/instructions/runtime-instructions.taskflow-protocol.md`
2. Runtime transition from legacy helpers:
   - continue to `vida/config/instructions/system-maps.runtime-transition-map.md`
3. Script/runtime architecture boundary:
   - continue to `vida/config/instructions/system-maps.script-runtime-architecture.md`
4. Concrete runtime commands and workspace details:
   - continue to `taskflow-v0/README.md`
   - default human-facing output is `TOON`; use `--json` only for explicit machine/debug output
5. Handoff/context shaping:
   - continue to `vida/config/instructions/runtime-instructions.agent-handoff-context-protocol.md`
6. Checkpoint, replay, or recovery:
   - continue to `vida/config/instructions/runtime-instructions.checkpoint-replay-recovery-protocol.md`
7. Verification/proving lane behavior:
   - continue to `vida/config/instructions/runtime-instructions.verification-lane-protocol.md`
8. Runtime kernel bundle composition:
   - continue to `vida/config/instructions/runtime-instructions.runtime-kernel-bundle-protocol.md`
9. Verification merge/admissibility:
   - continue to `vida/config/instructions/runtime-instructions.verification-merge-protocol.md`
10. Runtime observability, traces, and proving:
   - continue to `vida/config/instructions/system-maps.observability-map.md`
11. Final runtime-consumption activation and closure handoff:
   - continue to `vida/config/instructions/runtime-instructions.direct-runtime-consumption-protocol.md`
   - continue to `vida/config/instructions/system-maps.runtime-family-codex.md`
   - taskflow remains the runtime-consumption owner, but it must activate the bounded `codex` surface to resolve canonical inventory, readiness, bundle, and documentation-consumption evidence before final trust/closure is considered satisfied.
12. Project role/skill/profile/flow extension activation:
   - continue to `vida/config/instructions/runtime-instructions.project-agent-extension-protocol.md`

## Boundary Rule

1. `taskflow` is the current execution substrate, not the owner of framework-wide semantic canon.
2. Framework law remains in `AGENTS.md`, `vida/config/**`, and canonical docs.
3. `taskflow-v0/**` is the bounded implementation/runtime family surface that consumes that law.
4. When final runtime consumption is being wired or evaluated, `taskflow` must not infer canonical inventory/readiness consumption from implementation presence alone; it must explicitly activate the bounded `codex` runtime-family surface as the canonical downstream documentation/readiness map.

-----
artifact_path: config/system-maps/runtime-family.taskflow
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps.runtime-family-taskflow.md
created_at: '2026-03-10T08:45:00+02:00'
updated_at: '2026-03-10T17:22:42+02:00'
changelog_ref: system-maps.runtime-family-taskflow.changelog.jsonl
