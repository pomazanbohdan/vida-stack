# Internal Codex Agent Execution Fail Closed Design

Status: approved

Use this template for one bounded feature/change design before implementation.

## Summary
- Feature / change: Fail closed when the internal Codex backend materializes only an activation view and does not actually execute a delegated agent lane, and update operator/runtime instructions so this cannot be misread as completed agent-first execution.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher | project activation`
- Status: proposed

## Current Context
- Existing system overview
  - Project canon already says root stays orchestrator and normal write-producing work routes through `vida agent-init`.
  - The current internal Codex backend path can render an activation view for a dispatch packet.
  - Downstream orchestration logic currently accepts `packet_ready` as a lane state with enough evidence to continue certain runtime chains.
- Key components and relationships
  - `AGENTS.md` and project process docs define agent-first delegated execution and forbid root-session local write fallback.
  - `crates/vida/src/runtime_dispatch_state.rs` chooses between external backend execution and internal activation-view rendering.
  - `crates/vida/src/runtime_dispatch_execution.rs` wraps internal activation views into a dispatch result.
  - `crates/vida/src/runtime_dispatch_packet_text.rs` and generated init guidance shape the operator interpretation of delegated execution.
- Current pain point or gap
  - For the internal Codex path, runtime currently reports `execution_state = "packet_ready"` and `lane_status = "lane_running"` even when no actual worker execution happened.
  - That creates a false positive delegated-execution signal for the orchestrator.
  - Existing instructions strongly forbid root-side write fallback, but they do not make the internal-host activation-view limitation explicit enough at the runtime handoff point.
  - The result is a dual failure mode: operator/orchestrator confusion and runtime state that overstates execution progress.

## Goal
- What this change should achieve
  - Make internal Codex delegated execution fail closed unless there is real execution evidence, not just an activation view.
  - Make instructions and runtime prompts explicitly say that an internal activation view is analysis/shaping evidence only and does not mean an agent lane executed.
  - Preserve agent-first routing by surfacing a bridge blocker instead of allowing misleading `packet_ready/lane_running` state.
- What success looks like
  - Internal Codex activation-view-only dispatches become explicitly blocked or activation-only, not pseudo-running lanes.
  - Orchestrator-facing instructions state the correct next move: re-dispatch through a real executor path or fix the backend bridge, never substitute root-side implementation.
  - Targeted tests pin the fail-closed semantics and prevent regression.
- What is explicitly out of scope
  - Building a full internal packet executor in this change.
  - Reworking carrier selection or the full lane lifecycle model.
  - Changing external backend execution behavior.

## Requirements

### Functional Requirements
- Must detect when the selected host execution class is internal and the dispatch result is only an activation view.
- Must not mark that case as `execution_state = "packet_ready"` or `dispatch_status = "lane_running"` as if execution already started.
- Must surface an explicit blocker/result state that tells the orchestrator real execution has not yet happened.
- Must update operator/runtime instructions so internal activation-view-only dispatch is described as non-executing and non-authoritative, and so bounded read-only diagnosis continues until a code-level blocker or next bounded fix is explicit.
- Must keep root-session write-guard semantics fail-closed.
- Must preserve external backend execution semantics.

### Non-Functional Requirements
- Performance
  - No meaningful overhead beyond one additional state classification branch.
- Scalability
  - The fix must remain backend-neutral and apply to future internal host systems, not only Codex.
- Observability
  - Dispatch results, receipts, and prompts must distinguish activation-only from executed lanes.
- Security
  - The change must reduce unsafe fallback ambiguity and never widen root-session write authority.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `AGENTS.md`
  - `docs/process/project-orchestrator-operating-protocol.md`
  - `docs/process/project-packet-and-lane-runtime-capsule.md`
  - `docs/process/codex-agent-configuration-guide.md`
  - `docs/product/spec/clarify-enforce-immediate-project-agent-first-design.md`
  - `docs/product/spec/internal-codex-agent-execution-fail-closed-design.md`
- Framework protocols affected:
  - none beyond bootstrap-carried wording
- Runtime families affected:
  - `taskflow`
  - `launcher`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow consume final`
  - `vida agent-init`
  - runtime dispatch result artifacts
  - run-graph dispatch receipts
  - root-session write-guard interpretation surfaces

## Design Decisions

### 1. Activation view without execution evidence will fail closed
Will implement / choose:
- Reclassify internal activation-view-only dispatch as blocked or activation-only rather than `packet_ready/executed`.
- Why
  - Agent-first routing cannot rely on pseudo-running lane state.
- Trade-offs
  - Some current tests and downstream assumptions about `packet_ready` will need tightening.
- Alternatives considered
  - Keep `packet_ready` and rely only on stronger prose.
  - Rejected because the machine-readable state is the source of the confusion.
- ADR link if this must become a durable decision record
  - none

### 2. Instructions will explicitly name internal activation-view limitation
Will implement / choose:
- Add wording to bootstrap/runtime guidance and packet prompt text that internal activation views are not execution receipts and must be treated as bridge blockers for write-producing work.
- Why
  - The operator/orchestrator should fail closed before any local compensating behavior starts.
- Trade-offs
  - More explicit prompt text and guidance.
- Alternatives considered
  - Leave the rule implicit in root-session write-guard docs alone.
  - Rejected because the runtime emitted a misleading state even while those docs existed.
- ADR link if needed
  - none

## Technical Design

### Core Components
- Main components
  - internal-vs-external runtime dispatch branch
  - dispatch result classification
  - downstream dispatch readiness logic
  - runtime packet prompt / init guidance wording
- Key interfaces
  - `execute_runtime_dispatch_handoff`
  - `agent_lane_dispatch_result`
  - `execute_and_record_dispatch_receipt`
  - `vida agent-init --dispatch-packet ... --json`
- Bounded responsibilities
  - runtime code must report truthful dispatch state
  - docs/prompts must explain the truthful next action

### Data / State Model
- Important entities
  - activation view
  - execution evidence
  - dispatch receipt
  - root-session write guard
- Receipts / runtime state / config fields
  - `execution_state`
  - `dispatch_status`
  - `lane_status`
  - `activation_semantics.activation_kind`
  - `selected_cli_execution_class`
  - blocker codes for activation-only internal dispatch
- Migration or compatibility notes
  - Existing historical `packet_ready` receipts remain readable, but new internal activation-only results should stop claiming running-lane semantics.

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - orchestrator -> analysis/worker lane dispatch
  - dispatch result -> run-graph receipt -> downstream preview
- Cross-document / cross-protocol dependencies
  - agent-first delegated execution law
  - root-session write-guard fail-closed law

### Bounded File Set
- `AGENTS.md`
- `docs/process/project-orchestrator-operating-protocol.md`
- `docs/process/project-packet-and-lane-runtime-capsule.md`
- `docs/process/codex-agent-configuration-guide.md`
- `docs/product/spec/clarify-enforce-immediate-project-agent-first-design.md`
- `docs/product/spec/internal-codex-agent-execution-fail-closed-design.md`
- `crates/vida/src/runtime_dispatch_execution.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/runtime_dispatch_packet_text.rs`
- `crates/vida/src/main.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - No local root-session implementation as compensation for an activation-view-only internal dispatch.
  - No pseudo-success dispatch state when no execution happened.
- Required receipts / proofs / gates
  - Dispatch/result artifacts must distinguish activation-only from executed.
  - Root-session write guard must remain blocked by default.
  - Runtime must point to bridge repair or explicit external dispatch, not silent local fallback.
- Safety boundaries that must remain true during rollout
  - External backend execution stays intact.
  - Agent-first routing stays canonical.
  - Internal activation views remain view-only.

## Implementation Plan

### Phase 1
- Update this design with the exact blocker semantics and proof targets.
- Tighten orchestrator/runtime instruction surfaces to name activation-view-only dispatch explicitly.
- First proof target
  - `vida docflow check --root . docs/product/spec/internal-codex-agent-execution-fail-closed-design.md`

### Phase 2
- Change runtime dispatch code so internal activation-view-only handoffs stop reporting `packet_ready/lane_running` as if execution happened.
- Update downstream receipt/state handling and targeted tests.
- Second proof target
  - targeted `cargo test -p vida ...` for internal dispatch classification and lane-state semantics

### Phase 3
- Re-run canonical runtime checks and release build.
- Refresh the installed binary and continue the next delegated cycle.
- Final proof target
  - green bounded tests plus `vida taskflow consume bundle check --json`

## Validation / Proof
- Unit tests:
  - internal agent-lane dispatch classification tests
  - downstream lane-state tests
- Integration tests:
  - `vida taskflow consume final` / `vida agent-init` contract tests for internal execution class
- Runtime checks:
  - `vida status --json`
  - `vida taskflow consume final "<request>" --json`
- Canonical checks:
  - `activation-check`
  - `protocol-coverage-check`
  - `check`
  - `doctor`

## Observability
- Logging points
  - dispatch result classification branch
- Metrics / counters
  - none new required in this bounded change
- Receipts / runtime state written
  - dispatch result artifact
  - dispatch receipt
  - run-graph status

## Rollout Strategy
- Development rollout
  - tighten docs and runtime state together in one bounded change
- Migration / compatibility notes
  - allow older receipts to be read, but do not emit new misleading internal activation-only states
- Operator or user restart / restart-notice requirements
  - rebuild and refresh installed `vida` binary after the bounded fix

## Future Considerations
- Follow-up ideas
  - add a true internal execution bridge for Codex instead of fail-closed activation-only results
- Known limitations
  - this change may still leave internal execution unimplemented; it just makes the gap explicit and safe
- Technical debt left intentionally
  - full internal lane execution remains a separate bounded feature

## References
- Related specs
  - `docs/product/spec/clarify-enforce-immediate-project-agent-first-design.md`
  - `docs/product/spec/release-1-plan.md`
- Related protocols
  - `docs/process/project-orchestrator-operating-protocol.md`
  - `docs/process/team-development-and-orchestration-protocol.md`
- Related ADRs
  - none
- External references
  - none

-----
artifact_path: product/spec/internal-codex-agent-execution-fail-closed-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-04-08'
schema_version: '1'
status: draft
source_path: docs/product/spec/internal-codex-agent-execution-fail-closed-design.md
created_at: '2026-04-08T19:45:00+03:00'
updated_at: '2026-04-08T19:45:00+03:00'
changelog_ref: internal-codex-agent-execution-fail-closed-design.changelog.jsonl
