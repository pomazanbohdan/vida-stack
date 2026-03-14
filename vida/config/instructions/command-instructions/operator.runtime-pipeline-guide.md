# Runtime Pipeline Operator Guide

Purpose: keep one thin command-layer pointer surface for runtime operator guidance without storing the operator tooling catalog inside the command-layer guide itself.

## Canonical Owners

This file does not own:

1. execution health-check gate law,
2. generic shell/runtime discipline,
3. wrapper migration law,
4. project command boundary law.

These laws and operator maps are owned by:

1. `runtime-instructions/work.execution-health-check-protocol`
2. `runtime-instructions/work.command-execution-discipline-protocol`
3. `system-maps/migration.script-runtime-architecture-map`
4. `system-maps/migration.runtime-transition-map`
5. `runtime-instructions/bridge.project-overlay-protocol`
6. `system-maps/tooling.runtime-operator-tooling-map`

## Operator Tooling Map

Practical operator-facing runtime commands now live in:

```bash
vida/config/instructions/system-maps/tooling.runtime-operator-tooling-map.md
```

That map covers:

1. health-check entrypoints,
2. workflow wrapper commands,
3. boot preflight commands,
4. context compression commands,
5. optional backup/eval helpers,
6. GitHub CLI operator entrypoints,
7. migration-only pack wrapper pointers.

This guide stays only as the command-layer pointer surface to those canonical owners and maps.

-----
artifact_path: config/command-instructions/runtime-pipeline-operator-guide
artifact_type: command_instruction
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/command-instructions/operator.runtime-pipeline-guide.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-11T13:25:29+02:00'
changelog_ref: operator.runtime-pipeline-guide.changelog.jsonl
