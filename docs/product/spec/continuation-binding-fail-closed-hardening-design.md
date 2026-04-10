# Continuation Binding Fail Closed Hardening Design

Status: `implemented`

Use this template for one bounded feature/change design before implementation.

## Summary
- Feature / change: harden continued-development routing so automatic continuation remains inside one explicitly bound bounded unit and fails closed when that binding is ambiguous.
- Owner layer: `mixed`
- Runtime surface: `launcher | taskflow | project activation`
- Status: `implemented`

## Current Context
- Existing system overview
  - Project canon already says continued-development sessions should auto-continue after non-gating reports when the next lawful step is already known.
  - Project canon also already says continuation is lawful only after the active bounded unit is explicitly bound and must fail closed on ambiguity.
  - Current runtime surfaces expose continuation and write-guard state, but do not materialize a dedicated machine-readable continuation-binding summary.
- Key components and relationships
  - `AGENTS.md` and the generated scaffold shape the top bootstrap contract.
  - `docs/process/project-orchestrator-*.md` and the runtime capsules define launch-readiness and continuation law for the orchestrator lane.
  - `docs/product/spec/autonomous-report-continuation-law.md` and `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md` define the product-law continuation boundary.
  - `crates/vida/src/taskflow_runtime_bundle.rs`, `crates/vida/src/status_surface*.rs`, and `crates/vida/src/release1_contracts.rs` define the machine-readable startup/status/operator surfaces.
- Current pain point or gap
  - The active law is strong enough in prose, but the runtime still leaves a gap between “continue when lawful” and “prove which bounded unit is lawful now”.
  - That gap allows an orchestrator to over-index on anti-stop continuation and self-select adjacent work after a bounded slice closes.
  - The generated host guidance warns against pausing, but it does not yet require checking an explicit `active_bounded_unit` / `why_this_unit` / `primary_path` surface before continuation.

## Goal
- What this change should achieve
  - Make explicit continuation binding a first-class runtime/operator surface.
  - Fail closed whenever continued-development intent is active but no uniquely evidenced bounded unit is bound.
  - Repeat that requirement across the bootstrap/process guidance actually seen by the orchestrator.
- What success looks like
  - `vida orchestrator-init --json` exposes a machine-readable continuation-binding summary.
  - `vida status --json` exposes the same summary and blocks on continuation-binding ambiguity.
  - Generated prompt/instruction surfaces explicitly forbid self-selecting `ready_head[0]` or adjacent work when the active bounded unit is not explicit.
  - Targeted tests pin the ambiguity and fail-closed behavior.
- What is explicitly out of scope
  - redesign of TaskFlow backlog semantics
  - changing delegated lane ownership law
  - inventing a new multi-task planning workflow beyond the existing bounded-unit model

## Requirements

### Functional Requirements
- Must expose a machine-readable continuation-binding summary containing:
  - `status`
  - `active_bounded_unit`
  - `binding_source`
  - `why_this_unit`
  - `primary_path`
  - `sequential_vs_parallel_posture`
  - `next_actions`
- Must fail closed when continued-development intent is active and the runtime cannot prove one uniquely evidenced bounded unit.
- Must treat “completed run graph with no named next lawful bounded unit” as ambiguous rather than as permission to self-select the next ready slice.
- Must update active bootstrap/process guidance so continuation without explicit binding is forbidden.
- Must add a canonical blocker code and operator next action for continuation-binding ambiguity.

### Non-Functional Requirements
- Performance
  - negligible overhead on status/init surfaces
- Observability
  - ambiguity must be visible in JSON and text status outputs
  - tests must pin the surfaced blocker and next action
- Security
  - no implicit fallback from ambiguity into automatic ready-task selection

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `AGENTS.md`
  - `install/assets/AGENTS.scaffold.md`
  - `docs/process/project-orchestrator-operating-protocol.md`
  - `docs/process/project-orchestrator-session-start-protocol.md`
  - `docs/process/project-packet-and-lane-runtime-capsule.md`
  - `docs/process/project-start-readiness-runtime-capsule.md`
  - `docs/product/spec/autonomous-report-continuation-law.md`
  - `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
- Config / receipts / runtime surfaces affected:
  - `vida orchestrator-init`
  - `vida status`
  - runtime-generated host instruction surfaces

## Design Decisions

### 1. Continuation binding will be surfaced explicitly instead of inferred ad hoc
Will implement / choose:
- Add a dedicated continuation-binding summary to runtime init/status surfaces.
- Why
  - The main failure mode is not missing law; it is missing machine-readable proof of which bounded unit is currently lawful.
- Trade-offs
  - Adds a small amount of summary logic and new JSON surface fields.
- Alternatives considered
  - Rely on prose-only instruction tightening.
  - Reuse only the existing dispatch-receipt ambiguity signal.

### 2. Ambiguity will block operator continuation rather than degrade to heuristics
Will implement / choose:
- Add an explicit blocker code and next action for continuation-binding ambiguity.
- Treat “no active unit”, “completed run without named next bounded unit”, and “evidence mismatch” as fail-closed states for continuation.
- Why
  - The project canon already requires fail-closed handling when the active bounded unit cannot be named explicitly.
- Trade-offs
  - Operators/agents will sometimes need to re-bind or report ambiguity instead of continuing automatically.
- Alternatives considered
  - Allow a best-effort fallback when exactly one ready task exists.

## Technical Design

### Core Components
- `taskflow_runtime_bundle`
  - emits continuation-binding summary inside `orchestrator_init_view`
- `status_surface`
  - computes and renders the same summary in JSON/text/operator contracts
- `release1_contracts`
  - owns the canonical blocker vocabulary
- generated instruction surfaces
  - repeat the fail-closed continuation-binding rule in the active orchestrator guidance

### Data / State Model
- New continuation-binding summary fields:
  - `status`
  - `active_bounded_unit`
  - `binding_source`
  - `why_this_unit`
  - `primary_path`
  - `sequential_vs_parallel_posture`
  - `next_actions`
- New blocker code:
  - `continuation_binding_ambiguous`
- Compatibility notes
  - additive-only surface change
  - older snapshots remain readable; ambiguity summary may fail closed when required evidence is absent

### Integration Points
- `vida orchestrator-init --json`
- `vida status --json`
- text status render
- operator contracts / blocker reporting
- generated host-runtime instruction scaffolds

### Bounded File Set
- `docs/product/spec/continuation-binding-fail-closed-hardening-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `docs/product/spec/autonomous-report-continuation-law.md`
- `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- `AGENTS.md`
- `install/assets/AGENTS.scaffold.md`
- `docs/process/project-orchestrator-operating-protocol.md`
- `docs/process/project-orchestrator-session-start-protocol.md`
- `docs/process/project-packet-and-lane-runtime-capsule.md`
- `docs/process/project-start-readiness-runtime-capsule.md`
- `crates/vida/src/release1_contracts.rs`
- `crates/vida/src/taskflow_runtime_bundle.rs`
- `crates/vida/src/runtime_consumption_surface.rs`
- `crates/vida/src/status_surface.rs`
- `crates/vida/src/status_surface_json_report.rs`
- `crates/vida/src/status_surface_operator_contracts.rs`
- `crates/vida/src/status_surface_signals.rs`
- `crates/vida/src/status_surface_text_report.rs`
- `crates/vida/src/runtime_dispatch_packet_text.rs`
- `crates/vida/src/init_surfaces.rs`
- `crates/vida/src/main.rs`

## Fail-Closed Constraints
- Do not treat sticky continuation intent as authorization to select the first ready backlog item.
- Do not treat a completed bounded slice as authorization to continue into adjacent work without a newly explicit bounded-unit binding.
- Do not let missing continuation-binding evidence degrade silently into informational-only status.

## Implementation Plan

### Phase 1
- Register the bounded design and tighten the canonical instruction surfaces.
- First proof target
  - `vida docflow check --root . docs/product/spec/continuation-binding-fail-closed-hardening-design.md`

### Phase 2
- Add continuation-binding runtime/status surfaces and blocker enforcement.
- Second proof target
  - targeted `cargo test -p vida ...` for init/status/operator-contract coverage

### Phase 3
- Align generated prompt/instruction surfaces and run bounded validation.
- Final proof target
  - docflow + targeted cargo tests + release build

## Validation / Proof
- Unit tests:
  - `cargo test -p vida orchestrator_init_view_exposes_continuation_binding_fail_closed_summary -- --exact --nocapture`
  - `cargo test -p vida continuation_binding_ambiguous_blocks_operator_contracts -- --exact --nocapture`
  - `cargo test -p vida latest_run_graph_dispatch_receipt_signal_ambiguous_blocks_drifted_lane_status -- --exact --nocapture`
- Integration tests:
  - targeted `cargo test -p vida status_surface -- --nocapture`
- Runtime checks:
  - `vida status --json`
  - `vida orchestrator-init --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/continuation-binding-fail-closed-hardening-design.md docs/product/spec/autonomous-report-continuation-law.md docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md docs/process/project-orchestrator-operating-protocol.md docs/process/project-orchestrator-session-start-protocol.md docs/process/project-packet-and-lane-runtime-capsule.md docs/process/project-start-readiness-runtime-capsule.md`

## Observability
- New status/init JSON fields expose continuation-binding posture directly.
- Operator blocker codes and next actions call out ambiguity.
- Text status render prints continuation-binding summary and recovery action.

## Rollout Strategy
- Land docs and runtime enforcement in one bounded change.
- Keep the change additive so older state remains readable.
- Rebuild release binary after tests pass.

## Future Considerations
- Surface the active bounded-unit summary through more TaskFlow recovery/continue views.
- Add a stricter receipt-backed `why_this_unit` lineage once TaskFlow emits explicit continuation receipts for every transition.

## References
- `AGENTS.md`
- `docs/process/project-orchestrator-operating-protocol.md`
- `docs/process/project-orchestrator-session-start-protocol.md`
- `docs/process/project-packet-and-lane-runtime-capsule.md`
- `docs/process/project-start-readiness-runtime-capsule.md`
- `docs/product/spec/autonomous-report-continuation-law.md`
- `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`

-----
artifact_path: product/spec/continuation-binding-fail-closed-hardening-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-04-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/continuation-binding-fail-closed-hardening-design.md
created_at: '2026-04-10T12:30:00+03:00'
updated_at: '2026-04-10T12:30:00+03:00'
changelog_ref: continuation-binding-fail-closed-hardening-design.changelog.jsonl
