# Lawful Closure Continuation Rebinding Design

Status: `implemented`

Purpose: define the bounded architectural change that allows runtime to bind the next lawful bounded unit after completed closure using explicit canonical evidence rather than heuristic task picking.

## Summary
- Feature / change: add a post-closure explicit continuation-binding path for backlog tasks so completed runs can be rebound lawfully from operator-cited task evidence.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher`
- Status: `implemented`

## Current Context
- Existing system overview
  - The runtime already persists `RunGraphStatus`, `RunGraphDispatchReceipt`, `RunGraphDispatchContext`, and `RunGraphContinuationBinding`.
  - `vida taskflow continuation bind <run-id>` already records explicit continuation binding for active run-graph state.
  - `vida status --json` and `vida orchestrator-init --json` already prefer explicit continuation binding when it is admissible for the current run.
- Key components and relationships
  - `crates/vida/src/taskflow_continuation.rs` owns explicit continuation-binding mutation.
  - `crates/vida/src/continuation_binding_summary.rs` decides whether status/init surfaces accept a binding or fail closed to ambiguity.
  - `crates/vida/src/taskflow_proxy.rs` exposes advisory `task next` and `task ready` state, but does not currently persist any binding artifact from backlog selection.
- Current pain point or gap
  - After a bounded run reaches `completed` with no `downstream_dispatch_target`, runtime correctly fails closed to `completed_without_explicit_next_bounded_unit`.
  - Existing explicit bindings for in-flight run-graph work are not sufficient for the post-closure case because a completed run with no next node cannot lawfully keep reusing a stale in-flight binding.
  - There is no first-class continuation-binding surface for the operator to cite one explicit backlog task as the next lawful bounded unit after closure.

## Goal
- What this change should achieve
  - Let the operator record one explicit backlog task as the next lawful bounded unit for a completed run.
  - Keep automatic continuation fail-closed when no explicit next unit exists.
  - Make status/init surfaces accept that explicit post-closure binding without reopening heuristic task picking.
- What success looks like
  - `vida taskflow continuation bind <run-id> --task-id <task-id> --json` records a canonical continuation-binding row.
  - `vida status --json` and `vida orchestrator-init --json` report `continuation_binding.status = "bound"` for that completed run.
  - A stale pre-closure `run_graph_task` binding does not unblock a completed run by itself.
- What is explicitly out of scope
  - automatic binding from `vida task next` or `vida task ready`
  - redesign of task graph ordering or TaskFlow backlog semantics
  - widening root-session local write authority

## Requirements

### Functional Requirements
- Must extend explicit continuation binding so the operator can cite one backlog `task_id` after closure.
- Must validate that the cited task exists and is not closed before recording the binding.
- Must treat explicit post-closure backlog-task binding as admissible for summary/status/init surfaces.
- Must continue rejecting stale active-run `run_graph_task` bindings for completed runs with no next node.
- Must keep advisory `task next` output read-only and non-binding by itself.

### Non-Functional Requirements
- Performance
  - task lookup must remain a single authoritative state-store read
- Observability
  - the persisted binding must remain visible through existing status/init JSON surfaces
  - operator next actions should point to explicit binding rather than heuristic continuation
- Security
  - no fallback to `ready_head[0]`, `primary_ready_task`, or adjacent-slice plausibility

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/lawful-closure-continuation-rebinding-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
  - `docs/product/spec/README.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow continuation bind`
  - `vida status`
  - `vida orchestrator-init`

## Design Decisions

### 1. Post-closure rebinding remains explicit, not inferred
Will implement / choose:
- Extend `vida taskflow continuation bind` with `--task-id <task-id>` rather than auto-binding from ready-task order.
- Why
  - `task next` and `task ready` are advisory surfaces today and the active specs forbid heuristic self-selection after closure.
- Trade-offs
  - Requires one explicit operator action before continuation resumes after an otherwise ambiguous closure.
- Alternatives considered
  - bind from `vida task next` automatically
  - auto-bind from the first ready child task in the backlog

### 2. Completed-run summary acceptance must distinguish fresh explicit bindings from stale in-flight bindings
Will implement / choose:
- Accept explicit bindings for completed runs only when they name an admissible post-closure unit such as a backlog task or explicit downstream target.
- Keep stale `run_graph_task` bindings for completed runs non-admissible.
- Why
  - The runtime already persists explicit bindings during active execution; those older rows must not silently survive completion and masquerade as the next bounded unit.
- Trade-offs
  - Adds one more kind-level admissibility check in the summary path.
- Alternatives considered
  - accept every explicit binding row after completion
  - delete every binding row on completion and rely only on manual rebind

## Technical Design

### Core Components
- `taskflow_continuation`
  - parse `--task-id`
  - build a `task_graph_task` continuation binding
- `continuation_binding_summary`
  - admit explicit post-closure backlog-task bindings
  - keep stale active-run bindings rejected after completion
- `taskflow_layer4`
  - help text for the new explicit binding shape

### Data / State Model
- Existing `RunGraphContinuationBinding.active_bounded_unit` gains one additional admissible `kind`:
  - `task_graph_task`
- `task_graph_task` payload includes:
  - `task_id`
  - `run_id`
  - `task_status`
  - `issue_type`
- Compatibility notes
  - additive-only
  - existing bindings remain readable
  - completed runs without explicit post-closure binding remain ambiguous

### Integration Points
- `vida taskflow continuation bind <run-id> --task-id <task-id> --json`
- `vida status --json`
- `vida orchestrator-init --json`

### Bounded File Set
- `docs/product/spec/lawful-closure-continuation-rebinding-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `docs/product/spec/README.md`
- `crates/vida/src/taskflow_continuation.rs`
- `crates/vida/src/continuation_binding_summary.rs`
- `crates/vida/src/taskflow_layer4.rs`

## Fail-Closed Constraints
- Do not bind from `primary_ready_task` or `ready_tasks.first()` without an explicit operator-cited `task_id`.
- Do not let a completed run reuse a stale `run_graph_task` binding as the next bounded unit.
- Do not treat operator commentary or report wording as an implicit binding mutation.

## Implementation Plan

### Phase 1
- Register the bounded design in canonical spec maps and README.
- First proof target
  - `vida docflow check --root . docs/product/spec/lawful-closure-continuation-rebinding-design.md`

### Phase 2
- Extend explicit continuation binding with backlog-task rebinding and summary admissibility rules.
- Second proof target
  - targeted `cargo test -p vida continuation_binding_summary -- --nocapture`

### Phase 3
- Run live CLI proof, rebuild the release binary, and update the installed system binary.
- Final proof target
  - docflow + targeted cargo tests + live `vida status --json` proof + release build

## Validation / Proof
- Unit tests:
  - completed run accepts explicit `task_graph_task` binding
  - completed run rejects stale `run_graph_task` explicit binding
  - explicit active binding remains preferred during non-completed runs
- Integration tests:
  - bounded `vida taskflow continuation bind ... --task-id ...` proof through `vida status --json`
- Runtime checks:
  - `vida taskflow continuation bind <run-id> --task-id <task-id> --json`
  - `vida status --json`
  - `vida orchestrator-init --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/lawful-closure-continuation-rebinding-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md docs/product/spec/README.md`

## Observability
- Explicit backlog-task continuation binding is written to the existing authoritative binding row.
- Status/init surfaces expose the accepted `task_graph_task` binding directly.
- Ambiguity next-actions point to explicit binding rather than heuristic backlog selection.

## Rollout Strategy
- Land as one bounded additive change.
- Keep the existing fail-closed ambiguity behavior as the default when no explicit post-closure binding exists.
- Rebuild and reinstall the local `vida` binary after tests pass.

## Future Considerations
- Add a separate canonical operator surface for scope-aware next-task rebinding once task-scope lineage is persisted through runtime-consumption artifacts.
- Consider clearing stale in-flight continuation bindings on completion in a follow-up cleanup slice if operator evidence starts relying on row hygiene elsewhere.

## References
- `docs/product/spec/autonomous-report-continuation-law.md`
- `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`
- `docs/product/spec/continuation-and-seeded-dispatch-bridge-design.md`

-----
artifact_path: product/spec/lawful-closure-continuation-rebinding-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-13
schema_version: 1
status: canonical
source_path: docs/product/spec/lawful-closure-continuation-rebinding-design.md
created_at: 2026-04-13T15:00:46.008185935Z
updated_at: 2026-04-13T15:09:11.114126765Z
changelog_ref: lawful-closure-continuation-rebinding-design.changelog.jsonl
