# Implement Execution Protocol (IEP)

Purpose: define one canonical development execution flow after `/vida-form-task` launch approval.

Scope:

1. Command mode: `/vida-implement`.
2. Applies to autonomous development execution for a ready task pool in the DB-backed task runtime.
3. Uses one canonical command (`/vida-implement`) and forbids historical split aliases as runtime path.
4. When the user explicitly wants the agent to keep following a settled plan/spec/task pool to completion, activate `instruction-contracts/overlay.autonomous-execution-protocol` as the trigger/stop doctrine layered on top of this protocol.
5. When queue selection or reprioritization is ambiguous, activate `runtime-instructions/work.execution-priority-protocol` before selecting the next writer task.

## Core Contract

`/vida-implement` must:

1. accept only tasks that passed form-task launch gate,
2. build or refresh the active pool dependency graph before selecting the next writer task when the scope includes multiple tasks/subtasks,
3. pick next `ready` task(s) from `vida taskflow task`,
4. execute implementation loop to completion or explicit blocker,
5. run mandatory verification and review gates,
6. continue automatically with next ready task until pool completion.

## Mandatory Inputs

1. active DB-backed task/pool context.
2. Approved spec and acceptance criteria, or approved `issue_contract` for equivalent bug paths.
2.1. When the work originated from mixed research/release/user-negotiation inputs, a normalized `spec_intake` artifact must already have routed that work into SCP or ICP before launch.
2.2. `draft_execution_spec` is a pre-launch review artifact only; it does not authorize `/vida-implement` by itself and must be absorbed by the canonical form-task launch path before writer execution.
2.3. A launch-approved bounded `delivery_task_card` from `command-instructions/planning.form-task-protocol` is mandatory launch context, not optional planning residue.
2.4. The active `delivery_task_card` must expose at minimum:
   - `goal`
   - `non_goals`
   - `scope_in`
   - `scope_out`
   - `owned_paths` or `owned_areas`
   - `acceptance_checks`
   - `validation_commands`
   - `definition_of_done`
   - `stop_rules`
   - `handoff_target`
2.5. If execution is entering one leaf task from a larger milestone, the active execution slice must also identify the current `execution_block` or equivalent bounded writer packet.
3. Research evidence relevant to scope.
4. External API reality evidence (when integration exists).
5. Decision log (`docs/decisions.md`) and feature checklist entries.
6. `runtime-instructions/work.web-validation-protocol` for external assumptions during execution.
7. Hydrated context capsule for active task (`.vida/logs/context-capsules/<task_id>.json`).
8. Route-level control limits when declared by the active route or overlay:
   - `budget_policy`
   - `max_budget_units`
   - `max_total_runtime_seconds`
   - `max_coach_passes`
   - `max_verification_passes`
   - any explicit `max_rounds|max_stalls|max_resets` runtime-derived receipt.

## Command Layer Mapping

For `/vida-implement`, IEP layers map to CLP as follows:

1. `CL1 Intake` -> `IEP-0 Launch Intake` + `IEP-1 Context Hydration`
2. `CL2 Reality And Inputs` -> `IEP-1.5 Pool Graph Analysis` + `IEP-2 Queue Intake` + `IEP-3 Skills Routing` + `IEP-4 Preflight`
3. `CL3 Contract And Decisions` -> `IEP-4.5 Change-Impact Gate`
4. `CL4 Materialization` -> `IEP-5 Implement Loop`
5. `CL5 Gates And Handoff` -> `IEP-6 Verify And Review` + `IEP-7 Close And Continue` + `IEP-8 Pool Completion`

Canonical layer source: `command-instructions/routing.command-layer-protocol`

## Gate Sequence (Canonical)

1. `IEP-0 Launch Intake`
   - confirm launch approved by `/vida-form-task`.
   - if no approval: `BLOCKED (BLK_LAUNCH_NOT_CONFIRMED)`.
2. `IEP-1 Context Hydration`
   - load spec/research/decisions/contracts for selected task.
3. `IEP-1.2 Delivery-Task Contract Hydration`
   - hydrate the approved `delivery_task_card` and current `execution_block`.
   - confirm `goal`, `non_goals`, `owned_paths`, `validation_commands`, `definition_of_done`, and `stop_rules` are present and non-empty.
   - if the task is entering from a review-pool-capable milestone, hydrate `review_pool` and merge-checkpoint metadata before writer execution.
   - if the contract is incomplete or stale relative to launch approval: `BLOCKED (BLK_DELIVERY_TASK_CARD_MISSING)`.
3. `IEP-1.5 Pool Graph Analysis`
   - for epic, wave, and multi-task execution, derive the active dependency graph before choosing a writer lane.
   - classify `ready`, `blocked`, `soft-blocked`, `parallel_read_only`, and `single_writer`.
   - if the graph is missing, stale, or contradictory: `BLOCKED (BLK_POOL_GRAPH_MISSING)`.
4. `IEP-2 Queue Intake`
   - select next `ready` task from `vida taskflow task`.
   - if none: move to `IEP-8 Pool Completion`.
   - if multiple candidates remain after queue intake, apply `runtime-instructions/work.execution-priority-protocol` and keep `instruction-contracts/core.agent-system-protocol` route law active while selecting the next writer task.
5. `IEP-3 Skills Routing`
   - run dynamic skill selection for current task scope.
6. `IEP-4 Preflight`
   - baseline checks, dependency readiness, risk scan.
   - derive the active effective route control limits from route metadata and overlay:
     - `max_rounds`
     - `max_stalls`
     - `max_resets`
     - `max_budget_units`
     - `max_total_runtime_seconds`
   - initialize or refresh run-graph control counters before entering writer work.
7. `IEP-4.2 Execution Authorization Gate`
   - confirm route receipt, analysis lane, analysis receipt (when required), `issue_contract` readiness when the task is issue-driven, non-empty `issue_contract.proven_scope`, symptom-level evidence for any multi-symptom issue, author lane, verifier lane (or explicit `no_eligible_verifier`), and writer ownership before deep local implementation prep.
   - confirm the active writer packet is a lawful consumption of the current `delivery_task_card`, not a widened reinterpretation of the milestone or epic.
   - confirm all writable paths proposed by the writer packet stay within `owned_paths` or an explicit serialized exception receipt.
   - if analysis routing is unavailable because the route records explicit `no_eligible_analysis_lane`, remain fail-closed by default; only framework-owned tracked remediation may proceed via a structured execution-auth override receipt, never by silent local fallback.
   - if `issue_contract` emits a mixed-issue split artifact, keep writer ownership on the primary executable slice only and preserve the unresolved slice as follow-up work.
   - if local mutation is proposed under active worker mode, require route authorization or lawful escalation receipt.
   - if the gate is not satisfied: `BLOCKED (BLK_EXECUTION_AUTH_MISSING)`.
8. `IEP-4.3 Runtime Control Gate`
   - before each writer pass, compare run-graph counters and route limits for:
     - `round_count`
     - `stall_count`
     - `reset_count`
     - `budget_units_consumed`
     - `runtime_seconds_consumed`
   - treat two consecutive no-progress passes, repeated rereads without narrower hypothesis, or repeated validation failures without new state delta as a `stall`.
   - when a control limit is hit, stop the current writer loop and route to replan, escalation, or fresh-start recovery rather than silently continuing.
   - if the gate is not satisfied: `BLOCKED (BLK_RUNTIME_CONTROL_EXHAUSTED)`.
8. `IEP-4.5 Change-Impact Gate`
   - detect scope/AC/dependency/decision drift before continuing.
   - if drift detected: stop and route per `runtime-instructions/work.change-impact-reconciliation-protocol`.
9. `IEP-5 Implement Loop`
   - consume one bounded `execution_block` at a time from the active `delivery_task_card`.
   - code changes + targeted checks + state delta capture only inside owned scope.
   - do not broaden from `execution_block` to milestone-wide mutation inside one writer pass.
   - after each bounded pass:
     - record the changed artifact set,
     - record progress against `definition_of_done`,
     - update run-graph node state and control counters,
     - write a resumable checkpoint when the task remains open.
   - if the current `execution_block` closes but the parent `delivery_task_card.definition_of_done` is still unmet:
     - keep execution inside the same task,
     - rebuild the next lawful `execution_block` or proof slice,
     - continue under the same task instead of entering closure-style reporting.
10. `IEP-5.5 Coach Review` (when the selected route declares `coach_required=yes`)
   - run the post-write coach ensemble against the current implementation,
   - default policy is two independent cheaper coaches when the route exposes enough eligible lanes,
   - approve only when the required coach quorum approves; any valid `return_for_rework` vote blocks advancement,
   - each coach lane judges verifier-readiness independently; pending sibling coach lanes are not blockers and must not force `merge_ready=no`,
   - lane-local tool/environment gaps are recorded in verification notes/results unless they prove a concrete implementation defect,
   - runtime may recover coach evidence from ordered fallback sources, but only a valid machine-readable coach verdict may approve the route,
   - any structured rework handoff must include coach feedback provenance before the next writer pass consumes it,
   - if coach returns `return_for_rework`: emit the structured fresh-start rework handoff, rerun `prepare-execution`, and go back to `IEP-5 Implement Loop` using the effective prompt from that handoff instead of prior writer context,
   - if coach/review evidence proves a compile blocker in the current mutated packet, treat that as rework-routing evidence, not as implicit permission for root-session local repair; local repair still requires an explicit pre-write exception-path receipt from the orchestration layer,
   - if the coach quorum approves: continue to final verification.
11. `IEP-6 Verify And Review`
   - regression checks + independent review + API live validation (when applicable).
11.1. `IEP-6.1 Review-Pool Intake`
   - when the active task belongs to a declared `review_pool`, do not treat single-task completion as closure-ready until the pool reaches its declared merge checkpoint.
   - admit a task to a review pool only when:
     - the task is individually verifier-ready,
     - the task shares the same milestone and merge checkpoint as its siblings,
     - writable scope overlap is already resolved or serialized,
     - the current result bundle includes `done_verdict`, `stop_reason`, and `residual_risks`.
   - if review-pool admissibility fails: `BLOCKED (BLK_REVIEW_POOL_PENDING)`.
11.2. `IEP-6.2 Human Approval Gate` (when the selected route or verifier manifest lands in `policy_gate_required`, `senior_review_required`, or `human_gate_required`)
   - record a matching approval or rejection receipt through `runtime-instructions/work.human-approval-protocol`,
   - missing approval receipt keeps the task in `approval_pending`,
   - rejection receipt blocks closure-ready state and feeds the next rework/escalation decision.
12. `IEP-7 Close And Continue`
   - first reconcile the just-finished `execution_block` against the parent `delivery_task_card`,
   - if the parent task is still open:
     - do not close the task,
     - do not run next-task boundary analysis yet,
     - shape the next lawful in-task leaf or fail closed with an explicit blocker/escalation receipt,
   - only after the parent task is actually closed: close task in the DB-backed runtime, sync logs, auto-pick next `ready` task,
   - before starting that next task, run the `instruction-contracts/overlay.autonomous-execution-protocol` boundary step when continuous autonomy is active:
     - inspect nearby specs/protocols and controlling code for the next slice,
     - produce a brief implementation-plan report outside the next task's TaskFlow gating,
     - refresh dependent spec/task coverage if the boundary analysis discovers stale or missing ownership.
13. `IEP-8 Pool Completion`
   - final summary, documentation/spec synchronization, completion verdict.

Layer boundary:

1. `CL1` and `CL2` confirm that the approved queue is executable.
2. `CL3` is the only decision checkpoint allowed after launch and before code mutation.
2.1. `IEP-4.2` is the execution-authorization stop-gate between routing decisions and any local implementation prep.
3. `CL4` owns implementation changes for the current ready task.
4. `CL5` owns verification, closure, and queue handoff only.
4.1. The task-boundary analysis/report step belongs to handoff, not to the next task's implementation loop.

Hard law:

1. Multi-task execution without `IEP-1.5 Pool Graph Analysis` is invalid.
2. Parallelism decisions must be graph-backed, not intuitive.
3. Single-writer serialization is mandatory unless explicit isolation is proven.
4. Under active worker mode, generic autonomous-coding defaults do not authorize local orchestrator-first implementation.
5. Missing `IEP-4.2 Execution Authorization Gate` is a blocking protocol violation, not a soft warning.
6. If an implementation action, fallback path, or local mutation step is not explicitly described by the active VIDA/project protocol stack or justified by an escalation receipt, it is forbidden by default.
7. For write-producing routes in `hybrid`, the canonical default is `analysis -> writer -> coach -> review` when `coach_required=yes`; otherwise it remains `analysis -> writer -> review`. Bounded writer dispatch without the analysis receipt is invalid.
8. Continuous autonomy does not authorize skipping the post-task boundary analysis/report step before the next task starts.
9. Boundary-discovered spec/task drift must be reconciled before the next task is treated as lawfully executable.
9.1. `execution_block` closure inside an open `delivery_task` does not trigger `IEP-7` task-boundary behavior; it must return to in-task reconciliation first.
10. Execution without an active bounded `delivery_task_card` is protocol-invalid even if broader spec context exists.
11. Runtime control exhaustion must stop the current path; it does not authorize one more silent retry.
12. A resumable implementation run is lawful only when the current `execution_block`, control counters, next verification target, and next resumable node are checkpoint-visible.
13. Review-pool membership does not bypass per-task verification; it only delays merge admissibility until the declared checkpoint.

## Change-Impact Gate

Drift triggers:

1. acceptance criteria changed during execution,
2. new dependency discovered that changes task order/scope,
3. decision update invalidates current implementation assumptions,
4. active task no longer matches approved spec.

On trigger:

1. set blocker `BLK_CHANGE_IMPACT_PENDING`,
2. stop implementation for current queue,
3. execute the canonical reconciliation route from `runtime-instructions/work.change-impact-reconciliation-protocol`,
4. resume only after explicit launch confirmation is renewed.
5. record drift alert from `context-drift-sentinel.sh` evidence in logs/report.

## Multi-Agent And Parallelism Policy

1. Default topology: single writer lane.
2. Parallel workers are allowed for read-heavy tasks:
   - discovery,
   - risk analysis,
   - review triage,
   - docs checks.
3. When a downstream or dependent task needs preparation but must not enter writer execution yet, use a prep-only route such as `read_only_prep` and keep writer authorization blocked.
4. Parallel write lanes are forbidden unless explicit isolation exists (separate worktrees + merge gate).
5. Keep main thread clean: workers return concise artifacts, not raw noisy logs.
6. When route metadata declares `fanout_workers`, use that fanout only for read-only phases and keep the writer lane singular under the orchestrator.
7. If route metadata declares hard routing requirements, local/manual bypass is invalid unless a lawful escalation receipt exists.

## Skills Policy

1. Skills are selected dynamically per task.
2. Minimal sufficient set only.
3. If capability missing, scaffold project skill candidate and continue with best fallback.

## Verification Policy

1. Every completed task must pass:
   - targeted tests for changed behavior,
   - regression checks for touched modules,
   - code review findings triaged (bugs/risks first).
2. For server/API behaviors: live request validation is mandatory evidence.
3. For package/platform/security/migration decisions during implementation, execute WVP and log evidence.
4. No silent error handling in new/changed code paths.
5. Single-verifier closure is lawful only when verifier independence is explicit through a verifier-independence receipt or an explicit override receipt.
6. Review-pool closure is lawful only through `runtime-instructions/work.verification-merge-protocol`.

## Blocker Codes

1. `BLK_LAUNCH_NOT_CONFIRMED`
2. `BLK_SPEC_CONTEXT_MISSING`
3. `BLK_AC_MISSING`
4. `BLK_API_REALITY_MISSING`
5. `BLK_VERIFY_FAILED`
6. `BLK_ENVIRONMENT_UNREADY`
7. `BLK_CHANGE_IMPACT_PENDING`
8. `BLK_CONTEXT_NOT_HYDRATED`
9. `BLK_POOL_GRAPH_MISSING`
10. `BLK_EXECUTION_AUTH_MISSING`
11. `BLK_DELIVERY_TASK_CARD_MISSING`
12. `BLK_RUNTIME_CONTROL_EXHAUSTED`
13. `BLK_REVIEW_POOL_PENDING`

## Exit States

1. `SUCCESS`: task pool completed and verified.
2. `BLOCKED`: execution cannot proceed due to unresolved blocker.
3. `PARTIAL`: safe checkpoint saved, can resume.
4. `STOP`: hard safety stop or unresolved critical contradiction.

## Logging Requirements

1. Execute only via TaskFlow blocks (`block-start -> block-end -> reflect -> verify`).
2. Record evidence for each gate.
3. Before reporting completion to user, ensure TaskFlow sync is visible.
4. Keep `vida taskflow task` as the only task-state source of truth.
5. Emit Telemetry V1 events with minimum fields: `trace_id`, `task_id`, `block_id`, `action`, `duration_ms`, `result`, `success`.
6. At each resumable boundary, checkpoint at minimum:
   - `delivery_task_id`
   - `execution_block_id`
   - current `definition_of_done` progress signal
   - run-graph control counters
   - next `review_pool` or verification target
   - `resume_hint`

## Output Schema

1. `Current Task`: id + short description.
2. `Completed In Iteration`: task id + changes + verification result.
3. `Open Blockers`: blocker code + required action.
4. `Next Task`: next ready id + short description.
5. `Pool Status`: done / remaining / blocked counts.
6. `Control Status`: rounds / stalls / resets / budget summary.

-----
artifact_path: config/command-instructions/implement-execution.protocol
artifact_type: command_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/command-instructions/execution.implement-execution-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-13T12:39:11+02:00'
changelog_ref: execution.implement-execution-protocol.changelog.jsonl
