# Autonomous Execution Protocol (AEP)

Purpose: define the canonical follow-through mode for executing a settled VIDA plan/spec/task pool to completion with minimal re-planning overhead while preserving TaskFlow, routing, verification, and fail-closed behavior.

Scope: activates only after tracked execution exists; applies to "continue until done", "follow the plan", "implement all remaining work", or equivalent; reuses `runtime-instructions/work.taskflow-protocol`, `runtime-instructions/runtime.task-state-telemetry-protocol`, `command-instructions/execution.implement-execution-protocol`, and `instruction-contracts/core.agent-system-protocol`; owns default next-task boundary analysis/report unless `runtime-instructions/bridge.task-approval-loop-protocol` inserts an approval gate.

## Core Contract

Autonomous execution mode means settled plan/spec scope, continuous selection of the next lawful ready task/block without chat confirmation at every small step, and completion of remaining lawful steps inside the active bounded task before any intermediate summary pause.

Stop horizon: pool completion, explicit blocker, material scope/risk/ownership change, user interruption, or user reprioritization.

It does not mean silent scope expansion, skipping TaskFlow lifecycle, bypassing route/verification gates, inventing missing specs/laws, or leaving law-bearing behavior only in code when a nearby canonical spec/protocol should be updated.

Default boundary behavior: after a complex task or material slice closes, analyze the next lawful task boundary before entering the next task; read nearest governing specs/protocols, inspect controlling code/runtime context, and produce a brief implementation-plan report outside TaskFlow gating. The report is informational unless an explicit approval-loop contract or lawful stop condition makes it a wait state. If analysis finds stale dependent specs/tasks or missing executable coverage, update existing artifacts or create missing coverage before continuing.

Execution-entry validation rule:

1. `validation_report_required_before_implementation=true` makes implementation entry pause for a validation report before each implementation-bearing slice or task.
2. That validation report is gating, not merely informational.
3. Spec-ready transition into downstream implementation flow and post-validation continuation are runtime-defined execution-entry behaviors, not live project overlay toggles.
4. Project overlay must not advertise additional execution-entry toggles unless runtime compilation and continuation surfaces consume them directly.

User-prompt and non-pause rule:

1. When AEP is active, do not ask for micro-decisions already answered by plan/spec/task contracts, TaskFlow state, or canonical priority rules.
2. Ask again only on stop condition, genuinely missing approval contract, explicit interruption, or reprioritization.
3. A successful bounded step, green validation result, or progress report is not a natural pause point.
4. Reports are progress markers only; they transfer control back to the user only when a separate stop condition is active.
5. "This slice is done" means select the next lawful ready slice now.

Worker-first continuity rule:

1. AEP does not suspend `instruction-contracts/core.agent-system-protocol`.
2. If the active route/mode requires worker-first analysis, review, coach, or verification lanes, autonomous follow-through must keep using them rather than collapsing into local-only continuation.
3. Local-only continuation during AEP is lawful only when route metadata allows it or when the runtime records explicit worker exhaustion/blocker evidence.
4. An explicit exception-path receipt remains necessary for local write-producing continuation during AEP.
5. That receipt is not sufficient while a lawful delegated cycle for the same bounded packet remains open; autonomous continuation must first synthesize, supersede, or hard-block the delegated path.

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
4. on a clean session, do not interpret execution intent as permission for root-session implementation before orchestrator-first route visibility is explicit.

## Canonical Next-Task Sources

At least one next-work source must exist: active DB-backed ready queue, TaskFlow next block chain (`next_step`), canonical plan wave/task ordering, approved form-task or issue-contract launch output, or active pool dependency graph under `command-instructions/execution.implement-execution-protocol`.

Precedence: blocker/verification receipts -> active TaskFlow block / next block -> `vida taskflow task ready` + dependency state -> canonical implementation plan ordering -> chat-level instruction.

Fallback helper:

1. if `vida taskflow task ready` cannot express lawful ordering because dependency readiness is temporarily unreliable, use `python3 autonomous-next-task.py` with bounded prefix/label scope as the fallback selector,
2. this helper is a bounded runtime workaround and must not silently override higher-precedence receipts or active TaskFlow state.

Clean-session route visibility rule:

1. before AEP continues implementation on a clean session, the orchestrator must already have an explicit route receipt showing orchestrator-first control and the next lawful delegated or otherwise lawful lane,
2. absent that receipt, autonomous execution may prepare routing state but must not begin local implementation.

## Operating Loop

When AEP is enabled: hydrate task context and route/gates; apply `runtime-instructions/work.execution-priority-protocol` for multiple lawful tasks or reprioritization pressure; select next lawful ready task/block from canonical sources; run task-boundary analysis after task completion or complex-slice closure; pre-register non-trivial upcoming blocks; execute current block; record evidence/artifacts/risks; run verify/review gates; advance automatically after block completion; move to next ready same-pool/plan task after task completion; stop only on explicit blocker, gate failure, pool completion, or user redirect. When implementation entry is validation-gated by overlay, emit validation report before implementation and resume only after that gate is satisfied.

Execution-block boundary rule:

1. closure of one `execution_block` is not by itself a task boundary, pool boundary, or session boundary,
2. after an `execution_block` closes, first reconcile against the parent `delivery_task` card:
   - if the parent `delivery_task` still has unmet `definition_of_done`, rebuild the active bounded unit and select the next lawful `execution_block` or proof step inside the same task,
   - only if the parent `delivery_task` is actually closed may the orchestrator enter task-boundary analysis for the next task,
3. do not reinterpret "one execution_block closed" as "the current task is done" merely because a local report is available,
4. if no clean next block can be shaped inside the still-open parent task, fail closed with an explicit blocker or escalation receipt rather than yielding a closure-style answer.

Normal success path: pick lawful ready slice from canonical sources; execute to a real technical result, not analysis-only; update canonical project development-conditions artifact when proven conditions changed; update relevant framework-owned protocol/map in the same bounded cycle when framework law/routing/triggers/canonical behavior changed; run bounded validation; on failure fix and rerun validation; on success emit only a concise progress marker when reporting is active; immediately select the next lawful ready slice unless a separate stop condition applies.

## Stop Conditions

Autonomous follow-through must stop and return control to routing/slicing when any of these happen:

1. active block enters `failed` or unresolved `partial`,
2. next work would widen scope beyond current plan/spec authority,
3. missing or contradictory task/verification state appears,
4. project/framework ownership boundary changes materially,
5. external reality validation is required but missing,
6. no lawful next task can be selected from canonical sources,
7. reprioritization is implied but cannot be justified by `runtime-instructions/work.execution-priority-protocol`.
8. the next required move depends on a product, architectural, or ownership decision that is not already resolved by canonical sources,
9. a framework/project protocol conflict appears and cannot be resolved by precedence/routing law alone,
10. the user explicitly says `stop` or otherwise interrupts autonomous continuation.

## Mandatory Runtime Behaviors

1. keep all work inside TaskFlow block lifecycle,
2. keep one writer owner per writable scope,
3. continue automatically only across lawfully connected tasks/blocks,
4. use the canonical tracked-execution block-finish surface so the next block can activate deterministically,
5. use the canonical planning-validation surface when extending or reshaping planned blocks,
6. use the canonical task-state reconciliation surface before closing or skipping drifted tasks,
7. preserve compact-safe state through TaskFlow evidence and context capsules.
8. prefer continuing to the next lawful task/block over pausing for user confirmation when no stop condition is active.
8.1. if spec-ready auto development is enabled, treat ready spec state as sufficient to enter implementation routing without a new user prompt.
8.2. if validation-before-implementation is enabled, implementation entry still pauses for the validation report even under spec-ready auto development.
8.3. if resume-after-validation is enabled, accepted validation returns immediately to autonomous execution for the same lawful implementation path.
9. when behavior changes materially, run a nearby-spec check and update/add the governing spec before closure.
9.1. when the current task closes and a next lawful task exists, run the boundary analysis/report step before starting the next task rather than jumping directly from closure into implementation.
9.2. boundary analysis/report lives outside TaskFlow execution for the next task; it prepares lawful continuation but does not replace the next task's tracked flow.
9.3. when the boundary analysis finds dependent executable scope, update existing dependent specs/tasks or create the missing coverage before claiming lawful continuation.
9.4. do not run next-task boundary analysis merely because one `execution_block` closed; that analysis belongs only after the parent `delivery_task` or equivalent bounded task actually closes.
10. when the same technical error repeats twice or an external API/format is uncertain, escalate via `diagnostic-instructions/escalation.debug-escalation-protocol` instead of continuing blind local retries.
10.1. under active worker mode, pair that escalation with a bounded external catch/review agent whenever an eligible lane exists.
10.2. if primary-source lookup still leaves ambiguity after one pass, execute Google/web search on the next pass rather than repeating another blind local attempt.
11. if `runtime-instructions/bridge.task-approval-loop-protocol` is active, stop after the current task completes and present the next task for approval before starting it.
11.1. if the user enables continuous autonomous execution with next-task reporting, do not stop after progress reports inside the current task, but do present the next task briefly at task boundary before starting it.
11.2. under that mode, the report must stay concise and task-scoped; it is a task-boundary planning artifact, not a pause after micro-steps or after ordinary progress updates.
11.3. if the user disables next-task boundary reporting too, the orchestrator must still perform the boundary analysis and dependent-coverage refresh internally, but may skip the user-facing report while continuing directly into the next lawful task.
12. when planning or spec coverage already exists, prefer updating existing tasks/specs rather than creating new ones.
13. run `diagnostic-instructions/analysis.protocol-self-diagnosis-protocol` checks when behavior suggests reporting barriers, task-coverage drift, verification gaps, or route drift.
14. treat `green -> sync docs/protocols -> validate -> next slice` as the default autonomous success path.
15. do not reinterpret "task-local closure" as "execution finished" while the epic/program still has lawful ready work.
16. when a behavior is now proven runnable/buildable/installable, update the canonical development-conditions artifact in the same cycle rather than batching that evidence for later.

Reporting continuity rule:

1. progress reports are informational, not execution barriers.
2. After reporting, continue into the next lawful task/block unless a separate stop condition is active.
3. If reporting repeatedly interrupts lawful continuation, treat it as protocol drift and correct the protocol/runtime surface.
4. Next-task boundary analysis/report is mandatory by default for complex task transitions even when non-gating.
5. If next-task boundary approval is active, present the next-task report once per task boundary and wait there, not after ordinary intra-task reports.
6. If next-task boundary approval is inactive, keep the boundary report non-gating and continue automatically.
7. If overlay disables user-facing boundary reporting, internal boundary analysis remains mandatory.
8. Do not confuse "report emitted" with "task complete", "task complete" with "epic complete", or "execution_block complete" with "delivery_task complete"; block closure must first return through parent `definition_of_done`.
9. Do not treat "rework packet already dispatched" as a safe pause boundary; if the rework lane is in flight, reporting remains non-blocking only.

## Relationship To Existing Protocols

1. `runtime-instructions/work.taskflow-protocol` owns task/block execution lifecycle,
2. `runtime-instructions/runtime.task-state-telemetry-protocol` owns task-state SSOT and workflow commands,
3. `command-instructions/execution.implement-execution-protocol` owns queue selection, implement loop, and continue-to-next-task behavior,
4. `instruction-contracts/core.agent-system-protocol` still owns worker routing/fallback law during autonomous continuation,
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
artifact_path: config/instructions/instruction-contracts/overlay.autonomous-execution.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/overlay.autonomous-execution-protocol.md
created_at: '2026-03-09T12:00:46+02:00'
updated_at: 2026-07-03T12:20:00+03:00
changelog_ref: overlay.autonomous-execution-protocol.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: semantic-atom-coverage+rfc2119-rewrite+duplicate-rule-merge
protocol_compression_baseline_ref: 062a45c3d:vida/config/instructions/instruction-contracts/overlay.autonomous-execution-protocol.md
protocol_compression_audit_at: 2026-07-03T12:20:00+03:00
protocol_compression_before_tokens: 3583
protocol_compression_after_tokens: 3265
protocol_compression_content_sha256: 37a93c0cdf2ad1ef5797fc9adb8a6a147c71a85662f3b9c51309980581031f8b
