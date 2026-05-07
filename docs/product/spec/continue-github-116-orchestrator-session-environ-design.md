# Continue GitHub #116 Consume Continue Run-Graph Drift Design Gate

Status: `approved`

## Summary
- Feature / change: tracked design-gate wrapper for the `github-116-consume-continue-bound-task-run-graph-drift` continuation / run-graph reconciliation slice.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow | docflow | status`
- Canonical design: `docs/product/spec/fix-continuation-reconciliation-resumed-executed-design.md`
- Status: `approved for work-pool and developer handoff`

## Current Context
- The active bounded request is: finalize design/spec evidence and prepare the developer handoff for continuation / run-graph resume reconciliation.
- The packet path for this lane still points at this wrapper document, so the wrapper must describe the active run-graph drift slice rather than the older orchestrator-session-identity wave.
- The underlying implementation design already exists in `docs/product/spec/fix-continuation-reconciliation-resumed-executed-design.md` and defines the bounded runtime fix.
- The remaining gap for this lane is design-gate truth and an explicit developer handoff package that the orchestrator can use to resume tracked work without spec/doc ambiguity.

## Design Gate Decision
- Use `docs/product/spec/fix-continuation-reconciliation-resumed-executed-design.md` as the canonical design for this bounded `#116` continuation slice.
- Treat this file as the tracked handoff wrapper expected by the active run `feature-github-116-consume-continue-bound-task-spec`.
- Keep scope limited to:
  - resumed `dispatch_status == executed` downstream preview refresh
  - stale spec-blocker reconciliation for `vida taskflow consume continue`
  - status / run-graph parity after refreshed receipt persistence
- Out of scope:
  - orchestrator session identity work
  - lane-order redesign
  - carrier/backend routing changes

## Developer Handoff
- Active bounded unit:
  - refresh already executed continuation receipts from current persisted task/doc evidence so stale spec blockers do not survive lawful spec completion
- Why this unit:
  - the active packet request is specifically the run-graph drift after specification completion, and the canonical runtime fix is already bounded in the resumed-executed reconciliation design
- Sequential vs parallel posture:
  - `sequential`
  - this slice mutates shared continuation / dispatch reconciliation logic and should land as one bounded implementation packet before adjacent continuation repairs

## Bounded Implementation Scope
- Primary code surfaces:
  - `crates/vida/src/taskflow_consume_resume.rs`
  - `crates/vida/src/runtime_dispatch_state.rs`
  - `crates/vida/src/state_store_run_graph_summary.rs` only if projection parity needs a narrow follow-up adjustment
- Canonical docs:
  - `docs/product/spec/fix-continuation-reconciliation-resumed-executed-design.md`
  - `docs/product/spec/continue-github-116-orchestrator-session-environ-design.md`

## Acceptance Targets
- `vida taskflow consume continue` refreshes downstream preview for an explicitly resumed receipt even when `dispatch_status == executed`.
- Closed/finalized specification evidence clears stale blockers such as:
  - `pending_specification_evidence`
  - `pending_design_finalize`
  - `pending_spec_task_close`
- Persisted receipt, run-graph projection, and status projection converge on the same refreshed downstream readiness.
- Incomplete evidence remains fail-closed; no blocker is cleared by heuristic override.

## Proof Evidence
- Canonical design proof:
  - `vida docflow check --root . docs/product/spec/fix-continuation-reconciliation-resumed-executed-design.md`
- Wrapper proof:
  - `vida docflow check --root . docs/product/spec/continue-github-116-orchestrator-session-environ-design.md`
- Developer proof targets for the next lane:
  - focused regression coverage for resumed executed receipts
  - `vida taskflow consume continue --json`
  - `vida status --json`

## Handoff Readiness
- This design gate is complete once this wrapper is docflow-valid and cites the canonical runtime-fix design above.
- The next lawful lane is the tracked developer/work-pool handoff for the bounded runtime reconciliation implementation.
- Do not reopen the older orchestrator-session-identity scope from this wrapper path unless a separate packet explicitly supersedes this slice.

-----
artifact_path: product/spec/continue-github-116-orchestrator-session-environ-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-05-07
schema_version: 1
status: canonical
source_path: docs/product/spec/continue-github-116-orchestrator-session-environ-design.md
created_at: 2026-05-07T00:00:00+03:00
updated_at: 2026-05-07T13:47:08.7484687Z
changelog_ref: continue-github-116-orchestrator-session-environ-design.changelog.jsonl
