# Beads Protocol (MANDATORY)

Purpose: one operational contract for task state + execution visibility.

## 1) SSOT Rule

`br` is the only source of truth for task lifecycle state.

Forbidden:

1. Editing markdown checkboxes (`[ ]`, `[x]`) as task state.
2. Using archived `tasks.md` as readiness source.
3. Tracking parallel task states outside `br`.

Required:

1. Find work via `br ready`.
2. Start with `br update <id> --status in_progress`.
3. Close with `br close <id> --reason "..."`.
4. Flush at checkpoints/session end: `br sync --flush-only`.
5. Optional background backup worker must use sparse cadence (`>=120s`, default 600s):

```bash
bash _vida/scripts/beads-bg-sync.sh start --interval 600
bash _vida/scripts/beads-bg-sync.sh status
bash _vida/scripts/beads-bg-sync.sh stop
```

Autostart note:

1. `beads-workflow.sh` attempts to auto-start a backup-only background worker at session entry commands (`ready|start|...`).
2. Control via env:
   - `VIDA_BG_SYNC_AUTOSTART=1|0` (default `1`)
   - `VIDA_BG_SYNC_AUTOSTART_INTERVAL=<sec>` (default `600`, minimum enforced `120`).

## 2) Two-Layer Model

1. Task lifecycle/state: `br` in JSONL-first mode (`open`, `in_progress`, `closed`, `deferred`, deps).
2. Execution trace/visibility: TODO blocks in beads logs (`block-plan/start/end/reflect/verify`).

Rule: TODO is not a second task-state engine. It is execution telemetry only.

Wrapper rule:

1. `_vida/*` wrappers operate in JSONL-first mode while `beads_mutate` owns task writes.
2. Direct `br`/SQLite usage is diagnostic-only until the mutator path is fully retired.

## 3) Daily Core Commands

```bash
br ready
br update <id> --status in_progress
br close <id> --reason "All ACs met"
br sync --flush-only
```

Status snapshots:

```bash
bash _vida/scripts/vida-status.sh [task_id]
bash _vida/scripts/todo-tool.sh board <task_id>
```

## 4) Workflow Wrapper (Canonical)

Use `_vida/scripts/beads-workflow.sh` for consistent logging and gates.

Main commands:

1. `ready`, `start <id>`, `checkpoint <id> <done> <next> [risk]`
2. `redirect <id> <from_block_id> <to_block_id> <reason>` for user-driven scope/focus changes during active execution
2. `pack-start`, `pack-end`
3. `block-plan`, `block-start`, `block-end`
4. `block-finish` (compact close cycle: `block-end + reflect + verify`)
4.1. `block-finish` should emit visible next-block status when sequential flow continues (`✅ done`, `🔄 active next`, or `ℹ️ planned next`).
5. `reflect`, `verify`, `finish`, `sync`

Execution contract:

1. Non-trivial work: `block-plan` before execution.
2. All work runs inside active block lifecycle.
3. Default close path for done blocks: `block-finish`.
3.1. Equivalent explicit path remains valid: `block-end -> reflect -> verify`.
4. `next_step` must reference next block id (`-` only for terminal).
5. Auto-start of next block is allowed only within the same track.
6. If user changes focus mid-execution, use `redirect` instead of ad hoc partial logging so source block closure and next active block remain explicit in telemetry.
7. Redirected source blocks are execution history, not pending backlog. Runtime TODO views should surface them as `superseded`.

Auto-sync level:

1. Default `TODO_AUTO_SYNC_LEVEL=lean`.
2. `full` for debugging-heavy sessions.
3. `off` only for controlled manual sync scenarios.

Boot profile validation:

```bash
bash _vida/scripts/boot-profile.sh run lean <task_id>
bash _vida/scripts/boot-profile.sh verify-receipt <task_id> [profile]
```

Escalate to `standard|full` only when complexity/risk requires broader read-set.

## 5) Pack Coverage Contract

For non-trivial requests routed via use-case packs:

1. run `pack-start` before block execution,
2. run `pack-end` on completion,
3. keep pack events balanced (`start == end`).

## 6) Compact Contract

Use `_vida/scripts/beads-compact.sh` around context compaction:

```bash
bash _vida/scripts/beads-compact.sh pre <task_id> <done> <next> [risk]
bash _vida/scripts/beads-compact.sh post [task_after]
```

Rules:

1. `pre` is mandatory before compact/clear.
2. `post` restores status view and records task drift (`task_before` vs `task_after`).
3. `pre` writes Context Capsule (`.vida/logs/context-capsules/<task_id>.json`) with epic/task goal linkage.
4. `post` must pass hydration gate via `context-capsule.sh hydrate <task_id>` before execution resumes.
5. If hydration fails, stop with blocker `BLK_CONTEXT_NOT_HYDRATED`.

## 6.1) Context Capsule Contract

Purpose: preserve global epic intent across compact/clear and restore deterministic execution context.

Required capsule fields:

1. `epic_id`, `epic_goal`
2. `task_id`, `task_role_in_epic`
3. `done`, `next`
4. `constraints`
5. `open_risks`
6. `acceptance_slice`

Operational hooks:

1. Write capsule on `block-finish` and compact `pre`.
2. Hydrate capsule on compact `post` before any task continuation.
3. Emit telemetry events: `context_capsule_written`, `context_hydrated`, `context_hydration_failed`, `context_drift_checked`.

## 7) Quality Gates

Before close/handoff:

1. `bash _vida/scripts/quality-health-check.sh <task_id>`.
2. `bash _vida/scripts/beads-workflow.sh verify <task_id>`.

Finish gate:

1. `finish` runs strict log checks.
2. If critical contradictions exist, finish is blocked.
3. At least one `self_reflection` entry is required in strict mode.

## 8) Files

1. Execution log: `.vida/logs/beads-execution.jsonl`.
2. TODO snapshot cache: `.vida/logs/todo-sync-<task_id>.json`.
3. State source: `.beads/issues.jsonl`.

## 9) Optional Phase Gating

If phase gating is used, handle future work with `deferred` status and open by policy script.

Rule:

1. This does not replace `br ready`.
2. This does not introduce any second state model.
