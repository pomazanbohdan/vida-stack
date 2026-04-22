# Coach Retry Reuses Same Blocked Hermes Packet Without Fallback Design

Purpose: Bound the audit blocker where repeated `vida taskflow consume continue` retries relaunch the same blocked coach packet on `hermes_cli` instead of rotating to another admissible review backend, rewriting a fresh retry packet, or failing closed with a non-retry posture.

Status: `proposed`

## Summary
- Feature / change: repair coach-lane retry semantics so a blocked external review packet cannot be reopened indefinitely on the same packet identity without a lawful retry transition.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher`
- Status: `proposed`

## Current Context
- Existing system overview
  - `taskflow_consume_resume.rs` resolves persisted dispatch receipts, prepares retry artifacts, rewrites some blocked downstream packets, and then re-enters `execute_runtime_dispatch_chain(...)` / downstream dispatch execution.
  - `prepare_explicit_resume_retry_artifact(...)` already treats blocked `timeout_without_takeover_authority` receipts as retry-eligible and may replace `dispatch_receipt.selected_backend`.
  - `rewrite_retry_dispatch_packet_if_downstream_carrier(...)` only rewrites packets whose existing `packet_kind` is `runtime_downstream_dispatch_packet`.
- Key components and relationships
  - The live qwen remediation run reached `coach` on `hermes_cli` after implementer execution already succeeded.
  - Persisted artifacts show the same coach packet path and the same `lane_execution_receipt` `started_at` value surviving across multiple retries:
    - packet: `.../dispatch-packets/feature-reconcile-qwen-cli-carrier-drift-across-config-code-2026-04-21T19-33-27.253874589Z.json`
    - blocked results: `...19-36-41.71827872Z.json`, `...19-40-49.156634471Z.json`
  - That packet is a normal `runtime_dispatch_packet`, not a `runtime_downstream_dispatch_packet`.
- Current pain point or gap
  - Retry readiness can be restored without guaranteeing a fresh packet identity or a changed effective backend.
  - Because the persisted coach packet stays unchanged, the next retry relaunches the same `hermes_cli` packet and reproduces the same timeout loop.
  - The explicit coach route in the packet carries `fanout_executor_backends = [hermes_cli, opencode_cli]`, but current retry selection mainly reasons about `fallback_executor_backend`, so review-safe backend rotation is under-modeled.

## Goal
- What this change should achieve
  - Ensure blocked coach retries only reopen when the runtime can point at a fresh lawful retry packet or a distinct admissible retry backend.
  - Prefer review-safe backend rotation from the explicit coach route before collapsing into internal fallback for an external review timeout.
  - Prevent an identical same-packet timeout loop from being projected as canonical retry progress.
- What success looks like
  - A blocked coach retry either writes a fresh packet path with new retry semantics, rotates to a distinct admissible review backend, or stays blocked with explicit non-retry truth.
  - Repeated `consume continue` no longer relaunches the same coach packet file on `hermes_cli` after an unchanged timeout.
  - Packet/receipt/run-graph truth stays aligned on whether a lawful retry transition actually happened.
- What is explicitly out of scope
  - Reopening the parked MemPalace lane.
  - Broad redesign of all external backend supervision.
  - Reversing the qwen template-only policy or re-adding qwen to the active carrier catalog.

## Requirements

### Functional Requirements
- Retry preparation for blocked agent-lane receipts must not restore `packet_ready` unless there is a lawful retry transition.
- For coach/review-safe routes, retry selection must consider distinct admissible review backends from the explicit route fanout before internal fallback.
- When retry semantics change the effective dispatch backend or retry packet identity, the runtime must write a fresh canonical dispatch packet and update `dispatch_packet_path`.
- When no fresh retry packet or admissible backend change is available, the receipt must remain blocked instead of reopening the same packet identity.

### Non-Functional Requirements
- Observability
  - Runtime artifacts must make it clear whether a retry actually produced a fresh packet/backend change or stayed blocked fail-closed.
- Safety
  - No root-session write activation, exception takeover, or hidden packet deletion.
- Compatibility
  - Existing internal-host timeout handling and genuine downstream packet rewrites must remain intact.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/coach-retry-reuses-same-blocked-hermes-packet-without-fallback-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
- Runtime families affected:
  - `taskflow`
  - `launcher`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow consume continue --run-id <id> --json`
  - runtime dispatch receipts
  - runtime dispatch packets
  - downstream dispatch traces

## Design Decisions

### 1. Retry-ready status requires a new retry artifact, not only retry eligibility
Will implement / choose:
- Treat `dispatch_receipt_retry_eligible(...)` as a candidate gate, not sufficient evidence by itself.
- Only restore `packet_ready` when retry preparation produces a fresh packet path or a rewritten packet whose effective retry contract differs from the previously blocked packet.
- Why
  - Current logic can reopen the same blocked packet without any new execution contract.
- Trade-offs
  - Some previously automatic retries will now remain blocked until a lawful retry artifact can be materialized.

### 2. Coach timeout retries should prefer explicit review fanout rotation before internal fallback
Will implement / choose:
- For review-safe coach lanes, search the explicit route fanout for the next admissible backend distinct from the blocked backend before using `fallback_executor_backend`.
- Why
  - The live packet already advertises `opencode_cli` as an admissible peer review backend, while `internal_subagents` is a broader fallback and not the first architectural match for an external review timeout.
- Trade-offs
  - Retry backend selection becomes lane-aware and slightly richer than the current fallback-only heuristic.

### 3. Runtime dispatch packets need the same rewrite path as downstream dispatch packets when retry semantics change
Will implement / choose:
- Generalize retry packet rewriting so blocked `runtime_dispatch_packet` retries can be rewritten through the canonical packet renderer when the effective retry backend changes.
- Why
  - Today only `runtime_downstream_dispatch_packet` gets a fresh packet path; ordinary runtime dispatch packets keep the same file and re-execute unchanged semantics.
- Trade-offs
  - Packet rewrite logic becomes broader and needs regression coverage for both packet kinds.

## Technical Design

### Core Components
- Main components
  - `taskflow_consume_resume.rs`
  - `taskflow_routing.rs`
  - `runtime_dispatch_state.rs`
- Key interfaces
  - `dispatch_receipt_retry_eligible(...)`
  - `retry_backend_for_dispatch_receipt(...)`
  - `prepare_explicit_resume_retry_artifact(...)`
  - `rewrite_retry_dispatch_packet_if_downstream_carrier(...)` or its generalized replacement
  - `write_runtime_dispatch_packet(...)`
- Bounded responsibilities
  - determine whether a blocked coach receipt can lawfully retry
  - choose a distinct admissible retry backend for review-safe routes
  - materialize a fresh packet when retry semantics change
  - keep blocked truth when no lawful retry transition exists

### Bounded File Set
- `docs/product/spec/coach-retry-reuses-same-blocked-hermes-packet-without-fallback-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `crates/vida/src/taskflow_consume_resume.rs`
- `crates/vida/src/taskflow_routing.rs`
- `crates/vida/src/runtime_dispatch_state.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - no silent reuse of the same blocked packet identity as if it were a fresh retry
  - no hidden local takeover
  - no heuristic success projection after timeout
- Required receipts / proofs / gates
  - retry-ready truth must be backed by a fresh packet path or a clearly changed retry contract
  - unchanged blocked packet identity must remain blocked

## Implementation Plan

### Phase 1
- Register this blocker in the active spec maps and finalize the bounded design.
- First proof target
  - `vida docflow check --root . docs/product/spec/coach-retry-reuses-same-blocked-hermes-packet-without-fallback-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md`

### Phase 2
- Tighten retry eligibility and packet rewrite semantics for blocked coach receipts.
- Second proof target
  - targeted `cargo test -p vida` for retry preparation, packet rewrite, and blocked no-transition behavior

### Phase 3
- Re-run the active qwen remediation continuation path and confirm the coach retry loop no longer replays the same blocked hermes packet.
- Final proof target
  - targeted tests plus live `cargo run -p vida -- taskflow consume continue --run-id feature-reconcile-qwen-cli-carrier-drift-across-config-code --json`

## Validation / Proof
- Unit tests:
  - blocked coach timeout retry rotates to a distinct admissible review backend when one exists
  - blocked `runtime_dispatch_packet` retry rewrites to a fresh packet path when retry semantics change
  - unchanged blocked packet identity stays blocked and does not restore `packet_ready`
- Runtime checks:
  - inspect the resulting `dispatch_packet_path` and `selected_backend` after retry preparation
  - confirm repeated `consume continue` does not reuse the same coach packet file/path for the unchanged timeout case

## References
- `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`
- `docs/product/spec/coach-lane-inherits-internal-fallback-over-explicit-review-route-design.md`
- `docs/product/spec/external-coach-timeout-truth-does-not-return-cleanly-design.md`
- `docs/product/spec/blocked-external-coach-artifact-truth-not-reconciled-design.md`

-----
artifact_path: product/spec/coach-retry-reuses-same-blocked-hermes-packet-without-fallback-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/coach-retry-reuses-same-blocked-hermes-packet-without-fallback-design.md
created_at: 2026-04-21T19:49:30.468539654Z
updated_at: 2026-04-21T19:51:57.255540405Z
changelog_ref: coach-retry-reuses-same-blocked-hermes-packet-without-fallback-design.changelog.jsonl
