# Implement Execution Protocol (IEP)

Purpose: define one canonical development execution flow after `/vida-form-task` launch approval.

Scope:

1. Command mode: `/vida-implement`.
2. Applies to autonomous development execution for a ready task pool in `br`.
3. Uses one canonical command (`/vida-implement`) and forbids historical split aliases as runtime path.
4. When the user explicitly wants the agent to keep following a settled plan/spec/task pool to completion, activate `docs/framework/autonomous-execution-protocol.md` as the trigger/stop doctrine layered on top of this protocol.
5. When queue selection or reprioritization is ambiguous, activate `docs/framework/execution-priority-protocol.md` before selecting the next writer task.

## Core Contract

`/vida-implement` must:

1. accept only tasks that passed form-task launch gate,
2. build or refresh the active pool dependency graph before selecting the next writer task when the scope includes multiple tasks/subtasks,
3. pick next `ready` task(s) from `br`,
4. execute implementation loop to completion or explicit blocker,
5. run mandatory verification and review gates,
6. continue automatically with next ready task until pool completion.

## Mandatory Inputs

1. `br` active task/pool context.
2. Approved spec and acceptance criteria, or approved `issue_contract` for equivalent bug paths.
2.1. When the work originated from mixed research/release/user-negotiation inputs, a normalized `spec_intake` artifact must already have routed that work into SCP or ICP before launch.
2.2. `draft_execution_spec` is a pre-launch review artifact only; it does not authorize `/vida-implement` by itself and must be absorbed by the canonical form-task launch path before writer execution.
3. Research evidence relevant to scope.
4. External API reality evidence (when integration exists).
5. Decision log (`docs/decisions.md`) and feature checklist entries.
6. `docs/framework/web-validation-protocol.md` for external assumptions during execution.
7. Hydrated context capsule for active task (`.vida/logs/context-capsules/<task_id>.json`).

## Command Layer Mapping

For `/vida-implement`, IEP layers map to CLP as follows:

1. `CL1 Intake` -> `IEP-0 Launch Intake` + `IEP-1 Context Hydration`
2. `CL2 Reality And Inputs` -> `IEP-1.5 Pool Graph Analysis` + `IEP-2 Queue Intake` + `IEP-3 Skills Routing` + `IEP-4 Preflight`
3. `CL3 Contract And Decisions` -> `IEP-4.5 Change-Impact Gate`
4. `CL4 Materialization` -> `IEP-5 Implement Loop`
5. `CL5 Gates And Handoff` -> `IEP-6 Verify And Review` + `IEP-7 Close And Continue` + `IEP-8 Pool Completion`

Canonical layer source: `docs/framework/command-layer-protocol.md`

## Gate Sequence (Canonical)

1. `IEP-0 Launch Intake`
   - confirm launch approved by `/vida-form-task`.
   - if no approval: `BLOCKED (BLK_LAUNCH_NOT_CONFIRMED)`.
2. `IEP-1 Context Hydration`
   - load spec/research/decisions/contracts for selected task.
3. `IEP-1.5 Pool Graph Analysis`
   - for epic, wave, and multi-task execution, derive the active dependency graph before choosing a writer lane.
   - classify `ready`, `blocked`, `soft-blocked`, `parallel_read_only`, and `single_writer`.
   - if the graph is missing, stale, or contradictory: `BLOCKED (BLK_POOL_GRAPH_MISSING)`.
4. `IEP-2 Queue Intake`
   - select next `ready` task from `br`.
   - if none: move to `IEP-8 Pool Completion`.
   - if multiple candidates remain after queue intake, apply `docs/framework/execution-priority-protocol.md` and keep `docs/framework/agent-system-protocol.md` route law active while selecting the next writer task.
5. `IEP-3 Skills Routing`
   - run dynamic skill selection for current task scope.
6. `IEP-4 Preflight`
   - baseline checks, dependency readiness, risk scan.
7. `IEP-4.2 Execution Authorization Gate`
   - confirm route receipt, analysis lane, analysis receipt (when required), `issue_contract` readiness when the task is issue-driven, non-empty `issue_contract.proven_scope`, symptom-level evidence for any multi-symptom issue, author lane, verifier lane (or explicit `no_eligible_verifier`), and writer ownership before deep local implementation prep.
   - if analysis routing is unavailable because the route records explicit `no_eligible_analysis_lane`, remain fail-closed by default; only framework-owned tracked remediation may proceed via a structured execution-auth override receipt, never by silent local fallback.
   - if `issue_contract` emits a mixed-issue split artifact, keep writer ownership on the primary executable slice only and preserve the unresolved slice as follow-up work.
   - if local mutation is proposed under active worker mode, require route authorization or lawful escalation receipt.
   - if the gate is not satisfied: `BLOCKED (BLK_EXECUTION_AUTH_MISSING)`.
8. `IEP-4.5 Change-Impact Gate`
   - detect scope/AC/dependency/decision drift before continuing.
   - if drift detected: stop and route through reflection+spec+form-task reconciliation.
9. `IEP-5 Implement Loop`
   - code changes + tests + incremental checks.
10. `IEP-5.5 Coach Review` (when the selected route declares `coach_required=yes`)
   - run the post-write coach ensemble against the current implementation,
   - default policy is two independent cheaper coaches when the route exposes enough eligible lanes,
   - approve only when the required coach quorum approves; any valid `return_for_rework` vote blocks advancement,
   - each coach lane judges verifier-readiness independently; pending sibling coach lanes are not blockers and must not force `merge_ready=no`,
   - lane-local tool/environment gaps are recorded in verification notes/results unless they prove a concrete implementation defect,
   - runtime may recover coach evidence from ordered fallback sources, but only a valid machine-readable coach verdict may approve the route,
   - any structured rework handoff must include coach feedback provenance before the next writer pass consumes it,
   - if coach returns `return_for_rework`: emit the structured fresh-start rework handoff, rerun `prepare-execution`, and go back to `IEP-5 Implement Loop` using the effective prompt from that handoff instead of prior writer context,
   - if the coach quorum approves: continue to final verification.
11. `IEP-6 Verify And Review`
   - regression checks + independent review + API live validation (when applicable).
11.1. `IEP-6.2 Human Approval Gate` (when the selected route or verifier manifest lands in `policy_gate_required`, `senior_review_required`, or `human_gate_required`)
   - record a matching approval or rejection receipt through `docs/framework/human-approval-protocol.md`,
   - missing approval receipt keeps the task in `approval_pending`,
   - rejection receipt blocks closure-ready state and feeds the next rework/escalation decision.
12. `IEP-7 Close And Continue`
   - close task in `br`, sync logs, auto-pick next `ready` task,
   - before starting that next task, run the `docs/framework/autonomous-execution-protocol.md` boundary step when continuous autonomy is active:
     - inspect nearby specs/protocols and controlling code for the next slice,
     - produce a brief implementation-plan report outside the next task's TODO gating,
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

## Change-Impact Gate (Absorbed Cascade)

Drift triggers:

1. acceptance criteria changed during execution,
2. new dependency discovered that changes task order/scope,
3. decision update invalidates current implementation assumptions,
4. active task no longer matches approved spec.

On trigger:

1. set blocker `BLK_CHANGE_IMPACT_PENDING`,
2. stop implementation for current queue,
3. execute reconciliation route: `reflection-pack -> /vida-spec review -> /vida-form-task`,
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

## Exit States

1. `SUCCESS`: task pool completed and verified.
2. `BLOCKED`: execution cannot proceed due to unresolved blocker.
3. `PARTIAL`: safe checkpoint saved, can resume.
4. `STOP`: hard safety stop or unresolved critical contradiction.

## Logging Requirements

1. Execute only via TODO blocks (`block-start -> block-end -> reflect -> verify`).
2. Record evidence for each gate.
3. Before reporting completion to user, ensure TODO sync is visible.
4. Keep `br` as the only task-state source of truth.
5. Emit Telemetry V1 events with minimum fields: `trace_id`, `task_id`, `block_id`, `action`, `duration_ms`, `result`, `success`.

## Output Schema

1. `Current Task`: id + short description.
2. `Completed In Iteration`: task id + changes + verification result.
3. `Open Blockers`: blocker code + required action.
4. `Next Task`: next ready id + short description.
5. `Pool Status`: done / remaining / blocked counts.
