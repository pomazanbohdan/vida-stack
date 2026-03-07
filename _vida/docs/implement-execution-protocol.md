# Implement Execution Protocol (IEP)

Purpose: define one canonical development execution flow after `/vida-form-task` launch approval.

Scope:

1. Command mode: `/vida-implement`.
2. Applies to autonomous development execution for a ready task pool in `br`.
3. Uses one canonical command (`/vida-implement`) and forbids historical split aliases as runtime path.

## Core Contract

`/vida-implement` must:

1. accept only tasks that passed form-task launch gate,
2. pick next `ready` task(s) from `br`,
3. execute implementation loop to completion or explicit blocker,
4. run mandatory verification and review gates,
5. continue automatically with next ready task until pool completion.

## Mandatory Inputs

1. `br` active task/pool context.
2. Approved spec and acceptance criteria.
3. Research evidence relevant to scope.
4. External API reality evidence (when integration exists).
5. Decision log (`docs/decisions.md`) and feature checklist entries.
6. `_vida/docs/web-validation-protocol.md` for external assumptions during execution.
7. Hydrated context capsule for active task (`.vida/logs/context-capsules/<task_id>.json`).

## Command Layer Mapping

For `/vida-implement`, IEP layers map to CLP as follows:

1. `CL1 Intake` -> `IEP-0 Launch Intake` + `IEP-1 Context Hydration`
2. `CL2 Reality And Inputs` -> `IEP-2 Queue Intake` + `IEP-3 Skills Routing` + `IEP-4 Preflight`
3. `CL3 Contract And Decisions` -> `IEP-4.5 Change-Impact Gate`
4. `CL4 Materialization` -> `IEP-5 Implement Loop`
5. `CL5 Gates And Handoff` -> `IEP-6 Verify And Review` + `IEP-7 Close And Continue` + `IEP-8 Pool Completion`

Canonical layer source: `_vida/docs/command-layer-protocol.md`

## Gate Sequence (Canonical)

1. `IEP-0 Launch Intake`
   - confirm launch approved by `/vida-form-task`.
   - if no approval: `BLOCKED (BLK_LAUNCH_NOT_CONFIRMED)`.
2. `IEP-1 Context Hydration`
   - load spec/research/decisions/contracts for selected task.
3. `IEP-2 Queue Intake`
   - select next `ready` task from `br`.
   - if none: move to `IEP-8 Pool Completion`.
4. `IEP-3 Skills Routing`
   - run dynamic skill selection for current task scope.
5. `IEP-4 Preflight`
   - baseline checks, dependency readiness, risk scan.
6. `IEP-4.5 Change-Impact Gate`
   - detect scope/AC/dependency/decision drift before continuing.
   - if drift detected: stop and route through reflection+spec+form-task reconciliation.
7. `IEP-5 Implement Loop`
   - code changes + tests + incremental checks.
8. `IEP-6 Verify And Review`
   - regression checks + code review + API live validation (when applicable).
9. `IEP-7 Close And Continue`
   - close task in `br`, sync logs, auto-pick next `ready` task.
10. `IEP-8 Pool Completion`
   - final summary, documentation/spec synchronization, completion verdict.

Layer boundary:

1. `CL1` and `CL2` confirm that the approved queue is executable.
2. `CL3` is the only decision checkpoint allowed after launch and before code mutation.
3. `CL4` owns implementation changes for the current ready task.
4. `CL5` owns verification, closure, and queue handoff only.

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
2. Parallel subagents are allowed for read-heavy tasks:
   - discovery,
   - risk analysis,
   - review triage,
   - docs checks.
3. Parallel write lanes are forbidden unless explicit isolation exists (separate worktrees + merge gate).
4. Keep main thread clean: subagents return concise artifacts, not raw noisy logs.
5. When route metadata declares `fanout_subagents`, use that fanout only for read-only phases and keep the writer lane singular under the orchestrator.

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
