# Authoritative State Lock Recovery Design

Purpose: Define bounded stale-lock and hung-holder recovery for long-lived authoritative state roots.

Status: proposed

Use this template for one bounded feature/change design before implementation.

## Summary
- Feature / change: shorten authoritative state-store lock lifetime around long-running TaskFlow dispatch execution, surface lock-specific remediation hints, and keep long-lived-state recovery explicit rather than heuristic.
- Owner layer: `mixed`
- Runtime surface: `taskflow | status | doctor`
- Status: proposed

## Current Context
- Existing system overview
  - `vida taskflow consume final` and `vida taskflow consume continue` open the authoritative store early and currently keep that live `StateStore` handle across runtime dispatch execution.
  - Agent-lane dispatch for internal and external backends can block on subprocess completion.
  - `status`, `doctor`, and many other surfaces rely on `StateStore::open_existing(...)` and fail closed when the datastore lock is unavailable.
- Key components and relationships
  - `crates/vida/src/taskflow_consume.rs` and `crates/vida/src/taskflow_consume_resume.rs` shape dispatch receipts and trigger runtime handoff.
  - `crates/vida/src/runtime_dispatch_state.rs` owns dispatch execution, downstream preview refresh, and receipt/result persistence.
  - `crates/vida/src/runtime_dispatch_execution.rs` runs backend subprocesses and already classifies timeout / activation-only internal dispatch correctly.
  - `crates/vida/src/state_store_open.rs` owns bounded lock retry for `open` / `open_existing`.
  - `crates/vida/src/state_store.rs` owns `StateStoreError` rendering and recovery hints.
- Current pain point or gap
  - Even after the internal-Codex timeout/blocked-receipt fix, `consume final` / `consume continue` still keep the authoritative datastore lock for the full duration of long-running agent-lane dispatch waits because the live store handle remains open.
  - That makes unrelated operator surfaces fail closed on `LOCK is already locked` even when the runtime is only waiting on external work rather than actively mutating state.
  - The current lock-path hint covers missing/broken state layouts but does not classify prolonged lock contention as a first-class remediation case.
  - There is no bounded regression that proves authoritative state can be reopened while a dispatch subprocess is still in flight.

## Goal
- What this change should achieve
  - Release the authoritative datastore lock before long-running agent-lane subprocess waits begin.
  - Reopen the authoritative store only for bounded read/write phases before and after dispatch execution.
  - Surface clearer remediation text when lock contention persists beyond the bounded retry window.
- What success looks like
  - `consume final` and `consume continue` no longer monopolize `.vida/data/state/LOCK` for the whole agent-lane execution window.
  - A concurrent read-only open such as `StateStore::open_existing(...)` can succeed while the dispatch subprocess is still running.
  - Persisted dispatch results and receipts still remain truthful and fail closed.
  - Operator-facing errors distinguish lock contention from broken/missing state layout and point to explicit recovery behavior.
- What is explicitly out of scope
  - Silent auto-deletion of lock files from the long-lived default state root.
  - Redesign of continuation binding, backlog semantics, or lane ownership law.
  - Replacing the SurrealKV backend.

## Requirements

### Functional Requirements
- Must stop holding an open authoritative `StateStore` handle across long-running agent-lane subprocess waits.
- Must preserve current `taskflow_pack` behavior for fast local packet materialization and closure-preview routing.
- Must keep blocked-receipt persistence for timeout / activation-only internal dispatch intact.
- Must preserve downstream dispatch preview refresh and receipt persistence semantics after dispatch completion.
- Must extend lock-path remediation hints so prolonged lock contention reports explicit next actions instead of only generic backend text.

### Non-Functional Requirements
- Performance
  - Additional reopen operations must stay bounded and only occur at clear phase boundaries.
- Scalability
  - The same lock-lifetime reduction must apply to both `consume final` and `consume continue`, including downstream dispatch chains.
- Observability
  - Tests must prove that the authoritative store becomes reopenable while a dispatch subprocess is still running.
- Security
  - The fix must not silently self-heal broken long-lived state by deleting lock files or mutating backing-store files without an explicit operator action.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/authoritative-state-lock-recovery-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
  - `docs/product/spec/README.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
  - `status`
  - `doctor`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow consume final`
  - `vida taskflow consume continue`
  - `vida status --json`
  - `vida doctor --json`
  - runtime dispatch result artifacts
  - run-graph dispatch receipts

## Design Decisions

### 1. Fix lock contention by shortening store lifetime, not by widening retries
Will implement / choose:
- Restructure dispatch execution so agent-lane subprocess waits happen with no live authoritative `StateStore` handle held by the current process.
- Why
  - The dominant issue is lock ownership duration, not missing retry loops.
- Trade-offs
  - Requires reopening the store at explicit phase boundaries before preview/receipt updates.
- Alternatives considered
  - Add more lock retries only.
  - Rejected because retries do not help while the same process is still holding the lock for the full dispatch wait.
- ADR link if this must become a durable decision record
  - none

### 2. Long-lived state recovery remains explicit and fail-closed
Will implement / choose:
- Improve lock-specific remediation hints, but do not silently delete lock files or auto-repair long-lived backing-store state.
- Why
  - Product/process law already treats long-lived state as durable DB-first authority and requires explicit reset/reinit for broken roots.
- Trade-offs
  - A truly orphaned lock may still require an explicit operator recovery step later.
- Alternatives considered
  - Auto-remove `LOCK` when retry expires.
  - Rejected because the runtime cannot prove safely that the lock is orphaned in every case.
- ADR link if needed
  - none

## Technical Design

### Core Components
- Main components
  - `taskflow_consume.rs`
  - `taskflow_consume_resume.rs`
  - `runtime_dispatch_state.rs`
  - `runtime_dispatch_execution.rs`
  - `state_store.rs`
  - `state_store_open.rs`
- Key interfaces
  - `execute_and_record_dispatch_receipt(...)`
  - `execute_downstream_dispatch_chain(...)`
  - `StateStore::open_existing(...)`
  - `StateStoreError` display / recovery-hint rendering
- Bounded responsibilities
  - consume surfaces shape receipts and packet paths, then drop the live store before agent-lane dispatch waits
  - runtime dispatch helpers perform agent-lane handoff without requiring a live store lock during subprocess wait
  - post-dispatch persistence reopens the store for bounded preview and receipt updates
  - state-store error rendering explains lock remediation separately from broken-state remediation

### Data / State Model
- Important entities
  - authoritative state root
  - runtime dispatch receipt
  - runtime dispatch result artifact
  - downstream dispatch preview
- Receipts / runtime state / config fields
  - `dispatch_result_path`
  - `dispatch_status`
  - `lane_status`
  - `downstream_dispatch_*`
  - lock-related `StateStoreError` display text
- Migration or compatibility notes
  - No state schema migration is required.
  - Historical receipts remain readable.

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - consume surfaces shape packet/receipt state first
  - dispatch execution runs without a live authoritative lock during long agent-lane waits
  - post-dispatch refresh reopens the store for preview and receipt persistence
- Cross-document / cross-protocol dependencies
  - `docs/product/spec/ops-state-and-runtime-evidence-hygiene-design.md`
  - `docs/process/project-operations.md`
  - `docs/product/spec/internal-codex-agent-execution-fail-closed-design.md`

### Bounded File Set
- `docs/product/spec/authoritative-state-lock-recovery-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `docs/product/spec/README.md`
- `crates/vida/src/state_store.rs`
- `crates/vida/src/state_store_open.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/runtime_dispatch_execution.rs`
- `crates/vida/src/taskflow_consume.rs`
- `crates/vida/src/taskflow_consume_resume.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - No silent lock-file deletion from the long-lived default state root.
  - No heuristic local root-session coding because dispatch execution is waiting.
- Required receipts / proofs / gates
  - Timeout / activation-only internal dispatch must still persist blocked results and receipts.
  - Broken long-lived state must remain visibly blocked on operator surfaces.
- Safety boundaries that must remain true during rollout
  - Dispatch result / receipt truthfulness must not regress.
  - Downstream preview refresh semantics must remain explicit and bounded.

## Implementation Plan

### Phase 1
- Register this design and validate the document surfaces.
- First proof target
  - `vida docflow check --root . docs/product/spec/authoritative-state-lock-recovery-design.md`

### Phase 2
- Refactor agent-lane dispatch execution so the authoritative store is not held open across subprocess waits.
- Extend lock-specific recovery hints in `StateStoreError`.
- Second proof target
  - targeted `cargo test -p vida` for runtime dispatch and lock-path behavior

### Phase 3
- Add a regression proving the store can reopen while a dispatch subprocess is in flight.
- Rebuild the release binary and refresh the installed `vida`.
- Final proof target
  - targeted cargo tests + `cargo build --release -p vida`

## Validation / Proof
- Unit tests:
  - existing internal timeout / blocked-receipt tests remain green
  - new regression proving reopen during in-flight agent-lane dispatch
- Integration tests:
  - bounded consume surface proof for lock-lifetime reduction where justified
- Runtime checks:
  - `vida status --json`
  - `vida doctor --json`
  - `vida taskflow consume continue --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/authoritative-state-lock-recovery-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md docs/product/spec/README.md`

## Observability
- Logging points
  - none new required beyond clearer lock-path error text
- Metrics / counters
  - none new required
- Receipts / runtime state written
  - existing dispatch result artifacts
  - existing run-graph dispatch receipts

## Rollout Strategy
- Development rollout
  - land doc registration and runtime refactor in one bounded slice
- Migration / compatibility notes
  - no schema migration
- Operator or user restart / restart-notice requirements
  - rebuild and reinstall the local `vida` binary after proof passes

## Future Considerations
- Follow-up ideas
  - add one explicit operator-facing reset/reinit surface for broken long-lived state roots
  - add richer structured lock diagnostics if backend ownership metadata becomes available
- Known limitations
  - a truly orphaned external backend lock may still require explicit operator recovery outside this bounded slice
- Technical debt left intentionally
  - long-lived-state reset/reinit remains a future dedicated surface rather than part of this dispatch-focused refactor

## References
- `docs/product/spec/ops-state-and-runtime-evidence-hygiene-design.md`
- `docs/product/spec/internal-codex-agent-execution-fail-closed-design.md`
- `docs/process/project-operations.md`
- `docs/process/environments.md`

-----
artifact_path: product/spec/authoritative-state-lock-recovery-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-13
schema_version: 1
status: canonical
source_path: docs/product/spec/authoritative-state-lock-recovery-design.md
created_at: 2026-04-13T16:08:31.740873424Z
updated_at: 2026-04-13T16:12:52.793908871Z
changelog_ref: authoritative-state-lock-recovery-design.changelog.jsonl
