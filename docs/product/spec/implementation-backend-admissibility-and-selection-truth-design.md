# Implementation Backend Admissibility And Selection Truth Design

Status: approved

## Summary
- Feature / change: unify backend admissibility-aware selection truth for implementation lanes across dispatch execution, packet rendering, receipt summaries, and operator/status surfaces
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher`
- Status: `approved`

## Current Context
- `runtime_lane_summary.rs` already builds a `backend_admissibility_matrix`.
- `runtime_dispatch_execution.rs` already blocks inadmissible external backends before launch.
- `runtime_dispatch_state.rs` already has `admissible_selected_backend_for_dispatch_target(...)` and `canonical_selected_backend_for_receipt(...)`.
- Remaining drift:
  - packet/render/operator summaries can still project raw `receipt.selected_backend`,
  - route posture can report an inadmissible primary backend as the effective selected backend,
  - implementation-lane fallback choice is therefore not consistently visible across runtime surfaces.

## Goal
- Ensure implementation-lane runtime surfaces always project the canonical admissible backend, not stale raw receipt drift.
- Keep route-primary/fallback/fanout visibility for operator diagnosis, but separate that from the effective selected backend.
- Reuse one canonical admissibility-aware selection path everywhere the runtime renders selected-backend truth.

## Requirements

### Functional Requirements
- Implementation-lane packet rendering must not emit an inadmissible read-only backend as `selected_backend` when an admissible fallback exists.
- Route/effective posture summaries must expose:
  - the canonical effective selected backend,
  - where that canonical backend came from,
  - the original route primary/fallback/fanout hints for diagnosis.
- Dispatch receipt summary projection must expose canonical backend truth when execution-plan context is available.
- Existing external-dispatch inadmissibility gating must remain fail-closed.

### Non-Functional Requirements
- Keep changes bounded to backend-selection truth projection and directly affected tests/docs.
- Preserve operator observability of route-primary drift rather than hiding it.
- Avoid introducing a second backend-selection algorithm.

## Design Decisions

### 1. Effective selected backend is admissibility-aware, route hints remain diagnostic
Will implement / choose:
- Use the admissibility-aware resolver for `selected_backend`.
- Keep `route_primary_backend`, `route_fallback_backend`, and `route_fanout_backends` unchanged for diagnosis.
- Why:
  - Operators need to see both the canonical selected backend and the route hint that was rejected.

### 2. Receipt projection is canonicalized when execution-plan context exists
Will implement / choose:
- When role-selection/execution-plan context is present, summary/packet surfaces rewrite `selected_backend` to the canonical admissible backend.
- Why:
  - The receipt may preserve historical raw selection drift; user-facing runtime truth should not.

## Bounded File Set
- `docs/product/spec/implementation-backend-admissibility-and-selection-truth-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `docs/product/spec/README.md`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/state_store_run_graph_summary.rs`

## Validation / Proof
- Unit / contract:
  - effective posture canonicalizes inadmissible implementer primary backend to admissible fallback
  - dispatch packet rendering emits canonical selected backend while preserving route-primary diagnosis
- Runtime checks:
  - targeted `cargo test -p vida runtime_dispatch_state -- --nocapture`

## References
- `docs/product/spec/taskflow-execution-semantics-and-scheduler-design.md`
- `docs/product/spec/release-1-operator-surface-contract.md`
- `docs/product/spec/release-1-runtime-enum-and-code-contracts.md`

-----
artifact_path: product/spec/implementation-backend-admissibility-and-selection-truth-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-04-17
schema_version: '1'
status: canonical
source_path: docs/product/spec/implementation-backend-admissibility-and-selection-truth-design.md
created_at: '2026-04-17T18:35:00+03:00'
updated_at: 2026-04-17T18:35:00+03:00
changelog_ref: implementation-backend-admissibility-and-selection-truth-design.changelog.jsonl
