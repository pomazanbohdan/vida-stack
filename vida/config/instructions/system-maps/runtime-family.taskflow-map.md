# Runtime Family Map: TaskFlow

Purpose: define the bounded `taskflow` runtime family surface used for tracked execution, boot, task routing, run-graph state, and transitional implementation runtime behavior.

## Runtime Identity

1. runtime family: `taskflow`
2. root surface: `vida taskflow`
3. current role: task management plus optional dispatch/runtime substrate exposed through the TaskFlow runtime family
4. framework relationship: independently usable runtime family under the unified VIDA framework map

## Canonical Surfaces

1. runtime launcher surface:
   - `vida taskflow`
   - `vida taskflow dispatch` (opt-in worker/run-graph runtime)
2. runtime implementation/workspace:
   - TaskFlow runtime family implementation surfaces referenced through the runtime-family map and launcher source tree
3. runtime tests:
   - TaskFlow runtime family proof/test surfaces in the active source tree
4. governing framework protocols:
   - `runtime-instructions/work.taskflow-protocol`
   - `system-maps/migration.runtime-transition-map`
   - `runtime-instructions/lane.agent-handoff-context-protocol`
   - `runtime-instructions/recovery.checkpoint-replay-recovery-protocol`
   - `runtime-instructions/work.verification-lane-protocol`
   - `runtime-instructions/runtime.runtime-kernel-bundle-protocol`
   - `runtime-instructions/work.verification-merge-protocol`
   - `runtime-instructions/runtime.direct-runtime-consumption-protocol`
   - `system-maps/observability.map`

## Activation Triggers

Read this map when:

1. tracked execution, boot, route, run-graph, task, or system runtime behavior is active,
2. the task is about `vida taskflow` commands or TaskFlow runtime-family ownership,
3. runtime-family discoverability or runtime-transition questions are active,
4. a task asks about the `taskflow` runtime family directly.
5. handoff/context shaping, checkpoint/recovery, replay safety, verification-lane routing, or runtime observability is active.
6. the final `taskflow` runtime layer is being discussed, especially direct runtime consumption, runtime closure trust, or final consumption wiring into canonical documentation/readiness surfaces.
7. project role/skill/profile/flow extension activation or validation is active.

## Routing

1. Transitional runtime behavior and execution law:
   - continue to `runtime-instructions/work.taskflow-protocol`
2. Runtime transition from legacy helpers and script/runtime boundary:
   - continue to `system-maps/migration.runtime-transition-map`
3. Concrete runtime commands and workspace details:
   - continue to `vida taskflow help`
   - default human-facing output is `TOON`; use `--json` only for explicit machine/debug output
   - concrete `run-graph`, `registry`, and `context` command syntax lives there rather than in peer `core` owner protocols
4. Handoff/context shaping:
   - continue to `runtime-instructions/lane.agent-handoff-context-protocol`
5. Checkpoint, replay, or recovery:
   - continue to `runtime-instructions/recovery.checkpoint-replay-recovery-protocol`
6. Verification/proving lane behavior:
   - continue to `runtime-instructions/work.verification-lane-protocol`
7. Runtime kernel bundle composition:
   - continue to `runtime-instructions/runtime.runtime-kernel-bundle-protocol`
8. Verification merge/admissibility:
   - continue to `runtime-instructions/work.verification-merge-protocol`
9. Runtime observability, traces, and proving:
   - continue to `system-maps/observability.map`
10. Final runtime-consumption activation and closure handoff:
   - continue to `runtime-instructions/runtime.direct-runtime-consumption-protocol`
   - continue to `system-maps/runtime-family.docflow-map`
   - taskflow remains the runtime-consumption owner, but it must activate the bounded `DocFlow` surface to resolve canonical inventory, readiness, bundle, and documentation-consumption evidence before final trust/closure is considered satisfied.
11. Project role/skill/profile/flow extension activation:
   - continue to `runtime-instructions/work.project-agent-extension-protocol`

## Boundary Rule

1. `taskflow` contains an always-on management runtime and an opt-in dispatch runtime; neither owns framework-wide semantic canon.
2. Management closure is valid without dispatch when `taskflow.dispatch.enabled` is false.
3. Dispatch owns execution-driven transitions only for execution-bound tasks when enabled.
4. Framework law remains in `AGENTS.md`, `vida/config/**`, and canonical docs.
5. The TaskFlow runtime family implementation surfaces are the bounded implementation/runtime family surface that consumes that law.
6. When final runtime consumption is being wired or evaluated, `taskflow` must not infer canonical inventory/readiness consumption from implementation presence alone; it must explicitly activate the bounded `DocFlow` runtime-family surface as the canonical downstream documentation/readiness map.

-----
artifact_path: config/system-maps/runtime-family.taskflow
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/runtime-family.taskflow-map.md
created_at: '2026-03-10T08:45:00+02:00'
updated_at: '2026-03-11T15:17:28+02:00'
changelog_ref: runtime-family.taskflow-map.changelog.jsonl
