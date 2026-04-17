# Repair Fail Closed Resume Closure Truth Design

Status: active bounded design

Purpose: define the bounded architectural repair that keeps `consume continue` and resume-time packet reconciliation fail-closed while still repairing stale persisted specification packet lineage into the tracked design-doc scope before validation.

## Summary
- Feature / change: fail-closed resume and persisted specification packet reconciliation
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: `approved`

## Current Context
- `taskflow_consume_resume::read_dispatch_packet(...)` reads the persisted dispatch packet, applies bounded normalization, and then validates the packet contract.
- Runtime law now requires `delivery_task_packet` packets with `handoff_task_class=specification` to keep `owned_paths` exactly equal to `tracked_flow_bootstrap.design_doc_path`.
- The live A1 run `task-recovery-cluster-fail-closed-resume-rewrite-truth` still carried a persisted specification packet whose `owned_paths` pointed at `crates/vida/src/taskflow_consume_resume.rs` even though the tracked design document is `docs/product/spec/repair-fail-closed-resume-closure-truth-design.md`.
- Because normalization only repaired missing `owned_paths`, `vida taskflow consume continue` failed closed on the stale packet before the specification gate could progress.

## Goal
- Repair stale persisted specification packet lineage to the tracked design-doc scope before validation.
- Preserve the existing fail-closed validation rule once the packet has been normalized to current law.
- Unblock the active A1 specification cycle without reintroducing code-scope widening for specification lanes.

## Requirements

### Functional Requirements
- Resume-time packet normalization must repair specification delivery packets when `owned_paths` are present but do not match the tracked design-doc scope.
- Implementer delivery packets must keep the existing request-text-derived repair behavior.
- Validation must still fail closed after normalization if the packet remains inconsistent or malformed.
- Persisted packet repair must write the corrected packet back to disk so later runtime surfaces see the same truth.

### Non-Functional Requirements
- Compatibility: legacy persisted packets should converge toward the current packet law rather than requiring manual state surgery.
- Safety: only specification packets may be auto-repaired to tracked design-doc scope; do not widen or rewrite other task classes opportunistically.
- Observability: repaired packet JSON must remain inspectable through existing packet/state surfaces.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/repair-fail-closed-resume-closure-truth-design.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
- Config / receipts / runtime surfaces affected:
  - persisted runtime dispatch packets
  - `vida taskflow consume continue`
  - run-graph recovery / lane inspection surfaces that read the repaired packet

## Design Decisions

### 1. Specification packet repair happens on read before validation
Will implement / choose:
- extend `normalize_runtime_dispatch_packet(...)` so specification delivery packets are repaired when `owned_paths` are mismatched, not only when they are missing
- Why: the persisted packet already exists; the safest bounded repair point is the existing read-normalize-validate flow
- Trade-offs: normalization becomes slightly stronger for specification packets, but only toward the tracked design doc already recorded in the packet context
- Alternatives considered: manual state repair or relaxing validation; rejected because both weaken runtime law and operator trust

### 2. Fail-closed validation remains unchanged after repair
Will implement / choose:
- keep `validate_runtime_dispatch_packet_contract(...)` strict about specification `owned_paths`
- Why: repair should converge stale packets to current law, not replace or weaken the law
- Trade-offs: malformed packets without tracked design-doc context still fail closed
- Alternatives considered: downgrading the validation mismatch to a warning; rejected because it would let illegal packet scope continue through delegated execution

### 3. The bounded fix stays in the resume path
Will implement / choose:
- limit this slice to `taskflow_consume_resume.rs` plus regression tests
- Why: the live failure reproduces in the resume/read path; no broader packet-rendering refactor is required for this bounded correction
- Trade-offs: broader cleanup can stay for later waves
- Alternatives considered: reworking dispatch-state rendering in the same change; rejected because A1 needs the smallest lawful repair first

## Technical Design

### Core Components
- `taskflow_consume_resume.rs`
  - detect mismatched specification `owned_paths`
  - rewrite them to the tracked design-doc scope before validation
  - persist the repaired packet

### Data / State Model
- Canonical specification write scope remains `tracked_flow_bootstrap.design_doc_path`
- Persisted packet `delivery_task_packet.owned_paths` becomes repairable state when it diverges from that tracked path
- Validation remains the canonical enforcement point for the final packet contract

### Integration Points
- `read_dispatch_packet(...)`
- `normalize_runtime_dispatch_packet(...)`
- `validate_runtime_dispatch_packet_contract(...)`
- `vida taskflow consume continue`

### Bounded File Set
- `docs/product/spec/repair-fail-closed-resume-closure-truth-design.md`
- `crates/vida/src/taskflow_consume_resume.rs`

## Fail-Closed Constraints
- Do not relax specification packet scope validation.
- Do not derive specification write scope from free-form request text.
- Do not auto-repair non-specification packets beyond the already-supported legacy implementer behavior.
- Do not move to A2-A4 while the live A1 specification cycle still fails on stale packet lineage.

## Implementation Plan

### Phase 1
- encode the bounded design and anchor the active A1 repair to the persisted packet mismatch evidence
- First proof target: design doc finalized and accepted by DocFlow

### Phase 2
- extend normalization to repair mismatched specification `owned_paths` before validation
- Second proof target: unit/regression tests for mismatched specification packet repair

### Phase 3
- run the live `consume continue` path against the active run and confirm the fail-closed mismatch is removed
- Final proof target: runtime proof that the active A1 packet is normalized and no longer blocks on the stale code-owned scope

## Validation / Proof
- Unit tests:
  - `normalize_runtime_dispatch_packet_repairs_mismatched_specification_owned_paths`
  - `read_dispatch_packet_repairs_mismatched_specification_owned_scope_before_validation`
- Integration tests:
  - targeted `vida taskflow consume continue --run-id task-recovery-cluster-fail-closed-resume-rewrite-truth --json`
- Runtime checks:
  - inspect the persisted dispatch packet after normalization
  - inspect status/recovery summaries for the active run
- Canonical checks:
  - `vida docflow finalize-edit docs/product/spec/repair-fail-closed-resume-closure-truth-design.md "record bounded A1 repair design"`
  - `vida docflow check --root . docs/product/spec/repair-fail-closed-resume-closure-truth-design.md`

## Observability
- persisted packet JSON shows repaired `owned_paths`
- `consume continue` stops failing on stale specification scope mismatch
- recovery/lane inspection surfaces keep reading one repaired packet truth

## Rollout Strategy
- ship as a bounded A1 runtime-family repair
- no schema migration beyond repair-on-read persistence
- refresh the release binary after bounded proof passes so installed `vida` matches the repaired runtime

## Future Considerations
- consider surfacing packet normalization/repair events explicitly in operator summaries
- consider broader stale-packet reconciliation for other task classes only when there is a concrete fail-closed runtime need

## References
- `docs/product/spec/specification-lane-scope-hardening-design.md`
- `crates/vida/src/taskflow_consume_resume.rs`
- `crates/vida/src/runtime_dispatch_state.rs`

-----
artifact_path: product/spec/repair-fail-closed-resume-closure-truth-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-16
schema_version: 1
status: canonical
source_path: docs/product/spec/repair-fail-closed-resume-closure-truth-design.md
created_at: 2026-04-16T06:11:37.967559197Z
updated_at: 2026-04-17T11:49:34.294393255Z
changelog_ref: repair-fail-closed-resume-closure-truth-design.changelog.jsonl
