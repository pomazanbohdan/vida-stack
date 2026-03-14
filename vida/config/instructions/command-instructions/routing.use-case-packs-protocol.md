# Use-Case Packs (Slim-VIDA)

Purpose: define the canonical pack taxonomy and high-level routing intent for bounded VIDA work without leaving step-level execution law in the pack selector itself.

## Core Packs

| Pack | Trigger | Minimal Inputs | Mandatory Outputs |
|---|---|---|---|
| `research-pack` | Unknown domain, external validation needed | user goal, scope limits | source-backed findings, risks, next options |
| `spec-pack` | Requirement/spec creation or update | target feature, constraints | updated spec scope, AC, edge cases |
| `work-pool-pack` | build/update task pool between spec and dev | approved scope/spec, priority, dependencies | decomposed task pool in the DB-backed task runtime + launch decision |
| `dev-pack` | start/continue implementation | active `TaskFlow` task, target files | code/test changes + verification |
| `bug-pool-pack` | bug triage/fix loop | bug evidence, reproduction | root-cause fix + regression checks |
| `reflection-pack` | decisions/docs drift, scope/AC/dependency drift, or explicit tracked framework reflection flow | accepted decisions, touched docs, drift trigger | synchronized contracts/docs/task-pool or reflection evidence |

## Runtime Contract

Generic orchestration lifecycle is owned by `instruction-contracts/core.orchestration-protocol`.

This file owns only:

1. pack taxonomy,
2. pack-selection intent,
3. high-level pack sequence,
4. pack trigger/input/output matrix,
5. thin pointers to deeper canonical owners.

This file does not own:

1. step-level pack execution law,
2. wrapper command behavior,
3. pack handoff law,
4. change-impact reconciliation law,
5. framework self-analysis workflow law.

## Canonical Pack Routing Map

Use each pack only as a routing surface to the deeper canonical owner.

This section is the active pack-level routing/discoverability map.

It must stay thin:

1. it may show pack -> canonical owner routing,
2. it must not absorb step-level pack law,
3. it must not duplicate handoff, completion, or diagnostic law already owned elsewhere.

1. `research-pack`
   - primary owners:
     - `runtime-instructions/work.spec-contract-protocol`
     - `runtime-instructions/work.web-validation-protocol` when external-fact triggers fire
2. `spec-pack`
   - primary owner:
     - `runtime-instructions/work.spec-contract-protocol`
3. `work-pool-pack`
   - primary owner:
     - `command-instructions/planning.form-task-protocol`
4. `dev-pack`
   - primary owner:
     - `command-instructions/execution.implement-execution-protocol`
5. `bug-pool-pack`
   - primary owners:
     - `command-instructions/execution.bug-fix-protocol`
     - `runtime-instructions/bridge.issue-contract-protocol` when bug intake must be normalized before writer start
6. `reflection-pack`
   - primary owners:
     - `instruction-contracts/work.documentation-operation-protocol`
     - `runtime-instructions/work.pack-handoff-protocol` for pack-boundary reconciliation and admissible handoff state
     - `diagnostic-instructions/analysis.framework-self-analysis-protocol` for tracked framework self-analysis

## Pack Boundary Rules

1. `research-pack`, `spec-pack`, `work-pool-pack`, `bug-pool-pack`, and `reflection-pack` remain non-dev packs and must respect `runtime-instructions/work.spec-contract-protocol` where applicable.
2. `dev-pack` remains the implementation route and must use `command-instructions/execution.implement-execution-protocol`.
3. When external-fact uncertainty is material, activate `runtime-instructions/work.web-validation-protocol` rather than embedding ad hoc lookup law inside pack routing.
4. When scope, AC, dependency order, or approved decisions drift, route reconciliation per `instruction-contracts/core.orchestration-protocol`, `command-instructions/planning.form-task-protocol`, and `command-instructions/execution.implement-execution-protocol`.
5. When explicit tracked framework self-analysis is in scope, `reflection-pack` acts only as the routing bridge; the diagnostic law remains owned by `diagnostic-instructions/analysis.framework-self-analysis-protocol`.
6. Ordinary documentation drift, spec/task-pool synchronization, or generic change-impact reconciliation inside `reflection-pack` must not be relabeled as FSAP unless tracked framework self-analysis/remediation is the actual target.

## Pack Sequence

Default multi-pack order:

1. `research-pack`
2. `spec-pack`
3. `work-pool-pack`
4. `dev-pack` or `bug-pool-pack`
5. `reflection-pack`

Pack handoff admissibility, required outputs, and boundary law are owned by `runtime-instructions/work.pack-handoff-protocol`.

## Migration Boundary

Legacy wrapper surfaces such as `vida-pack-helper.sh` and `nondev-pack-init.sh` remain migration-only helpers.

They are not canonical pack-law owners.

Migration-only wrapper cataloging remains centralized in `command-instructions/migration.pack-wrapper-note`.

Wrapper retirement and historical-only mapping remain owned by `system-maps/migration.runtime-transition-map`.

## Notes

1. `vida taskflow task` remains the only task-state source of truth.
2. TaskFlow board is execution visibility, not task-state authority.
3. This file should stay thin; deeper flow, gate, and recovery law must remain in their canonical protocol owners.

-----
artifact_path: config/command-instructions/use-case-packs
artifact_type: command_instruction
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/command-instructions/routing.use-case-packs-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-11T13:25:34+02:00'
changelog_ref: routing.use-case-packs-protocol.changelog.jsonl
