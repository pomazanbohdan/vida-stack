# Serialize authoritative state access lock mitigation design

Purpose: Bound the current-release design for authoritative state access serialization and snapshot-first read-surface mitigation of SurrealKV lock contention.

Status: `proposed`

Use this template for one bounded feature/change design before implementation.

## Summary
- Feature / change: serialize authoritative state access and move read-oriented task/runtime surfaces toward snapshot-first or lock-tolerant behavior so operator reads stop contending aggressively on `.vida/data/state`.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher | status`
- Status: `proposed`

## Current Context
- Existing system overview
  - Canonical `vida task list/show/ready` surfaces still open the authoritative state store directly.
  - `StateStore::open_existing(...)` and `open_existing_read_only(...)` both use the embedded SurrealKV path and bounded retry loops.
  - Some read helpers already contain a degraded snapshot path, but the fallback is inconsistent across surfaces.
- Key components and relationships
  - `crates/vida/src/state_store_open.rs` owns authoritative open behavior and retry posture.
  - `crates/vida/src/task_surface.rs` owns root `vida task` list/show/ready behavior.
  - `crates/vida/src/state_store_task_store.rs` owns task snapshot export.
  - `crates/vida/src/init_surfaces.rs` owns `vida orchestrator-init`, which currently competes for the same store.
  - `crates/vida/src/taskflow_proxy.rs` already demonstrates one lock-tolerant degraded-read pattern.
- Current pain point or gap
  - Read-oriented operator surfaces still contend on embedded SurrealKV locks often enough to block ordinary backlog inspection.
  - “Read-only” behavior is not actually read-only in all paths because some helpers still refresh snapshots on the way through.
  - Serialization is currently procedural discipline, not an explicit runtime guard or architectural contract.

## Goal
- What this change should achieve
  - Prevent overlapping local authoritative-store opens where one bounded process can serialize access up front instead of colliding at SurrealKV lock time.
  - Prefer snapshot-first reads for operator/reporting surfaces that do not require fresh mutation-capable state.
  - Keep fail-closed truth: do not fake success when both live store and snapshot evidence are unavailable.
- What success looks like
  - `vida task list/show/ready` stop failing under common lock contention caused by another local `vida` process.
  - Snapshot reads remain truthful about freshness/degraded posture.
  - Mutation-capable surfaces still use authoritative opens, but their access is bounded and serialized.
- What is explicitly out of scope
  - Replacing SurrealKV.
  - Reworking MemPalace or any parked lane.
  - Adding destructive lock-file cleanup or heuristic force-unlock behavior.

## Requirements

### Functional Requirements
- Must introduce one explicit local serialization guard around authoritative store access for `vida` runtime surfaces that need live state.
- Must move read-oriented operator surfaces toward snapshot-first or lock-tolerant degraded reads instead of unconditional live opens.
- Must keep operator-visible indication when a response is served from snapshot/degraded evidence rather than a fresh authoritative open.
- Must preserve authoritative writes and fail closed if a write-capable surface cannot obtain the live store legally.

### Non-Functional Requirements
- Performance
  - Snapshot-first reads should reduce lock retries and lower operator latency during concurrent local activity.
- Scalability
  - The guard must help across root `vida task` and adjacent runtime status/init surfaces, not only one command.
- Observability
  - Runtime outputs should expose when a surface degraded to snapshot evidence.
- Security
  - No silent unlock, no file deletion, and no mutation under a read-only/degraded path.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/serialize-authoritative-state-access-lock-mitigation-design.md`
  - `docs/product/spec/current-spec-map.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
  - `status`
  - `launcher`
- Config / receipts / runtime surfaces affected:
  - `vida task list`
  - `vida task show`
  - `vida task ready`
  - `vida orchestrator-init --json`

## Design Decisions

### 1. Serialization guard comes before wider retry budgets
Will implement / choose:
- Add one bounded local serialization layer for authoritative store opens instead of primarily extending SurrealKV retry windows.
- Why
  - The core problem is overlapping local opens, not only short retry timing.
- Trade-offs
  - Some commands may wait briefly on a project-local guard before even attempting the embedded backend open.
- Alternatives considered
  - Only increase retry counts and timeout windows.
  - Rejected because it leaves operator contention noisy and does not reduce collision frequency.

### 2. Read surfaces prefer snapshot truth over live-lock failure
Will implement / choose:
- Make read-oriented task/operator surfaces prefer existing snapshot evidence or a lock-tolerant degraded path before failing hard on live-store contention.
- Why
  - For inspection/reporting surfaces, slightly stale but explicit evidence is better than a hard failure.
- Trade-offs
  - Some outputs may be degraded/stale and must say so.
- Alternatives considered
  - Keep all reads authoritative-only.
  - Rejected because the embedded backend and current launcher topology make that posture too fragile.

## Technical Design

### Core Components
- Main components
  - `state_store_open.rs`
  - `task_surface.rs`
  - `state_store_task_store.rs`
  - `taskflow_proxy.rs`
  - `init_surfaces.rs`
- Key interfaces
  - `StateStore::open_existing(...)`
  - `StateStore::open_existing_read_only(...)`
  - `ready_tasks_scoped_read_only(...)`
  - task list/show/ready render paths
- Bounded responsibilities
  - state-store open logic enforces local serialization before embedded-store open
  - read surfaces prefer snapshot/degraded evidence
  - authoritative mutation surfaces remain live-store only

### Data / State Model
- Important entities
  - authoritative store access guard
  - tasks snapshot export
  - degraded-read posture
- Receipts / runtime state / config fields
  - snapshot freshness metadata if already available
  - operator-visible degraded/fallback flags where needed
- Migration or compatibility notes
  - additive behavior only; no schema migration required

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - launcher/status/task surfaces may read from snapshot-first paths
  - mutation surfaces still open the authoritative store under guard
- Cross-document / cross-protocol dependencies
  - `docs/product/spec/authoritative-state-lock-recovery-design.md`
  - `docs/product/spec/taskflow-task-command-parity-and-proxy-alignment-design.md`
  - `docs/product/spec/reconciled-runtime-projection-output-design.md`

### Bounded File Set
- `docs/product/spec/serialize-authoritative-state-access-lock-mitigation-design.md`
- `docs/product/spec/current-spec-map.md`
- `crates/vida/src/state_store_open.rs`
- `crates/vida/src/task_surface.rs`
- `crates/vida/src/state_store_task_store.rs`
- `crates/vida/src/taskflow_proxy.rs`
- `crates/vida/src/init_surfaces.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - no lock-file deletion
  - no pretending snapshot data is fresh authoritative state
  - no heuristic continuation into adjacent lanes
- Required receipts / proofs / gates
  - runtime must surface degraded posture explicitly when snapshot-first logic is used
  - write-capable surfaces must still block if authoritative state cannot be opened lawfully
- Safety boundaries that must remain true during rollout
  - authoritative mutation truth must remain unchanged
  - snapshot fallback must never perform hidden writes

## Implementation Plan

### Phase 1
- Register this design and make the current read/open architecture explicit.
- First proof target
  - `vida docflow check --root . docs/product/spec/serialize-authoritative-state-access-lock-mitigation-design.md docs/product/spec/current-spec-map.md`

### Phase 2
- Implement a bounded local serialization guard for authoritative opens.
- Refactor read-oriented task/operator surfaces to prefer snapshot-first or explicit degraded behavior.
- Second proof target
  - targeted `cargo test -p vida` for task surfaces and lock/degraded-read behavior

### Phase 3
- Extend adjacent launcher/status surfaces only where they still contend on the same pattern.
- Rebuild and refresh the local `vida` binary.
- Final proof target
  - targeted cargo tests plus `cargo build --release -p vida`

## Validation / Proof
- Unit tests:
  - new tests for serialized authoritative open behavior
  - new tests for snapshot/degraded read behavior on lock contention
- Integration tests:
  - targeted `vida task ...` command proofs under simulated contention
- Runtime checks:
  - `vida orchestrator-init --json`
  - `vida task list --json --status in_progress`
  - `vida task show feature-current-release-delivery-efficiency-hardening --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/serialize-authoritative-state-access-lock-mitigation-design.md docs/product/spec/current-spec-map.md`

## Observability
- Logging points
  - explicit log/debug markers when a surface degrades to snapshot evidence
- Metrics / counters
  - none required initially
- Receipts / runtime state written
  - existing task snapshot export
  - optional degraded-read flags in command output if additive-only

## Rollout Strategy
- Development rollout
  - land design + runtime changes in one bounded task slice
- Migration / compatibility notes
  - keep snapshot format backward-compatible
- Operator or user restart / restart-notice requirements
  - rebuild and reinstall `vida` after proof passes

## Future Considerations
- Follow-up ideas
  - isolate more status/init reads from authoritative-store opens entirely
  - add richer freshness metadata to snapshot-backed task responses
- Known limitations
  - one local serialization guard does not solve contention from truly external processes opening the same store outside VIDA control
- Technical debt left intentionally
  - full remote/server-backed state topology remains a separate architectural track

## References
- `docs/product/spec/authoritative-state-lock-recovery-design.md`
- `docs/product/spec/taskflow-task-command-parity-and-proxy-alignment-design.md`
- `docs/product/spec/reconciled-runtime-projection-output-design.md`
- SurrealDB embedded guidance: use embedded/file-backed mode for single-application local storage and remote/server mode when multiple applications need shared access

-----
artifact_path: product/spec/serialize-authoritative-state-access-lock-mitigation-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/serialize-authoritative-state-access-lock-mitigation-design.md
created_at: 2026-04-21T10:32:49.194766213Z
updated_at: 2026-04-21T10:34:55.137566929Z
changelog_ref: serialize-authoritative-state-access-lock-mitigation-design.changelog.jsonl
