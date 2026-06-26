# RunWorkflow Aggregate And Hierarchical Statig Machine Design

Status: `proposed`

Use this document as the bounded analyst/specification packet for TaskFlow task `ldr-020`.

Structured-template rule:
1. Keep the scope limited to the deterministic RunWorkflow aggregate and hierarchical Statig state machine.
2. Treat implementation code as downstream proof surface, not as documentation authority.
3. Do not expand this packet into unrelated TaskFlow scheduler, lane, dispatch, or journal cutover work.

## Summary
- Feature / change: `RunWorkflowAggregate` plus a hierarchical `statig` machine for run workflow progression.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: `proposed for developer proof`

## Current Context
- `ldr-004` defines the Run aggregate as the owner of run graph state, active node, checkpoint, and resume state.
- The LDRK migration law requires run workflow mutation to move toward command-envelope and event-journal authority without direct projection writes after cutover.
- `taskflow-core` already exposes deterministic runtime-domain primitives that can host the aggregate without importing CLI, filesystem, adapter, or storage-specific details.
- `runtime-library-fsm-pilot-decision.md` keeps `rust-fsm` limited to the consume/resume pilot and identifies `statig` as the better fit for hierarchical state machines.
- The bounded implementation surface is `taskflow-core::run_workflow`, with proof exposure through `taskflow-authority::run_workflow`.

## Goal
- Provide one deterministic aggregate for run workflow state transitions.
- Provide one hierarchical `statig` machine that groups active, blocked, and terminal states under explicit superstates.
- Provide replayable transition events with effect intents rather than direct I/O.
- Provide proof targets for transition matrix coverage, replay determinism, terminal mutation rejection, status mapping, and authority summary exposure.
- Out of scope: replacing the operational journal, cutting over `run.advance`, rewriting CLI/TUI/service adapters, changing scheduler admission, or mutating lane/dispatch artifacts.

## Requirements

### Functional Requirements
- `RunWorkflowAggregate` MUST carry `run_id`, `task_id`, current `RunWorkflowState`, and monotonic `version`.
- `RunWorkflowCommand` MUST cover start, dispatch, lane completion, block, recover, close, fail, and repair reopen.
- `RunWorkflowEvent` MUST carry command, before state, after state, effect intents, and optional blocker code.
- The machine MUST admit normal forward progression from idle to role-step activity and then terminal closure.
- The machine MUST map blocked states by reason: approval, lane, and recovery.
- The machine MUST reject non-repair mutations from terminal states with blocker code `terminal_state_mutation_rejected`.
- `RepairReopen` MUST be the only command admitted from terminal states.
- Lifecycle/status mapping MUST map known TaskFlow lifecycle tokens into aggregate states and fail closed on unknown mappings with `status_mapping_unknown`.

### Non-Functional Requirements
- Aggregate logic MUST remain deterministic and storage-neutral.
- Aggregate logic MUST emit effect intents only; it MUST NOT write files, mutate DB tables, dispatch agents, or call host bridges directly.
- Replay from an initial snapshot and ordered commands MUST produce stable state, version, events, and replay hash.
- The `statig` dependency MUST remain scoped to the core runtime-domain crate unless a later task expands adoption with proof.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/runworkflow-aggregate-hierarchical-statig-machin-design.md`
  - `docs/product/spec/local-durable-runtime-kernel-architecture-and-migration-law.md`
  - `docs/product/spec/runtime-library-fsm-pilot-decision.md`
- Framework protocols affected:
  - TaskFlow runtime binding and LDRK migration law only as downstream constraints.
- Runtime families affected:
  - `taskflow-core`
  - `taskflow-authority`
- Config / receipts / runtime surfaces affected:
  - No new config owner.
  - No new receipt format in this analyst packet.
  - Future cutover must route command-envelope/journal writes through a separate task.

## Design Decisions

### 1. Use A Core Aggregate With Effect Intents
Will implement / choose:
- Keep transition authority in `RunWorkflowAggregate::handle`.
- Return a `RunWorkflowEvent` for every command.
- Emit `RunWorkflowEffectIntent` values for persistence, dispatch, blocker, terminal, and rejection follow-up.
- Reason: the aggregate can be unit-tested and replayed without binding to redb, CLI rendering, lane packets, or host bridge artifacts.
- Alternative rejected: direct mutation of run graph projections inside the state machine, because `ldr-004` classifies those projections as rebuildable views after cutover.

### 2. Use Statig For Hierarchical State Handling
Will implement / choose:
- Model concrete states as `Idle`, `Active { step }`, `ApprovalBlocked`, `LaneBlocked`, `RecoveryBlocked`, `Completed`, and `Failed`.
- Model superstates as `Active`, `Blocked`, and `Terminal`.
- Let concrete handlers process admitted commands and fall through to superstate handling for rejected/unhandled commands.
- Reason: `statig` directly supports hierarchical state machines and matches the accepted library decision that did not authorize broad `rust-fsm` use beyond consume/resume.
- Alternative rejected: flat match-only transition logic, because it hides the active/blocked/terminal grouping that this task is meant to prove.

### 3. Keep Authority Proof Separate From Runtime I/O
Will implement / choose:
- Expose a small authority proof summary from `taskflow-authority`.
- Count transition rows, Mermaid rows, status mapping cases, and unknown-status blockers.
- Reason: developer and verifier lanes can prove the domain surface without requiring a full runtime cutover.
- Alternative rejected: treating generated docs or Mermaid output as state authority.

## Technical Design

### Core Components
- `RunWorkflowAggregate`
  - owns aggregate identity, state, version, command handling, and snapshot replay hash.
- `RunWorkflowState`
  - owns canonical state names and terminal/active/blocked classification.
- `RunWorkflowCommand`
  - owns the bounded command vocabulary for run workflow movement.
- `RunWorkflowEvent`
  - owns replayable transition evidence and fail-closed blocker code.
- `RunWorkflowEffectIntent`
  - owns side-effect classification without executing effects.
- `RunWorkflowMachine`
  - owns the `statig` transition implementation.
- `RunWorkflowProofSummary`
  - owns minimal authority proof visibility for downstream checks.

### Data / State Model
- Aggregate snapshot fields:
  - `run_id: String`
  - `task_id: String`
  - `state: RunWorkflowState`
  - `version: u64`
- Event fields:
  - `command`
  - `state_before`
  - `state_after`
  - `effect_intents`
  - `blocker_code`
- Version rule:
  - increment only when `state_after != state_before`.
- Replay hash rule:
  - hash or stable serialized representation MUST include run id, task id, state, and version.
- Compatibility note:
  - this packet does not require journal schema migration; journal integration is a downstream LDRK cutover task.

### Integration Points
- `taskflow-core/src/run_workflow/mod.rs`
  - aggregate, commands, states, status mapping, transition matrix, replay helper, and tests.
- `taskflow-core/Cargo.toml`
  - `statig` dependency scoped to `taskflow-core`.
- `taskflow-core/src/lib.rs`
  - public module export.
- `taskflow-authority/src/run_workflow/mod.rs`
  - proof summary adapter.
- `taskflow-authority/src/lib.rs`
  - public authority module registration.

### Bounded File Set
- Expected implementation files:
  - `crates/taskflow-core/Cargo.toml`
  - `crates/taskflow-core/src/lib.rs`
  - `crates/taskflow-core/src/run_workflow/mod.rs`
  - `crates/taskflow-authority/src/lib.rs`
  - `crates/taskflow-authority/src/run_workflow/mod.rs`
- Expected specification file:
  - `docs/product/spec/runworkflow-aggregate-hierarchical-statig-machin-design.md`
- No unrelated files are in scope for this packet.

## Fail-Closed Constraints
- Do not let the aggregate mutate filesystem, DB, lane packets, dispatch receipts, host bridge files, or projections directly.
- Do not make terminal states mutable except through `RepairReopen`.
- Do not treat unknown lifecycle/status tokens as active or completed states.
- Do not broaden `statig` into unrelated TaskFlow state machines in this task.
- Do not claim LDRK run-operation cutover until command-envelope, journal, and projection rebuild proof exists in a separate task.

## Implementation Plan

### Phase 1
- Add the aggregate, state, command, event, effect-intent, and replay model.
- First proof target: deterministic replay reaches `Completed` and increments version once per admitted transition.

### Phase 2
- Add the hierarchical `statig` machine with active, blocked, and terminal superstates.
- Second proof target: blocked recovery, terminal rejection, and repair reopen behavior are covered by tests.

### Phase 3
- Add status mapping corpus, transition matrix/Mermaid generation, and authority proof summary.
- Final proof target: authority summary reports transition, diagram, mapping, and unknown-blocker coverage.

## Validation / Proof
- Unit tests:
  - `cargo test -p taskflow-core run_workflow`
  - `cargo test -p taskflow-authority run_workflow`
- Integration tests:
  - Not required for this analyst packet unless developer lane wires CLI/operator surfaces.
- Runtime checks:
  - No runtime mutation check required for this analyst packet.
  - Future journal cutover must add command-envelope and projection-rebuild proof.
- Canonical checks:
  - `vida docflow check-file --path docs/product/spec/runworkflow-aggregate-hierarchical-statig-machin-design.md`
  - `vida docflow fastcheck --root docs/product/spec docs/product/spec/runworkflow-aggregate-hierarchical-statig-machin-design.md`

## Observability
- Aggregate-level observability is the emitted `RunWorkflowEvent`.
- Proof-level observability is `RunWorkflowProofSummary`.
- Runtime/operator observability is deferred to a downstream cutover task.

## Rollout Strategy
- Merge the core/domain aggregate first with unit proof.
- Keep it storage-neutral until command-envelope and event-journal integration is scheduled.
- Use authority summary as the developer/verifier handoff signal.
- No operator restart or installed-binary behavior change is required by this spec packet alone.

## Future Considerations
- Wire `run.advance` into `VidaCommandEnvelope` and the operational journal.
- Rebuild run graph projections from journaled run workflow events.
- Add conformance tests for external durable engines once `RuntimeEngine` ports are active.
- Add public CLI/operator proof only when a downstream task changes an operator surface.

## References
- `docs/product/spec/local-durable-runtime-kernel-architecture-and-migration-law.md`
- `docs/product/spec/runtime-library-fsm-pilot-decision.md`
- `docs/product/spec/taskflow-protocol-runtime-binding-model.md`
- `crates/taskflow-core/src/run_workflow/mod.rs`
- `crates/taskflow-authority/src/run_workflow/mod.rs`

-----
artifact_path: product/spec/runworkflow-aggregate-hierarchical-statig-machin-design
artifact_type: product_spec
artifact_version: "1"
artifact_revision: 2026-06-26
schema_version: "1"
status: proposed
source_path: docs/product/spec/runworkflow-aggregate-hierarchical-statig-machin-design.md
created_at: 2026-06-26T10:00:34Z
updated_at: 2026-06-26T10:00:34Z
changelog_ref: runworkflow-aggregate-hierarchical-statig-machin-design.changelog.jsonl
