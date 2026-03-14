# Observability Map

Purpose: expose the canonical observability, trace, proving, and runtime-health surfaces of VIDA so diagnostics and health discovery do not depend on ad hoc filesystem guessing.

## Canonical Surfaces

1. run-graph resumability ledger
   - `runtime-instructions/core.run-graph-protocol`
2. context-governance ledger
   - `runtime-instructions/core.context-governance-protocol`
3. local trace grading and trace datasets
   - `runtime-instructions/observability.trace-grading-protocol`
4. proving-pack scaffolds
   - `diagnostic-instructions/evaluation.product-proving-pack-scaffold-contract`
5. task-state reconciliation
   - `runtime-instructions/work.task-state-reconciliation-protocol`
6. protocol/runtime drift diagnostics
   - `diagnostic-instructions/analysis.protocol-self-diagnosis-protocol`
   - `diagnostic-instructions/analysis.silent-framework-diagnosis-protocol`

## Activation Triggers

Read this map when:

1. runtime health or observability questions are active,
2. a task needs traces, proving evidence, or resumability inspection,
3. a restart/recovery/debug path needs the canonical health/diagnostic surfaces,
4. `VIDA 1.0` runtime-family design needs explicit observability ownership.

## Routing

1. resumability and node-level runtime state:
   - continue to `runtime-instructions/core.run-graph-protocol`
2. governed context and provenance:
   - continue to `runtime-instructions/core.context-governance-protocol`
3. trace grading and regression datasets:
   - continue to `runtime-instructions/observability.trace-grading-protocol`
4. proving packs and bounded proving scaffolds:
   - continue to `diagnostic-instructions/evaluation.product-proving-pack-scaffold-contract`
5. runtime/task drift diagnosis:
   - continue to `runtime-instructions/work.task-state-reconciliation-protocol`
   - or the relevant diagnostic protocol

## Boundary Rule

1. This map is for observability and runtime-health discovery.
2. It does not replace runtime-family maps.
3. Runtime families should point here when tasks need traces, diagnostics, proving, or health surfaces.

## External Alignment Note

This map aligns with the `VIDA 1.0` runtime observability target captured in:

1. `docs/process/framework-source-lineage-index.md`

-----
artifact_path: config/system-maps/observability.map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/observability.map.md
created_at: '2026-03-10T15:05:00+02:00'
updated_at: '2026-03-11T13:40:46+02:00'
changelog_ref: observability.map.changelog.jsonl
