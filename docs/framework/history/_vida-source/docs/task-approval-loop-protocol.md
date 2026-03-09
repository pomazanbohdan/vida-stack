# Task Approval Loop Protocol

Purpose: define an optional execution mode where autonomous development pauses between tasks for user approval, while still implementing each approved task to completion.

Activation rule:

1. This protocol is inactive by default unless the user explicitly asks for between-task approval or the active route contract explicitly requires it.
2. A direct user switch back to continuous autonomous execution suspends this protocol immediately for subsequent task boundaries.
3. While suspended, next-task analysis must still happen under `_vida/docs/autonomous-execution-protocol.md`; the suspension removes the wait, not the analysis discipline.
4. While suspended, the default brief boundary plan report may remain active unless the user explicitly disables user-facing boundary reporting.
5. A user may request a hybrid mode: continuous execution inside the current task, but a brief report-and-approval gate only at the boundary before each new task.

Override rule:

1. If the user explicitly switches back to continuous autonomous execution, this protocol no longer inserts a mandatory wait between completed tasks.
2. In that mode, the orchestrator may still present concise progress reports, but reporting must not pause lawful next-task execution.
3. The protocol remains available for future re-activation when the user asks for between-task approval again.

## Core Contract

1. Before starting the next task, gather the relevant specs, nearby tasks, and implementation context.
2. Perform bounded meta-analysis of the next lawful task.
3. Present the proposed next task to the user for approval outside TODO execution.
4. Wait for user approval before starting that next task.
5. After approval, update nearby specs/protocols as needed and implement the task fully.
6. Repeat the same loop for the following task only while this protocol remains active.

When continuous autonomous execution override is active:

1. keep doing the same next-task analysis internally,
2. keep refreshing dependent spec/task coverage when the boundary analysis discovers drift or missing ownership,
3. default to a concise non-gating boundary plan report unless the user explicitly disables that report,
4. do not wait for user approval after reporting task completion,
5. continue directly into the next lawful task unless a separate stop condition or approval contract applies.

## What Happens Before Approval

1. collect the next-task candidate,
2. collect governing specs and nearby protocols,
3. collect known blockers and dependencies,
4. check whether fresher date-bearing specs or architectural updates supersede older nearby artifacts,
5. prefer updating existing tasks/specs over creating new ones,
6. create a new task only if no existing plan/task already covers the approved scope,
7. if a newly added spec requirement creates executable scope, ensure task coverage is updated before considering the planning step complete,
7. summarize the intended implementation slice,
8. present it for user approval.

When this protocol is inactive or suspended:

1. the same collection and coverage-refresh step still belongs to the autonomous next-task boundary pass,
2. only the approval wait is removed,
3. a disabled user-facing boundary report does not disable the underlying analysis/update step.

## What Happens After Approval

1. update stale or missing specs if the task needs them,
2. update the nearest existing planned/tracked task when that scope already exists,
3. create a new task only when coverage is genuinely missing,
4. enter TODO/tracked execution for that task,
3. implement to completion,
4. verify and reconcile,
5. stop before the following task and repeat the approval loop.

Continuous-autonomy override:

1. when explicitly enabled by the user, replace item 5 with immediate progression into the next lawful task,
2. preserve concise reporting, but do not make reporting a blocking gate.
3. do not restate approval requests at later task boundaries unless the user re-enables this protocol.
4. if the user explicitly disables next-task boundary reporting as well, skip only the user-facing report and continue directly into the next lawful task after the internal boundary analysis/update step.

Next-task boundary variant:

1. If the user asks to keep continuous execution but still wants the next task briefly agreed, apply this protocol only at task boundaries.
2. Under this variant, do not pause after ordinary progress or completion reports for the current task.
3. Present only a concise next-task proposal at the boundary before the next task starts.
4. Wait only at that boundary gate.
5. If the user later disables this variant, remove the boundary gate immediately and return to uninterrupted autonomous continuation.

## Relationship To Autonomous Execution

1. This protocol does not disable autonomous implementation inside an approved task.
2. It inserts a user approval gate between tasks, not between micro-steps.
3. Inside the approved task, normal autonomous follow-through still applies unless another approval contract overrides it.
4. A direct user instruction to continue automatically across tasks suspends the between-task wait while keeping all other TODO, verification, and scope controls intact.
5. When suspended, `_vida/docs/autonomous-execution-protocol.md` owns between-task continuation behavior.
6. Under the next-task boundary variant, this protocol owns only the single concise boundary gate; all intra-task behavior remains under autonomous execution.
7. This protocol must not suppress the default boundary analysis/report/update behavior owned by `_vida/docs/autonomous-execution-protocol.md`; it may only add or remove the approval wait.

## Fail-Closed Rule

1. Do not silently start the next task when this mode is active and approval has not yet been given.
2. Do not treat task approval as permission to widen into multiple future tasks without repeating the loop.
3. Under continuous-autonomy override, do not reintroduce a blocking wait unless the user re-enables it or another protocol explicitly requires approval.
