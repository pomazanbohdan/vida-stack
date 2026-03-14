# Direct Runtime Consumption Protocol

Purpose: define the canonical final-layer runtime consumption loop where `taskflow` consumes the compiled runtime bundle directly and activates the bounded `DocFlow` surface for inventory, readiness, and proof evidence before closure trust.

## Core Contract

1. `taskflow` remains the closure authority for direct runtime consumption.
2. Direct runtime consumption must start from a runtime-owned compiled bundle, not ad hoc file inference.
3. The final layer must activate the bounded `DocFlow` branch explicitly.
4. The resulting final-layer payload must be persisted as a runtime-owned snapshot.
5. The compiled runtime bundle may carry compact startup/init projections so direct runtime consumption can reuse them without re-reading the full owner markdown stack.

## Canonical Runtime Surfaces

1. `system-maps/runtime-family.taskflow-map`
2. `vida taskflow consume final <request_text> [--json]`
3. `vida taskflow bundle check --json`
4. `system-maps/runtime-family.taskflow-map`
5. `system-maps/runtime-family.docflow-map`
6. `vida docflow overview --profile active-canon`
7. `vida docflow readiness-check --profile active-canon`
8. `vida docflow proofcheck --profile active-canon`

## Final Consumption Loop

The lawful final loop is:

1. build the runtime-owned compiled bundle,
2. validate bundle readiness,
3. resolve the lane class or conversational mode for the request,
4. activate the bounded `DocFlow` branch,
5. consume overview/readiness/proof evidence,
6. persist the resulting runtime-consumption snapshot,
7. keep closure authority in `taskflow`.

## Bridge Rule

For the current transitional runtime:

1. canonical specs and root config remain the source law,
2. transitional inputs and export surfaces remain allowed where the active runtime still depends on them,
3. the direct runtime layer is considered closed once `taskflow` consumes the compiled bundle and explicit `DocFlow` evidence directly,
4. future DB-first substrate may replace the current persistence mechanism without weakening this consumption contract.

## Persistence Rule

Each final-layer run must write a runtime-owned snapshot under:

1. `.vida/state/runtime-consumption/*.json`

The snapshot must carry:

1. compiled runtime bundle,
2. bundle readiness verdict,
3. lane-selection result,
4. DocFlow evidence,
5. final direct-consumption readiness verdict.

## Boundary Rule

This file owns:

1. the final runtime-consumption loop,
2. the requirement that `taskflow` explicitly activate the bounded `DocFlow` branch before closure trust,
3. the runtime-owned persistence contract for final consumption snapshots.

This file does not own:

1. kernel bundle composition or bundle-check law,
2. `DocFlow` documentation-operation law or documentation inventory/readiness law by itself,
3. generic lane-selection law outside the final consumption loop.

Adjacent canonical owners:

1. `runtime-instructions/runtime.runtime-kernel-bundle-protocol`
   - owns bundle composition and bundle-readiness closure before final consumption begins.
2. `system-maps/runtime-family.docflow-map`
   - owns the bounded `DocFlow` runtime-family branch that must be activated here.
3. `runtime-instructions/work.agent-lane-selection-protocol`
   - owns generic lane-selection law outside this final-loop context.

## References

1. `docs/process/framework-source-lineage-index.md`
2. `docs/product/spec/canonical-runtime-layer-matrix.md`
3. `docs/product/spec/root-map-and-runtime-surface-model.md`

-----
artifact_path: config/runtime-instructions/direct-runtime-consumption.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md
created_at: '2026-03-10T16:00:00+02:00'
updated_at: '2026-03-11T12:57:07+02:00'
changelog_ref: runtime.direct-runtime-consumption-protocol.changelog.jsonl
