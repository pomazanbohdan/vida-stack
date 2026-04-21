# Existing design-backed implementation routing blocker fix design

Purpose: Bound the current-release fix for tasks that already have a finalized design but are rerouted back into specification/spec-pack and blocked on non-executing internal dispatch.

Status: `proposed`

Use this template for one bounded feature/change design before implementation.

## Summary
- Feature / change: stop design-backed implementation tasks from re-entering `specification/spec-pack`, and restore lawful implementer routing plus admissible delegated execution once a bounded design is already finalized.
- Owner layer: `mixed`
- Runtime surface: `taskflow | launcher`
- Status: `proposed`

## Current Context
- Existing system overview
  - Design-first feature intake correctly starts on `spec-pack` and requires one bounded design document before implementation.
  - After that design exists, TaskFlow should progress through `work-pool-pack` / `dev-pack` toward implementer execution rather than regenerating specification handoff.
  - Current runtime still derives parts of design-gate truth from request wording and tracked-flow entry hints, not only from persisted task/run evidence.
- Key components and relationships
  - `crates/vida/src/development_flow_orchestration.rs` computes `requires_design_gate` and shapes tracked-flow bootstrap.
  - `crates/vida/src/runtime_dispatch_status.rs` maps conversation/design-first states to `spec-pack` fallback run-graph status.
  - `crates/vida/src/taskflow_run_graph.rs` seeds and stores task-class / route-task-class state.
  - `crates/vida/src/taskflow_consume.rs` and `crates/vida/src/taskflow_consume_resume.rs` reconcile dispatch/continue behavior against persisted receipts and tracked-flow blockers.
  - `crates/vida/src/runtime_dispatch_execution.rs` and related receipt logic surface `activation_view_only` / `configured_backend_dispatch_failed`.
- Current pain point or gap
  - A task with an already created and finalized design document can still seed or resume into `specification`, because request text mentioning the design/spec is treated like fresh design intake.
  - That misrouting then selects `business_analyst` / `specification` and can hit a non-executing internal dispatch path instead of lawful implementer routing.
  - The result is a blocked delegated cycle even though design-gate work is already complete and implementation should be the next bounded step.

## Goal
- What this change should achieve
  - Make runtime routing recognize “design already complete” as stronger evidence than request wording containing spec/design terms.
  - Route design-backed implementation tasks to implementation/dev-pack/implementer flow instead of regenerating spec-pack.
  - Keep true spec-first requests on the existing design-first path.
- What success looks like
  - The existing design-backed serialization task no longer seeds/dispatches into `specification`.
  - Run-graph / packet / receipt surfaces agree on implementation-oriented routing for already-designed tasks.
  - Delegated execution reaches an admissible implementer path instead of a blocked internal activation-view-only specification lane.
- What is explicitly out of scope
  - Replacing the overall design-first tracked flow model.
  - Changing specification-lane document-scope law.
  - Implementing the serialization lock-mitigation code itself in this blocker slice.

## Requirements

### Functional Requirements
- Must distinguish fresh design-intake requests from existing design-backed implementation continuation.
- Must not derive `requires_design_gate = true` solely from request text when runtime/task evidence shows design-gate work is already complete for the active bounded task.
- Must keep `spec-pack` routing for genuine research/specification/planning requests that have not yet satisfied the design gate.
- Must ensure run-graph fallback/bootstrap and dispatch receipt projection do not drift back to `specification` once the task is implementation-ready.
- Must keep delegated execution fail-closed when the selected backend is inadmissible or activation-view-only, but must not choose a non-executing specification path for an implementation-ready task.

### Non-Functional Requirements
- Performance
  - The fix should reuse existing persisted task/run evidence and avoid broad extra store scans.
- Observability
  - Runtime surfaces should make the effective implementation-ready route visible in run-graph/packet/receipt outputs.
- Compatibility
  - Existing genuine design-first bootstrap behavior must remain unchanged.
- Safety
  - No local root-session bypass may be inferred from the blocker fix itself.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/existing-design-implementation-routing-blocked-design.md`
  - `docs/product/spec/current-spec-map.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
  - `launcher`
- Config / receipts / runtime surfaces affected:
  - `vida taskflow run-graph seed`
  - `vida taskflow run-graph dispatch-init`
  - `vida taskflow consume continue`
  - `vida taskflow packet render`

## Design Decisions

### 1. Persisted design-gate completion outranks request-text design terms
Will implement / choose:
- Use task/run evidence for design completion to suppress fresh `spec-pack` re-entry when the active task already has a finalized design and is moving toward implementation.
- Why
  - Request wording can mention the existing design doc while still being an implementation request.
- Trade-offs
  - Routing becomes slightly more state-aware instead of relying mostly on prompt classification.
- Alternatives considered
  - Further keyword tuning only.
  - Rejected because the live failure happens after a design already exists, so wording-only classification is insufficient.

### 2. Implementation-ready tasks must project implementation/dev-pack truth consistently
Will implement / choose:
- Keep run-graph bootstrap, packet rendering, and dispatch-receipt projection aligned on implementation-oriented targets once the design gate is satisfied.
- Why
  - Partial fixes in only one surface would let status/packet/continue drift out of sync again.
- Trade-offs
  - The fix may need changes in more than one runtime file.
- Alternatives considered
  - Patch only one receipt or one fallback mapper.
  - Rejected because the current bug spans orchestration planning, run-graph fallback, and continue/dispatch behavior.

## Technical Design

### Core Components
- `development_flow_orchestration.rs`
  - replace prompt-only design-gate activation with evidence-aware logic
- `runtime_dispatch_status.rs`
  - ensure fallback status reflects implementation-ready tasks correctly
- `taskflow_run_graph.rs`
  - preserve implementation task class and route-task-class truth after seed/init
- `taskflow_consume.rs`
  - keep packet/bootstrap generation aligned with implementation-ready routing
- `taskflow_consume_resume.rs`
  - continue/recovery must not preserve stale spec-pack blockers once the task is implementation-ready

### Data / State Model
- Important entities
  - active task status/labels/notes
  - run-graph task class and route-task-class
  - design-gate completion evidence
  - dispatch receipt / downstream blockers
- Receipts / runtime state / config fields
  - `tracked_flow_entry`
  - `task_class`
  - `route_task_class`
  - `dispatch_target`
  - `downstream_dispatch_target`
  - `pending_design_finalize`
  - `pending_spec_task_close`
- Migration or compatibility notes
  - additive behavior only; no schema migration required

### Integration Points
- APIs
  - none external
- Runtime-family handoffs
  - design-first intake remains `spec-pack`
  - implementation-ready continuation becomes `dev-pack` / implementer path
- Cross-document / cross-protocol dependencies
  - `docs/product/spec/repair-selector-precedence-crates-vida-src-design.md`
  - `docs/product/spec/specification-lane-scope-hardening-design.md`
  - `docs/product/spec/implementation-backend-admissibility-and-selection-truth-design.md`

### Bounded File Set
- `docs/product/spec/existing-design-implementation-routing-blocked-design.md`
- `docs/product/spec/current-spec-map.md`
- `crates/vida/src/development_flow_orchestration.rs`
- `crates/vida/src/runtime_dispatch_status.rs`
- `crates/vida/src/taskflow_run_graph.rs`
- `crates/vida/src/taskflow_consume.rs`
- `crates/vida/src/taskflow_consume_resume.rs`

## Fail-Closed Constraints
- Do not let a task with finalized design evidence drift back into `specification` solely because the request mentions “design”, “spec”, or a design-doc path.
- Do not weaken genuine spec-first routing for fresh design-intake requests.
- Do not treat `configured_backend_dispatch_failed` or `activation_view_only` as permission for root-local coding.
- Do not mutate adjacent tasks or parked MemPalace lanes as part of this blocker slice.

## Implementation Plan

### Phase 1
- Register the design and pin the exact failure shape against current runtime surfaces.
- First proof target
  - `vida docflow check --root . docs/product/spec/existing-design-implementation-routing-blocked-design.md docs/product/spec/current-spec-map.md`

### Phase 2
- Implement evidence-aware design-gate suppression for already-designed implementation tasks.
- Align fallback status and packet/dispatch projection with the corrected route.
- Second proof target
  - targeted `cargo test -p vida` for run-graph / consume / routing regressions

### Phase 3
- Reproduce the original serialization-task scenario and verify it now reaches implementation-oriented delegated routing.
- Final proof target
  - targeted cargo tests plus runtime packet/run-graph verification

## Validation / Proof
- Unit tests:
  - implementation-ready task with existing design does not route to `spec-pack`
  - fresh design-intake request still routes to `spec-pack`
  - continue/dispatch projection keeps implementation target after design gate is satisfied
- Integration tests:
  - seeded run-graph and consume/continue regression for the live blocker shape
- Runtime checks:
  - `vida taskflow run-graph status feature-serialize-authoritative-state-access-lock-mitigation --json`
  - `vida taskflow packet render feature-serialize-authoritative-state-access-lock-mitigation --json`
  - `vida taskflow consume continue --run-id feature-serialize-authoritative-state-access-lock-mitigation --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/existing-design-implementation-routing-blocked-design.md docs/product/spec/current-spec-map.md`

## Observability
- Logging points
  - routing/debug output should show when design-ready evidence suppresses spec-pack re-entry
- Metrics / counters
  - none required initially
- Receipts / runtime state written
  - existing run-graph status
  - existing dispatch receipts
  - existing packet render state

## Rollout Strategy
- Development rollout
  - land as one bounded blocker fix before resuming serialization implementation
- Migration / compatibility notes
  - no state-schema migration
- Operator or user restart / restart-notice requirements
  - rebuild and reinstall `vida` after proof passes

## Future Considerations
- Follow-up ideas
  - unify design-gate completion into one explicit helper instead of repeated local checks
- Known limitations
  - backend admissibility issues outside this route family remain governed by separate backend-selection law
- Technical debt left intentionally
  - broader tracked-flow simplification across spec/work-pool/dev remains a later architectural wave

## References
- `docs/product/spec/repair-selector-precedence-crates-vida-src-design.md`
- `docs/product/spec/specification-lane-scope-hardening-design.md`
- `docs/product/spec/implementation-backend-admissibility-and-selection-truth-design.md`

-----
artifact_path: product/spec/existing-design-implementation-routing-blocked-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-21
schema_version: 1
status: canonical
source_path: docs/product/spec/existing-design-implementation-routing-blocked-design.md
created_at: 2026-04-21T11:57:56.52638996Z
updated_at: 2026-04-21T11:59:44.716158253Z
changelog_ref: existing-design-implementation-routing-blocked-design.changelog.jsonl
