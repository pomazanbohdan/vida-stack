# Orchestration Protocol (OP)

Purpose: define how the root orchestrator turns a non-trivial request into a delivery-ready result without losing active-unit truth, writer authority, or closure discipline.

Compression note: this file is the compact owner kernel. It keeps binding law, continuation law, writer-takeover law, reporting law, and routing dependencies loaded; detailed command catalogs and lower-level execution rules stay in the related owner protocols.

## Core Contract

1. Orchestration is protocol-driven: classify intent, bind one bounded unit, choose the lawful route, dispatch or continue, synthesize evidence, then close only through the active owner gates.
2. The orchestrator owns problem framing, lens selection, decomposition, route selection, agent routing, synthesis, and final quality gate.
3. Packs and tracked execution are route families; orchestration lenses are reasoning posture. A lens never replaces a pack.
4. Task state lives in the canonical task-state surface; execution telemetry lives in execution-telemetry/run-graph surfaces when tracked execution is engaged.
5. No interactive menu dependency may be required for normal orchestration.
6. Carrier identity is not runtime role identity: host `agent` means carrier/model/tier/cost/effectiveness, while runtime role remains a separate activation state.
7. Carrier selection order is mandatory: capability/admissibility, then local score/telemetry guard, then cheapest eligible carrier.

## Hard Runtime Law

1. Mandatory routing and verification requirements MUST be executable runtime behavior, not advisory prose.
2. Route metadata such as `external_first_required`, `dispatch_required`, `fanout_min_results`, and `independent_verification_required` is binding.
3. The orchestrator MUST NOT silently substitute local/manual analysis for a declared routed dispatch path.
4. Every option in protocol text MUST name the condition that selects it.
5. If `vida orchestrator-init --json`, `vida agent-init`, or `vida project-activator` reports `pending_activation`, normal execution is blocked until activation is handled or a runtime-defect bypass is explicitly in force.

## Scope

1. This protocol owns top-level orchestration law, active-unit binding, writer ownership, route posture, continuation law, and closure/reporting legality.
2. This protocol does not own command syntax, pack catalogs, task-state schema, worker-system mode law, capability admissibility, context governance, or node-level run resumability.
3. Tooling, command syntax, and migration helpers belong to command-instruction, runtime-family, and migration owners.
4. This protocol must not become an operator command catalog.

## Compressed Legacy Section Crosswalk

The compact kernel preserves these former owner sections as merged rules:

| Legacy section | Compact owner section |
| --- | --- |
| Continuation Invariant | Continuation Law |
| Dispatch-Readiness Invariant | Dispatch, Wait, And Partial-Return Law |
| Wait-Boundary Invariant | Dispatch, Wait, And Partial-Return Law |
| Agent-Saturation Recovery Invariant | Dispatch, Wait, And Partial-Return Law |
| Post-Worker-Partial Invariant | Dispatch, Wait, And Partial-Return Law |
| Exception-Path Receipt Contract | Writer Ownership And Exception Path |
| Lane-Identity Invariant | Writer Ownership And Exception Path |
| Runtime-Visible Orchestrator Control Loop | Control Loop |
| Request Branching Rule | Request Branching Rule |
| Reporting State Block | Reporting And Final-Report Gate |
| Progressive Decomposition Rule | Decomposition And Delegation |
| Delegation And Scope-Isolation Rule | Decomposition And Delegation |

## Activation Surface

Activate this protocol when:

1. a non-trivial request must become a delivery-ready result,
2. request intent must be classified before routing or tracked execution,
3. worker-first posture, writer ownership, or execution authorization is unclear,
4. dependency order across analysis, design, implementation, validation, and delivery must be selected,
5. task, pack, route, or active bounded-unit selection is required above a lower owner.

Primary companions:

1. `instruction-contracts/core.agent-system-protocol`
2. `instruction-contracts/core.skill-activation-protocol`
3. `instruction-contracts/core.packet-decomposition-protocol`
4. `instruction-contracts/core.agent-prompt-stack-protocol`
5. `runtime-instructions/core.capability-registry-protocol`
6. `runtime-instructions/core.context-governance-protocol`
7. `runtime-instructions/core.run-graph-protocol`
8. `instruction-contracts/bridge.instruction-activation-protocol`
9. canonical tracked-execution and execution-telemetry owners.

## Inputs

1. User request text.
2. Optional explicit `task_id`.
3. Current bootstrap/init state.
4. Existing task, packet, decision, spec, or runtime evidence when present.

## Intent And Source Classification

Classify request intent before task resolution:

1. `answer_only`: explanation, diagnosis, comparison, review, architecture recommendation; no automatic tracked execution.
2. `artifact_flow`: research artifact, spec, formal report, docs update, decision record; tracked execution plus pack is required when mutation is needed.
3. `execution_flow`: implementation, bug fix, refactor, protocol/script/code mutation; tracked execution is required unless an explicit runtime-defect bypass is in force.
4. `mixed`: answer first, enter tracked execution only after explicit mutation decision, approved task context, or user-confirmed execution scope.

Classify work-signal source without hardcoding source-specific workflow law:

1. `pull_request`
2. `external_downstream_report`
3. `runtime_defect`
4. `ci_failure`
5. `release_task`
6. `optimization`
7. `documentation_process`
8. `operator_surface_gap`
9. `feature_or_capability_request`

Source-specific protocols may add evidence requirements but do not replace active-unit binding, task truth, route admissibility, writer ownership, or proof law.

## Problem Framing

Before routing, normalize:

1. target problem or desired outcome,
2. goal,
3. constraints and hidden assumptions,
4. in/out scope and dependencies,
5. symptom versus root cause,
6. readiness risks or missing evidence.

Use the smallest sufficient lens set: `discovery`, `product_strategy`, `business_analysis`, `systems_analysis`, `architecture`, `delivery_planning`, `implementation_support`, `review_audit`, `multi_agent_debate`, `recovery_debug`, or `problem_party`.

## Active-Unit Binding Rule

Before packet shaping, dispatch, local proof, or write-producing continuation, bind the request to exactly one bounded unit.

Allowed binding inputs:

1. explicit user-named task, packet, backlog id, or artifact,
2. one uniquely active in-progress bounded unit proven by live TaskFlow/packet receipts,
3. one uniquely lawful continuation unit carried by continuation receipts.

Forbidden implicit bindings:

1. `ready_head[0]` as the next task,
2. generic `continue the next task` when multiple candidates exist,
3. silent rebinding from an open active unit to a merely ready candidate.

Before write-producing work, orchestration must be able to state:

1. `active_bounded_unit`
2. `why_this_unit`
3. `sequential_vs_parallel_posture`
4. selected next slice,
5. proof target.

If those fields are missing or stale, fail closed to diagnosis, binding recovery, clarification, or explicit runtime-defect bypass.

## Request Branching Rule

When a new user turn arrives while a bounded request or task is active, classify it before reusing context:

1. `same_task_continuation`: reuse active bounded context.
2. `branch_of_active_task`: split into a bounded subagent, packet, or fresh slice with separate context and return artifact.
3. `separate_task`: do not piggyback on the active task, taskflow state, writer packet, or growing context.

Parallel execution is lawful only for `branch_of_active_task` slices that are file-disjoint, contract-disjoint, or state-disjoint. Competing writable scope or resumable state remains serialized.

## Control Loop

During active execution, keep this loop visible:

1. identify the active bounded unit,
2. identify lawful next slices,
3. remove slices blocked by dependency, scope, route, or verification gates,
4. decide sequential versus parallel-safe posture,
5. apply priority law if more than one candidate remains,
6. shape the next bounded packet or bounded local slice,
7. dispatch, continue, or prove,
8. after discovery, worker return, timeout, blocker change, route drift, or bounded closure, rerun the loop before pause-like reporting.

Parallelization is lawful only when candidate slices are output-independent, writable-scope-disjoint, and mutable-contract-disjoint. If unclear, route sequentially or escalate.

## Continuation Law

Closure of one bounded leaf never by itself closes the parent bounded unit, task chain, or user-visible orchestration cycle.

After every leaf closure classify parent state as exactly one:

1. `next_leaf_required`: lawful in-scope work remains and a coherent next leaf can be shaped.
2. `blocked`: parent remains open but no coherent next leaf can be shaped without new evidence, approval, or escalation.
3. `fully_closed`: parent, task chain, and represented request are actually closed under active owners.

Rules:

1. closure-style reports are invalid while classification is `next_leaf_required`,
2. if the next lawful item is already evidenced, bind and continue rather than stop,
3. when parent is not fully closed, persist or cite a continuation receipt before terminal-looking reporting,
4. after any bounded success, green test, worker return, timeout, or intermediate report, immediately run the control loop again.

Continuation Receipt Contract minimum fields:

1. `parent_unit_id`
2. `closed_leaf_id`
3. classification: `next_leaf_required` or `blocked`
4. `next_leaf_id` when known
5. `blocking_reason` when blocked
6. `selection_basis`
7. `proof_target_for_next_leaf` when applicable.

## Dispatch, Wait, And Partial-Return Law

Dispatch readiness:

1. bounded context gathering does not authorize route suspension when the next lawful step is `shape packet -> activate skills -> dispatch`,
2. commentary/progress visibility is not a stop condition when write-producing work is unblocked,
3. if a packet is dispatch-ready, dispatch before progress-only reporting; if not, reshape or record a blocker.

Wait boundary:

1. worker timeout, poll timeout, or empty wait result is an internal event, not a blocker by itself,
2. while `in_work=1` and no blocker exists, continue by waiting, polling, inspecting bounded evidence, dispatching a lawful non-conflicting step, or recording a blocker/escalation receipt,
3. terminal-looking reporting after timeout is invalid unless the represented task is blocked or fully closed.

Delegation saturation:

1. before treating agent/thread/depth saturation as final, inspect delegated-lane inventory,
2. synthesize completed returns, supersede stale lanes, close/reclaim eligible lanes, and retry lawful dispatch/reuse,
3. saturation is exception-path evidence only after inventory, reconciliation, reclamation, and retry fail.

Partial return:

1. `partial`, unresolved, non-closure-ready, or review-repair findings reopen reroute law,
2. lawful exits are fresh rework packet, coach/verifier/escalation, explicit blocker, or pre-write exception-path receipt,
3. a broken delegated patch or first failing test does not transfer writer ownership to root by inertia.

## Writer Ownership And Exception Path

Root orchestrator may shape, route, synthesize, and run bounded orchestration-only validation. It must not become default local writer for normal development.

Local root-session writing is lawful only when one of these is true:

1. a receipt-backed delegated execution path owns the mutation,
2. higher-precedence local law declares the work local/proof-only/shaping-only,
3. an explicit runtime-defect bypass is in force for the current block,
4. runtime confirms active exception takeover for the same bounded packet with `local_exception_takeover_state=active` and `root_local_write_allowed=true`.

Insufficient states:

1. `receipt_recorded`
2. `admissible_not_active`
3. `activation_view_only`
4. `internal_activation_view_only`
5. packet location discovery,
6. dirty worktree evidence,
7. known patch location,
8. first local compiler/test failure.

Exception-path receipt minimum fields:

1. `reason_class`: `agent_saturation`, `failed_lawful_reuse`, `documented_normal_lane_failure`, or `higher_precedence_local_law`
2. `active_bounded_unit`
3. `owned_write_scope`
4. `why_delegated_or_rerouted_path_is_not_currently_lawful`
5. `why_local_write_is_the_smallest_safe_bounded_workaround`
6. `return_to_normal_posture_condition`
7. `verification_plan`.

Open delegated cycle gate: exception receipt is necessary but not sufficient while a delegated lane for the same packet remains active or unresolved. Record supersession, hard blocker, or higher-precedence takeover before local writing.

When the user explicitly declares the VIDA runtime defective and instructs bypass for the current work block, use bounded static analysis, source/file proof, and scoped diffs; do not invent TaskFlow receipts or run mutating VIDA runtime commands.

## Reporting And Final-Report Gate

Every user-facing orchestration report must show compact external state before substance:

1. `Thinking mode: <STC|PR-CoT|MAR|5-SOL|META|Error Search>.`
2. `Requests: active=<n> | in_work=<n> | blocked=<n>` for conversation mode,
3. `Tasks: active=<n> | in_work=<n> | blocked=<n>` for development orchestration mode,
4. `Agents: active=<n> | working=<n> | waiting=<n>`,
5. `Reasoning summary: <compact external reason for the next step, finding set, or blocker state>`.

Rules:

1. report bounded current state, not lifetime totals,
2. `in_work>0` means an active next step is still owed,
3. if `in_work>0`, the report is intermediate and must not be the last action when a lawful next step is already known,
4. do not omit the state block during development orchestration merely because the answer is short,
5. do not expose private chain-of-thought.

Before final transfer, confirm:

1. no delegated agents remain active,
2. no bounded handoff/reroute receipt remains unresolved,
3. no delegated closure proof, coach return, verifier return, or escalation return is outstanding,
4. `in_work=0`,
5. represented request/task is closure-ready under active route law.

If any item is false, emit at most an intermediate report and continue/wait/block lawfully. When explicit continuation intent is active (`continue development`, `продовжи агентами`, or equivalent), `final` transfer and terminal wording are fail-closed forbidden until the user explicitly asks to stop/close.

When final is lawful, end with:

`Session status: completed, closing this session.`

Then emit one extra blank line.

## Decomposition And Delegation

For non-trivial work:

1. decompose before expanding context,
2. convert request into bounded blocking questions or workstreams,
3. start with the smallest lawful discovery wave,
4. deep-read only the subset needed for the current question,
5. delegate read-only discovery first when it answers a blocking question without shared writable scope,
6. parallelize only file-disjoint or contract-disjoint slices,
7. keep shared writable scope sequential under one writer,
8. never dispatch a packet shaped like "implement the whole feature",
9. keep decomposition state in canonical task/evidence surfaces or bounded artifacts.

Explorer/read-only findings may narrow scope but do not authorize root patching by default. Writable gaps still require lawful packet shaping, writer lane, verification, and synthesis unless runtime-defect bypass is explicitly active.

## Algorithm

1. Frame the problem.
2. Classify intent, source, and lens.
3. Bind the active bounded unit.
4. Select the smallest lawful discovery wave.
5. Engage tracked execution for `artifact_flow` or `execution_flow` unless bypass law applies.
6. Resolve task, route family, writer posture, and proof target through canonical owners.
7. For worker routing: use `core.agent-system`, prove admissibility through `core.capability-registry`, govern evidence through `core.context-governance`, and rely on `core.run-graph` when continuity matters.
8. Shape bounded packets with explicit stop/proof boundaries.
9. Dispatch or execute the selected lawful slice.
10. Reconcile returns, tests, and artifacts.
11. Run coach/verifier gates when required.
12. Rebuild parent state after each bounded closure.
13. Close only through canonical tracked-flow owners when tracked execution was engaged.

## Evaluation And Stop Criteria

Evaluate continuously:

1. scope discipline,
2. evidence quality,
3. dependency clarity,
4. writer-scope isolation,
5. verification posture,
6. progress per context token.

Revise route when:

1. the same blocker repeats without new evidence,
2. the same broad file set is reread without a narrower hypothesis,
3. two replans fail to produce a smaller executable slice,
4. the slice lacks explicit closure proof,
5. safe decomposition cannot be established.

Escalate when:

1. coach/verifier posture cannot be satisfied,
2. decomposition collapses into shared-writer ambiguity,
3. route law and evidence conflict,
4. progress would depend on context expansion instead of a narrower lawful slice.

## Constraints

1. Do not mutate task state outside the canonical task-state surface unless explicit runtime-defect bypass is active.
2. Do not execute tracked work outside the active tracked-execution lifecycle.
3. Do not engage pack flow for `answer_only` by default.
4. Do not route through non-canonical command paths.
5. Do not use multiple writer lanes without scope isolation.
6. Do not expose raw worker reports as the default answer.
7. Do not use self-diagnosis to justify local-only closure of tracked remediation.
8. Do not start development boot with broad sweeps when compact boot snapshot is enough.
9. Do not bypass route-marked hard requirements with local/manual fallback unless runtime records blocker, escalation, or bypass law.
10. Do not front-load large repository reads before bounded decomposition.
11. Do not reread context when no new evidence or narrower slice was produced.

## Operational Proof And Closure

Closure-ready orchestration requires explicit:

1. request intent,
2. active bounded unit,
3. writer ownership,
4. route posture,
5. dependency order,
6. proof target,
7. parent-chain classification,
8. absence or resolution of delegated handoff state.

Worker-first routing closes by route-valid execution, not undocumented local substitution. Missing coach/verifier posture, escalation receipt, continuation receipt, or closure proof blocks closure.

## Related

1. `instruction-contracts/core.orchestration-runtime-capsule`
2. `instruction-contracts/core.agent-system-protocol`
3. `instruction-contracts/core.skill-activation-protocol`
4. `instruction-contracts/core.packet-decomposition-protocol`
5. `instruction-contracts/core.agent-prompt-stack-protocol`
6. `instruction-contracts/lane.worker-dispatch-protocol`
7. `runtime-instructions/core.capability-registry-protocol`
8. `runtime-instructions/core.context-governance-protocol`
9. `runtime-instructions/core.run-graph-protocol`
10. `runtime-instructions/work.taskflow-protocol`
11. `runtime-instructions/work.verification-lane-protocol`
12. `runtime-instructions/lane.agent-handoff-context-protocol`
13. `runtime-instructions/work.problem-party-protocol`
14. `command-instructions/routing.use-case-packs-protocol`
15. migration-only wrapper references remain non-canonical.

-----
artifact_path: config/instructions/instruction-contracts/core.orchestration.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: 2026-07-03
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/core.orchestration-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: 2026-07-03T11:12:39.6247137+03:00
changelog_ref: core.orchestration-protocol.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: semantic-atom-coverage+legacy-anchor-crosswalk+pre-commit-baseline-audit
protocol_compression_baseline_ref: e41e56132^:vida/config/instructions/instruction-contracts/core.orchestration-protocol.md
protocol_compression_audit_at: 2026-07-03T11:12:39.6247137+03:00
protocol_compression_before_tokens: 8726
protocol_compression_after_tokens: 4440
protocol_compression_content_sha256: 2bf99fc5cb92ecaca9f7ebdcb4813e52da73414c677fd2a22e9dac9c94ba3eeb
