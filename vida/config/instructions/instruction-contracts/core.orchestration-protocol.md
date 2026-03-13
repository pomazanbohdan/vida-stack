# Orchestration Protocol (OP)

Purpose: define how the top-level orchestrator turns a non-trivial request into a delivery-ready result.

## Core Contract

1. Orchestration is protocol-driven and uses the canonical task-state surface plus routed execution packs only when request intent requires task execution or artifact production.
2. The orchestrator owns problem framing, lens selection, workstream decomposition, agent routing, synthesis, and the final quality gate.
3. Packs are the canonical route family; orchestration lenses are the reasoning posture. A lens never replaces a pack.
4. Task state lives only in the canonical task-state surface; execution telemetry lives only in the canonical execution-telemetry surface when tracked execution is engaged.
5. No interactive menu dependencies.

## Hard Runtime Law

1. Mandatory routing and verification requirements must be expressed as executable runtime behavior, not advisory phrasing.
2. If route metadata declares `external_first_required`, `dispatch_required`, `fanout_min_results`, or `independent_verification_required`, the orchestrator must treat violations as invalid execution states.
3. The orchestrator must not silently substitute local/manual analysis for a declared routed dispatch path.
4. When protocol text describes options, it must also define the exact condition that selects each option.

## Continuation Invariant

1. Closure of one bounded leaf does not by itself authorize closure of the parent bounded unit, active task chain, or user-visible orchestration cycle.
2. After synthesis of a closed bounded leaf, the orchestrator must immediately classify the parent chain into exactly one of:
   - `next_leaf_required`
   - `blocked`
   - `fully_closed`
3. `next_leaf_required` is mandatory when the parent bounded unit still has lawful in-scope work and a coherent next bounded leaf can be shaped.
4. `blocked` is mandatory when the parent bounded unit remains open but no coherent next bounded leaf can yet be shaped without new evidence, approval, or escalation.
5. `fully_closed` is lawful only when the parent bounded unit, task chain, and represented request are all actually closed according to the active tracked-flow owners.
6. A closure-style user report is invalid while the correct classification is `next_leaf_required`.
7. When `next_leaf_required` holds, the orchestrator must persist a continuation receipt before yielding control or presenting closure-style language.

Cross-item continuation rule:

1. when the active user request is agentic continuation (`continue development`, `continue agently`, or equivalent autonomous continuation intent), closure of one bounded item does not by itself authorize session closure if another lawful in-scope item is already evidenced,
2. if a continuation receipt, task-state surface, or synthesized verifier/subagent return already identifies the next lawful bounded item, that evidence is a trigger to rebuild routing and continue, not a natural reporting boundary,
3. in that state, the orchestrator must either:
   - bind and shape the next lawful item,
   - dispatch or otherwise continue it,
   - or persist an explicit blocker explaining why continuation stopped being lawful
4. treating "the next item is now known" as a reason to stop after a closure-style report is protocol-invalid.

Local-fix narrowing rule:

1. when the user intent is `continue development` or equivalent tracked execution continuation, the orchestrator must not silently narrow the active context to "fix the first failing test", "fix the first local compiler error", or another ad hoc local symptom unless the canonical task/packet state already proves that symptom is the active bounded leaf,
2. before any local write-producing repair or proof command intended to advance an active development line, the orchestrator must be able to point to an active task/packet receipt naming:
   - the active parent bounded unit,
   - the current bounded leaf or lawful next leaf,
   - and the proof target for that leaf
3. if that receipt is missing, the lawful next action is to recover/shape the active packet or persist an explicit blocker, not to redefine the task as the first locally visible failing test,
4. a green local validation command for one repaired symptom does not by itself classify the parent chain as `fully_closed`; the parent bounded unit must still be rebuilt and classified under the continuation invariant.

Rework-in-flight rule:

1. when a rework packet has already been dispatched and the rework lane remains active or unresolved, the active chain remains `next_leaf_required` rather than report-ready,
2. a progress report during that state must be explicitly non-blocking and must not suspend the execution turn,
3. stopping after such a report is protocol-invalid even if the report is labeled intermediate rather than final.

Post-dispatch in-flight rule:

1. once an implementer, coach, verifier, or rework lane has been lawfully dispatched for the active bounded packet, the execution cycle remains in-flight until at least one of these occurs:
   - a delegated return is observed and synthesized,
   - a real blocker or escalation receipt is established,
   - or the delegated lane is explicitly superseded/redirection is recorded
2. dispatch itself is not a natural pause point and does not authorize yielding control through a progress report,
3. during this in-flight state, any user-facing progress update must remain non-blocking commentary and the orchestrator must continue waiting, polling, inspecting, or taking the next lawful orchestration action in the same execution turn,
4. stopping immediately after dispatch because "agents are now running" is protocol-invalid while `in_work=1`.

## Dispatch-Readiness Invariant

1. Bounded context gathering, discovery synthesis, or seam/context orientation does not by itself authorize route suspension when the next lawful step is already `shape packet -> activate skills -> dispatch`.
2. When the active bounded unit is write-producing, no blocker exists, and the next lawful step is known, commentary/progress visibility is not a valid stopping condition.
3. If the packet is already dispatch-ready, the orchestrator must dispatch it before any progress-only user update that could be mistaken for execution closure.
4. If the packet is not yet dispatch-ready, the orchestrator must reshape it immediately or persist an explicit blocker explaining why dispatch is not lawful.
5. Treating a commentary update as sufficient completion of a pre-dispatch step is an invalid orchestration state.

## Wait-Boundary Invariant

1. A worker wait timeout, poll timeout, or empty wait result is not by itself a lawful execution boundary, blocker, or user-facing pause point.
2. Timeout means only that closure has not yet been observed within the current wait window.
3. When `in_work=1` and no explicit blocker has been established, the orchestrator must treat timeout as an internal control event and continue through one lawful next action:
   - keep waiting for the bounded closure signal,
   - poll or inspect bounded runtime evidence,
   - dispatch the next already-lawful non-conflicting step,
   - or persist an explicit blocker/escalation receipt if continuation is no longer lawful
4. A terminal-looking or pause-like user report after timeout is invalid unless the represented request/task is actually blocked or fully closed.
5. Treating timeout as a generic assistant pause is protocol-invalid.

## Agent-Saturation Recovery Invariant

1. When a new delegated lane cannot be created because of thread, depth, or agent limits, the orchestrator must not treat saturation as final until it has inspected the existing delegated-lane inventory.
2. That inspection must determine whether any delegated lanes are already:
   - completed and unsynthesized,
   - superseded by newer routing,
   - or otherwise reclaimable without violating open handoff or verification law.
3. If reclaimable delegated lanes exist, the orchestrator must synthesize/supersede them, close/reclaim them, and retry lawful reuse or dispatch before considering local exception-path work.
4. A saturation report that skips this inventory-and-reclaim step is invalid.
5. Delegation failure becomes a lawful exception-path reason only after the orchestrator has:
   - inspected delegated-lane state,
   - reconciled open completed returns,
   - attempted lawful reclamation/reuse,
   - and still lacks a lawful delegated lane.

## Post-Worker-Partial Invariant

1. A partial worker return inside the same bounded write scope does not authorize root-session local completion by inertia.
2. When an implementer returns `partial`, unresolved, non-closure-ready, or otherwise open state for the active packet, the orchestrator must treat that result as a reroute boundary.
3. The next lawful actions after such a return are limited to:
   - fresh bounded rework packet to an implementer lane,
   - coach/verifier/escalation routing when their protocol conditions are satisfied,
   - explicit blocker handling,
   - or an explicit recorded exception-path receipt for root-session writing
4. Reusing the same writable scope locally without one of those receipts is protocol-invalid.
5. Partial worker output is internal evidence for rerouting, not implicit permission transfer of writer ownership back to the root session.

Review-repair rule:

1. if coach, review, or verification finds a compile blocker or similar concrete defect inside an already-mutated packet, that finding still reopens reroute law rather than granting silent local repair authority,
2. "the packet is currently broken" or "the shortest safe path is a quick compile fix" does not waive the pre-write exception-path receipt contract,
3. the lawful next actions remain:
   - fresh rework packet,
   - bounded reroute to the appropriate delegated lane,
   - or explicit pre-write exception-path receipt for the narrow local repair.

## Exception-Path Receipt Contract

Local root-session writing is lawful only when an explicit exception-path receipt exists before the first local mutation.

Minimum receipt fields:

1. `reason_class`
   - `agent_saturation`
   - `failed_lawful_reuse`
   - `documented_normal_lane_failure`
   - `higher_precedence_local_law`
2. `active_bounded_unit`
3. `owned_write_scope`
4. `why_delegated_or_rerouted_path_is_not_currently_lawful`
5. `why_local_write_is_the_smallest_safe_bounded_workaround`
6. `return_to_normal_posture_condition`
7. `verification_plan`

Rules:

1. the receipt must be recorded before the first local write, not after,
2. worker failure, partial return, or self-diagnosis pressure does not by itself satisfy this contract,
3. retroactive narration that "this was exception-path work" is invalid,
4. "the delegated packet already mutated the scope", "review found a compile blocker", or "local repair prevents leaving the task broken" are not receipts by themselves,
5. a dirty worktree, same-scope partial diff, timed-out delegated write packet, or partially applied delegated patch are evidence only; they do not grant root-session write authority by themselves,
6. missing exception-path receipt keeps local writing protocol-invalid even if the produced code later looks correct.

Open-delegation gate:

1. a pre-write exception-path receipt is necessary but not sufficient while a delegated lane for the same bounded packet remains active or its handoff remains unresolved,
2. local exception-path writing is unlawful while that delegated cycle remains open unless one of these is recorded first:
   - explicit supersession/redirection of the delegated lane,
   - a hard blocker proving the delegated lane cannot continue lawfully,
   - or higher-precedence route law that explicitly permits takeover
3. "continue development", wait delay, or silent self-diagnosis posture do not satisfy this gate,
4. do not use an exception-path receipt to bypass an otherwise still-lawful delegated cycle,
5. same-scope in-flight diffs, dirty worktree evidence, or partially recovered local patches do not close this gate and must be treated as reroute/supersession evidence only.

Silent-diagnosis precedence rule:

1. when silent framework diagnosis is active and live runtime evidence for the active packet reports `delegated_cycle_open=true` plus `local_exception_takeover_gate=blocked_open_delegated_cycle`, the root session must remain in orchestrator/diagnosis posture,
2. in that state, recovering delivery context, inspecting worker state, or reporting the process conflict are lawful orchestration actions, but local write-producing workaround work is not,
3. implementer delay, hanging subordinate lanes, or "the patch location is already known" do not by themselves convert that state into lawful local exception-path writing,
4. the lawful exits remain:
   - explicit supersession/redirection of the delegated lane,
   - hard blocker evidence proving the delegated lane cannot continue lawfully,
   - higher-precedence route law that explicitly permits takeover,
   - or non-write diagnosis/escalation/reporting behavior.

## Lane-Identity Invariant

1. Orchestrator lane identity survives bootstrap, tracked-execution entry, and generic execution-intent phrasing until a lawful protocol path explicitly changes the active lane for a bounded packet.
2. User intent to continue development authorizes execution routing, not root-session role collapse from orchestrator into implementer.
3. The root orchestrator may shape packets, route lanes, synthesize returns, and run bounded orchestration-only validation, but must not become the default local writer for normal development work.
4. Local root-session implementation is lawful only under an explicit recorded exception path or when higher-precedence protocol law declares the work shaping-only, proof-only, or otherwise local by exception.
5. Treating "execution requested" as implicit permission to bypass orchestration-first delegation is an invalid lane-identity state.
6. After boot, the root session must confirm whether it is acting as `orchestrator`, `worker`, or `exception-path local writer`; if that identity is implicit, execution is not route-ready.
7. If live runtime gates say local takeover is blocked by an open delegated cycle, "continue development" still means continue orchestrator-led control flow, not root-session bounded patching.

## Scope

1. Entry behavior is executed through the canonical tracked-execution owner, route owner, and pack-routing owner selected by the active path.
2. Migration-only wrappers may still exist, but their command-level ownership stays in the runtime-transition and runtime-family maps rather than in this protocol body.
3. This protocol covers request interpretation above command-level execution details.
4. This file must not become an operator command catalog or a runtime-family help surface.

## Activation Surface

Activate this protocol when at least one is true:

1. a non-trivial request must be framed into a delivery-ready result,
2. request intent must be classified before routing or tracked execution begins,
3. worker-first orchestration posture, writer ownership, or execution authorization must be determined,
4. dependency order across analysis, design, implementation, validation, and delivery must be selected,
5. task/pack/route selection is required at the orchestration layer rather than inside a lower owner.

Primary activating companions:

1. `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`
2. `vida/config/instructions/instruction-contracts/core.skill-activation-protocol.md`
3. `vida/config/instructions/instruction-contracts/core.packet-decomposition-protocol.md`
4. `vida/config/instructions/instruction-contracts/core.agent-prompt-stack-protocol.md`
5. `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md`
6. `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`
7. `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md`
8. the canonical tracked-execution owner
9. the canonical execution-telemetry owner
10. `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`

## Canonical State-Surface Note

1. `core.orchestration` does not own one standalone durable ledger,
2. orchestration law depends on the canonical split between task lifecycle truth, execution telemetry, and `run-graph` node-level resumability,
3. this protocol must preserve that split rather than creating a fourth competing task-state surface.

## Required Core Linkages

1. `core.orchestration` is the integration owner for the `core cluster`, not a standalone island.
2. When worker routing is active, orchestration must route through `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`.
3. When delegated candidate lanes are evaluated, admissibility must be proven through `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md` before scoring or cost ranking can authorize a route.
4. When routed execution consumes evidence or delegated context, governed usage must respect `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`.
5. When one routed execution run must be resumed or checkpointed at node level, orchestration must rely on `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md`.
6. Skill discovery and bounded skill activation must rely on `vida/config/instructions/instruction-contracts/core.skill-activation-protocol.md` when a visible skill catalog exists.
7. Bounded packet shaping and just-in-time deeper refinement must rely on `vida/config/instructions/instruction-contracts/core.packet-decomposition-protocol.md`.
8. Prompt-layer precedence between bootstrap, role prompt, packet, skill overlay, and runtime state must rely on `vida/config/instructions/instruction-contracts/core.agent-prompt-stack-protocol.md`.
9. Tooling, command syntax, and migration helpers stay outside this protocol and belong to runtime-family or migration surfaces.
10. Detailed task-state, tracked-execution, and worker-routing rules remain in their canonical protocols.

## Boundary Rule

1. this protocol owns top-level orchestration law, writer ownership, and execution authorization posture only,
2. it does not own generic worker-system mode law, which stays in `core.agent-system`,
3. it does not own typed admissibility, which stays in `core.capability-registry`,
4. it does not own governed evidence classification, which stays in `core.context-governance`,
5. it does not own node-level routed-run resumability, which stays in `core.run-graph`,
6. it does not own pack taxonomy, pack trigger matrix, or command-layer routing details beyond thin orchestration routing references,
7. it must not become a command catalog, migration note, or cluster map.

## Inputs

1. User request text.
2. Optional explicit `task_id`.
3. Existing task, decision, or spec context when present.

## Request Intent Classes

Classify request intent before task resolution:

1. `answer_only`
   - explanation, diagnosis, comparison, review findings, framework discussion, architecture recommendation,
   - no automatic tracked-execution or pack flow.
2. `artifact_flow`
   - research artifact, spec, task-pool, formal report, docs update, decision record,
   - tracked execution + pack required.
3. `execution_flow`
   - implementation, bug fix, refactor, protocol/script/code mutation,
   - tracked execution required.
4. `mixed`
   - starts as `answer_only`,
   - enters tracked execution only after explicit mutation decision, approved task context, or user-confirmed execution scope.

## Orchestration Lenses

1. `discovery`
2. `product_strategy`
3. `business_analysis`
4. `systems_analysis`
5. `architecture`
6. `delivery_planning`
7. `implementation_support`
8. `review_audit`
9. `multi_agent_debate`
10. `recovery_debug`
11. `problem_party`

Selection rule:

1. Pick the smallest sufficient lens set for the request.
2. Mixed requests may activate multiple lenses, but the orchestrator still returns one integrated outcome.

## Problem Framing Contract

Before routing work, normalize the request into:

1. target problem or desired outcome,
2. business/product/system goal,
3. explicit constraints and hidden assumptions,
4. scope boundary (`in`, `out`, dependencies),
5. symptom vs root-cause distinction when relevant,
6. readiness risks or missing evidence.

## Path-Selection Receipt

When one turn mixes reporting, diagnosis, or self-diagnosis intent with continued development intent, orchestration must persist one path-selection receipt before any write-producing action.

Minimum fields:

1. `request_shape`
   - `diagnosis_only`
   - `normal_delivery`
   - `mixed_report_plus_delivery`
   - `tracked_fsap_remediation`
2. `selected_path`
   - `diagnosis_path`
   - `normal_delivery_path`
3. `selection_basis`
4. `why_other_path_is_not_currently_primary`
5. `write_authority_posture`

Rules:

1. mixed `report + continue development` turns must not be resolved by intuition or generic execution defaults,
2. self-diagnosis posture does not by itself authorize local product fixing,
3. until the path-selection receipt exists, write-producing execution is not route-ready,
4. if `diagnosis_path` is selected, any write-producing follow-up must obey the stricter diagnosis-path law before mutation,
5. if `normal_delivery_path` is selected, diagnosis/reporting content remains secondary and must not override worker-first delivery law.

## Active-Unit Binding Rule

Before packet shaping, dispatch, or local proof/work intended to continue development, orchestration must bind the user request to one explicit bounded unit.

Binding inputs may include:

1. explicit user-named task/packet/backlog id,
2. one uniquely active in-progress bounded unit proven by TaskFlow/packet receipts,
3. one uniquely lawful continuation unit explicitly carried by continuation receipts.

Forbidden implicit bindings:

1. treating `ready_head[0]` or the first backlog item in canonical order as "the next task" without explicit bounded-unit confirmation,
2. treating generic wording like `continue the next task` as sufficient authorization to pick one candidate when more than one plausible unit exists,
3. silently rebinding from an open active unit to a merely ready candidate because that candidate looks technically sensible.

Fail-closed rule:

1. if bounded-unit reference remains ambiguous after bootstrap/state snapshot, do not shape or dispatch implementation work yet,
2. the lawful next action is either:
   - explicit binding to the uniquely evidenced active unit with that binding stated,
   - or user clarification,
   - or explicit blocker/report that bounded-unit selection remains ambiguous.

## Runtime-Visible Orchestrator Control Loop

The orchestrator must keep one compact control loop visible to itself during active execution.

Required loop:

1. identify the active bounded unit,
2. identify all currently lawful next slices,
3. remove slices blocked by dependency, scope, route, or verification gates,
4. decide whether one slice may continue sequentially now or whether safe parallel fanout exists,
5. if more than one lawful candidate remains, apply execution-priority law,
6. shape the next bounded packet or packets,
7. dispatch or continue the selected slice,
8. after each discovery wave, worker return, timeout, or bounded closure, run the loop again before any pause-like reporting.

Parallel-safety rule:

1. parallelization is lawful only when candidate slices are output-independent, writable-scope-disjoint, and mutable-contract-disjoint,
2. if any of those fail, route sequentially,
3. when parallel safety is unclear, fail closed to sequential continuation or bounded escalation.

Replanning rule:

1. re-run this loop after each discovery wave, worker return, timeout event, blocker change, bounded closure, or material route/drift signal,
2. do not re-plan merely because a summary is available,
3. do not skip re-planning when new evidence changes lawful-next selection or parallel-safety posture.

Visibility rule:

1. after bootstrap and after each loop re-run, the orchestrator must be able to state explicitly:
   - the active bounded unit,
   - the lawful next candidates,
   - whether execution is sequential or parallel-safe,
   - the selected next slice,
   - the reason that slice won
2. if those answers are not visible, execution is not route-ready.

Explorer-findings rule:

1. bounded gap discovery by an explorer, read-only analysis lane, or other non-writer lane does not authorize root-session patching by default,
2. once a bounded writable gap is identified, the orchestrator must still complete the lawful write-producing cycle:
   - packet shaping
   - implementer or other lawful writer lane
   - coach/verifier when required
   - synthesis
3. explorer findings may narrow scope and improve packet quality, but they do not transfer writer ownership to the root session.

## Request Branching Rule

When a new user turn arrives while one bounded task or route is already active, orchestration must classify the relationship before reusing context.

Allowed classifications:

1. `same_task_continuation`
2. `branch_of_active_task`
3. `separate_task`

Rules:

1. `same_task_continuation` may reuse the active bounded context normally.
2. `branch_of_active_task` must prefer a bounded subagent, packet, or fresh execution slice with separate context and a concise return artifact.
3. `separate_task` must not piggyback on the active task's growing context, taskflow state, or writer packet by inertia.
4. Parallel execution is lawful only for `branch_of_active_task` slices that are file-disjoint, contract-disjoint, or state-disjoint.
5. If two requests compete for the same writable scope or resumable state namespace, keep them serialized or split them into separate tasks/sessions.

## Reporting State Block

User-facing orchestration reports must expose one compact state block before the substantive answer.

Required lines:

1. `Thinking mode: <STC|PR-CoT|MAR|5-SOL|META>.`
2. `Requests: active=<n> | in_work=<n> | blocked=<n>` for user-request conversation mode
3. `Tasks: active=<n> | in_work=<n> | blocked=<n>` for development orchestration mode
4. `Agents: active=<n> | working=<n> | waiting=<n>`

Counter semantics:

1. `Requests.active`
   - number of currently recognized bounded requests in the active session/thread after request-branching classification
2. `Requests.in_work`
   - number of those requests with active execution, analysis, or synthesis still in progress
3. `Requests.blocked`
   - number of those requests currently stopped by an explicit blocker or missing gate
4. `Tasks.active`
   - number of currently active bounded development tasks in the current orchestration view
5. `Tasks.in_work`
   - number of those tasks with active execution, analysis, synthesis, or delegated work still in progress
6. `Tasks.blocked`
   - number of those tasks currently stopped by an explicit blocker or missing gate
7. `Agents.active`
   - number of currently allocated agents/subagents participating across the active requests or active tasks for the current mode
8. `Agents.working`
   - number of those agents currently executing a bounded packet or task
9. `Agents.waiting`
   - number of those agents currently idle, queued, or waiting for merge/input

Rules:

1. report the bounded current state, not lifetime totals,
2. refresh counters before progress reports, replans, blocker reports, and closure reports,
3. `in_work` means the agent still owes an active next step after the current report without waiting for a new user request,
4. `blocked` means the represented request/task remains open but cannot proceed until a blocker or gate is resolved,
5. if all represented requests/tasks are closed, report `active=0 | in_work=0 | blocked=0` for the relevant mode,
6. a closure-ready final report for the represented mode must not present `in_work>0`,
7. if `in_work>0`, the report is a progress/intermediate report and continued agent action is still expected after the report,
8. do not omit the state block during development orchestration merely because the answer is short,
9. if no delegated agents are active, still report `Agents: active=0 | working=0 | waiting=0`.
10. if a bounded leaf just closed but the parent chain still requires `next_leaf_required`, the report must remain intermediate and must not be phrased as terminal closure.
11. if the next lawful step is known pre-dispatch work (`shape packet`, `activate skills`, `dispatch`), a progress report must not suspend the route before that next step is executed or blocked explicitly.
12. if a wait call times out while `in_work=1`, the report must remain explicitly intermediate and must not suspend execution unless a blocker was newly established.
13. if a user-facing summary is emitted while `in_work=1`, that summary must not be the last action of the current execution turn; the orchestrator must immediately continue with the already-lawful next step or explicit blocker handling.
14. a `final` user-facing report is invalid while any delegated agent remains active, any bounded handoff remains unresolved, or `in_work=1` for the represented request/task.
15. if a delegated lane was just dispatched and no delegated return or blocker has yet been synthesized, a progress report must not become the boundary between dispatch and the next control action.

## Final-Report Gate

Before a `final` user-facing report, the orchestrator must confirm all are true:

1. no delegated agents remain active for the represented request/task,
2. no bounded handoff or reroute receipt remains unresolved,
3. no delegated closure proof, coach return, verifier return, or escalation return is still outstanding,
4. `in_work=0`,
5. the represented request/task is actually closure-ready under the active route law.

If any item is false:

1. emit at most an intermediate/progress report,
2. continue execution or wait lawfully,
3. do not transfer control to `final`.

Terminal-phrasing rule:

1. user-facing wording that sounds closure-ready, terminal, or “done for now” is forbidden until the final-report gate passes,
2. this applies even when local bounded workaround work has completed technically,
3. closure-ready phrasing belongs only after delegated state is synthesized and the represented request/task is actually closure-ready.

## Continuation Receipt Contract

When a bounded leaf closes and the represented request/task is not fully closed, orchestration must persist one continuation receipt containing:

1. `parent_unit_id`
2. `closed_leaf_id`
3. classification: `next_leaf_required` or `blocked`
4. `next_leaf_id` when known
5. `blocking_reason` when `blocked`
6. `selection_basis`
7. `proof_target_for_next_leaf` when `next_leaf_required`

Rules:

1. the continuation receipt is required evidence for lawful post-leaf continuation,
2. the receipt must be written before terminal-looking reporting or route suspension,
3. missing continuation receipt while the parent chain remains open is an invalid orchestration state, not a stylistic issue.

## Progressive Decomposition Rule

For non-trivial work, orchestration must decompose before it expands context.

1. Convert the request into bounded blocking questions or workstreams before deep repository reading.
2. Start with the smallest lawful discovery wave:
   - locate likely owner files or artifacts,
   - skim structure and ownership,
   - deep-read only the minimum subset needed for the current question.
3. Broad "read everything about X" behavior is forbidden unless the request is explicitly repository-wide and no narrower route exists.
4. Each delegated lane should receive one blocking question, minimum required context, explicit stop condition, and explicit verification boundary.
5. Prefer fresh bounded worker packets over long inherited context when sequential discovery or execution is sufficient.
6. Re-plan after each discovery or execution wave only when new evidence changes scope, dependency order, or risk posture.
7. Keep decomposition state in canonical task/evidence surfaces or bounded artifacts instead of relying on growing chat context alone.
8. Use the shallowest lawful bounded packet leaf first and refine to deeper packet leaves only just in time rather than pre-splitting future work.

## Delegation And Scope-Isolation Rule

When worker routing is active, decomposition must preserve isolation:

1. delegate read-only discovery first when the next blocking question does not require shared writable scope,
2. parallelize only across file-disjoint or contract-disjoint slices,
3. if two candidate slices still touch the same writable scope or mutable contract, keep them sequential under one writer,
4. do not dispatch a worker packet that is still shaped like "implement the whole feature",
5. if no safe decomposition exists, stop and narrow the task or escalate instead of widening context.

## Algorithm

1. Frame the problem.
2. Determine request intent class and active orchestration lens.
3. Convert the request into bounded blocking questions and select the smallest lawful discovery wave.
4. Capture only the minimum lawful boot and task-state context needed to route the work.
5. Apply the tracked-execution engagement gate:
   - `answer_only` stays outside tracked execution,
   - `artifact_flow` and `execution_flow` require tracked execution,
   - `mixed` enters tracked execution only when execution or artifact production becomes required.
6. When tracked execution is required, resolve the lawful task, route family, and execution posture through the canonical runtime owners named in `Related`.
7. When worker routing is active, preserve orchestration-first coordination:
   - route through `core.agent-system`,
   - require admissibility through `core.capability-registry`,
   - require governed context through `core.context-governance`,
   - require node-level resumability through `core.run-graph` when continuity matters.
8. Dispatch workers only with bounded blocking questions, minimal context, and explicit scope isolation.
9. After each discovery or execution wave, reconcile evidence and revise the next slice only when the route or risk posture materially changed.
10. For code-shaped implementation or patch work, run the lawful coach lane before independent verification when an eligible coach lane exists; if coach is unavailable, record an explicit blocker or override receipt before verifier routing.
11. Keep writer ownership singular and fail closed when route requirements, coach/verifier posture, or lawful escalation evidence are missing.
12. After each bounded leaf closure, rebuild the parent bounded unit before deciding whether the represented request/task is blocked, requires the next lawful leaf, or is fully closed.
13. After bounded discovery/context gathering inside an active write-producing unit, immediately decide whether packet shaping is complete, reshaping is required, or dispatch is blocked; do not stop at commentary-only progress visibility.
14. Synthesize the resulting analysis or execution outputs into one orchestration-owned result and close only through the canonical tracked-flow owners when tracked execution was engaged.

## Evaluation And Stop Criteria

Orchestration quality must be evaluated continuously on:

1. scope discipline,
2. evidence quality,
3. dependency clarity,
4. writer-scope isolation,
5. verification posture,
6. progress-per-context efficiency.

Stop and revise the route when any are true:

1. the same blocking question is being restated without new evidence,
2. the same broad file set is reread without a narrower hypothesis,
3. two consecutive replans fail to produce a smaller executable slice,
4. the current slice has no explicit closure proof or verification path,
5. safe file-disjoint or contract-disjoint decomposition cannot be established.

Escalate when any are true:

1. required coach or verifier posture cannot be satisfied,
2. decomposition keeps collapsing back into shared-writer ambiguity,
3. route law and available evidence conflict materially,
4. the orchestrator would otherwise continue by context expansion alone rather than by a narrower lawful slice.

## Pack Routing Note

1. pack taxonomy and trigger matrix are owned by the canonical pack-routing owner,
2. this protocol owns the orchestration decision to route into the lawful pack family, not the pack catalog itself,
3. explicit tracked framework self-analysis remains owned by the canonical framework self-analysis owner,
4. generic change-impact reconciliation law remains owned by the canonical change-impact reconciliation owner.

## Operational Proof And Closure

1. orchestration is closure-ready only when request intent, writer ownership, route posture, and dependency order are explicit enough to justify the selected path,
2. when worker-first routing is required, closure depends on route-valid execution rather than undocumented local substitution,
3. when tracked execution is required, orchestration closure depends on the canonical tracked-flow owners rather than chat-only completion claims,
4. unresolved route requirements, missing coach/verifier posture, or missing lawful escalation receipts must block closure,
5. final orchestration proof must remain bounded to explicit request intent, writer ownership, route posture, dependency order, and closure dependencies rather than ad hoc completion claims.

## Constraints

1. Do not mutate task state outside the canonical task-state surface.
2. Do not execute tracked execution work outside the active tracked-execution block lifecycle.
3. Do not engage tracked-execution/pack flow for `answer_only` requests by default.
4. Do not route through non-canonical command paths.
5. Do not use multiple writer lanes without explicit scope isolation.
6. Do not replace synthesis with unintegrated agent fragments.
7. Do not expose raw worker reports as the default user-facing deliverable.
8. Do not route explicit VIDA/framework self-analysis through tracked-execution/pack flow unless the user explicitly asks for tracked execution.
9. Do not use the self-diagnosis exception to justify local-only closure of tracked FSAP/remediation work; tracked closure-ready state requires delegated verification/proving or a structured override receipt.
10. Do not start dev-related boot with broad repository or task-state sweeps when the compact boot snapshot is sufficient.
11. Do not bypass a route-marked hard requirement with local/manual fallback unless the runtime also records a blocker or lawful escalation receipt.
12. Do not front-load large repository reads before bounded decomposition exists.
13. Do not keep rereading context when no new evidence or narrower slice was produced.

## Runtime Surface Note

1. concrete command syntax for pack routing, tracked-execution, migration helpers, or runtime commands belongs to command-instruction, runtime-family, and migration owners rather than this protocol body,
2. this protocol may name those owners as routing dependencies, but it must not restate their command catalogs,
3. migration-only wrappers remain non-canonical references even when orchestration still depends on their existence during transition.

## Related

1. `vida/config/instructions/command-instructions/routing.use-case-packs-protocol.md`
2. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
3. `vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md`
4. migration-only wrapper references remain non-canonical and must not become orchestration law
5. `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`
6. `vida/config/instructions/instruction-contracts/lane.worker-dispatch-protocol.md`
7. `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md`
8. `vida/config/instructions/runtime-instructions/lane.agent-handoff-context-protocol.md`
9. `vida/config/instructions/runtime-instructions/work.problem-party-protocol.md`
10. `vida/config/instructions/instruction-contracts/core.skill-activation-protocol.md`
11. `vida/config/instructions/instruction-contracts/core.packet-decomposition-protocol.md`
12. `vida/config/instructions/instruction-contracts/core.agent-prompt-stack-protocol.md`

-----
artifact_path: config/instructions/instruction-contracts/core.orchestration.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/core.orchestration-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-13T07:44:24+02:00'
changelog_ref: core.orchestration-protocol.changelog.jsonl
