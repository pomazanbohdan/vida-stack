# Protocol Self-Diagnosis Protocol

Purpose: define a canonical self-diagnosis layer for VIDA runtime protocols so the orchestrator can detect and correct protocol drift in TaskFlow, autonomous execution, worker routing, spec sync, approval gating, and debug escalation.

## Core Contract

1. Protocol self-diagnosis audits whether active runtime behavior actually matches the active protocol stack.
2. It is not limited to code defects; it also covers orchestration drift, missing task coverage, stale specs, invalid fallbacks, and reporting barriers.
3. Findings must be captured as explicit framework follow-up evidence, not left in chat memory only.
4. When an issue is detected, prefer updating existing framework tasks/specs before creating new ones.

## Scope

This protocol applies to runtime behavior governed by at least:

1. `runtime-instructions/work.taskflow-protocol`
2. `instruction-contracts/overlay.autonomous-execution-protocol`
3. `instruction-contracts/core.agent-system-protocol`
4. `runtime-instructions/bridge.spec-sync-protocol`
5. `runtime-instructions/bridge.task-approval-loop-protocol`
6. `diagnostic-instructions/escalation.debug-escalation-protocol`

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
6. worker-first law was bypassed without lawful exhaustion/blocker evidence,
7. TaskFlow execution drifted outside an active block,
8. task/task-pool progression no longer matches canonical next-task sources,
9. a closed `execution_block` was mistaken for parent-task or session closure even though lawful in-task continuation still existed.
10. the orchestrator stopped after dispatching a delegated lane even though no delegated return, blocker, or supersession had yet been synthesized.
11. the orchestrator silently narrowed `continue development` to a local symptom repair and stopped after that symptom turned green without rebuilding the parent bounded unit.
12. the orchestrator closed one bounded item and stopped even though the next lawful bounded item had already been identified by continuation evidence.

## Mandatory Checks

Minimum self-diagnosis checks:

1. `execution_continuity`
   - did reporting or narration incorrectly stop lawful next-task execution?
   - did the orchestrator mistake `execution_block` closure for `delivery_task` closure and stop before rebuilding the next lawful in-task leaf?
   - or did the orchestrator skip a required next-task boundary gate when that variant was active?
   - or did the orchestrator preserve a boundary gate after the user disabled it?
   - or did the orchestrator skip the mandatory non-gating boundary analysis/report/update pass before entering the next task?
   - or did the orchestrator stop after dispatch, treating `agents are running` as a natural pause before the first synthesized return or blocker?
   - or did the orchestrator treat a local green test/compile result as task closure without re-binding or rebuilding the parent bounded unit?
   - or did the orchestrator stop after one item closed even though verifier/subagent/taskflow evidence already named the next lawful item?
2. `task_coverage`
   - do all new executable spec requirements map to an existing updated task or a newly created task?
   - did boundary-discovered executable scope produce updated or newly created dependent task/spec coverage before continuation?
3. `verification_coverage`
   - did each completed write-producing slice receive required verify/catch-review coverage?
4. `route_compliance`
   - did runtime use the required worker/external/debug escalation paths?
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
1.4. `boundary_level_mismatch`
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
4. if the orchestrator advances into the next task without the required task-boundary analysis/report/update pass, that is `missing_boundary_analysis`,
5. if the orchestrator stops because it confused `execution_block` closure with parent-task closure, that is `boundary_level_mismatch`.
6. if the orchestrator stops after delegated dispatch while the cycle is merely in-flight, that is also `reporting_barrier_drift`.
7. if the orchestrator collapses an active development context into a symptom-only local fix and stops after that bounded success, that is `boundary_level_mismatch` or `invalid_fallback` depending on whether active task binding was lost.
8. if the orchestrator stops after one bounded item closes even though continuation evidence already identified the next lawful item, that is `boundary_level_mismatch` and `reporting_barrier_drift`.

## Catch-Review Rule

For completed write-producing slices:

1. catch/review is mandatory when an eligible lane exists,
2. if it was skipped, record `verification_gap`,
3. the gap must be corrected before claiming the slice is fully settled.

## Fail-Closed Rule

1. Do not keep executing as if protocol drift were harmless once it is detected.
2. Do not leave protocol fixes only as conversational intent.
3. Do not allow new executable spec scope to remain unowned in backlog/task coverage.

-----
artifact_path: config/diagnostic-instructions/protocol-self-diagnosis.protocol
artifact_type: diagnostic_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/diagnostic-instructions/analysis.protocol-self-diagnosis-protocol.md
created_at: '2026-03-09T12:00:46+02:00'
updated_at: '2026-03-13T12:39:11+02:00'
changelog_ref: analysis.protocol-self-diagnosis-protocol.changelog.jsonl
