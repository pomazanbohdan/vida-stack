# TODO Protocol (Execution Layer)

Purpose: decompose user requests into executable step-level work while keeping `br` as the single source of truth for task state.

## 1) Layer Model

1. `Intent Layer`: user request (goal, constraints, acceptance).
2. `Work Layer`: `br` issue lifecycle (`open/in_progress/closed`).
3. `Execution Layer`: TODO steps/tracks for actual implementation.

Rule: `br` tracks "what"; TODO tracks "how".

Task-state truth rule:

1. When task lifecycle state and execution telemetry appear out of sync, use `_vida/docs/task-state-reconciliation-protocol.md` as the canonical reconciliation layer before closing, reopening, or declaring the task stale.

Hard rule:

1. No execution without active TODO block.
2. Any implementation or research step MUST be enclosed by active TODO block lifecycle.
3. For done blocks, use `block-finish` by default. For partial/failed blocks, use `block-end` and then `reflect`/`verify`.
4. Before non-trivial execution, pre-register planned blocks via `block-plan` so board reflects total planned scope.
5. For command-layer documentation audits, pre-register one TODO block per protocol unit (`/vida-*#CLx`) and keep the pending list visible.
6. Every planned block should include `next_step` (block_id or `-` for terminal step).
7. Use rolling-window planning: keep 2-3 upcoming planned blocks visible; expand further plan just-in-time.
8. When the same technical error repeats twice inside an active block, the block must record an escalation event and switch to `_vida/docs/debug-escalation-protocol.md` before the next substantive fix attempt.
9. If subagent mode is active and an eligible catch/review lane exists, dispatch that external diagnostic lane in parallel with escalation lookup instead of debugging alone.
10. Escalation evidence must capture which outside inputs were used: `external_agent`, `primary_source`, `web/google`, or explicit `not_available` receipt.
11. Completed write-producing slices must receive catch/review coverage when an eligible lane exists; if not, record a protocol-drift finding and correct it.
12. Progress reporting must not interrupt lawful continuation by itself when continuous autonomous execution is active.

## 1.1) Silent Framework Diagnosis Integration

When root `vida.config.yaml` enables silent framework diagnosis, TODO flow must treat framework friction as a tracked deferred follow-up, not as an ad hoc side path.

Rules:

1. If framework/runtime friction is detected during an active non-framework task, capture or reuse a framework bug immediately.
2. Do not silently patch VIDA framework code mid-block unless the user explicitly reprioritizes framework work now.
3. Continue the current task with the lightest safe workaround when possible and record that workaround in TODO evidence.
4. At the next task boundary (`block-finish`, `block-end`, `reflect`, or `finish`), record the framework follow-up status in execution evidence or context capsule.
5. After the current task closes or is safely parked, route the captured framework bug into the normal framework queue and make it the next framework-facing execution target according to priority/recency rules.
6. Compact/context compression can happen at any moment, so pending framework follow-up state must survive via canonical artifacts, not chat memory.
7. Framework follow-up work still uses normal TODO/`br` flow, web/WVP validation when architecture claims depend on external reality, and delegated verification before closure.

Canonical helper:

```bash
python3 _vida/scripts/vida-silent-diagnosis.py capture \
  --summary "<framework issue>" \
  --details "<what happened>" \
  --current-task "<task_id>" \
  --workaround "<temporary workaround>"
```

Session reflection rule:

1. If `framework_self_diagnosis.session_reflection_required=true`, run a final self-reflection pass near task/session completion.
2. Reflection criteria come from overlay or default to:
   - `architecture_cleanliness`
   - `completeness`
   - `token_efficiency`
   - `orchestration_efficiency`
3. If reflection finds new framework gaps, create follow-up framework bugs/tasks and route them through the same deferred follow-up contract.

## 2) Decomposition + Clustering Algorithm

This algorithm is mandatory for non-trivial work (3+ steps).

1. `Q-Gate`: collect user decisions before decomposition.
   - Run focused question cards (scope boundary, delivery cut, dependency strategy, risk policy).
   - Record selected options and constraints as execution evidence.
2. `Conflict-Gate`: check decision compatibility.
   - If conflicts exist, resolve them before building TODO plan.
3. Attach to existing `br` task or create a new one.
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

## 3) Parallel Tracks Mode (Subagents)

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

## 4) TODO Step Definition

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

Start task:

```bash
bash _vida/scripts/beads-workflow.sh start <task_id>
bash _vida/scripts/beads-workflow.sh pack-start <task_id> <pack_id> "goal" [constraints]
bash _vida/scripts/beads-workflow.sh redirect <task_id> <from_block_id> <to_block_id> "reason"
bash _vida/scripts/beads-workflow.sh block-plan <task_id> <block_id> "goal" [track_id] [owner] [depends_on]
```

Track-aware step logging:

```bash
bash _vida/scripts/beads-workflow.sh block-start <task_id> <block_id> "goal" [track_id] [owner] [depends_on]
bash _vida/scripts/beads-workflow.sh block-finish <task_id> <block_id> <done|partial|failed> "next" "actions" [artifacts] [risks] [assumptions] [evidence] [confidence]
bash _vida/scripts/beads-workflow.sh block-end <task_id> <block_id> <done|partial|failed> "next" "actions" [artifacts] [risks] [assumptions] [evidence] [track_id] [owner] [merge_ready]
```

Reflection + quality gate (manual path):

```bash
bash _vida/scripts/beads-workflow.sh reflect <task_id> "goal" "constraints" "evidence" "decision" "risks" "next" [confidence]
bash _vida/scripts/beads-workflow.sh verify <task_id>
bash _vida/scripts/quality-health-check.sh <task_id>
```

Finish:

```bash
bash _vida/scripts/beads-workflow.sh finish <task_id> "reason"
```

TODO interface commands (derived view from execution log):

```bash
bash _vida/scripts/todo-tool.sh board <task_id>
bash _vida/scripts/todo-tool.sh compact <task_id> [limit]
bash _vida/scripts/todo-tool.sh list <task_id>
bash _vida/scripts/todo-tool.sh current <task_id>
bash _vida/scripts/todo-tool.sh next <task_id>
bash _vida/scripts/todo-tool.sh ui-json <task_id>
bash _vida/scripts/todo-sync-plan.sh <task_id>
bash _vida/scripts/todo-sync-plan.sh <task_id> --mode compact --max-items 3
bash _vida/scripts/todo-sync-plan.sh <task_id> --mode delta
bash _vida/scripts/todo-sync-plan.sh <task_id> --mode json-only --quiet
bash _vida/scripts/todo-overhead-report.sh <task_id>
bash _vida/scripts/todo-plan-validate.sh <task_id> [--strict] [--quiet] [--diff-aware] [--base REF]
bash _vida/scripts/vida-command-audit.sh report <task_id>
bash _vida/scripts/vida-command-audit.sh plan <task_id> [--limit N]
bash _vida/scripts/vida-command-audit.sh repair-next <task_id>
```

Framework-only lean starter:

```bash
bash _vida/scripts/framework-wave-start.sh <task_id> <pack_id> "<goal>" [constraints]
```

Use for framework-owned `_vida/*` work when you want the canonical start path with less routine overhead. It preserves:

1. `br` as SSOT,
2. pack logging,
3. TODO scaffolding/validation,
4. boot-profile validation.

Command audit mode:

1. Run `bash _vida/scripts/vida-command-audit.sh report <task_id>` to see done/pending coverage.
2. Run `bash _vida/scripts/vida-command-audit.sh plan <task_id>` to pre-register missing protocol-unit analysis blocks.
3. Execute protocol-unit analyses sequentially (`block-start` -> `block-end`) to keep pending list accurate.
4. Before reporting to user, confirm `board` and `report` snapshots are up to date.
5. Sequential automation: after `block-end ... done <next_block_id> ...`, workflow auto-starts `<next_block_id>` if it exists and is `todo`.
6. Auto-start is track-safe: it runs only when source and target blocks are in the same `track_id`.
7. If old plans have broken `next_step`, run `repair-next` to rebuild canonical `CMDxx` chain.
8. `block-start` may reopen a previously ended block (clears previous end marker in TODO view and sets status back to `doing`).
9. When execution focus changes because of a new user instruction, use `beads-workflow.sh redirect` so the current block is closed as `partial` and the replacement block becomes the active `doing` step in one canonical path.
10. Redirected/superseded blocks must not return to the active TODO backlog; runtime views should surface them as `superseded` instead of `todo`.
11. When more than one next task/block appears lawful, apply `_vida/docs/execution-priority-protocol.md` before choosing or redirecting.

Protocol-unit rule:

1. When command decomposition work is planned or delegated, refer to units as `<command>#CL1..CL5`.
2. `CL1`, `CL2`, and read-heavy `CL3` work is delegation-friendly.
3. `CL4` stays single-writer unless explicit write isolation exists.
4. `CL5` may delegate evidence collection, but orchestrator owns final gate decisions.

UI sync rule:

1. TODO UI reads from `_vida/scripts/todo-tool.sh ui-json <task_id>`.
2. Source of truth remains execution events in `.vida/logs/beads-execution.jsonl`.
3. Never mutate TODO state directly in UI without writing corresponding execution events.
4. Use `_vida/scripts/todo-sync-plan.sh <task_id>` to generate a deterministic checklist snapshot for UI mirroring.
5. `_vida/scripts/beads-workflow.sh` auto-runs `todo-sync-plan.sh` in `json-only` mode according to `TODO_AUTO_SYNC_LEVEL`.
5.1. `TODO_AUTO_SYNC_LEVEL=lean` (default): sync on `start`, `block-start`, `block-end`, `finish`.
5.2. `TODO_AUTO_SYNC_LEVEL=full`: sync on all workflow mutations.
5.3. `TODO_AUTO_SYNC_LEVEL=off`: disable auto sync (manual sync only).
6. For user-visible progress updates, prefer compact/delta snapshots over full snapshot.
7. Completion report order is mandatory: `sync -> confirm board/compact -> report done`.
8. Pack coverage: each non-trivial flow should have balanced `pack-start` and `pack-end` events.
9. Response visibility rule: when reporting task/todo state to user, include IDs and concise descriptions (not IDs only).
10. `quality-health-check` cadence: run on checkpoint boundaries, pre-handoff, and finish (not after every micro-step).
11. Runtime scripts should be quiet-by-default for progress chatter; keep human-facing status output in `todo-tool current|list` and `quality-health-check`.
12. Use `python3 _vida/scripts/task-state-reconcile.py status <task_id>` when a task looks done-but-open, stale-in-progress, or otherwise drifted between `br` and TODO.

Background worker policy (token/cost aware):

1. For continuous live backup without chat noise, use tmux-managed worker:

```bash
bash _vida/scripts/beads-bg-sync.sh start --interval 600
bash _vida/scripts/beads-bg-sync.sh status
bash _vida/scripts/beads-bg-sync.sh stop
```

2. Default interval is 600 sec (10 min).
3. Do not use aggressive intervals below 120 sec in normal workflow.
4. Prefer event-driven sync (`beads-workflow` auto-sync) + sparse background JSONL snapshots over high-frequency polling.

Silent diagnosis TODO handoff:

1. If silent diagnosis is active and a framework gap was captured during the current task, `reflect`/`finish` should reference the capture artifact or resulting framework task id.
2. Current-task completion is allowed with a bounded workaround, but framework capture must not be left only in chat.
3. When the current task is paused rather than closed, store the pending framework follow-up in the context capsule so post-compact recovery can resume the correct next action.

## 6) Gates

0. Plan gate: non-trivial work must pass `Q-Gate` + `Conflict-Gate` before execution materialization.
0.1. Tool-capability gate: for non-trivial flows, resolve required tool fallbacks and record evidence when fallback is used.
0.2. If a task enters bounded conflict-discussion mode via `_vida/docs/problem-party-protocol.md`, record the board artifact path in block evidence before resuming normal execution.
1. Step gate: `block-end` requires evidence or artifacts.
1.1. If WVP trigger fired, `block-end`/`reflect` evidence must include WVP markers per `_vida/docs/web-validation-protocol.md`.
2. Track gate: each parallel track must pass verify.
3. Task gate: strict verify + self-reflection required before close.
4. Compact gate: always record `compact_pre` and `compact_post`.
4.1. Drift gate: run `bash _vida/scripts/context-drift-sentinel.sh check <task_id>` after capsule write checkpoints (`block-finish`, compact restore).
4.2. If silent framework diagnosis is active and a framework gap was detected, compact-safe evidence must include the capture artifact path or follow-up framework task id.
5. Execution gate: if no active block exists, execution must not proceed.
6. Plan integrity gate: run `bash _vida/scripts/todo-plan-validate.sh <task_id>` after `block-plan` batch and before execution start. Use `--diff-aware` when the worktree already contains target-scope changes; coverage is evaluated against the whole task plan so already-completed blocks still count.
6.1. For framework-only tasks, compact evidence is valid when work is confined to `_vida/*` and the block records concrete actions plus canonical artifacts or task IDs. Runtime verification may downgrade missing artifact warnings to informational severity for these tasks in non-strict mode.
6.2. Silent diagnosis gate: when active, task closure is invalid if a detected framework gap was only discussed in chat and not captured in canonical execution evidence, context capsule, or framework task state.

## 7) Anti-Patterns

1. Running multiple writable tracks over the same files without dependencies.
2. Closing `br` task without TODO evidence and strict verify.
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
bash _vida/scripts/task-execution-mode.sh get <task_id>
bash _vida/scripts/task-execution-mode.sh recommend <task_id>
bash _vida/scripts/task-execution-mode.sh set <task_id> <decision_required|autonomous> [reason]
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
2. record the decision request and blocking reason in TODO evidence,
3. if blocked, label the pause as `BLK_USER_DECISION_PENDING`,
4. resume implementation only after the user answer is recorded.

## 9.2) Boot Profile Selection (Lean/Standard/Full)

Before non-trivial execution or post-compact recovery, select boot profile explicitly:

1. `lean` (default): minimal required reads + hydrate-minimal gate.
2. `standard`: `lean` + execution protocols (`todo/implement/use-case`).
3. `full`: `standard` + orchestration/pipeline deep context.

Validation command:

```bash
bash _vida/scripts/boot-profile.sh run <lean|standard|full> [task_id] [--non-dev]
bash _vida/scripts/boot-profile.sh verify-receipt <task_id> [profile]
```

Rule:

1. If hydration fails for provided `task_id`, stop with `BLK_CONTEXT_NOT_HYDRATED`.
2. Default to `lean`; escalate profile only when risk/complexity requires.

## 10) SCP Transparency (Non-Dev Flows)

For non-development flows (`research/spec/work-pool/bug-pool/reflection`), report SCP progress explicitly:

1. Current SCP step (`SCP-0..SCP-8`).
2. Open decisions and conflicts.
3. API validation status (if applicable).
3.1. WVP evidence status (`present|missing|conflicting`) when external assumptions are involved.
4. Next SCP step.

Development flow (`/vida-implement*`, `dev-pack`) is exempt from SCP.

## 11) BFP Transparency (Bug-Fix Flows)

For `/vida-bug-fix` and `bug-pool-pack`, report:

1. current BFP step (`BFP-0..8`),
2. active issue IDs and statuses,
3. regression status,
4. documentation/spec sync status,
5. confidence and residual risks.
`next_step` rule:

1. Must be populated for every planned/active block.
2. Use next block id (e.g. `B03`, `CMD07`) for sequential flow.
3. Use `-` only for terminal block.

## 12) FTP Transparency (Spec -> Dev Bridge)

For `/vida-form-task` (`work-pool-pack`) report:

1. current FTP step (`FT-00..FT-07`),
2. open blocker codes,
3. ready/blocked/deferred task counts,
4. launch-decision status (`approved|deferred|revise`),
5. exact next command (`/vida-implement ...` only after explicit user confirmation).
