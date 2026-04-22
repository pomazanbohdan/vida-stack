# Repair Task Close Closure Truth Exception Design

Status: `approved`

## Summary
- Feature / change: prevent downstream closure/task-close reconciliation from inheriting upstream exception-path or supersession evidence, so lawful exception-backed implementation closure yields authoritative closure truth instead of resurrecting stale implementer lineage.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher`
- Status: approved

## Current Context
- Existing system overview
  - The runtime already models exception-path evidence and explicit supersession separately.
  - Downstream packet preview already clears inherited downstream exception/supersession evidence before writing preview packets.
  - Resume-time reconciliation already contains a compatibility sanitizer for inherited downstream lane evidence.
- Key components and relationships
  - `crates/vida/src/runtime_dispatch_state.rs` synthesizes downstream receipts after bounded task-close / downstream chaining.
  - `build_downstream_dispatch_receipt(...)` currently copies `supersedes_receipt_id` and `exception_path_receipt_id` from the upstream/root receipt into the downstream receipt.
  - `root_receipt_fields_from_downstream_step(...)` currently copies those same fields back from the downstream step into the root receipt projection.
  - `crates/vida/src/taskflow_consume_resume.rs` sanitizes inherited downstream evidence while reconstructing resume inputs from persisted packets.
  - `crates/vida/src/release1_contracts.rs` treats any receipt carrying `exception_path_receipt_id` as `lane_exception_recorded`.
- Current pain point or gap
  - Live proof for `feature-repair-task-close-exception-reconcile-closure-truth` shows that after lawful exception-backed implementation/task-close closure, `vida taskflow consume continue --json` can still rebound stale implementer lineage while run-graph status remains closure-blocked.
  - The strongest current code hypothesis is upstream exception/supersession evidence contamination of downstream closure receipts, not a timeout-settlement problem.
  - Because downstream closure truth is born with inherited exception evidence, later status/continuation reconciliation can reinterpret the closure path as stale exception-recorded implementer lineage.

## Goal
- What this change should achieve
  - Make downstream closure/task-close receipts derive lane truth from their own downstream evidence, not inherited upstream exception metadata.
  - Preserve lawful exception-path semantics for the active implementation lane while preventing contamination of later closure lanes.
  - Make `consume continue`, run-graph status, and closure reconciliation converge on authoritative closure truth or fail closed explicitly.
- What success looks like
  - Exception-backed implementation closure no longer causes downstream closure receipts to carry stale inherited `exception_path_receipt_id` / `supersedes_receipt_id`.
  - `vida taskflow consume continue --json` does not rebound stale implementer lineage after the bounded task is lawfully closed.
  - Targeted tests pin the closure-truth behavior and keep non-exception downstream chaining intact.
- What is explicitly out of scope
  - Reworking the general exception-takeover model.
  - Changing the canonical meaning of `LaneExceptionRecorded` in `release1_contracts.rs`.
  - Broad redesign of downstream packet preview or full continuation-binding policy.

## Requirements

### Functional Requirements
- Must not propagate upstream `exception_path_receipt_id` into downstream closure/task-close receipts unless the downstream lane itself explicitly records exception-path evidence.
- Must not propagate upstream `supersedes_receipt_id` into downstream closure/task-close receipts unless the downstream lane itself explicitly records new supersession evidence.
- Must keep downstream closure/task-close lane status derivation based on the downstream dispatch truth/evidence only.
- Must prevent `root_receipt_fields_from_downstream_step(...)` from re-poisoning the root receipt with inherited downstream exception/supersession evidence.
- Must preserve current behavior for normal non-exception downstream chaining.
- Must preserve fail-closed behavior when closure evidence is incomplete.
- Must keep resume-time sanitization compatible with older persisted packets/results.

### Non-Functional Requirements
- Performance
  - No meaningful overhead beyond a bounded receipt-field sanitation branch.
- Scalability
  - The rule should apply to all downstream closure/task-close receipts, not only one task id.
- Observability
  - Persisted receipts and run-graph status should make closure-vs-exception lineage distinguishable without manual artifact forensics.
- Security
  - The change must not widen root-local takeover authority or blur exception-path state transitions.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/repair-task-close-closure-truth-exception-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
  - `docs/product/spec/lawful-closure-continuation-rebinding-design.md`
  - `docs/product/spec/lane-supersede-and-shared-truth-envelope-design.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
  - `launcher`
- Config / receipts / runtime surfaces affected:
  - run-graph dispatch receipts
  - downstream dispatch packet/preview reconciliation
  - `vida taskflow consume continue`
  - `vida taskflow run-graph status`

## Design Decisions

### 1. Downstream closure receipts start with clean lane-evidence fields
Will implement / choose:
- Downstream closure/task-close receipt synthesis will not inherit upstream `exception_path_receipt_id` or `supersedes_receipt_id` by default.
- Why
  - Closure is a new downstream lane and must not claim exception lineage that belongs to the prior implementation lane.
- Trade-offs
  - Older code that implicitly relied on inherited evidence must now derive closure truth from actual downstream readiness/execution state.
- Alternatives considered
  - Keep inheritance and rely only on resume-time sanitation.
  - Rejected because the bad lineage is being persisted too early.
- ADR link if this must become a durable decision record
  - none

### 2. Root receipt hydration must not copy inherited exception metadata back from downstream steps
Will implement / choose:
- Root receipt hydration from downstream step receipts will preserve downstream preview/blocker fields but will only copy lane-evidence ids when the downstream step actually owns them.
- Why
  - The current copy-back path can reintroduce stale exception/supersession evidence even if downstream preview/persistence tries to stay clean.
- Trade-offs
  - Slightly tighter distinction between root-lane evidence and downstream-lane evidence.
- Alternatives considered
  - Continue copying all evidence ids verbatim from downstream step receipts.
  - Rejected because that preserves the contamination path.
- ADR link if needed
  - none

### 3. Resume sanitation remains as a compatibility backstop, not the primary fix
Will implement / choose:
- Keep `sanitize_inherited_downstream_lane_evidence(...)` for persisted historical artifacts, but move the primary repair to receipt construction/hydration.
- Why
  - New receipts should be correct at write time; resume-time healing should only protect older or already persisted artifacts.
- Trade-offs
  - Two layers still exist, but each has a clear role: write-time correctness and read-time compatibility.
- Alternatives considered
  - Remove resume-time sanitation entirely.
  - Rejected because historical persisted packets can still exist.
- ADR link if needed
  - none

## Technical Design

### Core Components
- Main components
  - `build_downstream_dispatch_receipt(...)`
  - `root_receipt_fields_from_downstream_step(...)`
  - downstream preview / task-close reconciliation tests
  - resume-time sanitizer regression coverage
- Key interfaces
  - `RunGraphDispatchReceipt`
  - `derive_lane_status(...)`
  - `vida taskflow consume continue --json`
  - `vida taskflow run-graph status <run-id> --json`
- Bounded responsibilities
  - receipt construction must not persist inherited closure-lane exception evidence
  - root receipt refresh must not re-copy stale evidence into the active projection
  - resume logic must stay backward-compatible with already persisted contaminated packets

### Data / State Model
- Important entities
  - root dispatch receipt
  - downstream dispatch receipt
  - exception-path receipt id
  - supersedes receipt id
  - downstream lane status / closure truth
- Receipts / runtime state / config fields
  - `exception_path_receipt_id`
  - `supersedes_receipt_id`
  - `lane_status`
  - `dispatch_status`
  - `downstream_dispatch_*`
  - continuation binding fields in reconciled runtime status
- Migration or compatibility notes
  - Existing persisted contaminated downstream packets/results must still heal through resume-time sanitation and targeted reconciliation logic.

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - implementation/task-close -> downstream closure receipt
  - downstream receipt -> run-graph status
  - persisted packet/result -> consume/resume reconciliation
- Cross-document / cross-protocol dependencies
  - lawful exception-takeover recording vs active authority
  - lawful post-closure continuation rebinding
  - tracked-flow task-close reconciliation

### Bounded File Set
- `docs/product/spec/repair-task-close-closure-truth-exception-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/taskflow_consume_resume.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - No heuristic continuation rebinding to stale implementer lineage after lawful task close.
  - No widening of exception-path takeover semantics to downstream closure lanes.
- Required receipts / proofs / gates
  - Downstream closure truth must be supported by downstream receipt state, not upstream exception metadata.
  - Resume/status surfaces must either preserve closure truth or fail closed with explicit blocker evidence.
- Safety boundaries that must remain true during rollout
  - Exception-path semantics for the original active implementation lane remain intact.
  - Non-exception downstream chaining remains intact.
  - Historical contaminated packets remain readable through compatibility sanitization.

## Implementation Plan

### Phase 1
- Finalize this bounded design and register it in the current spec canon.
- Confirm the minimal code fix surfaces and proof targets.
- First proof target
  - `vida docflow check --root . docs/product/spec/repair-task-close-closure-truth-exception-design.md`

### Phase 2
- Repair downstream receipt construction/hydration so closure/task-close paths stop inheriting upstream exception/supersession evidence.
- Add or update targeted regression tests for exception-backed task-close -> downstream closure reconciliation.
- Second proof target
  - targeted `cargo test -p vida ...` for downstream closure truth / resume reconciliation

### Phase 3
- Re-run runtime reconciliation checks against the live task/run after the code patch.
- Confirm lawful continuation no longer rebounds stale implementer lineage.
- Final proof target
  - green targeted tests plus runtime proof on `feature-repair-task-close-exception-reconcile-closure-truth`

## Validation / Proof
- Unit tests:
  - downstream receipt construction should not inherit upstream exception/supersession evidence into closure/task-close steps
  - root receipt hydration should preserve downstream preview without re-copying stale evidence ids
  - resume-time reconciliation should still heal contaminated persisted packets
- Integration tests:
  - task-close reconciliation over exception-backed implementation closure
- Runtime checks:
  - `vida taskflow run-graph status feature-repair-task-close-exception-reconcile-closure-truth --json`
  - `vida taskflow consume continue --run-id feature-repair-task-close-exception-reconcile-closure-truth --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/repair-task-close-closure-truth-exception-design.md`
  - `vida docflow check --profile active-canon`

## Observability
- Logging points
  - downstream receipt synthesis and task-close reconciliation
- Metrics / counters
  - none new expected for this bounded slice
- Receipts / runtime state written
  - run-graph dispatch receipts
  - downstream dispatch packet/result previews
  - continuation-binding projections

## Rollout Strategy
- Development rollout
  - land the bounded receipt-sanitization fix with targeted regression tests
- Migration / compatibility notes
  - historical contaminated packets continue to heal through resume-time sanitation
- Operator or user restart / restart-notice requirements
  - refresh the installed `vida` binary before collecting runtime proof from CLI surfaces

## Future Considerations
- Follow-up ideas
  - unify downstream receipt-evidence sanitation into a shared helper used by both preview writing and receipt hydration
- Known limitations
  - older persisted artifacts may still require resume-time healing until the new fix is installed and becomes the only writer
- Technical debt left intentionally
  - `derive_lane_status(...)` remains exception-first by design; this bounded slice only fixes wrongful inheritance into downstream closure lanes

## References
- `docs/product/spec/lawful-closure-continuation-rebinding-design.md`
- `docs/product/spec/lane-supersede-and-shared-truth-envelope-design.md`
- `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/taskflow_consume_resume.rs`

-----
artifact_path: product/spec/repair-task-close-closure-truth-exception-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/repair-task-close-closure-truth-exception-design.md
created_at: 2026-04-21T17:48:37.672562947Z
updated_at: 2026-04-21T17:52:53.720240748Z
changelog_ref: repair-task-close-closure-truth-exception-design.changelog.jsonl
