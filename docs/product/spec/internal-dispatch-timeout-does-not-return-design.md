# Internal delegated dispatch timeout handoff blocker design

Purpose: Bound the audit-wave fix for internal delegated implementer handoff that reaches a lawful `agent-init` implementer packet but still outlives the bounded timeout window instead of settling promptly into receipt-backed execution or canonical blocked timeout truth.

Status: `approved`

## Summary
- Feature / change: restore bounded fail-closed return semantics when internal delegated dispatch starts an implementer handoff but no receipt-backed completion evidence arrives before the configured timeout window.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher`
- Status: `approved`

## Current Context
- Existing system overview
  - `execute_and_record_dispatch_receipt(...)` writes an in-flight dispatch-result artifact with `execution_state = "executing"` before awaiting the live delegated handoff.
  - For internal host dispatch, outer `tokio::time::timeout(...)` is intentionally disabled and runtime relies on the inner wrapper in `execute_internal_agent_lane_dispatch(...)`.
  - The inner wrapper delegates to `execute_wrapped_command_async(...)` and then to `execute_wrapped_command(...)`.
- Key components and relationships
  - `crates/vida/src/runtime_dispatch_execution.rs` owns the internal command wrapper and timeout behavior.
  - `crates/vida/src/runtime_dispatch_state.rs` owns handoff timeout policy, in-flight receipt recording, timeout normalization, and run-graph dispatch receipt reconciliation.
  - `crates/vida/src/taskflow_consume_resume.rs` and recovery surfaces project the run as `awaiting_implementer` / `implementation_dispatch_ready`.
  - `docs/product/spec/internal-codex-agent-execution-fail-closed-design.md` already governs truthful fail-closed semantics for internal Codex execution.
- Current pain point or gap
  - Live proof for `feature-repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen` now shows routing is repaired: `cargo run -p vida -- taskflow run-graph dispatch-init feature-reconcile-autonomous-execution-flag-runtime-drift --json` records a lawful implementer packet with `dispatch_target = implementer`, `handoff_runtime_role = worker`, `activation_agent_type = junior`, and `selected_backend = internal_subagents`.
  - Executing that packet via `cargo run -p vida -- agent-init --dispatch-packet ... --execute-dispatch --json` still records an in-flight artifact with `status = "pass"` and `execution_state = "executing"` plus `activation_view_only`, then only later returns `Timed out executing runtime dispatch handoff after 242s`.
  - Recovery truth eventually settles to `dispatch_status = blocked`, `lane_status = lane_blocked`, and `blocker_code = internal_activation_view_only`, but the launcher path still holds the operator call open until the full timeout window instead of returning more promptly once terminal blocked truth is known.

## Goal
- What this change should achieve
  - Guarantee prompt bounded return for internal delegated dispatch when terminal completion evidence does not arrive.
  - Preserve truthful blocked timeout state in receipts and runtime artifacts instead of leaving the orchestrator process hanging.
  - Keep routing/implementer selection unchanged; this slice is only about timeout-return correctness after lawful handoff start.
- What success looks like
  - Internal implementer handoff through both `vida agent-init --execute-dispatch` and the resumed TaskFlow path returns within the bounded timeout window plus small grace.
  - Persisted dispatch receipt/result move to blocked timeout truth instead of staying indefinitely in `executing` / `lane_running`.
  - The qwen carrier-drift remediation slice can resume past this blocker without regressing back into specification routing.
- What is explicitly out of scope
  - Implementing the serialization lock-mitigation code itself.
  - Replacing the internal Codex backend or redesigning carrier topology.
  - Broad state-store lock mitigation outside this dispatch timeout family.

## Requirements

### Functional Requirements
- Must return control from `vida agent-init --dispatch-packet ... --execute-dispatch --json` promptly when an internal delegated handoff exceeds its bounded timeout.
- Must return control from `vida taskflow consume continue` promptly when an internal delegated handoff exceeds its bounded timeout.
- Must persist blocked timeout truth for the in-flight dispatch receipt and runtime dispatch result.
- Must not leave the run indefinitely in `execution_state = "executing"` when no terminal completion evidence exists.
- Must preserve successful internal dispatch behavior when a terminal `agent_message` does arrive in time.
- Must preserve the corrected worker/dev-pack/implementer routing for existing design-backed implementation tasks.

### Non-Functional Requirements
- Performance
  - Timeout enforcement must remain bounded and should not add heavy polling overhead.
- Scalability
  - The fix should apply to the general internal-host dispatch wrapper, not only one serialization run id.
- Observability
  - Operator artifacts must continue to distinguish `executing`, `blocked`, and `executed` states truthfully.
- Safety
  - No root-session local write fallback may be inferred from this timeout fix.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/internal-dispatch-timeout-does-not-return-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/internal-codex-agent-execution-fail-closed-design.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
  - `launcher`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow consume continue`
  - `vida taskflow recovery status`
  - runtime dispatch result artifacts
  - run-graph dispatch receipts

## Design Decisions

### 1. Internal dispatch must have a bounded fail-closed return path independent of pipe EOF
Will implement / choose:
- Treat the child-process timeout boundary as sufficient to return a blocked timeout result even if stdout/stderr reader completion lags or descendants keep pipes open.
- Why
  - Receipt-backed blocked truth is more important than waiting indefinitely for perfect pipe drain.
- Trade-offs
  - Timeout results may capture partial or empty provider output when the process tree misbehaves.
- Alternatives considered
  - Trust only the existing reader-thread completion path.
  - Rejected because the live defect demonstrates that this can strand the orchestrator past the configured timeout window.

### 2. Keep inner wrapper hardening primary and preserve truthful downstream timeout normalization
Will implement / choose:
- Fix the inner wrapper/runtime execution path itself rather than only relying on stale in-flight normalization or pure outer timeout fallback.
- Why
  - The main defect is that live `consume continue` does not return promptly; stale normalization only repairs state after the fact.
- Trade-offs
  - The patch may span both command-wrapper logic and receipt timeout application surfaces.
- Alternatives considered
  - Enable only the outer timeout for internal host dispatch.
  - Rejected because it can abandon a stuck blocking task without guaranteeing prompt cleanup or truthful direct return semantics.

## Technical Design

### Core Components
- Main components
  - `runtime_dispatch_execution.rs`
  - `runtime_dispatch_state.rs`
  - targeted tests in `runtime_dispatch_execution.rs` and/or `runtime_dispatch_state.rs`
- Key interfaces
  - `execute_wrapped_command(...)`
  - `execute_wrapped_command_async(...)`
  - `execute_internal_agent_lane_dispatch(...)`
  - `execute_and_record_dispatch_receipt(...)`
- Bounded responsibilities
  - command wrapper returns on bounded timeout even when descendant pipe behavior is pathological
  - dispatch state persists blocked timeout truth promptly
  - recovery/status surfaces see canonical blocked evidence rather than a stranded executing receipt

### Data / State Model
- Important entities
  - runtime dispatch result artifact
  - run-graph dispatch receipt
  - lane execution receipt artifact
  - timeout wrapper state
- Receipts / runtime state / config fields
  - `execution_state`
  - `dispatch_status`
  - `lane_status`
  - `blocker_code`
  - `timeout_wrapper`
  - `stale_after_seconds`
- Migration or compatibility notes
  - additive behavior only; historical artifacts remain readable

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - `taskflow run-graph dispatch-init` -> `agent-init --execute-dispatch` -> internal delegated dispatch -> dispatch receipt/result reconciliation
  - `taskflow consume continue` -> internal delegated dispatch -> dispatch receipt/result reconciliation
- Cross-document / cross-protocol dependencies
  - `docs/product/spec/internal-codex-agent-execution-fail-closed-design.md`
  - `docs/product/spec/serialize-authoritative-state-access-lock-mitigation-design.md`
  - `docs/product/spec/existing-design-implementation-routing-blocked-design.md`

### Bounded File Set
- `docs/product/spec/internal-dispatch-timeout-does-not-return-design.md`
- `docs/product/spec/current-spec-map.md`
- `crates/vida/src/runtime_dispatch_execution.rs`
- `crates/vida/src/runtime_dispatch_state.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - no root-session implementation fallback
  - no silent success when terminal completion evidence is missing
  - no heuristic completion inference from an in-flight `executing` artifact alone
- Required receipts / proofs / gates
  - blocked timeout result must be written when bounded timeout is exceeded
  - recovery/status surfaces must observe blocked timeout truth after the direct command returns
- Safety boundaries that must remain true during rollout
  - successful internal execution remains allowed
  - routing repair for existing design-backed implementation work remains intact

## Implementation Plan

### Phase 1
- Record the bounded defect and pin the live failure shape against the wrapper/dispatch code.
- First proof target
  - `vida docflow check --root . docs/product/spec/internal-dispatch-timeout-does-not-return-design.md docs/product/spec/current-spec-map.md`

### Phase 2
- Fix the internal command-wrapper / dispatch-return path so bounded timeout always returns control with blocked truth.
- Add targeted regression coverage for the live failure shape.
- Second proof target
  - targeted `cargo test -p vida` for internal dispatch timeout return behavior

### Phase 3
- Re-run the repaired implementer launcher path with the source-built binary and confirm prompt blocked timeout return or lawful completion.
- Final proof target
  - targeted cargo tests plus live
    - `cargo run -p vida -- taskflow run-graph dispatch-init feature-reconcile-autonomous-execution-flag-runtime-drift --json`
    - `cargo run -p vida -- agent-init --dispatch-packet <fresh-implementer-packet> --execute-dispatch --json`
    - `cargo run -p vida -- taskflow recovery status feature-repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen --json`

## Validation / Proof
- Unit tests:
  - internal wrapped command returns promptly when timeout is exceeded even with pathological descendant/stdout behavior
  - internal dispatch receipt reconciliation records blocked timeout truth on bounded timeout return
- Integration tests:
  - `execute_and_record_dispatch_receipt` prompt-return regression for internal delegated implementer handoff
- Runtime checks:
  - `cargo run -p vida -- taskflow run-graph dispatch-init feature-reconcile-autonomous-execution-flag-runtime-drift --json`
  - `cargo run -p vida -- agent-init --dispatch-packet <fresh-implementer-packet> --execute-dispatch --json`
  - `cargo run -p vida -- taskflow recovery status feature-repair-design-backed-reseed-canonicalization-does-not-deadlock-qwen --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/internal-dispatch-timeout-does-not-return-design.md docs/product/spec/current-spec-map.md`

## Observability
- Logging points
  - timeout branch for internal wrapped command return
  - timeout-to-blocked receipt application for internal host handoff
- Metrics / counters
  - none required initially
- Receipts / runtime state written
  - runtime dispatch result artifact
  - run-graph dispatch receipt
  - lane execution receipt artifact

## Rollout Strategy
- Development rollout
  - land as one bounded blocker fix before resuming qwen carrier-drift remediation
- Migration / compatibility notes
  - no schema migration
- Operator or user restart / restart-notice requirements
  - rebuild and refresh the local `vida` binary after proof passes

## Future Considerations
- Follow-up ideas
  - strengthen process-tree cleanup or streaming output capture for internal host dispatch
- Known limitations
  - this slice focuses on bounded timeout return semantics, not full internal carrier execution redesign
- Technical debt left intentionally
  - richer internal child-process supervision can remain a later hardening slice

## References
- Related specs
  - `docs/product/spec/internal-codex-agent-execution-fail-closed-design.md`
  - `docs/product/spec/serialize-authoritative-state-access-lock-mitigation-design.md`
  - `docs/product/spec/existing-design-implementation-routing-blocked-design.md`
- Related protocols
  - `docs/process/project-orchestrator-operating-protocol.md`
- Related ADRs
  - none
- External references
  - none

-----
artifact_path: product/spec/internal-dispatch-timeout-does-not-return-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/internal-dispatch-timeout-does-not-return-design.md
created_at: 2026-04-21T12:18:33.729195679Z
updated_at: 2026-04-21T19:04:07.801125127Z
changelog_ref: internal-dispatch-timeout-does-not-return-design.changelog.jsonl
