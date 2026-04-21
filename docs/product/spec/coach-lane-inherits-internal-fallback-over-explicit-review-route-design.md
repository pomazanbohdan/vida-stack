# Coach lane inherited-internal-fallback blocker design

Purpose: Bound the current-release fix for coach-lane backend canonicalization so review-safe lanes do not inherit an internal implementer fallback over an explicit external review route.

Status: `proposed`

## Summary
- Feature / change: when an implementation run reaches `coach`, backend selection must prefer the lane's explicit review-safe route over an inherited internal backend from the previous implementer step.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher`
- Status: `proposed`

## Current Context
- Existing system overview
  - `build_downstream_dispatch_receipt(...)` derives the next lane receipt and seeds `selected_backend` through `downstream_selected_backend(...)`.
  - `downstream_selected_backend(...)` delegates to `admissible_selected_backend_for_dispatch_target(...)`.
  - For non-strict lanes such as `coach`, `admissible_selected_backend_for_dispatch_target(...)` currently returns the first candidate without enforcing stronger route preference.
- Key components and relationships
  - `crates/vida/src/runtime_dispatch_state.rs` owns route-to-backend canonicalization, downstream receipt construction, and run-graph/backend status lineage.
  - `crates/vida/src/runtime_dispatch_execution.rs` consumes the canonical selected backend and decides whether dispatch goes through external or internal execution surfaces.
  - `taskflow recovery latest` and `run-graph status` project the resulting `selected_backend`, `dispatch_target`, and `blocker_code`.
- Current pain point or gap
  - Live evidence for `feature-serialize-authoritative-state-access-lock-mitigation` shows the coach route declares `route_primary_backend = hermes_cli` and `route_fanout_backends = [hermes_cli, opencode_cli]`, but the persisted packet/result still select `internal_subagents`.
  - That inherited internal backend causes `vida agent-init` to return `internal_activation_view_only` instead of using an admissible external review-safe route.
  - A current unit test, `admissible_selected_backend_preserves_inherited_declared_fallback_for_coach_lane`, codifies the wrong behavior and matches the live blocker.

## Goal
- What this change should achieve
  - Make explicit coach/review route hints outrank inherited internal fallback lineage when the lane is review-safe and route-specific.
  - Preserve implementation fail-closed fallback behavior for write lanes.
  - Allow the serialization slice to progress from `coach` without regressing the repaired routing/timeout work.
- What success looks like
  - A coach downstream receipt from an internal implementer step resolves `selected_backend = hermes_cli` when coach route primary is `hermes_cli`.
  - Live serialization runtime no longer surfaces `selected_backend = internal_subagents` for the coach packet when external review-safe routes are admissible.
  - The old wrong test is replaced with route-primary-preference coverage for coach lanes, while implementation fallback tests stay green.
- What is explicitly out of scope
  - Implementing the serialization lock-mitigation code itself.
  - Redesigning carrier topology or removing all mixed-route fallback semantics.
  - Changing strict implementation admissibility behavior.

## Requirements

### Functional Requirements
- Coach-lane downstream backend selection must prefer explicit route primary/fanout review backends over inherited internal fallback when the route is not backend-agnostic.
- Implementation-lane backend selection must keep current fail-closed internal fallback behavior when the external primary is inadmissible.
- Run-graph status and downstream dispatch receipts must preserve truthful lane-specific backend lineage after the fix.
- External review-safe lanes must continue to use `vida agent-init`/external carrier dispatch when their resolved backend is external.

### Non-Functional Requirements
- Performance
  - Backend canonicalization must remain constant-time over the bounded route candidate set.
- Scalability
  - The rule should generalize across review-safe downstream lanes, not only one serialization run id.
- Observability
  - Route-primary vs effective-selected backend projections must stay explicit in runtime artifacts.
- Safety
  - No root-session write bypass or local exception-path inference may result from this routing fix.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/implementation-backend-admissibility-and-selection-truth-design.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
  - `launcher`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow recovery latest`
  - `vida taskflow run-graph status`
  - runtime dispatch packets/results
  - downstream run-graph dispatch receipts

## Design Decisions

### 1. Non-implementation route hints must outrank inherited fallback lineage
Will implement / choose:
- For route-specific non-strict lanes such as `coach`, candidate ordering will prefer explicit route primary/fallback/fanout before any inherited selected backend.
- Why
  - Review-safe downstream lanes have their own route contract and should not silently stay on an implementation fallback that only became necessary upstream.
- Trade-offs
  - Some historical mixed-lane flows that previously stayed on internal fallback will now move to the explicit review backend when one exists.
- Alternatives considered
  - Keep inherited backend first for all non-strict lanes.
  - Rejected because it exactly reproduces the live `internal_activation_view_only` blocker.

### 2. Strict implementation admissibility remains unchanged
Will implement / choose:
- Preserve the existing strict implementation fallback logic and only narrow the route-preference change to non-strict, route-specific review-safe lanes.
- Why
  - The implementation lane still needs fail-closed fallback to internal execution when external write backends are inadmissible.
- Trade-offs
  - Backend-selection logic becomes slightly more lane-aware.
- Alternatives considered
  - Global reordering for every lane.
  - Rejected because it risks regressing the already-proved implementer fallback behavior.

## Technical Design

### Core Components
- Main components
  - `runtime_dispatch_state.rs`
  - targeted tests in `runtime_dispatch_state.rs`
- Key interfaces
  - `admissible_backend_candidates_for_dispatch_target(...)`
  - `admissible_selected_backend_for_dispatch_target(...)`
  - `downstream_selected_backend(...)`
  - `build_downstream_dispatch_receipt(...)`
- Bounded responsibilities
  - route-aware candidate ordering for coach/review lanes
  - preservation of implementation fallback semantics
  - truthful downstream receipt and run-graph backend lineage

### Data / State Model
- Important entities
  - route primary backend
  - fallback backend
  - fanout backends
  - inherited selected backend
  - downstream dispatch receipt
- Receipts / runtime state / config fields
  - `selected_backend`
  - `route_primary_backend`
  - `route_fanout_backends`
  - `dispatch_target`
  - `blocker_code`
- Migration or compatibility notes
  - additive runtime behavior change only; no schema migration

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - implementer completion -> downstream coach receipt construction -> dispatch execution backend selection
- Cross-document / cross-protocol dependencies
  - `docs/product/spec/implementation-backend-admissibility-and-selection-truth-design.md`
  - `docs/product/spec/internal-dispatch-timeout-does-not-return-design.md`
  - `docs/product/spec/serialize-authoritative-state-access-lock-mitigation-design.md`

### Bounded File Set
- `docs/product/spec/coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md`
- `docs/product/spec/current-spec-map.md`
- `crates/vida/src/runtime_dispatch_state.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - no heuristic reuse of upstream internal fallback when the downstream lane has an explicit review route
  - no relaxation of strict implementation admissibility
  - no local root-session implementation fallback
- Required receipts / proofs / gates
  - downstream coach receipt must show the explicit review backend when one is admissible
  - runtime recovery/status must stop surfacing `internal_activation_view_only` for this coach-route case
- Safety boundaries that must remain true during rollout
  - implementer fallback to internal remains lawful when external write routes are inadmissible
  - mixed-route lineage stays truthful in run-graph status after each executed handoff

## Implementation Plan

### Phase 1
- Record the bounded defect in canonical docs and update the active spec map.
- First proof target
  - `vida docflow check --root . docs/product/spec/coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md docs/product/spec/current-spec-map.md`

### Phase 2
- Reorder backend candidate preference for route-specific coach/review lanes and adjust/extend tests.
- Second proof target
  - targeted `cargo test -p vida` for coach-lane backend selection and mixed-lane lineage

### Phase 3
- Re-run the serialization runtime surfaces and confirm coach no longer resolves to the internal fallback when external review routes are admissible.
- Final proof target
  - targeted tests plus live `cargo run -p vida -- taskflow recovery latest --json` / `run-graph status` evidence

## Validation / Proof
- Unit tests:
  - coach-lane candidate resolution prefers explicit review route over inherited internal fallback
  - implementation fallback still prefers internal when external primary is inadmissible
  - downstream receipt lineage remains lane-specific across mixed backend chains
- Integration tests:
  - none beyond targeted runtime-dispatch state tests expected
- Runtime checks:
  - `cargo run -p vida -- taskflow recovery latest --json`
  - `cargo run -p vida -- taskflow run-graph status feature-serialize-authoritative-state-access-lock-mitigation --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md docs/product/spec/current-spec-map.md`

## Observability
- Logging points
  - none required beyond existing route/effective backend projections
- Metrics / counters
  - none required
- Receipts / runtime state written
  - downstream dispatch receipts
  - runtime dispatch packets/results
  - run-graph status projections

## Rollout Strategy
- Development rollout
  - land as one bounded blocker fix before resuming the serialization implementation slice
- Migration / compatibility notes
  - no schema migration
- Operator or user restart / restart-notice requirements
  - rebuild and refresh the local `vida` binary after proofs pass

## Future Considerations
- Follow-up ideas
  - tighten lane-specific backend policy for `verification` and other read-only review surfaces if similar inherited-fallback drift appears there
- Known limitations
  - this slice only changes route preference where explicit downstream review routes already exist
- Technical debt left intentionally
  - broader mixed-route backend policy simplification can remain a later hardening slice

-----
artifact_path: product/spec/coach-lane-inherits-internal-fallback-over-explicit-review-route-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md
created_at: 2026-04-21T12:33:17.093861071Z
updated_at: 2026-04-21T12:36:32.610673237Z
changelog_ref: coach-lane-inherits-internal-fallback-over-explicit-review-route-design.changelog.jsonl
