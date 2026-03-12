# TaskFlow Protocol (Execution Layer)

Purpose: decompose user requests into executable step-level work while keeping `br` as the single source of truth for task state.

Output policy:

1. Human-facing `taskflow-v0` runtime commands default to `TOON`.
2. Structured JSON output is enabled only through explicit `--json`.
3. New runtime surfaces must not make raw JSON the default human-facing output.

## 1) Layer Model

1. `Intent Layer`: user request (goal, constraints, acceptance).
2. `Work Layer`: `br` issue lifecycle (`open/in_progress/closed`).
3. `Execution Layer`: TaskFlow steps/tracks for actual implementation.

Rule: `br` tracks "what"; TaskFlow tracks "how".

Task-state truth rule:

1. When task lifecycle state and execution telemetry appear out of sync, use `vida/config/instructions/runtime-instructions/work.task-state-reconciliation-protocol.md` as the canonical reconciliation layer before closing, reopening, or declaring the task stale.

Hard rule:

1. No execution without active TaskFlow block.
2. Any implementation or research step MUST be enclosed by active TaskFlow block lifecycle.
3. For done blocks, use `block-finish` by default. For partial/failed blocks, use `block-end` and then `reflect`/`verify`.
4. Before non-trivial execution, pre-register planned blocks via `block-plan` so board reflects total planned scope.
5. For command-layer documentation audits, pre-register one TaskFlow block per protocol unit (`/vida-*#CLx`) and keep the pending list visible.
6. Every planned block should include `next_step` (block_id or `-` for terminal step).
7. Use rolling-window planning: keep 2-3 upcoming planned blocks visible; expand further plan just-in-time.
8. When the same technical error repeats twice inside an active block, the block must record an escalation event and switch to `vida/config/instructions/diagnostic-instructions/escalation.debug-escalation-protocol.md` before the next substantive fix attempt.
9. If worker mode is active and an eligible catch/review lane exists, dispatch that external diagnostic lane in parallel with escalation lookup instead of debugging alone.
10. Escalation evidence must capture which outside inputs were used: `external_agent`, `primary_source`, `web/google`, or explicit `not_available` receipt.
11. Completed write-producing slices must receive catch/review coverage when an eligible lane exists; if not, record a protocol-drift finding and correct it.
12. Progress reporting must not interrupt lawful continuation by itself when continuous autonomous execution is active.
13. If overlay enables spec-ready auto development, TaskFlow may enter the next implementation-bearing task without a new user prompt only through the lawful execution-entry gates.
14. If overlay requires validation before implementation, TaskFlow must treat the pre-implementation report as a gating artifact rather than an informational progress report.
15. If overlay enables resume-after-validation, accepted validation should return TaskFlow to autonomous continuation for the same lawful task chain.

## 1.1) Diagnostic Integration Boundary

1. TaskFlow may carry deferred diagnostic evidence only as tracked execution data.
2. Silent framework diagnosis policy, capture rules, and reflection criteria are owned by `vida/config/instructions/diagnostic-instructions/analysis.silent-framework-diagnosis-protocol.md`.
3. This protocol owns only the execution-side requirement that deferred diagnostic follow-up, when active, must survive through canonical task/evidence surfaces rather than chat memory.

## 2) Decomposition + Clustering Algorithm

This algorithm is mandatory for non-trivial work (3+ steps).

1. `Q-Gate`: collect user decisions before decomposition.
   - Run focused question cards (scope boundary, delivery cut, dependency strategy, risk policy).
   - Record selected options and constraints as execution evidence.
2. `Conflict-Gate`: check decision compatibility.
   - If conflicts exist, resolve them before building TaskFlow plan.
3. Attach to existing `TaskFlow task or create a new one.
4. Build execution clusters:
   - split work into 15-90 minute steps with measurable outputs;
   - each step must have acceptance/evidence intent;
   - each step must declare `depends_on` and `next_step`.
5. Decide routing per step (`sequential` vs `parallel`) using the track gate:
   - `parallel` allowed only when there is no output dependency, no shared writable scope, and no contract coupling;
   - otherwise force `sequential` on `track_id=main`.
6. Pre-register planned steps via `block-plan`.
7. Validate plan integrity before execution (`todo-plan-validate.sh`; use `--diff-aware` when the worktree already contains target-scope changes). Diff-aware validation must accept coverage from the full task plan, not only remaining non-done blocks, so completed blocks do not create false drift failures.
8. Execute with evidence + verification at each step.

### 2.1 Q-Gate Output Contract

Minimum output fields to carry into planning:

1. `scope_boundary`.
2. `delivery_cut`.
3. `dependency_strategy`.
4. `risk_policy`.
5. `open_conflicts` (must be empty before execution).

### 2.2 Sequential/Parallel Decision Matrix

Choose `parallel` only if ALL are true:

1. no step depends on another step output,
2. no shared writable files/directories,
3. no shared mutable API/data contract in-flight.

If at least one condition fails, use sequential chain (`next_step`) on `main` track.

## 3) Parallel Tracks Mode (Workers)

Use this mode when 2+ independent chunks can run concurrently.

Track schema:

1. `track_id`: `main`, `A`, `B`, `C`, ...
2. `owner`: `orchestrator` or `agent:<id>`
3. `scope`: allowed files/directories
4. `depends_on`: upstream step IDs
5. `verify`: command proving completion
6. `merge_ready`: `yes/no`

Constraints:

1. Avoid overlapping writable scopes across active tracks.
2. If overlap is required, serialize by dependency order.
3. Merge only after per-track verify passes.
4. Default track is `main`; default owner is `orchestrator`.

## 4) TaskFlow Step Definition

Required fields:

1. `step_id` (`S01`, `S02`, ...)
2. `task_id` (`br` issue id)
3. `goal`
4. `status` (`todo|doing|done|blocked`)
5. `acceptance_check`
6. `evidence_ref`
7. `next_step`
8. `risk`

Optional parallel fields:

1. `track_id`
2. `owner`
3. `depends_on`
4. `merge_ready`

## 5) Operational Commands

Transition note:

1. the block-lifecycle command examples below are legacy wrapper examples,
2. transitioned runtime reads now live under `taskflow-v0 task`, `taskflow-v0 todo`, and `taskflow-v0 run-graph`,
3. wrapper retirement is tracked by `vida/config/instructions/system-maps/migration.runtime-transition-map.md`.

Transition verification baseline:

1. use this as the minimum proof before close or handoff on transitioned runtime slices:
   - `nim c taskflow-v0/src/vida.nim`
   - `nim c -r taskflow-v0/tests/test_boot_profile.nim`
   - `nim c -r taskflow-v0/tests/test_worker_packet.nim`
   - `nim c -r taskflow-v0/tests/test_kernel_runtime.nim`
2. `vida/config/instructions/system-maps/migration.runtime-transition-map.md` remains the migration registry only and must not keep this verification law as a competing owner.

Start task:

```bash
bash beads-workflow.sh start <task_id>
bash beads-workflow.sh pack-start <task_id> <pack_id> "goal" [constraints]
bash beads-workflow.sh redirect <task_id> <from_block_id> <to_block_id> "reason"
bash beads-workflow.sh block-plan <task_id> <block_id> "goal" [track_id] [owner] [depends_on]
```

Track-aware step logging:

```bash
bash beads-workflow.sh block-start <task_id> <block_id> "goal" [track_id] [owner] [depends_on]
bash beads-workflow.sh block-finish <task_id> <block_id> <done|partial|failed> "next" "actions" [artifacts] [risks] [assumptions] [evidence] [confidence]
bash beads-workflow.sh block-end <task_id> <block_id> <done|partial|failed> "next" "actions" [artifacts] [risks] [assumptions] [evidence] [track_id] [owner] [merge_ready]
```

Reflection + quality gate (manual path):

```bash
bash beads-workflow.sh reflect <task_id> "goal" "constraints" "evidence" "decision" "risks" "next" [confidence]
bash beads-workflow.sh verify <task_id>
bash quality-health-check.sh <task_id>
```

Finish:

```bash
bash beads-workflow.sh finish <task_id> "reason"
```

TaskFlow interface commands (derived view from execution log):

```bash
bash taskflow-tool.sh board <task_id>
bash taskflow-tool.sh compact <task_id> [limit]
bash taskflow-tool.sh list <task_id>
bash taskflow-tool.sh current <task_id>
bash taskflow-tool.sh next <task_id>
bash taskflow-tool.sh ui-json <task_id>
bash taskflow-sync-plan.sh <task_id>
bash taskflow-sync-plan.sh <task_id> --mode compact --max-items 3
bash taskflow-sync-plan.sh <task_id> --mode delta
bash taskflow-sync-plan.sh <task_id> --mode json-only --quiet
bash taskflow-overhead-report.sh <task_id>
bash todo-plan-validate.sh <task_id> [--strict] [--quiet] [--diff-aware] [--base REF]
bash vida-command-audit.sh report <task_id>
bash vida-command-audit.sh plan <task_id> [--limit N]
bash vida-command-audit.sh repair-next <task_id>
```

Framework-only lean starter:

```bash
bash framework-wave-start.sh <task_id> <pack_id> "<goal>" [constraints]
```

Use only as a migration-only helper for framework-owned legacy wrapper flow. It preserves:

1. `br` as SSOT,
2. pack logging,
3. TaskFlow scaffolding/validation,
4. boot-profile validation.

Command audit mode:

1. Run `bash vida-command-audit.sh report <task_id>` to see done/pending coverage.
2. Run `bash vida-command-audit.sh plan <task_id>` to pre-register missing protocol-unit analysis blocks.
3. Execute protocol-unit analyses sequentially (`block-start` -> `block-end`) to keep pending list accurate.
4. Before reporting to user, confirm `board` and `report` snapshots are up to date.
5. Sequential automation: after `block-end ... done <next_block_id> ...`, workflow auto-starts `<next_block_id>` if it exists and is `todo`.
6. Auto-start is track-safe: it runs only when source and target blocks are in the same `track_id`.
7. If old plans have broken `next_step`, run `repair-next` to rebuild canonical `CMDxx` chain.
8. `block-start` may reopen a previously ended block (clears previous end marker in TaskFlow view and sets status back to `doing`).
9. When execution focus changes because of a new user instruction, use `beads-workflow.sh redirect` so the current block is closed as `partial` and the replacement block becomes the active `doing` step in one canonical path.
10. Redirected/superseded blocks must not return to the active TaskFlow backlog; runtime views should surface them as `superseded` instead of `taskflow`.
11. When more than one next task/block appears lawful, apply `vida/config/instructions/runtime-instructions/work.execution-priority-protocol.md` before choosing or redirecting.

Protocol-unit rule:

1. When command decomposition work is planned or delegated, refer to units as `<command>#CL1..CL5`.
2. `CL1`, `CL2`, and read-heavy `CL3` work is delegation-friendly.
3. `CL4` stays single-writer unless explicit write isolation exists.
4. `CL5` may delegate evidence collection, but orchestrator owns final gate decisions.

UI sync rule:

1. TaskFlow UI reads from `taskflow-tool.sh ui-json <task_id>`.
2. Source of truth remains execution events in `.vida/logs/beads-execution.jsonl`.
3. Never mutate TaskFlow state directly in UI without writing corresponding execution events.
4. Use `taskflow-sync-plan.sh <task_id>` to generate a deterministic checklist snapshot for UI mirroring.
5. `beads-workflow.sh` auto-runs `taskflow-sync-plan.sh` in `json-only` mode according to `TASKFLOW_AUTO_SYNC_LEVEL`.
5.1. `TASKFLOW_AUTO_SYNC_LEVEL=lean` (default): sync on `start`, `block-start`, `block-end`, `finish`.
5.2. `TASKFLOW_AUTO_SYNC_LEVEL=full`: sync on all workflow mutations.
5.3. `TASKFLOW_AUTO_SYNC_LEVEL=off`: disable auto sync (manual sync only).
6. For user-visible progress updates, prefer compact/delta snapshots over full snapshot.
7. Completion report order is mandatory: `sync -> confirm board/compact -> report done`.
8. Pack coverage: each non-trivial flow should have balanced `pack-start` and `pack-end` events; lawful pack completion claims are owned by `vida/config/instructions/runtime-instructions/work.pack-completion-gate-protocol.md`.
9. Response visibility rule: when reporting task/todo state to user, include IDs and concise descriptions (not IDs only).
10. `quality-health-check` cadence: run on checkpoint boundaries, pre-handoff, and finish (not after every micro-step).
11. Runtime scripts should be quiet-by-default for progress chatter; keep human-facing status output in `taskflow-tool current|list` and `quality-health-check`.
12. Use `python3 task-state-reconcile.py status <task_id>` when a task looks done-but-open, stale-in-progress, or otherwise drifted between `br` and TaskFlow.

Background worker policy (token/cost aware):

1. For continuous live backup without chat noise, use tmux-managed worker:

```bash
bash beads-bg-sync.sh start --interval 600
bash beads-bg-sync.sh status
bash beads-bg-sync.sh stop
```

2. Default interval is 600 sec (10 min).
3. Do not use aggressive intervals below 120 sec in normal workflow.
4. Prefer event-driven sync (`beads-workflow` auto-sync) + sparse background JSONL snapshots over high-frequency polling.

Silent diagnosis execution persistence:

1. If silent diagnosis is active and a framework gap was already captured, `reflect`/`finish` should reference the capture artifact or resulting framework task id.
2. This protocol owns only execution-side persistence of that capture in TaskFlow evidence and context capsules.
3. Silent diagnosis policy, capture timing, and follow-up routing remain owned by `vida/config/instructions/diagnostic-instructions/analysis.silent-framework-diagnosis-protocol.md`.

## 6) Gates

0. Plan gate: non-trivial work must pass `Q-Gate` + `Conflict-Gate` before execution materialization.
0.1. Tool-capability gate: for non-trivial flows, resolve required tool fallbacks and record evidence when fallback is used.
0.2. If a task enters bounded conflict-discussion mode via `vida/config/instructions/runtime-instructions/work.problem-party-protocol.md`, record the board artifact path in block evidence before resuming normal execution.
1. Step gate: `block-end` requires evidence or artifacts.
1.1. If WVP trigger fired, `block-end`/`reflect` evidence must include WVP markers per `vida/config/instructions/runtime-instructions/work.web-validation-protocol.md`.
2. Track gate: each parallel track must pass verify.
3. Task gate: strict verify + self-reflection required before close.
3.1. Pack completion gate: a pack-complete claim is lawful only through `vida/config/instructions/runtime-instructions/work.pack-completion-gate-protocol.md`.
4. Compact gate: always record `compact_pre` and `compact_post`.
4.1. Drift gate: run `bash context-drift-sentinel.sh check <task_id>` after capsule write checkpoints (`block-finish`, compact restore).
4.2. If silent framework diagnosis is active and a framework gap was detected, compact-safe evidence must include the capture artifact path or follow-up framework task id.
5. Execution gate: if no active block exists, execution must not proceed.
6. Plan integrity gate: run `bash todo-plan-validate.sh <task_id>` after `block-plan` batch and before execution start. Use `--diff-aware` when the worktree already contains target-scope changes; coverage is evaluated against the whole task plan so already-completed blocks still count.
6.1. For framework-only tasks, compact evidence is valid when work is confined to migration-only helper surfaces and the block records concrete actions plus canonical artifacts or task IDs. Runtime verification may downgrade missing artifact warnings to informational severity for these tasks in non-strict mode.
6.2. Silent diagnosis gate: when active, task closure is invalid if a detected framework gap was only discussed in chat and not captured in canonical execution evidence, context capsule, or framework task state.

## 7) Anti-Patterns

1. Running multiple writable tracks over the same files without dependencies.
2. Closing `TaskFlow task without TaskFlow evidence and strict verify.
3. Tracking execution only in chat without structured log entries.

## 8) Blocked/Unblocked Algorithm

When a task must pause because another task is now the active dependency:

1. Add dependency: `br dep add <blocked_task_id> <active_task_id>`.
2. Set blocked status: `br update <blocked_task_id> --status blocked`.
3. Record reason in execution log (`checkpoint` or `block-end` risk/next_step fields).
4. Continue work only on the active dependency task.

When dependency is complete and task becomes runnable again:

1. Reopen status: `br update <blocked_task_id> --status open`.
2. Verify dependency state with `br show <blocked_task_id> --json`.
3. Pick next work via `br ready` (unblocked-first rule).
4. Start resumed task explicitly: `br update <id> --status in_progress`.

## 9) Execution Mode (Decision vs Autonomous)

Per-task execution mode must be explicit:

1. `decision_required`:
   - Assistant performs analysis/options.
   - User confirms key decisions before implementation edits.
2. `autonomous`:
   - Assistant executes implementation end-to-end inside agreed scope.
   - Checkpoints are still logged, but no per-step approval required.

Mode operations:

```bash
bash task-execution-mode.sh get <task_id>
bash task-execution-mode.sh recommend <task_id>
bash task-execution-mode.sh set <task_id> <decision_required|autonomous> [reason]
```

Routing rule:

1. Documentation/research-heavy tasks default to `decision_required`.
2. Implementation-heavy tasks (feature/bug execution) default to `autonomous` unless user overrides.

## 9.1) User Escalation Gate

Autonomous execution does not authorize silent product or contract choices.

Escalate to the user and pause implementation when at least one trigger is true:

1. more than one plausible product/UX behavior fits the evidence,
2. a fix changes navigation, auth, destructive data behavior, or user-facing semantics beyond the agreed slice,
3. live API/server reality contradicts the request or prior contract,
4. root-cause confidence is below 80% and different fixes have materially different outcomes,
5. the task must expand in scope/order/risk beyond the approved plan.

Operational contract:

1. ask one concise decision question with a recommended default and the main trade-off,
2. record the decision request and blocking reason in TaskFlow evidence,
3. if blocked, label the pause as `BLK_USER_DECISION_PENDING`,
4. resume implementation only after the user answer is recorded.

## 9.2) Boot Profile Selection (Lean/Standard/Full)

Before non-trivial execution or post-compact recovery, select boot profile explicitly:

1. `lean` (default): minimal required reads + hydrate-minimal gate.
2. `standard`: `lean` + execution protocols (`todo/implement/use-case`).
3. `full`: `standard` + orchestration/pipeline deep context.

Validation command:

```bash
taskflow-v0 boot run <lean|standard|full> [task_id] [--non-dev]
taskflow-v0 boot verify-receipt <task_id> [profile]
```

Rule:

1. If hydration fails for provided `task_id`, stop with `BLK_CONTEXT_NOT_HYDRATED`.
2. Default to `lean`; escalate profile only when risk/complexity requires.

## 10) Transparency Boundary

1. Pack- or methodology-specific transparency schemes such as SCP, BFP, and FTP do not belong to the execution substrate as owner-law.
2. This protocol owns execution materialization, block lifecycle, telemetry, gates, and resumable state only.
3. Higher-layer pack/methodology reporting must reference TaskFlow evidence rather than being redefined here.

`next_step` rule:

1. Must be populated for every planned/active block.
2. Use next block id (e.g. `B03`, `CMD07`) for sequential flow.
3. Use `-` only for terminal block.

-----
artifact_path: config/runtime-instructions/taskflow.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.taskflow-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-12T11:42:16+02:00'
changelog_ref: work.taskflow-protocol.changelog.jsonl
