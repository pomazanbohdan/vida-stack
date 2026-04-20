# Fix Bug Specification Lane Can Mutate Design

Status: `approved`

## Summary
- Feature / change: prevent specification/business_analyst packets with docs-only scope from mutating code outside the approved design-doc scope.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: `approved`

## Current Context
- Specification/business_analyst packets are intended to stay inside design-first, docs-scoped work before implementation begins.
- The active bug is that docs-only specification packets can still execute on write-capable backends because packet scope is currently advisory rather than enforced at backend admissibility and launch time.
- The feature epic already defines the bounded repair: add a runtime contract so selected backend write scope must be admissible for packet `owned_paths` and `read_only_paths`, then add an execute-time gate against persisted packet/backend mismatch.

## Goal
- Ensure docs-only specification packets cannot mutate non-doc code paths.
- Preserve lawful design-first routing while keeping implementation packets with explicit owned paths admissible.
- Out of scope: redesigning the whole backend selection model, changing implementation packet rules beyond admissibility enforcement, or widening beyond the bounded runtime fix.

## Requirements

### Functional Requirements
- Backend admissibility must account for packet `owned_paths` and `read_only_paths`, not just coarse task-class routing.
- Specification/business_analyst packets that are docs-only must fail closed when the selected backend write scope would allow code mutation outside the approved doc scope.
- Persisted packet/backend mismatches must be caught again at execute time, not only at initial shaping time.
- Implementation packets with explicit owned paths must remain admissible when their backend write scope matches the bounded packet ownership.

### Non-Functional Requirements
- Keep the change bounded to the runtime/backend admissibility path and focused regressions.
- Do not widen write scope inference beyond what the persisted packet explicitly owns.
- Preserve lawful spec-first flow and existing packet/runtime output contracts.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  `docs/product/spec/fix-bug-specification-lane-can-mutate-design.md`
- Framework protocols affected:
  design-first routing and bounded packet ownership rules already carried by project/runtime law
- Runtime families affected:
  packet/backend admissibility
  execute-time packet validation
  specification lane launch gating
- Config / receipts / runtime surfaces affected:
  persisted packet `owned_paths`
  persisted packet `read_only_paths`
  backend write-scope admissibility checks

## Design Decisions

### 1. Packet Scope Must Be Enforcement Truth, Not Advisory Metadata
Will implement / choose:
- Treat `owned_paths` and `read_only_paths` in the persisted packet as enforceable runtime constraints for backend admissibility and execution.
- This makes design-doc scope real rather than descriptive.
- Trade-off: stricter launch gates may reject packets that older runtime behavior previously let through.

### 2. Enforce The Same Rule At Selection Time And Execute Time
Will implement / choose:
- Add the scope check both when selecting an admissible backend and when executing against a persisted packet/backend combination.
- This prevents stale or drifted packet/backend pairings from bypassing the original gate.
- Trade-off: duplicated validation logic may be needed unless a shared helper is introduced.

## Technical Design

### Core Components
- Runtime/backend admissibility logic for packet launch
- Execute-time validation against persisted packet/backend mismatch
- Focused regressions for docs-only specification packets and explicit-path implementation packets

### Data / State Model
- Scope-bearing packet fields:
  `owned_paths`
  `read_only_paths`
- Backend capability data that matters:
  write-scope admissibility
  backend class and policy flags
- Compatibility note:
  implementation packets with explicit owned paths stay admissible; docs-only specification packets become fail-closed against non-doc mutation.

### Integration Points
- Packet shaping / persistence
  must keep `owned_paths` and `read_only_paths` intact for later enforcement
- Backend selection
  must reject backends whose write scope exceeds packet ownership
- Execute-time packet launch
  must fail closed when persisted packet/backend pairing is no longer admissible

### Bounded File Set
- runtime/backend admissibility code path
- execute-time packet/backend validation path
- focused regression tests for docs-only specification packets and explicit-path implementation packets

## Fail-Closed Constraints
- Docs-only specification packets must never produce code writes outside approved doc scope.
- If packet scope and backend write scope disagree, execution must fail closed.
- Do not reclassify implementation packets as invalid when they already carry explicit owned paths that match backend write scope.
- Do not introduce advisory-only fallback behavior after this fix.

## Implementation Plan

### Phase 1
- Inspect the current backend admissibility and launch path for where packet scope is ignored.
- Reproduce the docs-only specification packet mutation gap in focused tests.
- First proof target: a docs-only specification packet is shown admissible today when it should not be.

### Phase 2
- Add scope-aware admissibility checks using packet `owned_paths` / `read_only_paths`.
- Add execute-time guard against persisted packet/backend mismatch.
- Second proof target: docs-only specification packets fail closed, while explicit-path implementation packets remain admissible.

### Phase 3
- Re-run focused regressions and nearby runtime checks.
- Confirm spec-first routing remains lawful and bounded.
- Final proof target: bounded green tests for both the fail-closed docs-only case and the admissible implementation case.

## Validation / Proof
- Unit tests:
  targeted regressions for docs-only specification packets and explicit-path implementation packets
- Integration tests:
  bounded `cargo test -p vida` filters around backend admissibility and execute-time validation
- Runtime checks:
  inspect packet/backend launch behavior where needed through taskflow packet surfaces
- Canonical checks:
  `vida docflow check --root . docs/product/spec/fix-bug-specification-lane-can-mutate-design.md`

## Observability
- Operator-visible outcome:
  docs-only specification packets should fail closed instead of mutating code
- Runtime evidence:
  admissibility and execute-time mismatch should be visible through existing fail-closed packet/runtime surfaces
- Receipts / runtime state written:
  no new receipt family required; reuse persisted packet and runtime launch evidence

## Rollout Strategy
- Keep the fix in one bounded runtime-family packet.
- Validate through focused regressions before moving to implementation execution.
- No migration or restart path is required.

## Future Considerations
- A follow-up can unify packet-scope enforcement across more lane types if the same gap exists elsewhere.
- If packet scope metadata grows richer, a shared admissibility helper may be preferable to distributed checks.

## References
- `docs/product/spec/fix-bug-implementer-delivery-packet-can-design.md`
- `docs/product/spec/fix-bug-consume-continue-completed-closure-design.md`
- feature epic `feature-fix-bug-specification-lane-can-mutate`

-----
artifact_path: product/spec/fix-bug-specification-lane-can-mutate-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-14
schema_version: 1
status: canonical
source_path: docs/product/spec/fix-bug-specification-lane-can-mutate-design.md
created_at: 2026-04-14T06:16:46.483045815Z
updated_at: 2026-04-20T07:58:42.828995276Z
changelog_ref: fix-bug-specification-lane-can-mutate-design.changelog.jsonl
