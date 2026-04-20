# Fix Bug Implementer Delivery Packet Can Design

Status: `approved`

## Summary
- Feature / change: prevent implementer delivery packets from launching without an explicit owned write scope.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: `approved`

## Current Context
- `crates/vida/src/taskflow_consume_resume.rs` derives delivery-task packet content during runtime-consumption shaping and resume flow.
- The active bug is that an implementer delivery packet can still be shaped without a trustworthy owned-path scope, which weakens bounded write ownership and allows a packet to exist without explicit file authority.
- The epic already narrows the scope to `crates/vida/src/taskflow_consume_resume.rs` and a targeted proof that explicit owned paths are collected from request text.

## Goal
- Ensure implementer delivery packets only launch when an explicit owned write scope is present and derivable.
- Make request-text-derived owned paths the bounded authority for this packet family in the current fix.
- Out of scope: broader packet-template redesign, general routing changes, or widening beyond the one runtime file and focused proof.

## Requirements

### Functional Requirements
- Implementer delivery packet shaping must collect explicit owned paths from the request text for the bounded feature request.
- If no explicit owned paths can be derived, the packet must fail closed instead of launching with implicit or empty write scope.
- Existing lawful packet shaping must continue to work when explicit owned paths are present.
- Regression proof must cover the packet-collection path that previously allowed launch without owned write scope.

### Non-Functional Requirements
- Keep the implementation bounded to `crates/vida/src/taskflow_consume_resume.rs` and targeted tests.
- Do not add alternate fallback rules that infer wide write ownership from unrelated runtime state.
- Preserve current operator/runtime output contracts for any touched JSON payloads.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  `docs/product/spec/fix-bug-implementer-delivery-packet-can-design.md`
- Framework protocols affected:
  bounded write-ownership rules already enforced by project/runtime law
- Runtime families affected:
  `vida taskflow consume`
  implementer delivery-packet shaping
- Config / receipts / runtime surfaces affected:
  delivery-task packet shaping
  owned path derivation from request text

## Design Decisions

### 1. Explicit Request-Text Owned Paths Are Mandatory
Will implement / choose:
- Treat explicit owned paths named in the bounded request text as the authority for implementer delivery packet write scope in this fix.
- This keeps packet ownership narrow, inspectable, and aligned with the feature epic.
- Trade-off: packets with underspecified request text will now fail closed instead of proceeding with guessed ownership.

### 2. Keep The Fix In Packet-Shaping Logic
Will implement / choose:
- Repair the owned-scope derivation where the delivery packet is shaped, rather than adding late-stage guards in unrelated execution surfaces.
- This keeps the bugfix local and ensures bad packets are never produced in the first place.
- Trade-off: any adjacent scope-validation gaps outside this shaping path remain separate future work.

## Technical Design

### Core Components
- `crates/vida/src/taskflow_consume_resume.rs`
  delivery-packet owned-path derivation and fail-closed shaping guard
- Focused packet-shaping tests
  proof that explicit owned paths are collected from request text and required for launch

### Data / State Model
- Packet fields that matter:
  request text
  derived owned paths
  delivery-task packet write scope
- Compatibility note:
  valid packets with explicit owned paths stay valid; only missing-scope cases become fail-closed.

### Integration Points
- Runtime-consumption packet shaping
  must emit explicit owned write scope for implementer delivery packets
- Focused proof target:
  `cargo test -p vida runtime_delivery_task_packet_collects_explicit_owned_paths_from_request_text -- --nocapture`

### Bounded File Set
- `crates/vida/src/taskflow_consume_resume.rs`
- focused packet-shaping tests in existing consume/resume coverage

## Fail-Closed Constraints
- Do not allow implementer delivery packets to launch with missing or implicit owned write scope.
- Do not widen into multi-file ownership inference or unrelated routing changes.
- If explicit owned paths cannot be derived, fail closed at packet shaping time.
- Keep bounded write ownership narrower than or equal to the feature request scope.

## Implementation Plan

### Phase 1
- Inspect current delivery-packet shaping in `taskflow_consume_resume.rs`.
- Identify where owned write scope is lost or omitted during implementer packet creation.
- First proof target: reproduce the missing-owned-scope case in focused test coverage.

### Phase 2
- Add explicit request-text path collection and fail-closed guard for missing owned paths.
- Keep valid bounded packets working when owned paths are present.
- Second proof target: `runtime_delivery_task_packet_collects_explicit_owned_paths_from_request_text`.

### Phase 3
- Re-run focused packet-shaping and adjacent consume/resume tests.
- Verify no touched JSON/runtime output contracts regress.
- Final proof target: bounded green tests plus packet-shaping behavior stays explicit and fail-closed.

## Validation / Proof
- Unit tests:
  focused delivery-packet owned-path derivation coverage
- Integration tests:
  bounded `cargo test -p vida` filters around delivery packet shaping
- Runtime checks:
  inspect shaped packet content when needed through taskflow packet surfaces
- Canonical checks:
  `vida docflow check --root . docs/product/spec/fix-bug-implementer-delivery-packet-can-design.md`

## Observability
- Operator-visible outcome:
  implementer delivery packets should no longer exist without explicit owned paths
- Runtime evidence:
  shaped packet content should show explicit owned write scope derived from request text
- Receipts / runtime state written:
  no new receipt family; reuse existing packet shaping and runtime-consumption artifacts

## Rollout Strategy
- Keep the fix in one bounded runtime-family packet.
- Validate through focused tests before handing off into the dev packet.
- No migration or restart path is required.

## Future Considerations
- Later work can standardize explicit owned-path derivation across more packet families.
- If request-text parsing becomes too brittle, a follow-up may introduce a more structured bounded file-scope field.

## References
- `crates/vida/src/taskflow_consume_resume.rs`
- `docs/product/spec/fix-bug-consume-continue-completed-closure-design.md`

-----
artifact_path: product/spec/fix-bug-implementer-delivery-packet-can-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-14
schema_version: 1
status: canonical
source_path: docs/product/spec/fix-bug-implementer-delivery-packet-can-design.md
created_at: 2026-04-14T06:23:00.929920471Z
updated_at: 2026-04-20T07:37:57.122219018Z
changelog_ref: fix-bug-implementer-delivery-packet-can-design.changelog.jsonl
