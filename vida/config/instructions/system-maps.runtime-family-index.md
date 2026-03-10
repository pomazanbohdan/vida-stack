# Runtime Family Index

Purpose: expose the active VIDA runtime families so each bounded runtime surface is discoverable both independently and through the unified framework map.

## Active Runtime Families

1. `codex`
   - map: `vida/config/instructions/system-maps.runtime-family-codex.md`
2. `taskflow`
   - map: `vida/config/instructions/system-maps.runtime-family-taskflow.md`

Future rule:

1. every new runtime family must add one bounded runtime-family map,
2. the framework root map and this index must be updated in the same change,
3. runtime-family discovery must remain explicit rather than inferred from directory names alone.

## Activation Triggers

Read this index when:

1. the task is about runtime-family selection or discoverability,
2. the task asks where `codex`, `taskflow`, or another runtime belongs,
3. a new runtime family is being added,
4. framework map initialization needs runtime-family routing in one pass.

## Routing

1. Documentation/docsys runtime questions:
   - continue to `vida/config/instructions/system-maps.runtime-family-codex.md`
2. Tracked execution / boot / run-graph / route runtime questions:
   - continue to `vida/config/instructions/system-maps.runtime-family-taskflow.md`
3. Runtime health / traces / proving / observability questions:
   - continue to `vida/config/instructions/system-maps.observability-map.md`
4. Final `taskflow` runtime-consumption wiring:
   - continue to `vida/config/instructions/system-maps.runtime-family-taskflow.md`
   - then activate `vida/config/instructions/system-maps.runtime-family-codex.md` for canonical documentation/inventory/readiness consumption evidence

-----
artifact_path: config/system-maps/runtime-family.index
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps.runtime-family-index.md
created_at: '2026-03-10T08:45:00+02:00'
updated_at: '2026-03-10T15:10:43+02:00'
changelog_ref: system-maps.runtime-family-index.changelog.jsonl
