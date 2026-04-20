# Fix Bug Consume Continue Completed Closure Design

Status: `approved`

## Summary
- Feature / change: fix stale downstream dispatch reuse when `vida taskflow consume continue` resumes a run that is already completed and closure-bound.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: `approved`

## Current Context
- `crates/vida/src/taskflow_consume_resume.rs` owns the resume/continue path that reconciles persisted run-graph state, dispatch receipts, downstream preview, and final snapshot emission.
- The lawful closure model already exists in `docs/product/spec/lawful-closure-continuation-rebinding-design.md` and `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`.
- The active bug is that a completed run explicitly rebound to closure can still inherit stale downstream coach or verification packet lineage from older receipts, which makes `consume continue` look like it should dispatch again after closure is already the only lawful path.

## Goal
- Ensure completed closure-bound runs do not reuse stale downstream packet lineage during `consume continue`.
- Allow only lawful closure continuation behavior after a run is already completed and explicitly bound to closure.
- Out of scope: changing general dispatch routing law, changing non-closure continuation semantics, or widening beyond `taskflow_consume_resume` and focused resume tests.

## Requirements

### Functional Requirements
- When a run is `completed` and the continuation binding points to downstream closure, `vida taskflow consume continue` must not resume stale downstream coach or verification packet lineage.
- The resume path must either fail closed or consume only the lawful closure path when older downstream receipt evidence conflicts with completed closure state.
- Existing lawful downstream closure behavior must remain valid when the current closure path is truly packet-ready.
- Regression coverage must include the mixed case where completed closure state coexists with stale blocked downstream lineage from older receipts.

### Non-Functional Requirements
- Keep the implementation bounded to `crates/vida/src/taskflow_consume_resume.rs` and targeted resume tests.
- Preserve release-1 operator-contract parity in any JSON output touched by the fix.
- Avoid new fallback paths that silently reinterpret stale lineage as executable work.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  `docs/product/spec/fix-bug-consume-continue-completed-closure-design.md`
- Framework protocols affected:
  `docs/product/spec/lawful-closure-continuation-rebinding-design.md`
  `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- Runtime families affected:
  `vida taskflow consume continue`
- Config / receipts / runtime surfaces affected:
  persisted dispatch receipt reconciliation
  runtime-consumption final snapshot emission
  downstream closure packet-ready projection

## Design Decisions

### 1. Completed Closure State Wins Over Stale Downstream Lineage
Will implement / choose:
- Treat explicit completed closure state as the higher-authority runtime truth when older downstream coach or verification lineage disagrees.
- This preserves lawful closure semantics and prevents stale packet resurrection after completion.
- Trade-off: resume reconciliation becomes stricter and may fail closed in cases that previously looked resumable.

### 2. Keep The Fix Local To Resume Reconciliation
Will implement / choose:
- Apply the repair in `taskflow_consume_resume.rs` instead of spreading new guard logic across unrelated taskflow families.
- Keep tests focused on the exact stale-lineage regression and on preserving lawful closure packet-ready behavior.
- Trade-off: this keeps ownership clear but does not attempt a broader cleanup of adjacent receipt history semantics.

## Technical Design

### Core Components
- `crates/vida/src/taskflow_consume_resume.rs`
  Resume/continue reconciliation, stale-lineage guard, and final snapshot shaping.
- Focused resume tests in the same module or existing nearby test surface
  Regression proof for completed closure-bound runs and stale downstream lineage.

### Data / State Model
- Runtime truths that must stay aligned:
  completed run-graph state
  explicit continuation binding to closure
  persisted dispatch receipt lineage
  downstream packet-ready preview
- Compatibility note:
  stale downstream receipt history may remain in persisted state, but `consume continue` must not reinterpret it as the next lawful dispatch after closure.

### Integration Points
- `vida taskflow consume continue --json`
  Must report only lawful closure-path truth for the completed run.
- Runtime-consumption final snapshots
  Must preserve release-1 shared envelope fields while reflecting the corrected closure-only reconciliation.
- Resume regression tests
  Must cover both stale-lineage rejection and lawful closure packet-ready preservation.

### Bounded File Set
- `crates/vida/src/taskflow_consume_resume.rs`
- focused resume tests in existing taskflow consume/resume coverage

## Fail-Closed Constraints
- Do not resume stale downstream coach or verification packets after completed closure is already authoritative.
- Do not widen into unrelated routing, approval, or operator-surface redesign.
- If reconciliation cannot prove the closure path is still lawful, fail closed rather than guessing a downstream dispatch.
- Preserve current release-1 operator envelope/parity behavior for emitted JSON snapshots.

## Implementation Plan

### Phase 1
- Inspect current resume reconciliation for how completed closure state and downstream receipt lineage are merged.
- Identify the exact stale-lineage path that overrides closure truth.
- First proof target: reproduce the bug with a focused completed-closure regression.

### Phase 2
- Add the closure-first reconciliation guard in `taskflow_consume_resume.rs`.
- Preserve lawful downstream closure packet-ready behavior when the closure path is genuinely current.
- Second proof target: regression covers stale blocked downstream lineage and keeps closure-path behavior intact.

### Phase 3
- Re-run focused resume and operator-envelope tests.
- Verify `consume continue --json` preserves release-1 envelope fields after the fix.
- Final proof target: targeted green tests plus one runtime `consume continue` proof for the repaired case when feasible.

## Validation / Proof
- Unit tests:
  targeted resume regression for completed closure + stale downstream lineage
  existing resume envelope/parity tests
- Integration tests:
  bounded `cargo test -p vida` filters for resume reconciliation coverage
- Runtime checks:
  `vida taskflow consume continue --json`
- Canonical checks:
  `vida docflow check --root . docs/product/spec/fix-bug-consume-continue-completed-closure-design.md`

## Observability
- Operator-visible outcome:
  `consume continue` should no longer advertise stale downstream packet continuation after lawful closure.
- JSON/runtime evidence:
  final snapshot should show corrected closure-path truth while preserving shared release-1 envelope fields.
- Receipts / runtime state written:
  no new receipt families; reuse existing runtime-consumption final snapshot and dispatch receipt state.

## Rollout Strategy
- Keep the change in one bounded runtime-family packet.
- Validate through focused tests before handing off into the dev packet.
- No migration or restart path is required.

## Future Considerations
- Adjacent stale-lineage reconciliation bugs in other post-completion paths may deserve a separate cleanup packet.
- If similar bugs recur, a narrower reusable helper for “completed closure overrides stale downstream lineage” may be worthwhile later.

## References
- `docs/product/spec/lawful-closure-continuation-rebinding-design.md`
- `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- `crates/vida/src/taskflow_consume_resume.rs`

-----
artifact_path: product/spec/fix-bug-consume-continue-completed-closure-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-14
schema_version: 1
status: canonical
source_path: docs/product/spec/fix-bug-consume-continue-completed-closure-design.md
created_at: 2026-04-14T06:03:50.836024647Z
updated_at: 2026-04-20T07:31:51.597062729Z
changelog_ref: fix-bug-consume-continue-completed-closure-design.changelog.jsonl
