# Protocol Self-Diagnosis Protocol

Purpose: define a canonical self-diagnosis layer for VIDA runtime protocols so the orchestrator can detect and correct protocol drift in TODO flow, autonomous execution, subagent routing, spec sync, approval gating, and debug escalation.

## Core Contract

1. Protocol self-diagnosis audits whether active runtime behavior actually matches the active protocol stack.
2. It is not limited to code defects; it also covers orchestration drift, missing task coverage, stale specs, invalid fallbacks, and reporting barriers.
3. Findings must be captured as explicit framework follow-up evidence, not left in chat memory only.
4. When an issue is detected, prefer updating existing framework tasks/specs before creating new ones.

## Scope

This protocol applies to runtime behavior governed by at least:

1. `_vida/docs/todo-protocol.md`
2. `_vida/docs/autonomous-execution-protocol.md`
3. `_vida/docs/subagent-system-protocol.md`
4. `_vida/docs/spec-sync-protocol.md`
5. `_vida/docs/task-approval-loop-protocol.md`
6. `_vida/docs/debug-escalation-protocol.md`

## Trigger Conditions

Run protocol self-diagnosis when any are true:

1. the orchestrator pauses after reporting even though continuous autonomous execution is active,
1.1. the orchestrator fails to present the next task at task boundary when the hybrid next-task boundary variant is active,
1.2. the orchestrator still emits a blocking next-task boundary report after the user has disabled that variant,
1.3. the orchestrator skips the required non-gating next-task boundary analysis/report after a complex task closes,
2. a completed code slice did not receive the required catch/review lane,
3. a new executable spec requirement has no task coverage,
4. a task/protocol/spec decision contradicts current runtime behavior,
5. repeated local debugging continued without the required escalation path,
6. subagent-first law was bypassed without lawful exhaustion/blocker evidence,
7. TODO execution drifted outside an active block,
8. task/task-pool progression no longer matches canonical next-task sources.

## Mandatory Checks

Minimum self-diagnosis checks:

1. `execution_continuity`
   - did reporting or narration incorrectly stop lawful next-task execution?
   - or did the orchestrator skip a required next-task boundary gate when that variant was active?
   - or did the orchestrator preserve a boundary gate after the user disabled it?
   - or did the orchestrator skip the mandatory non-gating boundary analysis/report/update pass before entering the next task?
2. `task_coverage`
   - do all new executable spec requirements map to an existing updated task or a newly created task?
   - did boundary-discovered executable scope produce updated or newly created dependent task/spec coverage before continuation?
3. `verification_coverage`
   - did each completed write-producing slice receive required verify/catch-review coverage?
4. `route_compliance`
   - did runtime use the required subagent/external/debug escalation paths?
5. `spec_runtime_consistency`
   - does behavior still match the nearest governing specs/protocols?
6. `fail_closed_integrity`
   - were missing inputs/conflicts/cycles/escalation gaps treated as blockers rather than silently tolerated?

## Drift Classes

Canonical diagnosis classes:

1. `reporting_barrier_drift`
1.1. `missing_boundary_gate`
1.2. `stale_boundary_gate`
1.3. `missing_boundary_analysis`
2. `missing_task_coverage`
3. `stale_spec_drift`
4. `verification_gap`
5. `route_bypass`
6. `invalid_fallback`
7. `execution_outside_todo`
8. `escalation_gap`

## Remediation Order

When self-diagnosis finds drift:

1. capture or reuse the framework issue/task,
2. record the smallest correct workaround in current task evidence,
3. fix the nearest governing protocol/spec if it is stale,
4. update existing task coverage or create missing task coverage if required,
5. resume autonomous execution only after the drift no longer blocks lawful continuation.

## No-Pause-After-Reporting Rule

When continuous autonomous execution is active:

1. a user-facing progress report is not a stop condition,
2. a report must not become an implicit approval gate,
3. if the orchestrator stops after reporting without another lawful blocker, that is `reporting_barrier_drift`,
4. if the orchestrator advances into the next task without the required task-boundary analysis/report/update pass, that is `missing_boundary_analysis`.

## Catch-Review Rule

For completed write-producing slices:

1. catch/review is mandatory when an eligible lane exists,
2. if it was skipped, record `verification_gap`,
3. the gap must be corrected before claiming the slice is fully settled.

## Fail-Closed Rule

1. Do not keep executing as if protocol drift were harmless once it is detected.
2. Do not leave protocol fixes only as conversational intent.
3. Do not allow new executable spec scope to remain unowned in backlog/task coverage.
