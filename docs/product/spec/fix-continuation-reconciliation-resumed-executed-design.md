# Fix Continuation Reconciliation Resumed Executed Design

Status: `approved`

## Summary
- Feature / change: make resumed runtime reconciliation recompute downstream dispatch preview for already executed receipts when task/doc state has changed after the original lane execution
- Owner layer: `runtime-family`
- Runtime surfaces: `vida taskflow consume continue`, run-graph dispatch receipts, continuation binding / status projection
- Status: `approved for bounded implementation`

## Current Context
- The active runtime can execute a specification lane, persist execution evidence, and later rely on manual or host-assisted steps to finalize the design doc and close the spec/work-pool/dev tasks.
- In the reproduced case, the spec packet `feature-continue-post-main-carveout-with-next-spec` finished lawfully, the design doc was finalized, and the spec/work-pool/dev tasks were closed.
- Despite that, `vida status --json` continued to bind the active bounded unit to the spec lane and kept stale downstream blockers:
  - `pending_specification_evidence`
  - `pending_design_finalize`
  - `pending_spec_task_close`
- `vida taskflow consume continue --json` recorded a new final snapshot but did not clear or recompute those blockers.

## Goal
- Ensure resumed continuation logic re-evaluates downstream dispatch readiness from current persisted task/doc evidence after an already executed receipt is resumed.
- Make status / continuation binding stop reporting a stale spec-active cycle once the design/spec handoff requirements are satisfied.
- Preserve fail-closed behavior for genuinely incomplete packets.
- Out of scope:
  - changing the overall lane order
  - widening into approval / closure-surface redesign
  - changing external carrier routing or cost-selection policy

## Requirements

### Functional Requirements
- When `vida taskflow consume continue` resumes a receipt whose `dispatch_status` is already `executed`, it must still refresh downstream dispatch preview from current state when the receipt remains the latest active run-graph receipt.
- For a specification lane, refreshed preview logic must recognize when specification evidence is already recorded, the design doc is finalized, and the spec packet is closed, then unblock the tracked work-pool handoff.
- If downstream work-pool or dev tasks are already closed and their existing logic marks the next handoff ready, the resumed receipt must reflect that refreshed readiness rather than preserve stale blockers.
- The persisted dispatch receipt, downstream packet, and status/continuation projection must all become mutually consistent after the resume cycle.

### Non-Functional Requirements
- Keep the fix bounded to runtime dispatch / resume reconciliation logic.
- Do not loosen blocker semantics for incomplete or missing evidence.
- Prefer recomputation from current persisted state over ad hoc status overrides.
- Add regression tests that reproduce the stale-blocker scenario and prove the refreshed behavior.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/fix-continuation-reconciliation-resumed-executed-design.md`
  - `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`
  - `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`
- Runtime families affected:
  - runtime dispatch reconciliation
  - taskflow resume / continue
  - status / continuation binding projection
- Config / receipts / runtime surfaces affected:
  - run-graph dispatch receipt refresh
  - downstream dispatch packet rewrite during resume

## Design Decisions

### 1. Refresh Preview For Executed Receipts During Resume
Will implement / choose:
- Recompute downstream preview during resume not only for `dispatch_status == routed`, but also for `dispatch_status == executed` when the receipt still represents an open delegated cycle.
- Why:
  - the stale blockers live on the persisted receipt, not in the task store
  - resume is the lawful place to reconcile persisted runtime state with updated task/doc evidence
  - it avoids inventing a second side-channel for status correction
- Trade-offs:
  - resume does a little more work on already executed receipts
  - tests must prove that fail-closed behavior still holds for incomplete evidence
- Alternatives considered:
  - make `vida status` infer around stale receipt blockers without touching the receipt
  - add a separate manual reconciliation command for executed design-gate receipts

### 2. Keep Reconciliation Centered In Existing Preview Logic
Will implement / choose:
- Reuse `refresh_downstream_dispatch_preview` / `derive_downstream_dispatch_preview` rather than creating a separate spec-only cleanup path.
- Why:
  - preview derivation already contains the canonical blocker and next-target rules
  - a single source of truth is safer than duplicating blocker-clearing logic in status or continuation code
- Trade-offs:
  - the preview functions need to stay correct across routed and executed resume states
- Alternatives considered:
  - patch run-graph status directly in `taskflow_consume_resume.rs`
  - teach `status_surface` to special-case closed spec tasks

## Technical Design

### Core Components
- `crates/vida/src/taskflow_consume_resume.rs`
  - resume path that currently refreshes downstream preview only when `dispatch_status == "routed"`
- `crates/vida/src/runtime_dispatch_state.rs`
  - canonical downstream preview derivation and refresh logic
  - executed-lane chaining and receipt-backed evidence helpers
- `crates/vida/src/state_store_run_graph_summary.rs`
  - status projection behavior that should naturally improve once the receipt is refreshed

### Data / State Model
- No schema migration is required.
- The fix updates how existing persisted receipts are refreshed during resume.
- Canonical evidence remains:
  - dispatch execution evidence
  - finalized design artifact
  - closed tracked tasks

### Integration Points
- `vida taskflow consume continue`
- `refresh_downstream_dispatch_preview(...)`
- `derive_downstream_dispatch_preview(...)`
- status / continuation binding surfaces that read the refreshed receipt

### Bounded File Set
- `crates/vida/src/taskflow_consume_resume.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/state_store_run_graph_summary.rs` only if projection logic needs a small consistency adjustment
- `docs/product/spec/fix-continuation-reconciliation-resumed-executed-design.md`

## Fail-Closed Constraints
- Do not clear blockers unless the canonical preview derivation now returns the next handoff as ready.
- Do not treat a closed task alone as universal evidence; keep the existing targeted evidence rules.
- Do not widen the fix into approval, exception-path takeover, or carrier-selection behavior.
- If the stale state cannot be resolved through canonical preview recomputation, stop and shape a narrower runtime-lifecycle fix instead of layering manual overrides.

## Implementation Plan

### Phase 1
- Finalize this bounded design packet and validate it through DocFlow.
- Proof target:
  - `vida docflow check --root . docs/product/spec/fix-continuation-reconciliation-resumed-executed-design.md`

### Phase 2
- Update resume logic so an already executed receipt can still refresh downstream preview from current persisted evidence before status projection is emitted.
- Keep the refresh path bounded to the latest resumed receipt rather than global task scanning.
- Proof target:
  - targeted unit / regression coverage for resumed executed receipts

### Phase 3
- Validate that the refreshed receipt now clears the stale spec blockers and allows continuation to advance lawfully.
- Commit, push, build release, and update the installed `vida` binary.
- Final proof target:
  - `cargo test -p vida refresh_downstream_dispatch_preview -- --nocapture`
  - `cargo test -p vida taskflow_consume_resume -- --nocapture`
  - release build
  - installed binary hash parity

## Validation / Proof
- Unit tests:
  - refresh-preview regression for executed specification receipts
  - resume-command regression proving stale blockers are recomputed
- Integration tests:
  - not required beyond bounded resume/receipt coverage
- Runtime checks:
  - `vida taskflow consume continue --json`
  - `vida status --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/fix-continuation-reconciliation-resumed-executed-design.md`

## Observability
- No new schema fields are required.
- The proof artifact is the refreshed persisted receipt and the resulting status/continuation projection.
- Standard commit, push, release-build, and binary-update evidence remain required.

## Rollout Strategy
- Land as a bounded runtime reconciliation fix under the continuation/runtime blocker stream.
- Re-run the current reproduced operator flow after merge to confirm the stale spec-active cycle no longer persists.
- No migration or operator restart work beyond the normal binary update cycle.

## Future Considerations
- If additional stale-cycle cases appear for coach or verifier lanes, generalize the same resume-refresh treatment without introducing per-lane bespoke logic.
- The older umbrella epic `feature-fix-current-continuation-runtime-evidence-blocke` should later absorb or reference this narrower delivered slice so the roadmap remains understandable.

## References
- `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`
- `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`
- `feature-fix-current-continuation-runtime-evidence-blocke`

-----
artifact_path: product/spec/fix-continuation-reconciliation-resumed-executed-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-12
schema_version: 1
status: canonical
source_path: docs/product/spec/fix-continuation-reconciliation-resumed-executed-design.md
created_at: 2026-04-12T17:41:45.798477791Z
updated_at: 2026-04-12T17:43:11.13759138Z
changelog_ref: fix-continuation-reconciliation-resumed-executed-design.changelog.jsonl
