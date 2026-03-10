# Direct Runtime Consumption Protocol

Purpose: define the canonical final-layer runtime consumption loop where `taskflow` consumes the compiled runtime bundle directly and activates the bounded `codex` surface for inventory, readiness, and proof evidence before closure trust.

## Core Contract

1. `taskflow` remains the closure authority for direct runtime consumption.
2. Direct runtime consumption must start from a runtime-owned compiled bundle, not ad hoc file inference.
3. The final layer must activate the bounded `codex` branch explicitly.
4. The resulting final-layer payload must be persisted as a runtime-owned snapshot.

## Canonical Runtime Surfaces

1. `taskflow-v0/src/core/direct_consumption.nim`
2. `taskflow-v0 consume final <request_text> [--json]`
3. `taskflow-v0 bundle check --json`
4. `vida/config/instructions/system-maps.runtime-family-taskflow.md`
5. `vida/config/instructions/system-maps.runtime-family-codex.md`
6. `python3 codex-v0/codex.py overview --profile active-canon`
7. `python3 codex-v0/codex.py readiness-check --profile active-canon`
8. `python3 codex-v0/codex.py proofcheck --profile active-canon`

## Final Consumption Loop

The lawful final loop is:

1. build the runtime-owned compiled bundle,
2. validate bundle readiness,
3. resolve the role or conversational mode for the request,
4. activate the bounded `codex` branch,
5. consume overview/readiness/proof evidence,
6. persist the resulting runtime-consumption snapshot,
7. keep closure authority in `taskflow`.

## Bridge Rule

For the current transitional runtime:

1. canonical specs and root config remain the source law,
2. bridge files remain allowed inputs and export surfaces,
3. the direct runtime layer is considered closed once `taskflow` consumes the compiled bundle and explicit `codex` evidence directly,
4. future DB-first substrate may replace the current persistence mechanism without weakening this consumption contract.

## Persistence Rule

Each final-layer run must write a runtime-owned snapshot under:

1. `.vida/state/runtime-consumption/*.json`

The snapshot must carry:

1. compiled runtime bundle,
2. bundle readiness verdict,
3. role-selection result,
4. codex evidence,
5. final direct-consumption readiness verdict.

## References

1. `docs/framework/plans/vida-0.3-db-first-runtime-spec.md`
2. `docs/product/spec/canonical-runtime-layer-matrix.md`
3. `docs/product/spec/root-map-and-runtime-surface-model.md`

-----
artifact_path: config/runtime-instructions/direct-runtime-consumption.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions.direct-runtime-consumption-protocol.md
created_at: '2026-03-10T16:00:00+02:00'
updated_at: '2026-03-10T17:22:42+02:00'
changelog_ref: runtime-instructions.direct-runtime-consumption-protocol.changelog.jsonl
