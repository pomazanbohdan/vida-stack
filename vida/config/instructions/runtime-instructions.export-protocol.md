# Export Protocol

Purpose: define canonical export surfaces from the DB-owned VIDA runtime to filesystem artifacts such as JSON, documentation bundles, memory dumps, diagnostics, and logs.

## Core Contract

1. Runtime truth is DB-owned.
2. Filesystem exports are projections, not SSOT runtime stores.
3. Every export must declare source slice, export kind, schema version, and generation time.
4. Exports must be reproducible from canonical DB state.

Operator-facing rendering rule:

1. terminal-oriented user-visible exports and status surfaces may support colorized output and emoji markers,
2. this rendering layer is presentation-only and must not change canonical exported data semantics,
3. plain-text and machine-readable export modes must remain available as deterministic fallbacks,
4. color/emoji rendering should degrade cleanly when terminal capability, locale, accessibility, or user preference disables them.

## Minimum Export Families

1. `json_data_export`
2. `documentation_export`
3. `project_memory_export`
4. `framework_memory_export`
5. `instruction_bundle_export`
6. `diagnostic_export`
7. `log_export`
8. `parity_proof_export`

## Required Metadata

1. `export_kind`
2. `source_slice`
3. `schema_version`
4. `generated_at`
5. `product_version`
6. `db_runtime_version`
7. `selection_scope`

Optional rendering metadata for user-facing exports:

1. `render_mode` (`plain`, `color`, `color_emoji`),
2. `terminal_capability_detected`,
3. `emoji_enabled`

## Fail-Closed Rule

1. Do not treat exported files as the canonical runtime state after generation.
2. Do not silently re-import exports as live truth without an explicit import/migration contract.
3. Do not emit partial exports as if they were complete.

-----
artifact_path: config/runtime-instructions/export.protocol
artifact_type: runtime_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/runtime-instructions.export-protocol.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: runtime-instructions.export-protocol.changelog.jsonl
