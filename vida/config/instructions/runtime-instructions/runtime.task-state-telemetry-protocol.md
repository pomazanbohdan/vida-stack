# Task-State Telemetry Protocol (MANDATORY)

Purpose: one operational contract for task-state SSOT, workflow telemetry, and execution visibility.

Transition note: `vida task`, `vida taskflow graph-summary`, and `vida taskflow run-graph` are active transitioned read surfaces; legacy `beads-workflow.sh` and companion wrappers remain migration-source operator helpers only until their sequencing behavior is reimplemented or retired.

## 1) SSOT Rule

`vida taskflow task` is the only source of truth for task lifecycle state.

Forbidden: editing markdown checkboxes (`[ ]`, `[x]`) as task state; using archived `tasks.md` as readiness source; tracking parallel task states outside the DB-backed TaskFlow task surface.

Required: find work via `vida taskflow task ready`; start with `vida taskflow task update <id> --status in_progress`; close with `vida taskflow task close <id> --reason "..."`; emit JSONL only as bounded export under `.vida/exports/` when an external snapshot is explicitly required; optional background backup worker must use sparse cadence (`>=120s`, default 600s):

```bash
bash beads-bg-sync.sh start --interval 600
bash beads-bg-sync.sh status
bash beads-bg-sync.sh stop
```

Autostart note: `beads-workflow.sh` attempts to auto-start a backup-only background worker at session entry commands (`ready|start|...`). Control via env: `VIDA_BG_SYNC_AUTOSTART=1|0` (default `1`); `VIDA_BG_SYNC_AUTOSTART_INTERVAL=<sec>` (default `600`, minimum enforced `120`).

## 2) Two-Layer Model

1. Task lifecycle/state: `vida taskflow task` over `.vida/state/taskflow-state.db` (`open`, `in_progress`, `closed`, `deferred`, deps).
2. Execution trace/visibility: TaskFlow blocks in beads logs (`block-plan/start/end/reflect/verify`).

Rule: TaskFlow is not a second task-state engine. It is execution telemetry only.

Reconciliation rule: when DB-backed lifecycle state and TaskFlow execution state diverge, use `runtime-instructions/work.task-state-reconciliation-protocol` to classify the task before mutating lifecycle state.

Wrapper rule:

1. Migration-only helper wrappers operate in JSONL-first mode while `beads_mutate` owns task writes.
2. Direct raw DB usage is diagnostic-only; lifecycle mutation stays on the task runtime surface.
3. All mutating task-state writes must pass through one runtime-owned single-writer path.

## 3) Daily Core Commands

```bash
vida taskflow task ready
vida taskflow task update <id> --status in_progress
vida taskflow task close <id> --reason "All ACs met"
vida taskflow task export-jsonl .vida/exports/tasks.snapshot.jsonl --json
```

Mutation serialization rule: read-only task commands may execute directly through the runtime helper; mutating task commands (`create|update|close`) must run through the runtime-owned task surface; if the runtime helper fails, stop with a blocker instead of retrying ad hoc from multiple lanes.

Status snapshots:

```bash
bash vida-status.sh [task_id]
bash taskflow-tool.sh board <task_id>
```

## 4) Workflow Wrapper (Canonical)

Use `beads-workflow.sh` for consistent logging and gates.

Main commands: `ready`, `start <id>`, `checkpoint <id> <done> <next> [risk]`; `redirect <id> <from_block_id> <to_block_id> <reason>` for user-driven scope/focus changes during active execution; `pack-start`, `pack-end`; `block-plan`, `block-start`, `block-end`; `block-finish` (compact close cycle: `block-end + reflect + verify`) and visible next-block status when sequential flow continues (`✅ done`, `🔄 active next`, or `ℹ️ planned next`); `reflect`, `verify`, `finish`, `sync`.

Execution contract: non-trivial work requires `block-plan` before execution; all work runs inside active block lifecycle; default done-block close path is `block-finish`; equivalent explicit path remains valid: `block-end -> reflect -> verify`; `next_step` must reference next block id (`-` only for terminal); auto-start of next block is allowed only within the same track; if user changes focus mid-execution, use `redirect` so source block closure and next active block stay explicit in telemetry; redirected source blocks are execution history, not pending backlog, and runtime TaskFlow views should surface them as `superseded`; implementation-shaped routes must persist the continuation packet at each `checkpoint`, `block-finish`, or equivalent resumable boundary; when a bounded leaf closes and the parent chain remains open, `block-finish` is incomplete until post-leaf rebuild persists lawful `next_step`/`next_leaf_id` or explicit blocker/escalation receipt; a closed leaf with open parent chain and no persisted continuation receipt is invalid telemetry and must fail closed.

Implementation continuation packet for telemetry/checkpoint surfaces: `task_id`, `delivery_task_id`, `execution_block_id`, `owned_paths` or equivalent write boundary, active node, `review_pool` or explicit verification target when applicable, `resume_hint`, control summary (`round_count`, `stall_count`, `reset_count`, `budget_units_consumed` when budgeted), current blocker or next-step reason when the task remains open.

Post-leaf continuation receipt for non-terminal chains: `parent_unit_id`, `closed_leaf_id`, `next_leaf_id` or explicit blocker/escalation marker, `selection_basis`, `proof_target_for_next_leaf` when a next leaf exists, `resume_hint`.

Auto-sync level:

1. Default `TASKFLOW_AUTO_SYNC_LEVEL=lean`.
2. `full` for debugging-heavy sessions.
3. `off` only for controlled manual sync scenarios.

Boot profile validation:

```bash
vida taskflow boot run lean <task_id>
vida taskflow boot verify-receipt <task_id> [profile]
```

Escalate to `standard|full` only when complexity/risk requires broader read-set.

## 5) Pack Coverage Contract

For non-trivial requests routed via use-case packs: run `pack-start` before block execution; run `pack-end` on completion; keep pack events balanced (`start == end`); treat balanced pack events as coverage telemetry only. Lawful pack completion is owned by `runtime-instructions/work.pack-completion-gate-protocol`.

## 6) Compact Contract

Use `beads-compact.sh` around context compaction:

```bash
bash beads-compact.sh pre <task_id> <done> <next> [risk]
bash beads-compact.sh post [task_after]
```

Rules:

1. Treat compact/clear as something that may happen at any moment during active execution, not only as a planned step.
2. `pre` is mandatory before planned compact/clear and strongly preferred before any risky long-running transition that may strand chat-only state.
3. `post` restores status view and records task drift (`task_before` vs `task_after`).
4. `pre` writes Context Capsule (`.vida/logs/context-capsules/<task_id>.json`) with epic/task goal linkage.
5. `post` must pass hydration gate via `context-capsule.sh hydrate <task_id>` before execution resumes.
6. If hydration fails, stop with blocker `BLK_CONTEXT_NOT_HYDRATED`.

## 6.1) Context Capsule Contract

Purpose: preserve global epic intent across compact/clear and restore deterministic execution context.

Required capsule fields: `epic_id`, `epic_goal`, `task_id`, `task_role_in_epic`, `done`, `next`, `constraints`, `open_risks`, `acceptance_slice`.

Additional required fields for implementation-shaped resumable work: `delivery_task_id`, `execution_block_id`, `review_pool` when active, `resume_hint`, `control_status`.

Operational hooks: write capsule on `block-finish` and compact `pre`; hydrate capsule on compact `post` before any task continuation; emit `context_capsule_written`, `context_hydrated`, `context_hydration_failed`, `context_drift_checked`; for implementation-shaped work, missing continuation-packet fields are hydration failure, not soft warning; for non-terminal chains, missing post-leaf continuation receipt is hydration failure, not soft warning.

## 7) Quality Gates

Before close/handoff: `bash quality-health-check.sh <task_id>` and `bash beads-workflow.sh verify <task_id>`.

Boundary note: close/handoff admissibility semantics remain owned by `runtime-instructions/work.execution-health-check-protocol`; stale/drift closure classification remains owned by `runtime-instructions/work.task-state-reconciliation-protocol`; this file keeps only the workflow wrapper path and SSOT/telemetry integration.

Finish gate: `finish` runs strict log checks; critical contradictions block finish; strict mode requires at least one `self_reflection` entry; when a task appears done-but-open or stale-in-progress, run `python3 task-state-reconcile.py status <task_id>` before closure/reopen decisions; when a leaf is marked closed but the represented task line remains open, finish/checkpoint/closure reporting must fail unless telemetry contains persisted next-leaf receipt or explicit blocker/escalation receipt.

## 8) Files

1. Execution log: `.vida/logs/beads-execution.jsonl`.
2. TaskFlow snapshot cache: `.vida/logs/taskflow-sync-<task_id>.json`.
3. State source: `.vida/state/taskflow-state.db`.

## 9) Optional Phase Gating

If phase gating is used, handle future work with `deferred` status and open by policy script.

Rule:

1. This does not replace `vida taskflow task ready`.
2. This does not introduce any second state model.

-----
artifact_path: config/runtime-instructions/task-state-telemetry.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: 2026-07-03T14:40:00+03:00
changelog_ref: runtime.task-state-telemetry-protocol.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: list-compaction+continuation-atom-preserve-exact+gate-preserve-exact
protocol_compression_baseline_ref: 3aefbd5b8:vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md
protocol_compression_audit_at: 2026-07-03T14:40:00+03:00
protocol_compression_before_tokens: 2413
protocol_compression_after_tokens: 2353
protocol_compression_content_sha256: 75379cce02132b951016fb559611e10358aa082e80c018675a3a330f2c57d0fc
