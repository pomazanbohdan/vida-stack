# Pack Wrapper Migration Note

Purpose: keep one migration-only helper note for legacy pack wrappers so active pack/routing protocols do not need to repeat wrapper catalog text.

Status note:

1. this file is `non-canonical reference material`,
2. it does not own pack law, handoff law, completion law, or orchestration law,
3. it exists only to centralize migration-only wrapper entrypoints while the TaskFlow runtime family remains the target runtime home.

## Legacy Pack Wrapper Catalog

Legacy wrappers still seen in active canon:

1. `vida-pack-helper.sh`
2. `vida-pack-router.sh`
3. `nondev-pack-init.sh`

These wrappers remain migration-only helpers per `system-maps/migration.runtime-transition-map`.

## Wrapper-To-Owner Map

Use this note only to find the current canonical owner behind a legacy wrapper behavior.

1. route detection / pack selection
   - legacy helper:
     - `vida-pack-helper.sh detect`
     - `vida-pack-router.sh`
   - canonical owners:
     - `instruction-contracts/core.orchestration-protocol`
     - `command-instructions/routing.use-case-packs-protocol`
2. non-dev pack bootstrap
   - legacy helper:
     - `nondev-pack-init.sh`
     - `vida-pack-helper.sh start`
   - canonical owners:
     - `runtime-instructions/work.taskflow-protocol`
     - `runtime-instructions/runtime.task-state-telemetry-protocol`
3. pack session scaffolding
   - legacy helper:
     - `vida-pack-helper.sh scaffold`
   - canonical owner:
     - `runtime-instructions/work.taskflow-protocol`
4. pack completion / close
   - legacy helper:
     - `vida-pack-helper.sh end`
   - canonical owners:
     - `runtime-instructions/work.pack-completion-gate-protocol`
     - `runtime-instructions/work.pack-handoff-protocol`

## Hard Boundary

1. Do not treat wrapper availability as proof that the wrapper is still a canonical owner.
2. Do not put new pack law into this note.
3. If a wrapper behavior becomes active law again, promote or update the real canonical owner instead of expanding this note.

## Current Migration Direction

1. active pack routing law stays in `command-instructions/routing.use-case-packs-protocol`
2. active orchestration entry/routing law stays in `instruction-contracts/core.orchestration-protocol`
3. active pack handoff law stays in `runtime-instructions/work.pack-handoff-protocol`
4. active pack completion law stays in `runtime-instructions/work.pack-completion-gate-protocol`
5. active migration-status registry stays in `system-maps/migration.runtime-transition-map`

-----
artifact_path: config/command-instructions/pack-wrapper-migration.note
artifact_type: command_instruction
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/command-instructions/migration.pack-wrapper-note.md
created_at: '2026-03-11T00:00:00+02:00'
updated_at: '2026-03-11T13:25:12+02:00'
changelog_ref: migration.pack-wrapper-note.changelog.jsonl
