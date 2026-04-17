# Specification Lane Scope Hardening Design

Status: active bounded design

Purpose: define the architectural hardening that keeps `specification` and other design-first lanes inside lawful design-doc scope instead of inheriting code write scope from free-form request text.

## Summary
- Feature / change: fail-closed packet-scope hardening for specification/design lanes
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: `approved`

## Current Context
- `runtime_delivery_task_packet(...)` currently derives `owned_paths` from `explicit_request_scope_paths(request_text)` for every delivery packet.
- A `specification` downstream dispatch can therefore inherit code file paths from the user request even though product law says the design-first path must first materialize and finalize one bounded design document.
- `taskflow_consume_resume` already limits legacy `owned_paths` backfill to `implementer`, but the live packet-rendering path still lets specification packets carry code ownership before implementation admission.
- Runtime law already exposes the tracked design document through `tracked_flow_bootstrap.design_doc_path`.

## Goal
- Make packet scope depend on task-class policy, not only on request-text parsing.
- Ensure `specification` and design-first lanes can mutate only the tracked design document, never arbitrary code paths.
- Preserve bounded `implementation` packets that legitimately derive explicit code scope from request text.

## Requirements

### Functional Requirements
- `implementation` delivery packets may keep explicit `owned_paths` derived from request text.
- `specification` delivery packets must use doc-only `owned_paths` based on the tracked design document path when it is known.
- `planning`, `coach`, and `verifier` packets must not gain code write scope from request text.
- Persisted legacy packets must normalize toward the same policy instead of reintroducing broader scope.
- Validation must fail closed when a specification packet advertises non-design-doc code ownership.

### Non-Functional Requirements
- Compatibility: legacy implementer packets keep current repair behavior.
- Safety: spec-lane scope widening must fail before packet activation.
- Observability: packet JSON must keep the effective `owned_paths` and `read_only_paths` visible for operator inspection.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
  - `docs/product/spec/specification-lane-scope-hardening-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
- Config / receipts / runtime surfaces affected:
  - persisted runtime dispatch packets
  - downstream dispatch packets
  - continue/resume packet normalization

## Design Decisions

### 1. Scope policy is task-class-aware
Will implement / choose:
- derive delivery-packet write scope through a dedicated policy layer keyed by `handoff_task_class` and tracked flow context
- keep request-text scope extraction as an implementation-only mechanism
- Why: packet admissibility must follow runtime role/task-class law, not arbitrary user wording
- Trade-offs: adds one more policy seam, but removes duplicated conditional logic from render/validation paths
- Alternatives considered: special-case only `dispatch_target == specification`; rejected because the invariant is about task class and future design-first lanes, not one literal target string

### 2. Tracked design-doc path is the sole write authority for specification lanes
Will implement / choose:
- use `tracked_flow_bootstrap.design_doc_path` as the canonical doc-only `owned_paths` source for specification delivery packets
- fail closed when validation sees any non-design-doc owned scope on a specification packet
- Why: the tracked design artifact is already the canonical product object for the design-first path
- Trade-offs: specification packets without tracked design context remain read-only instead of opportunistically writable
- Alternatives considered: infer markdown paths from request text; rejected because that reintroduces heuristic scope ownership

### 3. Legacy packet normalization follows the same scope policy
Will implement / choose:
- keep legacy request-text backfill only for implementer packets
- optionally repair specification packets toward the tracked design-doc path when the active packet is missing owned scope and the tracked path exists
- Why: resume/continue must converge old persisted packets toward current law instead of preserving obsolete widening behavior
- Trade-offs: adds normalization logic that reads tracked flow context, but keeps compatibility bounded and explicit
- Alternatives considered: skip legacy spec repair entirely; rejected because old packets would keep failing with weaker operator guidance

## Technical Design

### Core Components
- `runtime_dispatch_packets.rs`
  - introduce a task-class-aware scope policy for delivery packets
- `runtime_dispatch_state.rs`
  - enforce fail-closed validation for specification/doc-only scope
- `runtime_dispatch_downstream_packets.rs`
  - propagate the same scope policy for downstream packets
- `taskflow_consume_resume.rs`
  - normalize legacy packets according to the same policy

### Data / State Model
- Canonical source of spec write scope: `execution_plan.tracked_flow_bootstrap.design_doc_path`
- Canonical implementation write scope: explicit request-text file paths
- Canonical runtime packet read scope remains:
  - `.vida/data/state/runtime-consumption`
  - `docs/product/spec`
  - `docs/process`

### Integration Points
- packet rendering for direct dispatch packets
- packet rendering for downstream dispatch packets
- persisted packet validation before activation
- consume/resume normalization for older packets

### Bounded File Set
- `docs/product/spec/specification-lane-scope-hardening-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `crates/vida/src/runtime_dispatch_packets.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/runtime_dispatch_downstream_packets.rs`
- `crates/vida/src/taskflow_consume_resume.rs`

## Fail-Closed Constraints
- `specification` packets must not advertise code file ownership outside the tracked design doc.
- request-text parsing must not widen write scope for non-implementation task classes.
- resume normalization must not synthesize code ownership for non-implementer packets.
- work-pool progression remains blocked until specification evidence, design finalization, and spec-pack closure are all satisfied.

## Implementation Plan

### Phase 1
- add the design doc and introduce a reusable delivery-scope policy helper
- First proof target: packet-building tests for implementation vs specification scope

### Phase 2
- enforce spec/doc-only validation and downstream packet parity
- Second proof target: validation tests and downstream rendering tests

### Phase 3
- harden legacy normalization and prove resume-time repair/fail-closed behavior
- Final proof target: targeted `cargo test -p vida ...` packet/normalization regressions plus `vida docflow check`

## Validation / Proof
- Unit tests:
  - delivery-packet scope tests
  - validation rejection tests for widened specification packets
  - legacy normalization tests
- Integration tests:
  - downstream packet rendering parity
- Runtime checks:
  - inspect active dispatch packet JSON for spec lanes
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/specification-lane-scope-hardening-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md`

## Observability
- packet JSON keeps effective `owned_paths`
- validation errors must identify specification-scope violations explicitly
- normalized persisted packets remain readable through existing runtime inspection commands

## Rollout Strategy
- ship as a bounded runtime-family hardening change
- no schema migration required beyond packet normalization on read
- release build and installed binary refresh required after tests pass

## Future Considerations
- extend task-class scope policy to additional doc-shaped or approval-shaped lanes if they later gain controlled write surfaces
- consider surfacing the resolved scope policy explicitly in operator summaries

## References
- `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- `docs/process/documentation-tooling-map.md`
- `docs/product/spec/feature-design-and-adr-model.md`

-----
artifact_path: product/spec/specification-lane-scope-hardening-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-17
schema_version: 1
status: canonical
source_path: docs/product/spec/specification-lane-scope-hardening-design.md
created_at: 2026-04-17T11:01:58.519248194Z
updated_at: 2026-04-17T11:08:56.934773106Z
changelog_ref: specification-lane-scope-hardening-design.changelog.jsonl
