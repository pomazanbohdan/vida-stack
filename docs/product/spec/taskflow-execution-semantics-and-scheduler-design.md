# Taskflow Execution Semantics And Scheduler Design

Status: `implemented`

## Summary
- Feature / change: add first-class execution semantics to TaskFlow tasks and expose a scheduler projection that separates graph readiness from parallel-safe admissibility
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: `implemented`

## Current Context
- The canonical task graph already models hard ordering through dependency edges such as `blocks` and `parent-child`.
- Practical sequencing and parallelism were still drifting into notes, labels, and operator memory instead of explicit task fields.
- Existing readiness surfaces could tell whether a task was graph-ready, but they could not tell whether it was safe to run beside the current bounded task.

## Goal
- Keep the dependency graph as the source of truth for hard ordering.
- Add explicit task metadata for execution semantics so operators and runtime surfaces do not infer concurrency from notes.
- Expose a scheduler projection that answers `ready_now`, `ready_parallel_safe`, `blocked_by`, `active_critical_path`, and `parallel_candidates_after_current`.
- Out of scope: full automatic concurrent lane dispatch; this change establishes the canonical schema and operator/runtime projection first.

## Requirements

### Functional Requirements
- Task records must persist explicit execution-semantics fields:
  - `execution_mode`
  - `order_bucket`
  - `parallel_group`
  - `conflict_domain`
- Task create/update surfaces must support setting and clearing those fields.
- Task read surfaces must return those fields in canonical task JSON.
- `vida taskflow graph-summary --json` must expose a scheduler projection alongside graph summary data.
- Wave/order grouping must prefer `order_bucket` over legacy wave labels.

### Non-Functional Requirements
- Legacy tasks must remain readable without migration breakage.
- Missing semantics must fail closed for parallel-safe admission.
- Snapshot import/export must remain backward-compatible by defaulting execution semantics for legacy rows.
- Proof must cover legacy fail-closed behavior and explicit compatible/incompatible parallel-safe cases.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/taskflow-execution-semantics-and-scheduler-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `taskflow`
- Config / receipts / runtime surfaces affected:
  - task store schema in `crates/vida/src/state_store_task_models.rs`
  - task mutation/read surfaces in `crates/vida/src/{cli,task_surface,taskflow_proxy}.rs`
  - scheduler projection in `crates/vida/src/state_store_task_graph.rs`

## Design Decisions

### 1. Graph Truth Remains Canonical
Will implement / choose:
- Dependency edges remain the only authority for hard ordering and blocking.
- Execution semantics are additive operator/runtime metadata, not a replacement for the graph.
- Trade-off: this keeps existing graph law stable, but means parallel-safe admission needs a second projection layer.
- Alternatives considered: overloading labels/notes or replacing graph edges with stage metadata. Rejected because both approaches hide runtime truth and increase drift.

### 2. Fail-Closed Parallel Admission
Will implement / choose:
- `execution_mode` supports `sequential`, `parallel_safe`, and `exclusive`.
- `parallel_safe` admission requires explicit compatibility with the current task.
- Compatibility rules in this bounded change:
  - both tasks must be `parallel_safe`
  - both must share the same explicit `order_bucket`
  - both must provide explicit `conflict_domain`, and those values must differ
  - `parallel_group` is optional, but if either side sets it then both must set the same value
- Trade-off: this is intentionally conservative and may under-admit some safe pairs until future refinement.
- Alternatives considered: treating missing fields as permissive defaults. Rejected because the user goal is to stop implicit parallelism.

### 3. Projection Before Full Dispatch Automation
Will implement / choose:
- First ship the canonical schema plus scheduler projection in `graph-summary`.
- Keep current dispatch flow unchanged and let operator/runtime consumers read the richer projection first.
- Trade-off: the runtime does not yet auto-dispatch multiple tasks in parallel, but it now has canonical data to do so safely later.
- Alternatives considered: wiring direct multi-lane dispatch immediately. Rejected as too wide for one bounded change.

## Technical Design

### Core Components
- `TaskExecutionSemantics`: canonical persisted metadata attached to each task record
- `TaskSchedulingCandidate`: per-task scheduler view with readiness and parallel-admission fields
- `TaskSchedulingProjection`: top-level graph-plus-semantics projection returned to operator surfaces

### Data / State Model
- New task fields:
  - `execution_mode: Option<String>`
  - `order_bucket: Option<String>`
  - `parallel_group: Option<String>`
  - `conflict_domain: Option<String>`
- Compatibility:
  - task JSONL and store rows deserialize with defaults
  - snapshot import/export defaults missing execution semantics to empty values
- Validation:
  - `execution_mode` accepts only `sequential`, `parallel_safe`, or `exclusive`
  - empty CLI values normalize to `None`

### Integration Points
- CLI:
  - `vida task create --execution-mode --order-bucket --parallel-group --conflict-domain`
  - `vida task update` supports the same setters plus clear flags
- State store:
  - create/update validation and persistence
  - scheduling projection over graph readiness plus semantics compatibility
- Operator surfaces:
  - `vida task show --json` now includes execution semantics via `TaskRecord`
  - `vida taskflow graph-summary --json` now includes `current_task_id` and `scheduling`

### Bounded File Set
- `docs/product/spec/taskflow-execution-semantics-and-scheduler-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `crates/vida/src/state_store_task_models.rs`
- `crates/vida/src/state_store_task_store.rs`
- `crates/vida/src/state_store_task_graph.rs`
- `crates/vida/src/state_store_taskflow_snapshot_codec.rs`
- `crates/vida/src/cli.rs`
- `crates/vida/src/task_surface.rs`
- `crates/vida/src/taskflow_proxy.rs`

## Fail-Closed Constraints
- Missing execution semantics must never imply parallel-safe admission.
- `parallel_safe` without explicit compatible classifiers remains non-admissible.
- Graph blocking still wins over execution semantics; a blocked task is never promoted by metadata.
- Legacy snapshots/tasks remain readable, but they default to non-parallel-safe until explicitly updated.

## Implementation Plan

### Phase 1
- Extend canonical task schema and persistence
- Add CLI/task mutation surfaces
- Proof target: compile-safe persistence and readback

### Phase 2
- Add scheduler projection and order-bucket-aware wave grouping
- Proof target: explicit compatible and incompatible parallel-safe cases

### Phase 3
- Update maps and canonical references
- Proof target: docflow and cargo verification

## Validation / Proof
- Unit tests:
  - `state_store::state_store_task_graph::tests::scheduling_projection_fail_closes_when_semantics_are_missing`
  - `state_store::state_store_task_graph::tests::scheduling_projection_allows_only_compatible_parallel_safe_tasks`
  - `taskflow_proxy::tests::graph_summary_waves_prefer_order_bucket_over_labels`
- Integration tests:
  - bounded compile/test coverage through `cargo test -p vida --no-run`
- Runtime checks:
  - inspect `vida task show --json`
  - inspect `vida taskflow graph-summary --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/taskflow-execution-semantics-and-scheduler-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md`

## Observability
- Scheduler projection emits:
  - chosen `current_task_id`
  - per-task `parallel_blockers`
  - `parallel_candidates_after_current`
- Graph summary still emits ready/blocked counts and critical path.

## Rollout Strategy
- Backward-compatible rollout because legacy tasks deserialize with empty execution semantics.
- Operators can start annotating tasks incrementally without snapshot migration.
- No restart/migration gate is required beyond rebuilding the `vida` binary.

## Future Considerations
- Feed scheduler projection directly into dispatch/routing decisions, not only graph-summary output.
- Add richer conflict-domain taxonomy or typed enums if usage stabilizes.
- Consider a dedicated `vida task scheduling` surface once the projection grows beyond graph-summary.

## References
- Local references:
  - `crates/vida/src/state_store_task_models.rs`
  - `crates/vida/src/state_store_task_graph.rs`
  - `crates/vida/src/taskflow_proxy.rs`
- External references:
  - OpenAI Function Calling guide: https://developers.openai.com/api/docs/guides/function-calling
  - OpenAI Agents SDK orchestration guide: https://openai.github.io/openai-agents-python/multi_agent/
  - Anthropic Parallel tool use: https://platform.claude.com/docs/en/agents-and-tools/tool-use/parallel-tool-use
  - Microsoft Semantic Kernel function invocation: https://learn.microsoft.com/en-us/semantic-kernel/concepts/ai-services/chat-completion/function-calling/function-invocation
  - Microsoft Agent Framework workflows: https://learn.microsoft.com/en-us/agent-framework/journey/workflows

-----
artifact_path: product/spec/taskflow-execution-semantics-and-scheduler-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-17
schema_version: 1
status: canonical
source_path: docs/product/spec/taskflow-execution-semantics-and-scheduler-design.md
created_at: 2026-04-17T07:31:17.227818559Z
updated_at: 2026-04-17T08:02:54.481753542Z
changelog_ref: taskflow-execution-semantics-and-scheduler-design.changelog.jsonl
