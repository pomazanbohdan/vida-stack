# Analysis Lane Can Close Implementation Without Write Evidence Design

Purpose: Bound the audit blocker where a write-producing implementation task dispatches only a read-only `analysis` lane, records execution evidence for that diagnostic handoff, and then reconciles the run graph to `implementation_complete` with a downstream `closure` packet ready even though no lawful write-capable implementer evidence exists.

Status: `proposed`

## Summary
- Feature / change: repair implementation-completion and closure-candidate truth so non-writer diagnostic lanes cannot satisfy completion for write-class work.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher | state_store`
- Status: `proposed`

## Current Context
- Existing system overview
  - `taskflow_run_graph.rs` owns seeded implementation lane sequencing and the explicit conditions that advance an implementation run to `implementation_complete`.
  - `state_store_run_graph_summary.rs` reconciles persisted dispatch receipts into read-side `RunGraphStatus` projections and currently upgrades some closure-ready downstream receipts into `status=completed` / `lifecycle_stage=implementation_complete`.
  - `state_store.rs` already carries a regression that encodes this closure-candidate upgrade for a generic executed receipt with downstream `closure` readiness.
- Key components and relationships
  - The live blocker run `feature-fix-bug-coach-retry-reuses-same-blocked-hermes-packet-without-fallback` dispatched only an `analysis` lane through a read-only internal Codex carrier:
    - dispatch packet: `.vida/data/state/runtime-consumption/dispatch-packets/feature-fix-bug-coach-retry-reuses-same-blocked-hermes-packet-without-write-evidence-2026-04-21T19-54-08.785792268Z.json`
    - executed result: `.vida/data/state/runtime-consumption/dispatch-results/feature-fix-bug-coach-retry-reuses-same-blocked-hermes-packet-without-fallback-2026-04-21T19-58-19.216486941Z.json`
    - downstream closure packet: `.vida/data/state/runtime-consumption/downstream-dispatch-packets/feature-fix-bug-coach-retry-reuses-same-blocked-hermes-packet-without-fallback-2026-04-21T19-58-22.626574202Z.json`
  - That analysis execution returned `execution_evidence_recorded`, but provider output itself showed write attempts rejected by the read-only sandbox.
  - Despite that, `cargo run -p vida -- taskflow run-graph status feature-fix-bug-coach-retry-reuses-same-blocked-hermes-packet-without-fallback --json` reconciled the run to:
    - `status=completed`
    - `lifecycle_stage=implementation_complete`
    - `active_node=analysis`
    - `downstream_dispatch_target=closure`
    - `downstream_dispatch_status=packet_ready`
- Current pain point or gap
  - The runtime is conflating “a delegated lane executed” with “implementation is complete”.
  - Closure-candidate reconciliation is not checking whether the completed lane was a lawful write-producing implementation finisher for the task class.
  - That lets a read-only diagnostic handoff prepare closure for a write-class bugfix task, which invalidates continuation, backlog truth, and operator trust.

## Goal
- What this change should achieve
  - Prevent read-only `analysis`, `coach`, or equivalent diagnostic lanes from satisfying `implementation_complete` or closure readiness for write-class tasks unless the run graph already reached a lawful terminal implementation stage.
  - Keep closure packets and completed run-graph truth aligned with actual write-capable execution evidence.
  - Preserve genuine completion for lawful terminal writer/verification/approval flows.
- What success looks like
  - An implementation run that only executed `analysis` remains in an open delegated state or fails closed with a specific blocker instead of reconciling to `implementation_complete`.
  - Downstream `closure` packets are not materialized from diagnostic-lane execution alone.
  - Read-side reconciliation, run-graph status, and recovery summaries agree on completion only when the lane/evidence class lawfully permits it.
- What is explicitly out of scope
  - Reopening the parked MemPalace lane.
  - Broad redesign of all delegated carrier classes or sandbox policy.
  - Replacing the separate coach-retry blocker; this fix is an upstream prerequisite for it.

## Requirements

### Functional Requirements
- Closure-candidate reconciliation for `task_class=implementation` must require a lawful terminal implementation state, not just any executed dispatch receipt with downstream `closure` readiness.
- Diagnostic lanes such as `analysis` must not produce `implementation_complete` unless the execution plan explicitly marks them as the terminal writer/approval outcome for the run.
- If a diagnostic lane executed successfully but no lawful writer completion evidence exists, the run graph must remain open or blocked with explicit continuation truth instead of projecting completion.
- Downstream closure packet preparation for write-class tasks must respect the same write-evidence/terminal-lane gate.

### Non-Functional Requirements
- Observability
  - Operator surfaces must clearly show why completion is denied when only diagnostic-lane evidence exists.
- Safety
  - No root-session write takeover, no hidden closure projection, and no silent conversion of read-only analysis success into write completion.
- Compatibility
  - Existing lawful completion paths for verification-approved or writer-terminal implementation runs must remain intact.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/analysis-lane-can-close-implementation-without-write-evidence-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
- Runtime families affected:
  - `taskflow`
  - `launcher`
  - `state_store`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow run-graph status <run-id> --json`
  - `vida orchestrator-init --json`
  - run-graph status reconciliation
  - downstream closure packet readiness

## Design Decisions

### 1. Closure-candidate reconciliation must honor terminal-lane law, not only downstream readiness
Will implement / choose:
- Restrict `implementation_complete` reconciliation so downstream `closure` readiness alone is insufficient for implementation tasks.
- Require the current or last completed lane/evidence class to match a lawful implementation terminal state.
- Why
  - The current summary path upgrades any executed receipt with closure-ready downstream truth into completion, even while `active_node=analysis`.
- Trade-offs
  - Some previously “completed” summaries will remain open/blocked until lawful terminal evidence exists.

### 2. Read-only diagnostic execution evidence is not write evidence
Will implement / choose:
- Treat successful execution of `analysis`/`coach`/review lanes as delegated progress only, not implementation completion, unless the execution plan explicitly says that lane is terminal for the task class.
- Why
  - A read-only diagnostic carrier can return receipt-backed execution evidence while still lacking write authority.
- Trade-offs
  - The runtime must reason about lane role and task class together instead of using a generic executed-receipt shortcut.

### 3. Regression proofs must cover both summary reconciliation and closure-packet readiness
Will implement / choose:
- Add focused tests for run-graph reconciliation from analysis-lane executed receipts with downstream `closure` readiness and assert that completion is denied.
- Preserve a positive control where lawful verification/approval completion still upgrades to `implementation_complete`.
- Why
  - The existing closure-ready regression in `state_store.rs` currently encodes the over-broad behavior.
- Trade-offs
  - Some existing assertions will need narrowing to match the stronger completion law.

## Technical Design

### Core Components
- Main components
  - `state_store_run_graph_summary.rs`
  - `taskflow_run_graph.rs`
  - `runtime_dispatch_state.rs`
  - `state_store.rs`
- Key interfaces
  - `reconcile_run_graph_status_with_dispatch_receipt(...)`
  - `run_graph_status(...)`
  - closure/downstream packet projection helpers
  - implementation handoff / completion derivation in `taskflow_run_graph.rs`
- Bounded responsibilities
  - classify whether an executed receipt is terminal for implementation
  - prevent closure-ready downstream projections from over-closing diagnostic runs
  - keep summary/recovery/operator truth aligned with lawful write-evidence gates

### Bounded File Set
- `docs/product/spec/analysis-lane-can-close-implementation-without-write-evidence-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `crates/vida/src/state_store_run_graph_summary.rs`
- `crates/vida/src/taskflow_run_graph.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/state_store.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - no projection of `implementation_complete` from read-only diagnostic-lane execution alone
  - no downstream closure packet readiness for write-class tasks without lawful terminal evidence
  - no root-session local write takeover as a workaround
- Required receipts / proofs / gates
  - completion must be backed by a lawful terminal lane/evidence class for the task class
  - diagnostic execution without writer completion must remain non-closure truth

## Implementation Plan

### Phase 1
- Register this blocker in spec maps and finalize the bounded design.
- First proof target
  - `vida docflow check --root . docs/product/spec/analysis-lane-can-close-implementation-without-write-evidence-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md`

### Phase 2
- Narrow reconciliation/closure-candidate logic so diagnostic-lane execution cannot over-close implementation runs.
- Second proof target
  - focused `cargo test -p vida` around run-graph reconciliation and closure-ready summary behavior

### Phase 3
- Re-run the live blocker path and confirm the analysis-only execution no longer generates `implementation_complete` plus a ready closure packet.
- Final proof target
  - targeted tests plus live `cargo run -p vida -- taskflow run-graph status feature-fix-bug-coach-retry-reuses-same-blocked-hermes-packet-without-fallback --json`

## Validation / Proof
- Unit tests:
  - analysis-only executed receipt with downstream `closure` readiness does not reconcile to `implementation_complete`
  - lawful verification/approval completion still reconciles to `implementation_complete`
  - downstream closure packet readiness for implementation tasks is denied when only diagnostic-lane evidence exists
- Runtime checks:
  - confirm `active_node=analysis` no longer appears together with `status=completed` / `lifecycle_stage=implementation_complete`
  - confirm downstream closure packet is absent or blocked until lawful write-capable terminal evidence exists

## References
- `docs/product/spec/coach-retry-reuses-same-blocked-hermes-packet-without-fallback-design.md`
- `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`
- `docs/product/spec/repair-explicit-continuation-bind-preservation-after-qwen-rebind-design.md`
- `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`

-----
artifact_path: product/spec/analysis-lane-can-close-implementation-without-write-evidence-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/analysis-lane-can-close-implementation-without-write-evidence-design.md
created_at: 2026-04-21T20:04:41.784245431Z
updated_at: 2026-04-21T20:06:36.289385795Z
changelog_ref: analysis-lane-can-close-implementation-without-write-evidence-design.changelog.jsonl
