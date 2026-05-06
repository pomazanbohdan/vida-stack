# GitHub #114 Post-Commit Runtime Diagnostics Design

Status: `approved`

## Summary
- Feature / change: deterministic post-commit VIDA runtime diagnostics for issue #114.
- Owner layer: `runtime-family`
- Runtime surface: `vida status`, `vida task next-lawful`, `vida taskflow recovery`, `vida taskflow consume continue`, `vida lane`, `vida agent-init`
- Status: approved for bounded runtime remediation and proof-driven follow-up work.

## Current Context
- The #114 umbrella tracks repeated post-commit cases where TaskFlow, DocFlow, run-graph recovery, continuation binding, lane exception takeover, and host dispatch evidence disagree after successful local work.
- The current active run is `github-114-post-commit-runtime-diagnostics`.
- Repairs already shipped in this wave fixed dispatch runtime assignment, exception-takeover recovery projection drift, stale next-lawful binding drift, dispatch packet path separator drift, consume-continue diagnostics/gating drift, and next-lawful `binding_source` output.
- After lane completion, the run graph is at `active_node=specification`, `lifecycle_stage=specification_complete`, with `pending_design_finalize` and `pending_spec_task_close` still blocking work-pool shaping.

## Goal
- Make operator surfaces deterministic enough that an orchestrator can identify the lawful continuation without guessing.
- Keep active run-graph truth, continuation binding, recovery advice, consume-continue failure payloads, root write guard, and GitHub evidence aligned.
- Preserve fail-closed behavior when recovery is not ready or a bounded unit has not been explicitly selected.
- Out of scope: closing the #114 umbrella issue, solving every #115/#116 design requirement, or bypassing delegated lane ownership.

## Requirements

### Functional Requirements
- `vida task next-lawful --json` must expose the authoritative binding source when one exists.
- `vida status --json` and recovery surfaces must not recommend a command that the runtime will reject for the current lifecycle state.
- Active exception takeover must remain path-scoped and must not imply broad root-local write authority.
- Downstream dispatch after specification must wait for finalized design evidence and a closed spec-pack task.

### Non-Functional Requirements
- Runtime outputs must use canonical release-1 operator fields and blocker codes.
- Windows path normalization must not weaken run-id, packet, or lane-status validation.
- GitHub comments must summarize proof and keep #114 open as an umbrella unless explicitly closed later.

## Ownership And Canonical Surfaces
- Project docs / specs affected: this design document, `docs/product/spec/README.md`.
- Framework protocols affected: root bootstrap and continuation-binding contracts through existing VIDA runtime surfaces only.
- Runtime families affected: TaskFlow run-graph, continuation binding, lane control, consume resume, status, and task surfaces.
- Config / receipts / runtime surfaces affected: `.vida/data/state/runtime-consumption/*`, lane exception metadata, TaskFlow notes/labels.

## Design Decisions

### 1. Prefer Reconciliation At Authoritative Runtime Boundaries
Will implement / choose:
- Reconcile stale continuation projections where the authoritative run graph or active lane receipt proves the current bounded unit.
- Keep command output additions close to the surface that owns the JSON contract.
- Reject heuristic continuation when status and recovery cannot agree on the active bounded unit.

### 2. Keep Exception Takeover Narrow
Will implement / choose:
- Use exception takeover only for a named runtime defect and a bounded file set.
- Return to normal posture with `vida lane complete` after tests, release build, install, commit, push, GitHub comment, and TaskFlow notes are complete.
- Record a new exception takeover for any next code repair rather than widening an old one.

### 3. Separate Spec-Pack Lifecycle From Runtime Code Fixes
Will implement / choose:
- Treat `pending_design_finalize` and `pending_spec_task_close` as lifecycle blockers for work-pool shaping, not as code-proof evidence by themselves.
- Close the spec-pack only after this design doc is finalized and checked.
- Continue implementation through tracked work-pool/dev packets after spec-pack closure.

## Technical Design

### Core Components
- `state_store_run_graph_summary.rs`: reconciles persisted run-graph dispatch and continuation evidence.
- `taskflow_consume_resume.rs`: validates consume-continue resume gates and JSON failure payloads.
- `task_surface.rs`: renders next-lawful continuation output.
- `taskflow_run_graph.rs` and `runtime_dispatch_state.rs`: own run-graph lifecycle, downstream dispatch readiness, and blocker projection.

### Data / State Model
- Continuation binding fields: `active_bounded_unit`, `binding_source`, `why_this_unit`, `primary_path`, `sequential_vs_parallel_posture`.
- Run-graph fields: `active_node`, `lifecycle_stage`, `recovery_ready`, `resume_target`, `downstream_dispatch_blockers`.
- Lane fields: `lane_status`, `exception_path_receipt_id`, `root_local_write_allowed_for_only_these_paths`.

### Integration Points
- GitHub issue #114 remains the upstream umbrella and receives proof comments for each bounded subclass.
- TaskFlow notes/labels record installed fingerprints and proof commands.
- DocFlow finalization/check gates this design document before spec-pack closure.

### Bounded File Set
- Already changed in this wave:
  - `crates/vida/src/state_store_run_graph_summary.rs`
  - `crates/vida/src/taskflow_consume_resume.rs`
  - `crates/vida/src/task_surface.rs`
- Candidate next runtime code areas:
  - `crates/vida/src/taskflow_run_graph.rs`
  - `crates/vida/src/runtime_dispatch_state.rs`
  - `crates/vida/src/state_store_run_graph_summary.rs`
  - `crates/vida/src/taskflow_consume_resume.rs`
- Spec lifecycle files:
  - `docs/product/spec/github-114-design-document-deterministic-post-design.md`
  - `docs/product/spec/README.md`

## Fail-Closed Constraints
- Do not continue implementation from `vida status --json` when it reports `continuation_binding_ambiguous`.
- Do not use explicit continuation bind when the run has not reached `closure_complete` and the bind surface rejects the lifecycle state.
- Do not stage generated runtime noise such as `vida/config/docflow-readiness.current.jsonl`.
- Do not close #114 until the umbrella issue is explicitly resolved.

## Implementation Plan

### Phase 1
- Close the current spec-pack lifecycle blockers by finalizing this design document and closing the spec-pack task.
- Proof target: `vida docflow check --root . docs/product/spec/github-114-design-document-deterministic-post-design.md`.

### Phase 2
- Ensure or create the work-pool packet from the runtime bootstrap command.
- Classify the next runtime defect from live surfaces before code writes.
- Proof target: `vida taskflow recovery status github-114-post-commit-runtime-diagnostics --json`.

### Phase 3
- For any next code repair, record a new bounded exception takeover or use lawful delegated execution.
- Run targeted tests, full `cargo test -p vida`, release build, install, commit, push, GitHub comment, and TaskFlow notes.

## Validation / Proof
- Unit tests: targeted tests for modified Rust surfaces.
- Integration tests: `cargo test -p vida` before release install.
- Runtime checks:
  - `vida task next-lawful --json`
  - `vida status --json`
  - `vida taskflow recovery status github-114-post-commit-runtime-diagnostics --json`
  - `vida taskflow consume continue --run-id github-114-post-commit-runtime-diagnostics --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/github-114-design-document-deterministic-post-design.md`
  - `git diff --check`

## Observability
- GitHub issue comments record commit hash, proof commands, installed fingerprint, and live verification.
- TaskFlow notes/labels record each bounded subclass fixed and the current installed runtime fingerprint.
- Runtime receipts under `.vida/data/state/runtime-consumption/` remain the evidence source for lane and run-graph state.

## Rollout Strategy
- Land changes in small commits on `main`.
- Install the release binary after runtime-code fixes.
- Keep #114 open while processing remaining subclasses.

-----
artifact_path: product/spec/github-114-design-document-deterministic-post-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-05-06
schema_version: 1
status: canonical
source_path: docs/product/spec/github-114-design-document-deterministic-post-design.md
created_at: 2026-05-06T17:05:31.1882276Z
updated_at: 2026-05-06T17:06:46.281931Z
changelog_ref: github-114-design-document-deterministic-post-design.changelog.jsonl
