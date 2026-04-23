# Task Graph Adaptive Planner Design

Status: `canonical`

Purpose: define the bounded implementation wave that turns TaskFlow's existing graph and scheduler projection into a first practical task-planning and adaptive-replanning operator/runtime surface.

## Summary
- Feature / change: add deterministic PlanGraph generation/materialization, graph explain diagnostics, adaptive task mutation commands, scheduler dispatch preview, and task-linked execution-preparation artifact shape.
- Owner layer: `runtime-family`
- Runtime surface: `taskflow`
- Status: `canonical`

## Current Context
- The task graph store already owns parent-child edges, non-parent dependencies, ready/blocked projections, critical path, and graph validation.
- Task execution semantics already persist `execution_mode`, `order_bucket`, `parallel_group`, and `conflict_domain`.
- `vida taskflow graph-summary --json` already exposes scheduler projection, including ready and parallel-admission truth.
- Design-first bootstrap can create a canonical `epic -> spec-pack -> work-pool-pack -> dev-pack` chain.
- Run-level adaptive execution already supports `analysis -> writer -> coach -> verification -> approval/rework`.
- The missing layer is backlog-level planning and replanning: deterministic decomposition into bounded tasks, materialization with dependencies and execution semantics, graph explanation, split/spawn/resequence mutations, and scheduler dispatch policy.

## Goal
- Turn a feature/spec or audit report into a deterministic PlanGraph draft that can be reviewed, explained, and materialized into TaskFlow tasks.
- Reuse the existing task graph store as canonical truth instead of creating a second backlog model.
- Add adaptive replanning commands that produce bounded graph mutations for blockers, splits, scope deltas, and proof gaps.
- Add a scheduler dispatch surface that starts as dry-run/preview-first and can later become controlled multi-run launch.
- Keep graph validation and fail-closed execution-preparation gates stronger than automatic launch.
- Out of scope for this wave: fully autonomous multi-project planning, non-deterministic LLM-only task decomposition, and unbounded background task spawning without receipts.

## Requirements

### Functional Requirements
- `vida taskflow plan generate` must produce a deterministic PlanGraph draft with task ids, parent ids, dependencies, execution semantics, owned paths, acceptance targets, proof targets, risk, estimate, and lane hints.
- `vida taskflow plan materialize` must write that draft into the existing task store and return materialization receipts plus graph validation output.
- `vida taskflow graph explain` must explain why one task is ready, blocked, parallel-safe, not parallel-safe, on the critical path, or selected as the next lawful action.
- `vida taskflow replan` must consume latest evidence or explicit operator input and return a bounded mutation plan for split, spawn, resequence, reparent, rebuild, or defer actions.
- `vida task split` must split one oversized task into bounded children with validated dependencies and inherited execution semantics only where safe.
- `vida task spawn-blocker` must create a blocker/dependency task linked to the blocked source task with an explicit reason.
- `vida taskflow scheduler dispatch` must select one critical-path task plus compatible parallel-safe siblings up to `max_parallel_agents`, dry-run by default.
- Execution-preparation output must be task-linked and queryable either as persisted entities or explicit packet sections.

### Non-Functional Requirements
- All generated plans must be deterministic for the same input and runtime state.
- Existing task CRUD, graph validation, ready-set, blocked-set, and critical-path logic must remain the source of truth.
- Materialization must fail closed on invalid dependencies, missing execution semantics for parallel candidates, missing proof targets, or conflicting owned paths.
- Operator output must explain decisions using stable fields, not chat-only reasoning.
- The first implementation wave should prefer narrow command surfaces and reusable internal helpers over a large autonomous scheduler loop.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/task-graph-adaptive-planner-design.md`
  - `docs/product/spec/taskflow-execution-semantics-and-scheduler-design.md`
  - `docs/product/spec/execution-preparation-and-developer-handoff-model.md`
  - `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
- Framework protocols affected:
  - TaskFlow runtime-family command and packet protocols only if surfaced through registered runtime maps.
- Runtime families affected:
  - `taskflow`
- Config / receipts / runtime surfaces affected:
  - `.vida/data/state` task graph rows and receipts
  - `crates/vida/src/state_store_task_graph.rs`
  - `crates/vida/src/state_store_task_store.rs`
  - `crates/vida/src/task_surface.rs`
  - `crates/vida/src/taskflow_proxy.rs`
  - `crates/vida/src/taskflow_layer4.rs`

## Design Decisions

### 1. PlanGraph Draft Before Materialization
Will implement / choose:
- `plan generate` returns a draft first; `plan materialize` performs writes.
- Why: generation can be inspected, diffed, and rejected without mutating the backlog.
- Trade-offs: one extra command step, but safer operator truth and easier test coverage.
- Alternatives considered: immediate automatic task creation from any intake. Rejected for this wave because evidence and graph shape must remain inspectable.

### 2. Existing Task Graph Remains Canonical
Will implement / choose:
- PlanGraph is a temporary draft format that materializes into existing task rows and dependency edges.
- Why: TaskFlow already has graph validation, ready-set, blocked-set, critical path, and execution semantics.
- Trade-offs: draft fields must map cleanly to current task schema; richer future planner metadata may need task notes or packet artifacts first.
- Alternatives considered: a separate planner database. Rejected because it would split truth between planner and task store.

### 3. Preview-First Scheduler Dispatch
Will implement / choose:
- `scheduler dispatch` selects candidates and emits launch policy decisions, with `--execute` reserved for a controlled later step or explicit opt-in.
- Why: current spec intentionally shipped projection before full automatic concurrent dispatch; this wave should not silently widen execution authority.
- Trade-offs: parallel throughput remains operator-gated initially.
- Alternatives considered: immediate multi-agent auto-launch. Rejected until dispatch receipts and active-lane reconciliation are complete for multi-task runs.

### 4. Adaptive Replanning Uses Bounded Mutations
Will implement / choose:
- Replanning emits explicit mutation actions: `spawn_blocker_task`, `spawn_dependency_task`, `split_task`, `resequence_siblings`, `reparent_subtree`, `rebuild_work_pool`, `defer_obsolete_task`.
- Why: each mutation can be validated and receipt-backed.
- Trade-offs: not every ambiguous scope drift can be auto-fixed; some will return a recommendation blocker.
- Alternatives considered: free-form backlog rewrite. Rejected because it would make graph lineage hard to audit.

## Technical Design

### Core Components
- `TaskPlanGraphDraft`: transient draft containing task nodes, dependency edges, execution semantics, proof targets, owned paths, estimates, and lane hints.
- `TaskPlanNodeDraft`: one proposed task node before materialization.
- `TaskPlanEdgeDraft`: one proposed dependency edge with edge type and reason.
- `TaskPlanMaterializationReceipt`: result of writing draft nodes and edges into the task store.
- `TaskGraphExplainReport`: operator-facing explanation of readiness, blockers, parallel admission, and critical-path selection.
- `TaskReplanMutationPlan`: validated adaptive mutation proposal or applied mutation result.
- `TaskSchedulerDispatchPlan`: critical-path plus compatible parallel-safe candidate selection up to configured limits.

### Data / State Model
- Draft plans should be serializable JSON and optionally persisted as proof artifacts.
- Materialized tasks use existing task fields:
  - `id`
  - `title`
  - `description`
  - `priority`
  - `labels`
  - `parent_id`
  - dependency edges
  - `execution_mode`
  - `order_bucket`
  - `parallel_group`
  - `conflict_domain`
- Planner-only metadata can initially live in structured task notes or packet/proof artifacts:
  - `owned_paths`
  - `acceptance_target`
  - `proof_target`
  - `risk`
  - `estimate`
  - `lane_hint`
- Future migrations may promote these planner-only fields into first-class task schema columns after usage stabilizes.

### Integration Points
- CLI/taskflow:
  - `vida taskflow plan generate`
  - `vida taskflow plan materialize`
  - `vida taskflow graph explain`
  - `vida taskflow replan`
  - `vida taskflow scheduler dispatch`
- CLI/task:
  - `vida task split`
  - `vida task spawn-blocker`
- State store:
  - reuse `create_task`, `update_task`, `add_task_dependency`, `remove_task_dependency`, `reparent_children`, `validate_task_graph_rows`, `scheduling_projection_scoped`, and `critical_path`.
- Execution preparation:
  - attach architecture-preparation and developer-handoff artifact references to concrete graph nodes before implementation dispatch.

### Bounded File Set
- `docs/product/spec/task-graph-adaptive-planner-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `crates/vida/src/taskflow_layer4.rs`
- `crates/vida/src/taskflow_proxy.rs`
- `crates/vida/src/task_surface.rs`
- `crates/vida/src/state_store_task_graph.rs`
- `crates/vida/src/state_store_task_store.rs`
- `crates/vida/src/state_store_task_models.rs`
- Potential new module: `crates/vida/src/taskflow_plan_graph.rs`
- Potential new module: `crates/vida/src/taskflow_replanner.rs`

## Fail-Closed Constraints
- Do not treat a generated draft as materialized task truth.
- Do not dispatch a task whose dependencies are blocked, whose execution semantics are incomplete for parallel admission, or whose proof target is missing.
- Do not auto-launch more than one task unless the scheduler plan has explicit conflict-domain compatibility and a configured `max_parallel_agents` ceiling.
- Do not close a parent task while open child tasks, blocker tasks, stale replan requirements, or missing execution-preparation artifacts remain.
- Do not bypass existing graph validation for split, spawn-blocker, resequence, or materialization writes.
- Do not let graph explain become a separate decision engine; it must report decisions derived from canonical graph/projection data.

## Implementation Plan

### Phase 1
- Add PlanGraph draft types and deterministic `plan generate` / `plan materialize` surfaces.
- Materialize bounded child tasks and dependencies through existing task store APIs.
- First proof target: dry-run JSON, materialized graph, and `vida task validate-graph --json` pass.

### Phase 2
- Add `graph explain`, `task split`, and `task spawn-blocker`.
- Reuse existing scheduling projection and dependency inspection functions for explanations.
- Second proof target: ready, blocked, critical-path, parallel-safe, split, and blocker examples.

### Phase 3
- Add `taskflow replan` and `scheduler dispatch` preview.
- Add execution-preparation artifact linkage as task notes or packet sections if persisted entities are not yet available.
- Final proof target: replan produces bounded mutation receipts, dispatch preview respects `max_parallel_agents`, release build passes, and installed CLI smoke confirms help surfaces.

## Validation / Proof
- Unit tests:
  - PlanGraph draft generation is deterministic.
  - PlanGraph materialization rejects cycles and missing dependencies.
  - Graph explain reports blocker and parallel-admission reasons from canonical projection.
  - Split and spawn-blocker mutations preserve graph validity.
  - Scheduler dispatch preview respects conflict domains and `max_parallel_agents`.
- Integration tests:
  - `cargo test -p vida --no-run`
  - targeted taskflow planner/replanner tests in `crates/vida`
- Runtime checks:
  - `vida taskflow plan generate --json`
  - `vida taskflow plan materialize --json`
  - `vida taskflow graph explain --json`
  - `vida task split --json`
  - `vida task spawn-blocker --json`
  - `vida taskflow scheduler dispatch --dry-run --json`
  - `vida task validate-graph --json`
  - `vida taskflow graph-summary --json`
- Canonical checks:
  - `vida docflow finalize-edit docs/product/spec/task-graph-adaptive-planner-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md "record task graph adaptive planner design"`
  - `vida docflow check --root . docs/product/spec/task-graph-adaptive-planner-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md`

## Observability
- Plan generation emits draft id, source input summary, planned node count, planned edge count, and validation posture.
- Materialization emits created/updated task ids, dependency edges, skipped existing nodes, and graph validation result.
- Graph explain emits ready reasons, blocked reasons, parallel blockers, critical-path position, and next lawful action.
- Replan emits mutation kind, target task, reason, applied status, receipt path, and graph validation result.
- Scheduler dispatch emits selected critical-path task, selected parallel siblings, rejected candidates, and rejection reasons.

## Rollout Strategy
- Ship preview/draft surfaces before auto-execution.
- Keep materialization explicit and receipt-backed.
- Preserve backward compatibility for existing task graph rows.
- Use task notes or packet sections for planner-only metadata until schema promotion is justified.
- After release build, update the system-installed `vida` binary so operator surfaces are immediately available.

## Future Considerations
- Promote `owned_paths`, proof targets, risk, estimate, and lane hints into first-class task columns if planner usage stabilizes.
- Add controlled `scheduler dispatch --execute` only after multi-run active-lane reconciliation and recovery parity are proven.
- Add stronger evidence ingestion from verification outputs to auto-propose replan mutations.
- Add work-pool rebuild that turns one work-pool packet into a real executable pool with many tasks.
- Add planner scoring that incorporates cost, carrier/model profile, file ownership, proof density, and context-switching.

## References
- Related specs:
  - `docs/product/spec/taskflow-execution-semantics-and-scheduler-design.md`
  - `docs/product/spec/execution-preparation-and-developer-handoff-model.md`
  - `docs/product/spec/partial-development-kernel-model.md`
  - `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
  - `docs/product/spec/unified-hybrid-runtime-selection-policy-design.md`
- Related runtime code:
  - `crates/vida/src/state_store_task_graph.rs`
  - `crates/vida/src/state_store_task_store.rs`
  - `crates/vida/src/task_surface.rs`
  - `crates/vida/src/taskflow_proxy.rs`
  - `crates/vida/src/taskflow_layer4.rs`

-----
artifact_path: product/spec/task-graph-adaptive-planner-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-23
schema_version: 1
status: canonical
source_path: docs/product/spec/task-graph-adaptive-planner-design.md
created_at: 2026-04-23T11:27:37.608011388Z
updated_at: 2026-04-23T11:29:17.899160983Z
changelog_ref: task-graph-adaptive-planner-design.changelog.jsonl
