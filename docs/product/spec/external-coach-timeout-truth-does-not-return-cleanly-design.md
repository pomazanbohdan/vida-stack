# External coach timeout truth and return blocker design

Purpose: Bound the current-release fix for external coach-lane timeout handling so blocked timeout artifacts stay truthful to the selected external backend and `taskflow consume continue` returns control promptly after timeout normalization.

Status: `proposed`

## Summary
- Feature / change: when coach dispatch runs through an external review backend such as `hermes_cli`, timeout classification and process-return behavior must remain external and must not strand the parent orchestrator process after blocked truth is already persisted.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher`
- Status: `proposed`

## Current Context
- Existing system overview
  - `execute_external_agent_lane_dispatch(...)` runs the configured external provider through `execute_wrapped_command_async(...)`.
  - `execute_wrapped_command(...)` applies timeout/kill-after-grace logic and returns `ObservedCommandOutput`.
  - `apply_dispatch_handoff_timeout_to_receipt(...)` chooses between generic timeout truth and `internal_activation_view_only` by calling `dispatch_handoff_uses_internal_host(...)`.
- Key components and relationships
  - `crates/vida/src/runtime_dispatch_execution.rs` owns wrapped-command timeout/kill behavior for external dispatch.
  - `crates/vida/src/runtime_dispatch_state.rs` owns timeout result classification and blocked receipt/result persistence.
  - `taskflow recovery latest` and `run-graph status` project the resulting blocked truth back into runtime state.
- Current pain point or gap
  - After the coach-lane backend-selection repair, live runtime proof for `feature-serialize-authoritative-state-access-lock-mitigation` now records `selected_backend = hermes_cli`.
  - The blocked timeout artifact still says `blocker_code = internal_activation_view_only` and `internal host carrier timed out`, which no longer matches the actual external backend path.
  - Process inspection showed the parent `target/debug/vida taskflow consume continue ...` process and its `hermes chat` child still alive after the blocked timeout artifact was already written.

## Goal
- What this change should achieve
  - External coach timeout artifacts must use generic external timeout truth instead of internal activation-view-only wording.
  - The wrapped external dispatch command must return bounded output to the orchestrator after timeout + kill-after grace, even when subprocess cleanup is not perfectly clean.
  - Preserve the just-repaired external coach backend selection.
- What success looks like
  - A timeout on `hermes_cli` coach dispatch yields `blocker_code = timeout_without_takeover_authority` or equivalent generic timeout truth, not `internal_activation_view_only`.
  - `taskflow consume continue` returns to the shell once the timeout result is persisted instead of remaining alive indefinitely.
  - `run-graph status` still shows `selected_backend = hermes_cli`.
- What is explicitly out of scope
  - Completing the serialization implementation slice itself.
  - Redesigning all provider process supervision.
  - Changing external backend admission policy beyond timeout truth/return semantics.

## Requirements

### Functional Requirements
- External coach timeout classification must not use internal-host-only blocker vocabulary.
- Wrapped external provider commands must return bounded timeout output after timeout + kill-after grace even if process-group cleanup or reader-thread completion lags.
- Selected backend truth in dispatch receipts and run-graph status must remain the actual external backend.
- Internal-host timeout behavior must remain internal-specific and must not regress.

### Non-Functional Requirements
- Performance
  - Timeout return must remain bounded and should not introduce long post-timeout waits.
- Scalability
  - The fix should apply to external backend dispatch generally, not only one `hermes_cli` run id.
- Observability
  - Persisted artifacts must distinguish external timeout truth from internal activation-view-only truth.
- Safety
  - No success result may be inferred when completion evidence is missing.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/external-coach-timeout-truth-does-not-return-cleanly-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/internal-dispatch-timeout-does-not-return-design.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
  - `launcher`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow consume continue`
  - `vida taskflow recovery latest`
  - `vida taskflow run-graph status`
  - runtime dispatch result artifacts
  - run-graph dispatch receipts

## Design Decisions

### 1. Timeout classification must follow effective backend class, not host execution class alone
Will implement / choose:
- Internal timeout truth will only be used when the resolved lane dispatch/backend class is actually internal.
- Why
  - Once coach selects `hermes_cli`, calling that timeout an internal activation-view-only blocker is factually wrong.
- Trade-offs
  - Internal-host detection becomes slightly narrower and more dependent on resolved lane dispatch metadata.
- Alternatives considered
  - Keep current host-execution-class heuristic.
  - Rejected because it misclassifies the repaired external coach path.

### 2. Wrapped-command timeout must return after kill-after grace without waiting forever for full cleanup
Will implement / choose:
- After timeout and final kill signal, the wrapper will return a bounded timed-out output with partial stdout/stderr if the child/readers still have not completed.
- Why
  - The orchestrator needs prompt control return more than perfect drain behavior from a misbehaving external subprocess tree.
- Trade-offs
  - Exit status and captured output may be synthetic/partial in timeout cases.
- Alternatives considered
  - Continue waiting until `try_wait()` and both readers complete.
  - Rejected because the live proof shows this can leave the parent process alive after blocked truth is already known.

## Technical Design

### Core Components
- Main components
  - `runtime_dispatch_state.rs`
  - `runtime_dispatch_execution.rs`
- Key interfaces
  - `dispatch_handoff_uses_internal_host(...)`
  - `apply_dispatch_handoff_timeout_to_receipt(...)`
  - `execute_wrapped_command(...)`
  - `execute_external_agent_lane_dispatch(...)`
- Bounded responsibilities
  - classify timeout truth by actual resolved backend class
  - return bounded timeout output after kill-after grace
  - preserve external selected-backend truth in receipts and status projections

### Data / State Model
- Important entities
  - wrapped command output
  - lane dispatch backend class
  - blocked dispatch result artifact
  - run-graph dispatch receipt
- Receipts / runtime state / config fields
  - `selected_backend`
  - `blocker_code`
  - `provider_error`
  - `timeout_wrapper.timed_out`
  - `backend_dispatch.backend_class`
- Migration or compatibility notes
  - additive behavior only; no schema migration

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - coach dispatch execution -> timeout normalization -> recovery/run-graph projection
- Cross-document / cross-protocol dependencies
  - `docs/product/spec/internal-dispatch-timeout-does-not-return-design.md`
  - `docs/product/spec/coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md`
  - `docs/product/spec/serialize-authoritative-state-access-lock-mitigation-design.md`

### Bounded File Set
- `docs/product/spec/external-coach-timeout-truth-does-not-return-cleanly-design.md`
- `docs/product/spec/current-spec-map.md`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/runtime_dispatch_execution.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - no internal-only blocker vocabulary for external selected backends
  - no success state without receipt-backed completion evidence
  - no indefinite post-timeout waiting in the wrapped-command path
- Required receipts / proofs / gates
  - external timeout must persist generic blocked timeout truth
  - parent `consume continue` must return after timeout normalization
- Safety boundaries that must remain true during rollout
  - internal-host timeout semantics remain internal-specific
  - selected backend stays `hermes_cli` for the repaired coach route

## Implementation Plan

### Phase 1
- Register the bounded blocker design and active spec map entry.
- First proof target
  - `vida docflow check --root . docs/product/spec/external-coach-timeout-truth-does-not-return-cleanly-design.md docs/product/spec/current-spec-map.md`

### Phase 2
- Narrow timeout classification to actual internal backend paths and bound wrapped-command return after kill-after grace.
- Second proof target
  - targeted `cargo test -p vida` for timeout classification and wrapped-command timeout return

### Phase 3
- Re-run the serialization coach path and confirm blocked truth is external/generic and the parent process no longer stays alive after timeout.
- Final proof target
  - targeted tests plus live `cargo run -p vida -- taskflow consume continue --run-id feature-serialize-authoritative-state-access-lock-mitigation --json`

## Validation / Proof
- Unit tests:
  - external coach timeout uses generic timeout blocker truth
  - internal host timeout keeps `internal_activation_view_only`
  - wrapped-command timeout returns even when child/readers lag after kill-after grace
- Integration tests:
  - external coach dispatch timeout result remains aligned with selected backend truth
- Runtime checks:
  - `cargo run -p vida -- taskflow recovery latest --json`
  - `cargo run -p vida -- taskflow run-graph status feature-serialize-authoritative-state-access-lock-mitigation --json`
  - `cargo run -p vida -- taskflow consume continue --run-id feature-serialize-authoritative-state-access-lock-mitigation --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/external-coach-timeout-truth-does-not-return-cleanly-design.md docs/product/spec/current-spec-map.md`

## Observability
- Logging points
  - wrapped-command timeout branch after kill-after grace
  - timeout classification branch in dispatch-state reconciliation
- Metrics / counters
  - none required initially
- Receipts / runtime state written
  - runtime dispatch result artifact
  - run-graph dispatch receipt
  - lane execution receipt artifact

## Rollout Strategy
- Development rollout
  - land as the next blocker fix before returning to the serialization implementation slice
- Migration / compatibility notes
  - no schema migration
- Operator or user restart / restart-notice requirements
  - rebuild and refresh the local `vida` binary after proofs pass

## Future Considerations
- Follow-up ideas
  - unify timeout classification around actual executed backend dispatch metadata across all lanes
- Known limitations
  - this slice focuses on timeout truth and parent return, not full provider tree supervision redesign
- Technical debt left intentionally
  - richer process-tree cleanup and receipt-backed completion bridging for external backends can remain a later hardening slice

-----
artifact_path: product/spec/external-coach-timeout-truth-does-not-return-cleanly-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/external-coach-timeout-truth-does-not-return-cleanly-design.md
created_at: 2026-04-21T12:41:56.384392174Z
updated_at: 2026-04-21T12:43:35.128720353Z
changelog_ref: external-coach-timeout-truth-does-not-return-cleanly-design.changelog.jsonl
