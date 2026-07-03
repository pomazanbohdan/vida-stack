# TaskFlow Protocol (Execution Layer)

Purpose: decompose user requests into executable step-level work while keeping `vida taskflow task` as the single source of truth for task state.

Output policy:

1. Human-facing TaskFlow runtime commands default to `TOON`.
2. Structured JSON output is enabled only through explicit `--json`.
3. New runtime surfaces MUST NOT make raw JSON the default human-facing output.

Compression note: this revision is a quality-preserving refactor. Legacy headings are retained in place; command atoms, gate names, owner ids, and stop conditions remain exact or explicitly named.

## 1) Layer Model

1. `Intent Layer`: user request, constraints, acceptance.
2. `Work Layer`: DB-backed `vida taskflow task` lifecycle (`open/in_progress/closed`).
3. `Execution Layer`: TaskFlow steps/tracks for implementation.

Rule: the DB-backed task surface tracks "what"; TaskFlow tracks "how".

Task-state truth rule: if lifecycle state and execution telemetry disagree, use `runtime-instructions/work.task-state-reconciliation-protocol` before closing, reopening, or declaring stale.

Work item taxonomy rule:

1. Persisted task-store `issue_type` values are provider-neutral work item types.
2. `issue_type` MUST resolve through the work item taxonomy registry before it drives flow binding, parent/root eligibility, or source-tier classification.
3. `dev_team.work_item_flow_bindings` keys MUST use canonical taxonomy ids or documented aliases.
4. Runtime task classes are separate routing inputs.
5. `delivery_task` and `execution_block` are TaskFlow step concepts, not persisted task-store `issue_type` values.
6. Unknown work item types fail closed for root eligibility until explicitly registered.

Hard rule:

1. No execution without active TaskFlow block; implementation/research outside block lifecycle is invalid.
2. TaskFlow plan plus block evidence is external working memory; chat memory is not an execution ledger.
3. Done uses `block-finish`; partial/failed uses `block-end` then `reflect`/`verify`.
4. Non-trivial work MUST pre-register `block-plan`, keep 2-3 upcoming blocks visible, include `next_step`, and use `-` only for terminal blocks.
5. Command documentation audits MUST pre-register one block per `/vida-*#CLx` unit.
6. The same technical error twice in one active block MUST record escalation, use `diagnostic-instructions/escalation.debug-escalation-protocol`, and dispatch eligible catch/review worker lanes when available.
7. Escalation evidence MUST name `external_agent`, `primary_source`, `web/google`, or `not_available`; write-producing slices MUST get eligible catch/review coverage.
8. Progress reports, summaries, wait timeouts, empty polls, green focused tests such as `cargo test`, partial implementer returns, and existing dirty diffs are not execution-state transitions.
9. While `in_work=1`, every report/timeout/partial result MUST be followed in-cycle by lawful continuation, reroute, renewed waiting, bind/shape/dispatch, blocker/override, or exception handling.
10. Validation overlays gate implementation; accepted resume-after-validation returns to autonomous continuation for the same lawful chain.
11. Each active block MUST record `what changed`, `what remains`, and `next step` before compact, handoff, or agent replacement.
12. Two no-progress iterations or broad rereads without a narrower hypothesis force no-progress close plus re-plan/escalation.
13. Parallel tracks merge only at explicit checkpoints and may enter review pools only as merge-ready siblings.
14. Closing one leaf does not close the parent task line; TaskFlow MUST rebuild the parent and persist next leaf, blocker/escalation receipt, or full chain closure.
15. Closure-style reports are invalid while the task line remains open without continuation receipt.
16. Multiple lawful next leaves require `runtime-instructions/work.execution-priority-protocol`.
17. Root-session writing after partial delegation requires explicit exception-path receipt and no open same-packet delegation; small fixes are not exceptions.
18. `continue development` and `continue the next task` MUST NOT rebind to first failing/ready item unless active receipts prove that bounded unit.

## 1.1) Diagnostic Integration Boundary

1. TaskFlow MAY carry deferred diagnostic evidence only as tracked execution data.
2. Silent framework diagnosis policy, capture rules, and reflection criteria are owned by `diagnostic-instructions/analysis.silent-framework-diagnosis-protocol`.
3. This protocol owns only the execution-side requirement that deferred diagnostic follow-up survive through canonical task/evidence surfaces rather than chat memory.

## 2) Decomposition + Clustering Algorithm

Mandatory for non-trivial work (3+ steps):

1. Run `Q-Gate`, then `Conflict-Gate`; conflicts MUST be empty before plan materialization.
2. Attach/create the `TaskFlow` task and build 15-90 minute steps with measurable output, acceptance/evidence intent, `depends_on`, and `next_step`.
3. Route each step as `sequential` or `parallel`; `parallel` requires no output dependency, shared writable scope, or contract coupling, otherwise use `track_id=main`.
4. Pre-register with `block-plan`; validate with `todo-plan-validate.sh`, adding `--diff-aware` when target-scope worktree changes exist.
5. Diff-aware validation MUST count the full task plan, including completed blocks.
6. Execute each step with evidence and verification.
7. Leaf closure without task-line closure MUST trigger parent rebuild before report/suspension.
8. Dispatch-ready write packets MUST dispatch or record blocker/override before progress-only report.
9. Wait timeout, summary, partial implementer return, or local repair/proof in an active task MUST lead to lawful next action, reroute, explicit blocker/exception, or receipt-confirmed parent/packet binding.

### 2.1 Q-Gate Output Contract

Minimum planning fields:

1. `scope_boundary`
2. `delivery_cut`
3. `dependency_strategy`
4. `risk_policy`
5. `open_conflicts` (MUST be empty before execution)

### 2.2 Sequential/Parallel Decision Matrix

Choose `parallel` only when ALL are true:

1. no step depends on another step output,
2. no shared writable files/directories,
3. no shared mutable API/data contract in-flight.

If any condition fails, use sequential chain (`next_step`) on `main` track.

### 2.3 Anti-Loop And Context-Discipline Contract

1. Research/diagnosis blocks MUST narrow reads: locate files -> skim structure -> deep-read owner subset -> record working set.
2. Each block MUST end with verified artifact change, bounded evidence, or blocker/no-progress receipt.
3. Repeated plan narration without state delta is invalid.
4. After no-progress, change task granularity, route/lane, evidence source, or validation strategy; otherwise escalate.
5. After leaf closure, dispatch-ready state, wait timeout, summary, partial return, or dirty/partial interrupted state, next attempt MUST change execution state by continuation, reroute, waiting, escalation, blocker/override, or pre-write exception receipt. Existing diff alone is insufficient.

## 3) Parallel Tracks Mode (Workers)

Use when 2+ independent chunks can run concurrently.

Track schema: `track_id`, `owner`, `scope`, `depends_on`, `verify`, `merge_ready` (`yes/no`), and optional `review_pool`.

Rules:

1. `track_id`: `main`, `A`, `B`, `C`, ...
2. `owner`: `orchestrator` or `agent:<id>`.
3. Default track is `main`; default owner is `orchestrator`.
4. Avoid overlapping writable scopes across active tracks.
5. If overlap is required, serialize by dependency order.
6. Merge only after per-track `verify` passes.
7. A review pool may group only merge-ready sibling tasks with the same milestone or merge checkpoint.
8. Review pools MUST NOT hide per-task blockers or skip per-task verification evidence.

## 4) TaskFlow Step Definition

Required fields: `step_id` (`S01`, `S02`, ...), `task_id`, `goal`, `status`, `acceptance_check`, `evidence_ref`, `next_step`, `risk`, `scope_in`, `scope_out`, `definition_of_done`, `stop_rule`.

`status` values: `planned|doing|done|blocked`.

Optional parallel fields: `track_id`, `owner`, `depends_on`, `merge_ready`, `review_pool`.

## 5) Operational Commands

Transition note: block-lifecycle examples are legacy wrappers; transitioned runtime reads live under `vida task`, `vida taskflow graph-summary`, and `vida taskflow run-graph`; wrapper retirement is tracked by `system-maps/migration.runtime-transition-map`.

Transition verification baseline: before close/handoff on transitioned runtime slices, run `vida taskflow help`, the bounded TaskFlow runtime-family implementation test/build suite, and targeted boot/runtime proofs. `system-maps/migration.runtime-transition-map` is the migration registry only, not a competing verification-law owner.

Command atoms:

```bash
bash beads-workflow.sh start <task_id>
bash beads-workflow.sh pack-start <task_id> <pack_id> "goal" [constraints]
bash beads-workflow.sh redirect <task_id> <from_block_id> <to_block_id> "reason"
bash beads-workflow.sh block-plan <task_id> <block_id> "goal" [track_id] [owner] [depends_on]
bash beads-workflow.sh block-start <task_id> <block_id> "goal" [track_id] [owner] [depends_on]
bash beads-workflow.sh block-finish <task_id> <block_id> <done|partial|failed> "next" "actions" [artifacts] [risks] [assumptions] [evidence] [confidence]
bash beads-workflow.sh block-end <task_id> <block_id> <done|partial|failed> "next" "actions" [artifacts] [risks] [assumptions] [evidence] [track_id] [owner] [merge_ready]
bash beads-workflow.sh reflect <task_id> "goal" "constraints" "evidence" "decision" "risks" "next" [confidence]
bash beads-workflow.sh verify <task_id>
bash quality-health-check.sh <task_id>
bash beads-workflow.sh finish <task_id> "reason"
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
bash todo-plan-validate.sh <task_id> [--strict] [--quiet] [--diff-aware] [--base REF]  # legacy script name
bash vida-command-audit.sh report <task_id>
bash vida-command-audit.sh plan <task_id> [--limit N]
bash vida-command-audit.sh repair-next <task_id>
bash framework-wave-start.sh <task_id> <pack_id> "<goal>" [constraints]
```

Framework-only lean starter: `bash framework-wave-start.sh <task_id> <pack_id> "<goal>" [constraints]` is migration-only for framework-owned legacy wrapper flow; it preserves `vida taskflow task` as SSOT, pack logging, scaffolding/validation, and boot-profile validation.

Command audit mode: `report` shows coverage; `plan` pre-registers missing protocol units; analyses run sequentially (`block-start` -> `block-end`); confirm `board` and `report` before user report; `block-end ... done <next_block_id> ...` may auto-start `<next_block_id>` on the same track; `repair-next` rebuilds `next_step`; `block-start` may reopen ended block as `doing`; focus change MUST use `beads-workflow.sh redirect`; superseded blocks surface as `superseded`; multiple lawful next blocks require `runtime-instructions/work.execution-priority-protocol`.

Protocol-unit rule:

1. Planned/delegated command decomposition units use `<command>#CL1..CL5`; command-unit ids may use `CMDxx`.
2. `CL1`, `CL2`, and read-heavy `CL3` are delegation-friendly.
3. `CL4` stays single-writer unless explicit write isolation exists.
4. `CL5` may delegate evidence collection, but orchestrator owns final gate decisions.

UI sync rule: UI reads from `taskflow-tool.sh ui-json <task_id>`; Source of truth remains execution events in `.vida/logs/beads-execution.jsonl`; UI MUST NOT mutate state without execution events; `taskflow-sync-plan.sh <task_id>` writes deterministic snapshots; `TASKFLOW_AUTO_SYNC_LEVEL=lean` syncs on `start`/`block-start`/`block-end`/`finish`; `TASKFLOW_AUTO_SYNC_LEVEL=full` syncs all mutations; `TASKFLOW_AUTO_SYNC_LEVEL=off` disables auto-sync. Prefer compact/delta progress; completion order is `sync -> confirm board/compact -> report done`; non-trivial packs should balance `pack-start`/`pack-end`; pack completion owner is `runtime-instructions/work.pack-completion-gate-protocol`; reports include IDs plus concise descriptions; `quality-health-check` runs at checkpoints/pre-handoff/finish; keep human-facing status output in `taskflow-tool current|list`; scripts stay quiet; use `vida taskflow reconcile status <task_id>` for done-but-open, stale-in-progress, or task-store/TaskFlow drift.

Background worker policy:

```bash
bash beads-bg-sync.sh start --interval 600
bash beads-bg-sync.sh status
bash beads-bg-sync.sh stop
```

Default interval is 600 sec; normal intervals below 120 sec are forbidden. Prefer event-driven sync plus sparse background JSONL snapshots over high-frequency polling.

Silent diagnosis execution persistence: if active and a framework gap was captured, `reflect`/`finish` SHOULD reference capture artifact or framework task id. This protocol owns only execution-side persistence in TaskFlow evidence/context capsules; policy, capture timing, and follow-up routing remain owned by `diagnostic-instructions/analysis.silent-framework-diagnosis-protocol`.

## 6) Gates

0. Plan gate: non-trivial work MUST pass `Q-Gate` + `Conflict-Gate`; tool fallbacks need evidence; `runtime-instructions/work.problem-party-protocol` conflict mode must record board artifact before normal execution resumes.
1. Step gate: `block-end` requires evidence/artifacts; WVP-triggered `block-end`/`reflect` evidence MUST include markers per `runtime-instructions/work.web-validation-protocol`.
2. Track gate: each parallel track MUST pass `verify`.
3. Task gate: strict verify plus self-reflection required before close; pack-complete claims are lawful only through `runtime-instructions/work.pack-completion-gate-protocol`.
4. Compact gate: record `compact_pre` and `compact_post`; run `bash context-drift-sentinel.sh check <task_id>` after capsule write checkpoints (`block-finish`, compact restore); silent diagnosis evidence must preserve capture artifact or follow-up task id.
5. Execution gate: without active block, execution MUST NOT proceed.
6. Plan integrity gate: run `bash todo-plan-validate.sh <task_id>` after `block-plan` batch and before execution; add `--diff-aware` when target-scope worktree changes exist. Framework-only compact evidence is valid only for migration helper surfaces with concrete actions plus canonical artifacts/task IDs. Closure is invalid when detected framework gaps exist only in chat. 6.3. No-progress gate: two no-progress iterations force re-plan, redirect, or escalation.

## 7) Anti-Patterns

1. Running multiple writable tracks over the same files without dependencies.
2. Closing a TaskFlow task without TaskFlow evidence and strict verify.
3. Tracking execution only in chat without structured log entries.
4. Treating a progress summary, timeout, partial delegated result, green focused test, or existing dirty diff as a state transition.
5. Selecting the first ready/failing item as active work without active task/packet/continuation receipt evidence.

## 8) Blocked/Unblocked Algorithm

When another task becomes the active dependency:

1. Add dependency through the DB-backed task runtime surface.
2. Set blocked status: `vida taskflow task update <blocked_task_id> --status blocked`.
3. Record reason in execution log (`checkpoint` or `block-end` risk/next_step fields).
4. Continue only on the active dependency task.

When the dependency completes:

1. Reopen status: `vida taskflow task update <blocked_task_id> --status open`.
2. Verify dependency state with `vida taskflow task show <blocked_task_id> --json`.
3. Pick next work via `vida taskflow task ready` using the unblocked-first rule.
4. Start resumed task explicitly: `vida taskflow task update <id> --status in_progress`.

## 9) Execution Mode (Decision vs Autonomous)

Per-task execution mode MUST be explicit:

1. `decision_required`: assistant analyzes/options; user confirms key decisions before implementation edits.
2. `autonomous`: assistant executes end-to-end inside agreed scope; checkpoints remain logged.

Mode operations:

```bash
bash task-execution-mode.sh get <task_id>
bash task-execution-mode.sh recommend <task_id>
bash task-execution-mode.sh set <task_id> <decision_required|autonomous> [reason]
```

Routing rule: documentation/research-heavy tasks default to `decision_required`; implementation-heavy feature/bug execution defaults to `autonomous` unless user overrides.

## 9.1) User Escalation Gate

Autonomous execution does not authorize silent product or contract choices.

Escalate to user and pause implementation when:

1. more than one plausible product/UX behavior fits evidence,
2. a fix changes navigation, auth, destructive data behavior, or user-facing semantics beyond agreed slice,
3. live API/server reality contradicts request or prior contract,
4. root-cause confidence is below 80% and fixes have materially different outcomes,
5. task must expand in scope, order, or risk beyond approved plan.

Operational contract: ask one concise decision question with recommended default and trade-off; record request and blocking reason in TaskFlow evidence; if blocked, label `BLK_USER_DECISION_PENDING`; resume only after user answer is recorded.

## 9.2) Boot Profile Selection (Lean/Standard/Full)

Before non-trivial execution or post-compact recovery, select boot profile explicitly:

1. `lean` (default): minimal required reads + hydrate-minimal gate.
2. `standard`: `lean` + execution protocols (`step/implement/use-case`).
3. `full`: `standard` + orchestration/pipeline deep context.

Validation command:

```bash
vida taskflow boot run <lean|standard|full> [task_id] [--non-dev]
vida taskflow boot verify-receipt <task_id> [profile]
```

Rule: if hydration fails for provided `task_id`, stop with `BLK_CONTEXT_NOT_HYDRATED`; default to `lean`; escalate profile only when risk/complexity requires.

## 10) Transparency Boundary

1. Pack- or methodology-specific transparency schemes such as SCP, BFP, and FTP do not belong to the execution substrate as owner-law.
2. This protocol owns execution materialization, block lifecycle, telemetry, gates, and resumable state only.
3. Higher-layer pack/methodology reporting MUST reference TaskFlow evidence rather than being redefined here.

`next_step` rule:

1. MUST be populated for every planned/active block.
2. Use next block id (for example `B03`, `CMD07`) for sequential flow.
3. Use `-` only for terminal block.

Compressed Legacy Anchor Crosswalk:

1. Legacy headings `## 1)` through `## 10)` and subheadings `### 2.1`, `### 2.2`, `### 2.3`, `## 9.1`, `## 9.2` are retained in place.
2. Command atoms from the legacy operational blocks are retained in `## 5) Operational Commands`.
3. Gate, anti-loop, blocked/unblocked, execution-mode, boot-profile, transparency, and `next_step` semantics are retained as binding rules.

-----
artifact_path: config/runtime-instructions/taskflow.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: 2026-07-03
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.taskflow-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: 2026-07-03T11:12:39.6247137+03:00
changelog_ref: work.taskflow-protocol.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: semantic-atom-coverage+conservative-llmlingua+pre-change-baseline-audit
protocol_compression_baseline_ref: HEAD:vida/config/instructions/runtime-instructions/work.taskflow-protocol.md
protocol_compression_audit_at: 2026-07-03T11:12:39.6247137+03:00
protocol_compression_before_tokens: 6338
protocol_compression_after_tokens: 4633
protocol_compression_content_sha256: bdcc5a1090535c4a9830017529d4b8c3e0f4a27e31195b272c6e784e76b04943
