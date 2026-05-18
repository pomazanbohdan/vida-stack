# TaskFlow Happy-Path Test Catalog Design

Status: `proposed`

Use this template for one bounded feature/change design before implementation.

## Summary
- Feature / change: define a concise happy-path test catalog for TaskFlow, ordered from the simplest operator flows to full delegated execution and closure flows.
- Owner layer: `mixed`
- Runtime surface: `taskflow | status | agent-init | run-graph | doctor`
- Status: `proposed`

## Current Context
- Existing system overview
  - The project already has a TaskFlow testing execution epic and a separate defect-remediation epic.
  - Happy-path runs are expected to expose defects; discovered defects belong under the correct defect epic and must be repaired through agent mode before the sequence advances.
  - Parent/child closure consistency is now a runtime invariant: a closed parent must not retain open children, and an open parent must not project as clean when all closure-dependent children are closed or inconsistent.
- Key components and relationships
  - `vida task`, `vida taskflow`, `vida orchestrator-init`, `vida agent-init`, `vida status`, `vida doctor`, and `vida taskflow consume/recovery` are the operator surfaces under test.
  - `crates/vida/tests/task_smoke.rs`, `crates/vida/tests/boot_smoke.rs`, and graph validation tests in `crates/vida/src/state_store_task_graph.rs` are the primary Rust proof homes.
- Current pain point or gap
  - Existing tests cover many operator fragments, but there is no canonical ordered catalog that says which happy-path sequence should run first, which Rust proof owns each case, and when a failure must stop forward progress for immediate defect repair.

## Goal
- What this change should achieve
  - Create one canonical happy-path test catalog for TaskFlow.
  - Order cases from trivial CLI confidence to full execution/closure confidence.
  - Bind each case to an existing or planned Rust proof target.
  - Make immediate defect routing and agent-mode repair part of the catalog contract.
- What success looks like
  - Operators can run the happy-path sequence without guessing the next case.
  - Each failure has a defect-epic routing rule before more complex cases continue.
  - Parent/child closure consistency is checked early enough to prevent false closure projections.
- What is explicitly out of scope
  - Implementing the Rust tests in this specification packet.
  - Reworking TaskFlow defect epic structure beyond requiring correct defect routing.
  - Broad release-closure validation outside the happy-path catalog.

## Requirements

### Functional Requirements
- Must list happy-path cases from simplest to most complex.
- Must map every case to an existing or planned Rust proof target.
- Must stop the sequence on any happy-path failure until a defect is created or updated under the correct defect epic.
- Must require immediate repair through VIDA agent mode via `vida agent-init`; host-local edits are not the canonical repair path.
- Must include parent/child closure consistency as an early defect gate.
- Must treat the just-added graph validation proof as the first example of converting a happy-path contradiction into a defect gate.

### Non-Functional Requirements
- Performance
  - Basic operator surfaces should remain fast enough for smoke execution; slow projections should be classified as defects, not tolerated as normal happy-path behavior.
- Scalability
  - New cases must be append-only and keep simple-to-complex ordering.
- Observability
  - Each case must produce a clear command sequence, expected JSON or status fields, and a named Rust proof target.
- Security
  - The catalog must not weaken fail-closed behavior or treat missing receipts as successful execution.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/taskflow-happy-path-test-catalog-design.md`
  - `docs/product/spec/taskflow-happy-path-test-catalog-design.changelog.jsonl`
  - `docs/product/spec/README.md`
  - `docs/product/spec/current-spec-map.md`
- Framework protocols affected:
  - none directly in this packet
- Runtime families affected:
  - `taskflow`
  - `vida`
- Config / receipts / runtime surfaces affected:
  - TaskFlow task graph state and validation
  - run-graph dispatch and recovery artifacts
  - delegated lane execution receipts

## Design Decisions

### 1. The catalog is ordered by operator confidence
Will implement / choose:
- Start with command availability, then task CRUD, graph readiness, scheduling, packet/run-graph, delegated execution, consume/recovery, and final end-to-end closure.
- Why
  - Later happy-path cases depend on earlier surfaces being trustworthy.
- Trade-offs
  - The catalog is sequential by default even when some tests could run independently.
- Alternatives considered
  - Group by crate or command family only; rejected because it hides dependency order.

### 2. Happy-path failures become immediate defect work
Will implement / choose:
- On failure, create or update the matching defect task under the correct defect epic and repair it through `vida agent-init` before advancing.
- Why
  - The testing epic is only useful if it feeds the defect epic without drift or delayed triage.
- Trade-offs
  - The happy-path sequence may pause frequently during early hardening.
- Alternatives considered
  - Batch all failures after the full run; rejected because later failures can be artifacts of earlier unfixed defects.

## Technical Design

### Core Components
- Main components
  - TaskFlow CLI smoke coverage in `crates/vida/tests/task_smoke.rs`
  - VIDA bootstrap and delegated-runtime smoke coverage in `crates/vida/tests/boot_smoke.rs`
  - Task graph invariant validation in `crates/vida/src/state_store_task_graph.rs`
- Key interfaces
  - `vida task ... --json`
  - `vida taskflow ... --json`
  - `vida orchestrator-init --json`
  - `vida agent-init --json`
  - `vida status --json`
  - `vida doctor --json`
- Bounded responsibilities
  - This document owns ordering, routing, and proof-target mapping.
  - Rust tests own executable proof.
  - The defect epic owns failure remediation tasks.

### Happy-Path Catalog

| Order | Case | Operator path | Proof target |
| --- | --- | --- | --- |
| H1 | CLI availability and help/version surfaces | `vida --help`, `vida --version`, `vida task --help`, `vida taskflow help` | Existing/planned `crates/vida/tests/boot_smoke.rs` command-surface smoke tests |
| H2 | Basic task lifecycle | create task, show task, list tasks, update fields, close task | Existing `task_command_round_trip_succeeds_via_binary_surface` and `task_update_title_priority` in `crates/vida/tests/task_smoke.rs`; planned close-specific happy-path assertion if needed |
| H3 | Parent/child closure consistency defect gate | create parent/child rows, validate closed-parent/open-child and open-parent/no-open-child contradictions | Added `validate_task_graph_flags_closed_parent_with_open_child`, `validate_task_graph_flags_in_progress_parent_with_no_open_child`, and `validate_task_graph_accepts_parent_child_closure_consistent_rows` in `crates/vida/src/state_store_task_graph.rs` |
| H4 | Dependency readiness and blocked/ready projection | add dependency, inspect ready/blocked list, confirm second parent-child edge fails closed | Existing `dep_add_fails_closed_when_second_parent_child_edge_is_added`; planned ready/blocked happy-path JSON fixture in `crates/vida/tests/task_smoke.rs` |
| H5 | Graph summary and planning view | create small graph, inspect planning graph, validate critical/ready/blocked fields | Existing `task_create_update_close_round_trip_supports_planning_graph_views`; planned graph-summary happy-path fixture in `crates/vida/tests/task_smoke.rs` |
| H6 | Bootstrap/spec flow | initialize bounded spec/design task flow and verify design artifact routing | Existing `taskflow_bootstrap_spec_creates_epic_spec_task_and_design_doc` and related bootstrap/spec smoke coverage in `crates/vida/tests/boot_smoke.rs` |
| H7 | Scheduler preview | inspect sequential versus parallel-safe plan and max-parallel admission | Existing `taskflow_scheduler_dispatch_reports_preview_plan` and `taskflow_scheduler_dispatch_execute_smoke_reports_projection_truth_and_parallel_cap` in `crates/vida/tests/boot_smoke.rs` |
| H8 | Packet and run-graph preparation | dispatch init, render/latest packet, inspect run-graph and recovery status | Existing boot/run-graph smoke coverage in `crates/vida/tests/boot_smoke.rs`; added `taskflow_packet_latest_happy_path_selects_latest_run_graph_dispatch_packet` in `crates/vida/tests/boot_smoke.rs` for packet-latest selection |
| H9 | Agent-init delegated execution truth | agent init exposes executable packet state, receipt boundaries, and non-execution activation-view truth | Existing `agent_init_dispatch_packet_reports_view_only_activation_semantics` in `crates/vida/tests/boot_smoke.rs` |
| H10 | Consume/recovery closure path | consume final, consume continue, recovery latest, status/doctor projection alignment | Existing `taskflow_consume_final_renders_direct_runtime_consumption_snapshot`, `taskflow_consume_final_executes_ready_downstream_closure_step`, `taskflow_consume_continue_resumes_from_persisted_final_snapshot`, `taskflow_consume_continue_auto_picks_ready_downstream_packet`, and `taskflow_consume_continue_auto_executes_ready_downstream_taskflow_packet` in `crates/vida/tests/boot_smoke.rs` |
| H11 | Full golden route | new feature/spec task through packet, delegated handoff, proof admission, closure-ready state | Existing `taskflow_golden_route_happy_path_stitches_bootstrap_dispatch_resume_status_and_doctor` in `crates/vida/tests/boot_smoke.rs` |

### Data / State Model
- Important entities
  - Task rows, dependency edges, task graph validation issues, execution packets, run-graph state, recovery artifacts, delegated execution receipts, defect tasks.
- Receipts / runtime state / config fields
  - Each happy-path case must name the expected receipt or JSON field family before implementation.
  - Failures must record enough command output and state context for defect reproduction.
- Migration or compatibility notes
  - Existing tests remain valid; this catalog adds ordering and missing proof targets rather than renaming current tests.

### Integration Points
- APIs
  - CLI JSON surfaces under `vida` and `vida taskflow`.
- Runtime-family handoffs
  - TaskFlow produces graph state, packets, and recovery/consume artifacts.
  - Agent lanes perform immediate defect repair through canonical `vida agent-init` routing.
- Cross-document / cross-protocol dependencies
  - `AGENTS.sidecar.md` project working rules
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/release-1-proof-scenario-catalog.md`
  - `docs/product/spec/taskflow-execution-semantics-and-scheduler-design.md`

### Bounded File Set
- This specification packet:
  - `docs/product/spec/taskflow-happy-path-test-catalog-design.md`
  - `docs/product/spec/taskflow-happy-path-test-catalog-design.changelog.jsonl`
  - `docs/product/spec/README.md`
  - `docs/product/spec/current-spec-map.md`
- Future implementation packets may touch:
  - `crates/vida/tests/task_smoke.rs`
  - `crates/vida/tests/boot_smoke.rs`
  - `crates/vida/src/state_store_task_graph.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - Do not continue to a later happy-path case after a failure without defect routing and immediate agent-mode repair.
  - Do not treat `activation_view_only`, missing receipts, stale snapshots, or ambiguous parent/child closure projections as success.
  - Do not repair happy-path failures through root-session local edits unless the runtime explicitly authorizes an active exception takeover for the same bounded packet.
- Required receipts / proofs / gates
  - Each case must have a named Rust proof target before implementation closure.
  - Each discovered defect must have a defect epic task or update before repair starts.
  - Parent/child closure consistency must pass before consuming higher-level closure projections.
- Safety boundaries that must remain true during rollout
  - The defect epic cannot be closed while any child defect remains open.
  - A parent defect cannot remain open as if unresolved when all child defects are consistently closed and no blocker remains.

## Implementation Plan

### Phase 1
- Land this design doc and register it in the active spec map surfaces.
- First proof target
  - `vida docflow check --root . docs/product/spec/taskflow-happy-path-test-catalog-design.md docs/product/spec/current-spec-map.md`

### Phase 2
- Implement missing H1-H5 happy-path tests and align existing tests to the catalog vocabulary.
- Second proof target
  - `cargo test -p vida task_command_round_trip_succeeds_via_binary_surface task_update_title_priority dep_add_fails_closed_when_second_parent_child_edge_is_added`
  - `cargo test -p vida state_store_task_graph --lib`

### Phase 3
- Implement H6-H11 packet, agent-init, consume/recovery, and golden-route happy-path tests.
- Final proof target
  - targeted `cargo test -p vida` cases for bootstrap, packet, consume, recovery, status, and doctor happy paths.

## Validation / Proof
- Unit tests:
  - `validate_task_graph_flags_closed_parent_with_open_child`
  - `validate_task_graph_flags_in_progress_parent_with_no_open_child`
  - `validate_task_graph_accepts_parent_child_closure_consistent_rows`
- Integration tests:
  - Existing `crates/vida/tests/task_smoke.rs` happy-path cases listed in the catalog.
  - Existing `crates/vida/tests/boot_smoke.rs` bootstrap, packet, delegated runtime, consume/recovery, and golden-route cases.
- Runtime checks:
  - `vida orchestrator-init --json`
  - `vida agent-init --json`
  - `vida status --json`
  - `vida doctor --json`
- Canonical checks:
  - `activation-check`
  - `protocol-coverage-check`
  - `check`
  - `doctor`

## Observability
- Logging points
  - No new runtime logging in this specification packet.
- Metrics / counters
  - Future happy-path runner may report case count, passed count, failed count, defect-routed count, and repaired count.
- Receipts / runtime state written
  - Defect tasks and delegated repair receipts for every failure.
  - Test output artifacts for each Rust proof target.

## Rollout Strategy
- Development rollout
  - Use this catalog as the ordered plan for the next happy-path test implementation packets.
- Migration / compatibility notes
  - Existing smoke tests can be mapped into the catalog without renaming.
- Operator or user restart / restart-notice requirements
  - none

## Future Considerations
- Follow-up ideas
  - Add a machine-readable happy-path case registry if repeated runs need automated sequencing.
  - Add a single test harness that executes H1-H11 in order and emits defect-routing hints.
- Known limitations
  - This packet does not implement the missing tests.
- Technical debt left intentionally
  - Some proof targets are planned because current coverage is fragmented across smoke tests.

## References
- Related specs
  - `docs/product/spec/release-1-proof-scenario-catalog.md`
  - `docs/product/spec/taskflow-execution-semantics-and-scheduler-design.md`
  - `docs/product/spec/current-spec-map.md`
- Related protocols
  - `vida orchestrator-init`
  - `vida agent-init`
- Related ADRs
  - none
- External references
  - none

-----
artifact_path: product/spec/taskflow-happy-path-test-catalog-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-05-18
schema_version: 1
status: canonical
source_path: docs/product/spec/taskflow-happy-path-test-catalog-design.md
created_at: 2026-05-18T00:00:00Z
updated_at: 2026-05-18T00:00:00Z
changelog_ref: taskflow-happy-path-test-catalog-design.changelog.jsonl
