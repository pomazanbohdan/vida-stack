# Autonomous Execution Protocol (AEP)

Purpose: define the canonical follow-through mode for executing a settled VIDA plan/spec/task pool to completion with minimal re-planning overhead while preserving TaskFlow, routing, verification, and fail-closed behavior.

Scope:

1. activates only after the task is already in tracked execution,
2. applies when the user intent is "continue until done", "follow the plan", "implement all remaining work", or equivalent,
3. reuses existing execution law from `vida/config/instructions/runtime-instructions.taskflow-protocol.md`, `vida/config/instructions/runtime-instructions.beads-protocol.md`, `vida/config/instructions/command-instructions.implement-execution-protocol.md`, and `vida/config/instructions/instruction-contracts.agent-system-protocol.md` rather than replacing them,
4. owns the default next-task boundary analysis/report behavior unless `vida/config/instructions/runtime-instructions.task-approval-loop-protocol.md` inserts an explicit approval gate.

## Core Contract

Autonomous execution mode means:

1. plan/spec scope is already settled enough for continued execution,
2. the orchestrator should keep selecting the next lawful ready task/block without waiting for chat confirmation at every small step,
3. execution continues until one of:
   - pool completion,
   - explicit blocker,
   - material scope/risk/ownership change,
   - user interruption or reprioritization.
4. within an active bounded task, the orchestrator should finish the remaining lawful steps to true task closure rather than stopping at an intermediate summary point.

It does not mean:

1. silent scope expansion,
2. skipping TaskFlow lifecycle,
3. bypassing route/verification gates,
4. inventing missing specs or laws,
5. leaving law-bearing behavior implemented only in code when a nearby canonical spec/protocol should be updated.

Default boundary behavior:

1. After a complex task or material slice closes, the orchestrator must analyze the next lawful task boundary before entering the next task.
2. That boundary analysis must read the nearest governing specs/protocols, inspect the local code/runtime context that controls the next slice, and produce a brief implementation-plan report outside TaskFlow gating.
3. The boundary report is informative by default, not an approval gate.
4. If the boundary analysis discovers stale dependent specs/tasks or missing executable coverage, the orchestrator must update existing artifacts first or create the missing coverage before continuing.
5. Only an explicit approval-loop contract or another lawful stop condition may turn that boundary into a wait state.

Execution-entry validation rule:

1. If overlay enables `spec_ready_auto_development=true`, autonomous execution may enter implementation flow after spec readiness without a new user prompt.
2. If overlay also enables `validation_report_required_before_implementation=true`, implementation entry must pause for a validation report before each implementation-bearing slice or task.
3. That validation report is gating, not merely informational.
4. If overlay also enables `resume_after_validation_gate=true`, accepted validation should resume autonomous execution directly into the implementation step rather than waiting for another redundant confirmation.

User-prompt minimization rule:

1. When AEP is active, do not stop to ask the user for micro-decisions that are already answered by plan/spec/task contracts, TaskFlow state, or canonical priority rules.
2. Ask the user again only when a stop condition is hit, a required approval contract is genuinely missing, or the user interrupts/reprioritizes explicitly.
3. "I have a plausible next step" is not a reason to ask; if the next step is lawful, execute it.

Worker-first continuity rule:

1. AEP does not suspend `vida/config/instructions/instruction-contracts.agent-system-protocol.md`.
2. If the active route/mode requires worker-first analysis, review, coach, or verification lanes, autonomous follow-through must keep using them rather than collapsing into local-only continuation.
3. Local-only continuation during AEP is lawful only when route metadata allows it or when the runtime records explicit worker exhaustion/blocker evidence.

## Activation Gate

Activate AEP only when all are true:

1. the request is already `execution_flow` or tracked `artifact_flow`,
2. a lawful `TaskFlow` task or task pool exists,
3. the relevant plan/spec/acceptance source is already selected,
4. unresolved architecture choice is not blocking the next ready work,
5. the orchestrator can point to the canonical next-task source.

If any item is false:

1. remain in normal tracked execution,
2. stop at task slicing / clarification / blocker capture,
3. do not claim autonomous follow-through mode is enabled.

## Canonical Next-Task Sources

At least one source must define the next lawful work:

1. active `br` ready queue,
2. TaskFlow next block chain (`next_step`),
3. canonical plan wave/task ordering,
4. approved form-task or issue-contract launch output,
5. active pool dependency graph under `vida/config/instructions/command-instructions.implement-execution-protocol.md`.

Precedence:

1. blocker/verification receipts,
2. active TaskFlow block / next block,
3. `br ready` + dependency state,
4. canonical implementation plan ordering,
5. chat-level instruction.

Fallback helper:

1. if `br ready` cannot express lawful ordering because dependency readiness is temporarily unreliable, use `python3 autonomous-next-task.py` with bounded prefix/label scope as the fallback selector,
2. this helper is a bounded runtime workaround and must not silently override higher-precedence receipts or active TaskFlow state.

## Operating Loop

When AEP is enabled, run this loop:

1. hydrate task context and verify current route/gates,
2. apply `vida/config/instructions/runtime-instructions.execution-priority-protocol.md` when choosing between multiple lawful next tasks or when reprioritization pressure exists,
3. select the next lawful ready task/block from canonical sources,
4. if the current task just completed or a complex slice just closed, run next-task boundary analysis before entering the next task:
   - study nearby governing specs/protocols,
   - inspect the controlling code/runtime context for the next slice,
   - prepare a brief implementation-plan report outside TaskFlow gating,
   - refresh dependent spec/task coverage before continuation when the analysis finds drift or missing ownership,
5. pre-register upcoming blocks if the next slice is non-trivial,
6. execute the current block,
7. record evidence/artifacts/risks,
8. run required verify/review gates,
9. if the block is complete, advance automatically to the next lawful block/task,
10. if the task completes, move to the next ready task in the same pool/plan,
11. stop only on explicit blocker, gate failure, pool completion, or user redirect.
12. when implementation entry is validation-gated by overlay, emit the validation report before implementation and resume only after that gate is satisfied.

## Stop Conditions

Autonomous follow-through must stop and return control to routing/slicing when any of these happen:

1. active block enters `failed` or unresolved `partial`,
2. next work would widen scope beyond current plan/spec authority,
3. missing or contradictory task/verification state appears,
4. project/framework ownership boundary changes materially,
5. external reality validation is required but missing,
6. no lawful next task can be selected from canonical sources,
7. reprioritization is implied but cannot be justified by `vida/config/instructions/runtime-instructions.execution-priority-protocol.md`.

## Mandatory Runtime Behaviors

1. keep all work inside TaskFlow block lifecycle,
2. keep one writer owner per writable scope,
3. continue automatically only across lawfully connected tasks/blocks,
4. prefer `beads-workflow.sh block-finish` for done steps so the next block can activate deterministically,
5. run `todo-plan-validate.sh` when extending or reshaping planned blocks,
6. use `task-state-reconcile.py` before closing or skipping drifted tasks,
7. preserve compact-safe state through TaskFlow evidence and context capsules.
8. prefer continuing to the next lawful task/block over pausing for user confirmation when no stop condition is active.
8.1. if spec-ready auto development is enabled, treat ready spec state as sufficient to enter implementation routing without a new user prompt.
8.2. if validation-before-implementation is enabled, implementation entry still pauses for the validation report even under spec-ready auto development.
8.3. if resume-after-validation is enabled, accepted validation returns immediately to autonomous execution for the same lawful implementation path.
9. when behavior changes materially, run a nearby-spec check and update/add the governing spec before closure.
9.1. when the current task closes and a next lawful task exists, run the boundary analysis/report step before starting the next task rather than jumping directly from closure into implementation.
9.2. boundary analysis/report lives outside TaskFlow execution for the next task; it prepares lawful continuation but does not replace the next task's tracked flow.
9.3. when the boundary analysis finds dependent executable scope, update existing dependent specs/tasks or create the missing coverage before claiming lawful continuation.
10. when the same technical error repeats twice or an external API/format is uncertain, escalate via `vida/config/instructions/diagnostic-instructions.debug-escalation-protocol.md` instead of continuing blind local retries.
10.1. under active worker mode, pair that escalation with a bounded external catch/review agent whenever an eligible lane exists.
10.2. if primary-source lookup still leaves ambiguity after one pass, execute Google/web search on the next pass rather than repeating another blind local attempt.
11. if `vida/config/instructions/runtime-instructions.task-approval-loop-protocol.md` is active, stop after the current task completes and present the next task for approval before starting it.
11.1. if the user enables continuous autonomous execution with next-task reporting, do not stop after progress reports inside the current task, but do present the next task briefly at task boundary before starting it.
11.2. under that mode, the report must stay concise and task-scoped; it is a task-boundary planning artifact, not a pause after micro-steps or after ordinary progress updates.
11.3. if the user disables next-task boundary reporting too, the orchestrator must still perform the boundary analysis and dependent-coverage refresh internally, but may skip the user-facing report while continuing directly into the next lawful task.
12. when planning or spec coverage already exists, prefer updating existing tasks/specs rather than creating new ones.
13. run `vida/config/instructions/diagnostic-instructions.protocol-self-diagnosis-protocol.md` checks when behavior suggests reporting barriers, task-coverage drift, verification gaps, or route drift.

Reporting continuity rule:

1. progress reports are informational, not execution barriers,
2. after reporting, continue directly into the next lawful task/block unless a separate stop condition is active,
3. if reporting repeatedly interrupts lawful continuation, treat it as protocol drift and correct the protocol/runtime surface.
4. next-task boundary analysis/report is mandatory by default for complex task transitions even when it is non-gating.
5. exception: when next-task boundary approval is active, present the next task report only once per task boundary and wait there, not after ordinary intra-task reports.
6. if next-task boundary approval is inactive, keep the boundary report non-gating and continue automatically after it.
7. if an overlay disables user-facing boundary reporting, the internal boundary analysis still remains mandatory.
8. do not confuse "report emitted" with "task complete"; if the active bounded task still has lawful remaining steps, continue until those steps are done or a real stop condition appears.

## Relationship To Existing Protocols

1. `vida/config/instructions/runtime-instructions.taskflow-protocol.md` owns task/block execution lifecycle,
2. `vida/config/instructions/runtime-instructions.beads-protocol.md` owns task-state SSOT and workflow commands,
3. `vida/config/instructions/command-instructions.implement-execution-protocol.md` owns queue selection, implement loop, and continue-to-next-task behavior,
4. `vida/config/instructions/instruction-contracts.agent-system-protocol.md` still owns worker routing/fallback law during autonomous continuation,
5. this file adds the trigger and stop doctrine for using those protocols in sustained follow-through mode.

## Canonical Entry Pattern

Use autonomous execution mode like this:

1. select/attach to the active `TaskFlow` task or pool,
2. start tracked execution,
3. declare the next 2-3 planned blocks,
4. mark autonomous follow-through as active in task evidence or reflection,
5. continue through ready work until a stop condition is hit.

## Anti-Patterns

1. claiming autonomy while still asking chat for every micro-step,
2. continuing into later waves because "the direction seems obvious",
3. skipping verification because the plan is already approved,
4. treating a stale task board as an acceptable next-task source,
5. closing tasks by narrative instead of by canonical TaskFlow/verification evidence.

-----
artifact_path: config/instructions/instruction-contracts/autonomous-execution.protocol
artifact_type: instruction_contract
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/instruction-contracts.autonomous-execution-protocol.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:29:19+02:00
changelog_ref: instruction-contracts.autonomous-execution-protocol.changelog.jsonl
