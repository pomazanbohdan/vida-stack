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

## Activation Triggers

Read this map when:

1. tracked execution, boot, route, run-graph, task, or system runtime behavior is active,
2. the task is about `taskflow-v0` commands or transitional runtime ownership,
3. runtime-family discoverability or runtime-transition questions are active,
4. a task asks about the `taskflow` runtime family directly.

## Routing

1. Transitional runtime behavior and execution law:
   - continue to `vida/config/instructions/runtime-instructions.taskflow-protocol.md`
2. Runtime transition from legacy helpers:
   - continue to `vida/config/instructions/system-maps.runtime-transition-map.md`
3. Script/runtime architecture boundary:
   - continue to `vida/config/instructions/system-maps.script-runtime-architecture.md`
4. Concrete runtime commands and workspace details:
   - continue to `taskflow-v0/README.md`

## Boundary Rule

1. `taskflow` is the current execution substrate, not the owner of framework-wide semantic canon.
2. Framework law remains in `AGENTS.md`, `vida/config/**`, and canonical docs.
3. `taskflow-v0/**` is the bounded implementation/runtime family surface that consumes that law.

-----
artifact_path: config/system-maps/runtime-family.taskflow
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps.runtime-family-taskflow.md
created_at: '2026-03-10T08:45:00+02:00'
updated_at: '2026-03-10T08:45:00+02:00'
changelog_ref: system-maps.runtime-family-taskflow.changelog.jsonl
