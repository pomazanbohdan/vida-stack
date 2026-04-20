# Compact Dispatch Route Truth Downstream Dispatch Design

Status: `approved`

## Summary
- Feature / change: expose one compact dispatch-diagnosis summary that keeps route-truth and downstream-dispatch-preview semantics aligned across the run-graph helpers and the operator status surfaces.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: `approved`

## Current Context
- `crates/vida/src/taskflow_run_graph.rs` already owns the compact summary model and the builder that reconciles persisted run-graph state, dispatch receipts, recovery state, continuation binding, and activation-versus-execution evidence.
- `crates/vida/src/status_surface_text_report.rs` already formats and emits compact dispatch diagnosis lines when a latest dispatch receipt exists.
- The bounded operator gap is to make the compact summary a stable, explicit status-surface contract for diagnose output, keep route-truth semantics readable in text output, and keep JSON output normalized around the same compact summary rather than drifting into ad hoc mirrors.

## Goal
- Ensure operators can read one stable dispatch-diagnosis summary that answers: where route truth came from, what evidence state is active, and what downstream dispatch state is pending next.
- Keep the implementation bounded to the existing compact summary semantics and the three target Rust files.
- Out of scope: changing delegated-lane routing law, changing packet shaping semantics, or widening into unrelated operator-surface redesign.

## Requirements

### Functional Requirements
- `taskflow_run_graph` must remain the canonical owner of the compact summary structure and fallback rules.
- `status_surface_text_report` must surface compact route-truth and downstream-dispatch-preview fields in operator-readable form without inventing parallel truth.
- `status_surface_json_report` must include the compact summary as a stable nested field for both full and summary views.
- The compact summary must degrade safely when a dispatch receipt is missing by falling back to persisted run-graph status and activation/evidence truth.

### Non-Functional Requirements
- No scope expansion beyond the three bounded files.
- No new write path that bypasses existing fail-closed execution evidence rules.
- Keep tests focused on summary shape, fallback behavior, and parity across text/json surfaces.

## Ownership And Canonical Surfaces
- Project docs / specs affected: `docs/product/spec/compact-dispatch-route-truth-downstream-dispatch-design.md`
- Framework protocols affected: none
- Runtime families affected: `vida taskflow`, `vida status`
- Config / receipts / runtime surfaces affected: run-graph dispatch receipt summaries, continuation binding projection, activation-versus-execution evidence projection

## Design Decisions

### 1. Keep Compact Summary Ownership In `taskflow_run_graph`
Will implement / choose:
- Reuse `RunGraphDispatchCompactSummary` and its helper builders as the only canonical source for route-truth and downstream-dispatch-preview composition.
- This prevents text and JSON surfaces from re-deriving slightly different truth models.
- Trade-off: status surfaces stay dependent on the run-graph helper contract, which is acceptable because the contract is already runtime-owned there.

### 2. Keep Status Surfaces Thin And Projection-Oriented
Will implement / choose:
- Limit status-surface work to rendering or embedding the compact summary rather than duplicating reconciliation logic.
- Preserve readable text lines for operators and a stable nested JSON field for machine consumers.
- Trade-off: some existing raw receipt/status fields remain for compatibility, but the compact summary becomes the preferred diagnose surface.

## Technical Design

### Core Components
- `crates/vida/src/taskflow_run_graph.rs`
  Canonical compact summary structs, fallback logic, and proof tests.
- `crates/vida/src/status_surface_text_report.rs`
  Human-readable diagnose lines for route truth, downstream preview, and next action.
- `crates/vida/src/status_surface_json_report.rs`
  Stable JSON embedding of compact dispatch diagnosis in summary and full views.

### Data / State Model
- Route truth fields:
  `projection_source`, `projection_reason`, `projection_vs_receipt_parity`, `dispatch_receipt_present`, `continuation_binding_present`, `evidence_state`, `activation_kind`, `receipt_backed_execution_evidence`
- Downstream preview fields:
  `dispatch_target`, `dispatch_status`, `lane_status`, `selected_backend`, `activation_agent_type`, `activation_runtime_role`, `downstream_dispatch_target`, `downstream_dispatch_status`, `downstream_dispatch_ready`, `downstream_dispatch_executed_count`, `downstream_dispatch_active_target`, `downstream_dispatch_last_target`
- Compatibility note:
  existing top-level status mirrors stay valid, but the compact summary becomes the bounded diagnose contract to verify.

### Integration Points
- `vida status --json`
  Must expose `latest_run_graph_dispatch_compact_summary` consistently in both summary and full views.
- `vida status`
  Must print route-truth and downstream-dispatch-preview lines derived from the same compact summary.
- Run-graph recovery/status projections:
  Must keep recommended command/surface and blocker-code fallbacks aligned with recovery truth.

### Bounded File Set
- `crates/vida/src/taskflow_run_graph.rs`
- `crates/vida/src/status_surface_text_report.rs`
- `crates/vida/src/status_surface_json_report.rs`

## Fail-Closed Constraints
- Do not widen into lane-routing or dispatch-execution behavior changes.
- Do not add alternate summary builders outside `taskflow_run_graph`.
- If receipt-backed dispatch evidence is absent, status surfaces must continue to show fallback truth rather than inventing executed state.
- Preserve existing operator-contract parity checks in status JSON output.

## Implementation Plan

### Phase 1
- Confirm the compact summary contract and fallback semantics in `taskflow_run_graph`.
- Tighten or extend tests around receipt-present and receipt-absent cases.
- First proof target: compact summary tests cover both reconciled and fallback projection paths.

### Phase 2
- Align `status_surface_text_report` so the diagnose lines always come from the compact summary helper path.
- Ensure next-action rendering stays attached to the compact summary recommendation fields.
- Second proof target: text-surface tests or assertions cover route-truth / downstream-preview rendering.

### Phase 3
- Normalize `status_surface_json_report` around the nested compact summary field in summary and full views.
- Keep operator-contract parity and existing compatibility mirrors intact.
- Final proof target: JSON report tests assert the compact summary presence and shape without mirror drift.

## Validation / Proof
- Unit tests: compact summary fallback and receipt-backed paths in `taskflow_run_graph`
- Integration tests: status text/json report tests for compact summary projection
- Runtime checks: `vida status --json` output shape remains contract-safe
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/compact-dispatch-route-truth-downstream-dispatch-design.md`
  - bounded `cargo test -p vida` filters for compact summary and status report coverage

## Observability
- Operator-visible text lines:
  `latest dispatch route truth`, `latest downstream dispatch preview`, `latest dispatch next action`
- JSON-visible field:
  `latest_run_graph_dispatch_compact_summary`
- Receipts / runtime state written:
  none beyond existing run-graph and dispatch receipt state

## Rollout Strategy
- Keep the change as one bounded runtime/operator-surface packet.
- Verify text and JSON output before handing off to implementation execution.
- No migration or restart path is required.

## Future Considerations
- Later work can decide whether older raw receipt mirrors should be reduced once compact-summary consumers are stable.
- A follow-up packet can expand diagnose coverage for adjacent operator surfaces only if this bounded packet closes cleanly first.

## References
- `docs/product/spec/templates/feature-design-document.template.md`
- `crates/vida/src/taskflow_run_graph.rs`
- `crates/vida/src/status_surface_text_report.rs`
- `crates/vida/src/status_surface_json_report.rs`

-----
artifact_path: product/spec/compact-dispatch-route-truth-downstream-dispatch-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-19
schema_version: 1
status: canonical
source_path: docs/product/spec/compact-dispatch-route-truth-downstream-dispatch-design.md
created_at: 2026-04-19T19:57:22.225064158Z
updated_at: 2026-04-19T20:00:48.469873486Z
changelog_ref: compact-dispatch-route-truth-downstream-dispatch-design.changelog.jsonl
