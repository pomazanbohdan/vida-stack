# Pack Wrapper Migration Note

Purpose: keep one migration-only helper note for legacy pack wrappers so active pack/routing protocols do not need to repeat wrapper catalog text.

Status note:

1. this file is `non-canonical reference material`,
2. it does not own pack law, handoff law, completion law, or orchestration law,
3. it exists only to centralize migration-only wrapper entrypoints while `taskflow-v0` remains the target runtime home.

## Legacy Pack Wrapper Catalog

Legacy wrappers still seen in active canon:

1. `vida-pack-helper.sh`
2. `vida-pack-router.sh`
3. `nondev-pack-init.sh`

These wrappers remain migration-only helpers per `vida/config/instructions/system-maps/migration.runtime-transition-map.md`.

## Wrapper-To-Owner Map

Use this note only to find the current canonical owner behind a legacy wrapper behavior.

1. route detection / pack selection
   - legacy helper:
     - `vida-pack-helper.sh detect`
     - `vida-pack-router.sh`
   - canonical owners:
     - `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
     - `vida/config/instructions/command-instructions/routing.use-case-packs-protocol.md`
2. non-dev pack bootstrap
   - legacy helper:
     - `nondev-pack-init.sh`
     - `vida-pack-helper.sh start`
   - canonical owners:
     - `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
     - `vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md`
3. pack session scaffolding
   - legacy helper:
     - `vida-pack-helper.sh scaffold`
   - canonical owner:
     - `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
4. pack completion / close
   - legacy helper:
     - `vida-pack-helper.sh end`
   - canonical owners:
     - `vida/config/instructions/runtime-instructions/work.pack-completion-gate-protocol.md`
     - `vida/config/instructions/runtime-instructions/work.pack-handoff-protocol.md`

## Hard Boundary

1. Do not treat wrapper availability as proof that the wrapper is still a canonical owner.
2. Do not put new pack law into this note.
3. If a wrapper behavior becomes active law again, promote or update the real canonical owner instead of expanding this note.

## Current Migration Direction

1. active pack routing law stays in `vida/config/instructions/command-instructions/routing.use-case-packs-protocol.md`
2. active orchestration entry/routing law stays in `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
3. active pack handoff law stays in `vida/config/instructions/runtime-instructions/work.pack-handoff-protocol.md`
4. active pack completion law stays in `vida/config/instructions/runtime-instructions/work.pack-completion-gate-protocol.md`
5. active migration-status registry stays in `vida/config/instructions/system-maps/migration.runtime-transition-map.md`

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
