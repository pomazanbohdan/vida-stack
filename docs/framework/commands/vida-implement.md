# /vida-implement — Unified Development Execution

Purpose: single execution command for development after `/vida-form-task` launch confirmation.

## Protocol Layers

This command maps layers as:

1. `CL1 Intake` -> `IEP-0 Launch Intake` + `IEP-1 Context Hydration`
2. `CL2 Reality And Inputs` -> `IEP-2 Queue Intake` + `IEP-3 Skills Routing` + `IEP-4 Preflight`
3. `CL3 Contract And Decisions` -> `IEP-4.5 Change-Impact Gate`
4. `CL4 Materialization` -> `IEP-5 Implement Loop`
5. `CL5 Gates And Handoff` -> `IEP-6 Verify And Review` + `IEP-7 Close And Continue` + `IEP-8 Pool Completion`

Canonical source: `docs/framework/history/_vida-source/docs/command-layer-protocol.md`

Execution boundary:

1. `/vida-implement` owns only post-launch development execution.
2. Scope/task-pool decisions stay upstream in `/vida-form-task`.
3. `CL4` remains single-writer unless isolated worktrees are explicitly approved.

## Runtime Position

1. `/vida-research` -> external/business evidence.
2. `/vida-spec` -> technical contract.
3. `/vida-form-task` -> task pool, dependencies, readiness, launch gate.
4. `/vida-implement` -> execution loop until pool completion or blocker.

## Mandatory Reads Before Execution

1. `docs/framework/history/_vida-source/docs/implement-execution-protocol.md`.
2. `docs/framework/history/_vida-source/docs/form-task-protocol.md`.
3. `docs/framework/history/_vida-source/docs/spec-contract-protocol.md`.
4. `docs/framework/history/_vida-source/docs/web-validation-protocol.md`.
5. `docs/framework/history/_vida-source/docs/beads-protocol.md`.
6. `docs/framework/history/_vida-source/docs/todo-protocol.md`.
7. `docs/framework/history/_vida-source/docs/subagents.md`.
8. `docs/decisions.md`.

## Inputs

1. Active `br` task context.
2. Ready queue created by `/vida-form-task`.
3. Approved spec + AC + research evidence.
4. API reality evidence (if integrations are in scope).
5. WVP evidence for external assumptions.

## Canonical Flow (IEP)

Use `docs/framework/history/_vida-source/docs/implement-execution-protocol.md` as source of truth.

1. `IEP-0 Launch Intake`.
2. `IEP-1 Context Hydration`.
3. `IEP-2 Queue Intake`.
4. `IEP-3 Skills Routing`.
5. `IEP-4 Preflight`.
6. `IEP-4.5 Change-Impact Gate`.
7. `IEP-5 Implement Loop`.
8. `IEP-6 Verify And Review`.
9. `IEP-7 Close And Continue`.
10. `IEP-8 Pool Completion`.

Layer interpretation:

1. `CL1` confirms that execution is allowed for this queue.
2. `CL2` gathers the ready task, required context, and execution prerequisites.
3. `CL3` absorbs scope/AC/decision drift before code changes begin.
4. `CL4` performs the implementation loop for the current ready task.
5. `CL5` verifies, closes, and hands off to the next ready task or pool-complete verdict.

## Execution Rules

1. Start only if launch was explicitly confirmed in `/vida-form-task`.
2. Work through `br` ready queue sequentially by default.
3. Subagents allowed for read-heavy and review-heavy work.
4. Keep one write lane unless isolated worktrees are explicitly used.
5. For server/API behavior, validate assumptions with live requests.
6. For package/platform/security/migration choices, execute WVP and log evidence.
7. Do not finalize with hotfix-style symptoms-only changes.
8. If scope/AC/dependency/decision drift is detected, stop and route through `reflection-pack -> /vida-spec review -> /vida-form-task`.

## Quality Gates

1. Targeted tests for changed behavior.
2. Regression checks for touched modules.
3. Code review pass (bugs/risks first).
4. Documentation/spec sync for accepted decisions.
5. `reflect` + `verify` before reporting completion.

## Output Contract

1. Current task id + short description.
2. What was implemented and verified in this iteration.
3. Open blockers with blocker code and action.
4. Next ready task id + short description.
5. Pool progress (done/remaining/blocked).

Key blocker for absorbed cascade behavior:

1. `BLK_CHANGE_IMPACT_PENDING`.

## Related

1. `docs/framework/history/_vida-source/docs/implement-execution-protocol.md`
2. `docs/framework/history/_vida-source/docs/form-task-protocol.md`
3. `docs/framework/history/_vida-source/docs/use-case-packs.md`
4. `docs/framework/history/_vida-source/docs/todo-protocol.md`
5. `docs/framework/commands/vida-form-task.md`
