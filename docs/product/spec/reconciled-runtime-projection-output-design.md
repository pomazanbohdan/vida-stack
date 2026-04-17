# Reconciled Runtime Projection Output Design

Status: `approved`

## Summary
- Feature / change: add additive operator/runtime output surfaces that expose reconciled projection truth after runtime handoff reconciliation, especially when runtime truth has advanced past a live delegated lane into post-design or post-spec-close state.
- Owner layer: `runtime-family`
- Runtime surfaces: `vida orchestrator-init`, `vida taskflow run-graph status`, `vida taskflow recovery latest|status`, `vida taskflow consume continue`
- Status: `approved for bounded implementation`

## Current Context
- Existing system overview
  - The runtime already reconciles run-graph status against persisted dispatch receipts and closed-task state.
  - Continuation-binding summaries already fail closed on ambiguity and can name the active bounded unit.
  - Recovery and run-graph inspection surfaces already expose canonical status, gate, checkpoint, and dispatch receipt summaries.
- Key components and relationships
  - `crates/vida/src/state_store_run_graph_summary.rs` is the current owner of reconciled run-graph and dispatch-receipt summary logic.
  - `crates/vida/src/continuation_binding_summary.rs` derives the current lawful bounded unit from status, recovery, and receipt evidence.
  - `crates/vida/src/taskflow_runtime_bundle.rs`, `crates/vida/src/status_surface*.rs`, `crates/vida/src/taskflow_run_graph.rs`, and `crates/vida/src/taskflow_consume_resume.rs` render operator-visible JSON/text output.
- Current pain point or gap
  - Operator-facing output still makes the reconciliation result implicit.
  - When a run has already crossed the design/spec gate and the truthful bounded unit is now downstream closure or the next packet, the output does not explicitly say which source won, why it won, whether persisted receipt lineage still looks stale, or what the next lawful operator action is.
  - That leaves post-reconciliation state harder to interpret than live delegated-lane state even though the runtime already has enough evidence to explain it.

## Goal
- Make reconciled projection truth explicit across the bounded operator surfaces in scope.
- Show whether the current truth came from live run-graph state, explicit/bound continuation, or downstream receipt reconciliation.
- Expose whether the projection is aligned with persisted receipt lineage or is healing/suspecting stale state.
- Keep the change additive to existing JSON/text output and preserve fail-closed ambiguity semantics.
- Out of scope:
  - redesigning continuation-binding law
  - changing carrier selection, lane ownership, or write-authority rules
  - widening into unrelated status/help surface cleanup

## Requirements

### Functional Requirements
- Must add an additive reconciled projection block that includes:
  - effective projection source
  - effective projection reason
  - reconciled downstream target/status/blockers
  - projection-vs-receipt parity
  - stale-state suspicion plus reason when present
  - next lawful operator action
- Must surface that block through:
  - `vida orchestrator-init`
  - `vida taskflow run-graph status`
  - `vida taskflow recovery latest`
  - `vida taskflow recovery status`
  - `vida taskflow consume continue`
- Must make the post-design/post-spec-close case explicit rather than leaving it to operator inference from raw receipt fields.
- Must preserve existing JSON keys and text output while only adding new lines/fields.
- Must keep ambiguity fail-closed: ambiguous evidence remains ambiguous rather than being repackaged as a confident projection.

### Non-Functional Requirements
- Performance
  - summary generation must stay local to already loaded run/binding/receipt state
- Observability
  - text output must surface the effective source and next lawful action
  - JSON output must be machine-readable and stable enough for future contract tests
- Compatibility
  - additive-only schema growth
  - no migration of stored state rows

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/reconciled-runtime-projection-output-design.md`
  - `docs/product/spec/current-spec-map.md`
- Runtime families affected:
  - `taskflow`
  - launcher-owned operator reporting
- Config / receipts / runtime surfaces affected:
  - reconciled run-graph status projection
  - run-graph dispatch receipt summary
  - continuation-binding summary consumers
  - init/status/recovery/continue output renderers

## Design Decisions

### 1. One shared reconciled-projection helper will feed every surface
Will implement / choose:
- Build a single helper that derives reconciled projection truth from current continuation-binding, run-graph status, recovery summary, and dispatch-receipt summary.
- Why:
  - The requested fields are explanatory projections over existing canonical evidence, not independent state.
  - A shared helper avoids drift between init, status, recovery, and continue surfaces.
- Trade-offs:
  - Adds one more reporting abstraction layer that must stay aligned with reconciliation law.
- Alternatives considered:
  - Duplicating small per-surface explanations.

### 2. Post-reconciliation truth will be described explicitly, not inferred from raw receipt fields
Will implement / choose:
- Add source/reason/parity/staleness language even when the active bounded unit is already bound or completed.
- Why:
  - The difficult operator case is not “lane still active”; it is “runtime truth already moved on”.
- Trade-offs:
  - Slightly larger JSON and a few additional text lines.
- Alternatives considered:
  - Relying on continuation-binding alone.
  - Relying on raw downstream receipt fields without a synthesized explanation.

## Technical Design

### Core Components
- `crates/vida/src/continuation_binding_summary.rs`
  - host the shared reconciled runtime projection helper beside continuation-binding derivation
- `crates/vida/src/state_store_run_graph_summary.rs`
  - expose per-run dispatch receipt summaries so recovery/run-graph/continue can request the same derived truth for a specific run
- `crates/vida/src/taskflow_runtime_bundle.rs`
  - add reconciled projection output to `orchestrator_init_view`
- `crates/vida/src/status_surface*.rs`
  - add reconciled projection output to JSON and text status reports
- `crates/vida/src/taskflow_run_graph.rs`
  - add reconciled projection output to recovery/run-graph inspection surfaces
- `crates/vida/src/taskflow_consume_resume.rs`
  - add reconciled projection output to `consume continue`

### Data / State Model
- No stored-schema migration is required.
- New output projection fields are computed from existing:
  - `RunGraphStatus`
  - `RunGraphRecoverySummary`
  - `RunGraphDispatchReceiptSummary`
  - `RunGraphContinuationBinding`
- The helper should classify:
  - live delegated-lane truth
  - bound downstream-target truth
  - post-design/post-spec-close reconciled truth
  - ambiguous/diagnosis-required truth

### Integration Points
- `vida orchestrator-init --json`
- `vida status --json`
- `vida taskflow run-graph status <run-id> [--json]`
- `vida taskflow recovery latest [--json]`
- `vida taskflow recovery status <run-id> [--json]`
- `vida taskflow consume continue [--run-id <run-id>] [--json]`

### Bounded File Set
- `docs/product/spec/reconciled-runtime-projection-output-design.md`
- `docs/product/spec/current-spec-map.md`
- `crates/vida/src/continuation_binding_summary.rs`
- `crates/vida/src/state_store_run_graph_summary.rs`
- `crates/vida/src/taskflow_runtime_bundle.rs`
- `crates/vida/src/status_surface.rs`
- `crates/vida/src/status_surface_json_report.rs`
- `crates/vida/src/status_surface_text_report.rs`
- `crates/vida/src/taskflow_run_graph.rs`
- `crates/vida/src/taskflow_consume_resume.rs`

## Fail-Closed Constraints
- Do not convert ambiguous evidence into a confident projection.
- Do not let the explanatory projection override canonical blocker semantics.
- Do not mutate stored receipts solely to make the new reporting block look cleaner.
- If receipt lineage and projection truth diverge, report the divergence as parity/staleness rather than hiding it.

## Implementation Plan

### Phase 1
- Register and validate this bounded design.
- Proof target:
  - `vida docflow check --root . docs/product/spec/reconciled-runtime-projection-output-design.md`

### Phase 2
- Implement the shared reconciled projection helper and per-run dispatch receipt summary path.
- Proof target:
  - targeted `cargo test -p vida ...` for helper classification and summary serialization

### Phase 3
- Thread the helper into the bounded surfaces and add text/JSON contract coverage.
- Final proof target:
  - targeted runtime/output tests plus bounded docflow check

## Validation / Proof
- Unit tests:
  - reconciled projection helper classification for live, bound-downstream, and ambiguous cases
  - per-run dispatch receipt summary parity with latest summary behavior
- Integration tests:
  - bounded `boot_smoke` / surface-contract tests for init, recovery, and continue JSON
- Runtime checks:
  - `vida orchestrator-init --json`
  - `vida taskflow recovery latest --json`
  - `vida taskflow recovery status <run-id> --json`
  - `vida taskflow consume continue --run-id <run-id> --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/reconciled-runtime-projection-output-design.md docs/product/spec/current-spec-map.md`

## Observability
- New output block clearly states which evidence source currently governs operator interpretation.
- Parity and stale-state suspicion become explicit instead of implicit receipt reading work.
- Existing blocker/next-action outputs remain authoritative and are complemented, not replaced.

## Rollout Strategy
- Land the design registration and runtime reporting changes together in one bounded slice.
- Keep JSON growth additive and plain-text additions compact.
- No migration or operator restart work beyond the normal rebuilt binary cycle.

## Future Considerations
- Reuse the same helper in `vida status --summary` or doctor-style reporting if future slices need parity there too.
- Extend parity classification if future run-graph receipt history becomes append-only instead of latest-row upsert.

## References
- `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`
- `docs/product/spec/fix-continuation-reconciliation-resumed-executed-design.md`
- `docs/product/spec/lawful-closure-continuation-rebinding-design.md`

-----
artifact_path: product/spec/reconciled-runtime-projection-output-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-17
schema_version: 1
status: canonical
source_path: docs/product/spec/reconciled-runtime-projection-output-design.md
created_at: 2026-04-17T15:30:00+03:00
updated_at: 2026-04-17T15:30:00+03:00
changelog_ref: reconciled-runtime-projection-output-design.changelog.jsonl
