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
2. `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md`
3. `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`
4. `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md`
5. the canonical tracked-execution owner
6. the canonical execution-telemetry owner
7. `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`

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
6. Tooling, command syntax, and migration helpers stay outside this protocol and belong to runtime-family or migration surfaces.
7. Detailed task-state, tracked-execution, and worker-routing rules remain in their canonical protocols.

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

## Algorithm

1. Frame the problem.
2. Determine request intent class and active orchestration lens.
3. Capture only the minimum lawful boot and task-state context needed to route the work.
4. Apply the tracked-execution engagement gate:
   - `answer_only` stays outside tracked execution,
   - `artifact_flow` and `execution_flow` require tracked execution,
   - `mixed` enters tracked execution only when execution or artifact production becomes required.
5. When tracked execution is required, resolve the lawful task, route family, and execution posture through the canonical runtime owners named in `Related`.
6. When worker routing is active, preserve orchestration-first coordination:
   - route through `core.agent-system`,
   - require admissibility through `core.capability-registry`,
   - require governed context through `core.context-governance`,
   - require node-level resumability through `core.run-graph` when continuity matters.
7. For code-shaped implementation or patch work, run the lawful coach lane before independent verification when an eligible coach lane exists; if coach is unavailable, record an explicit blocker or override receipt before verifier routing.
8. Keep writer ownership singular and fail closed when route requirements, coach/verifier posture, or lawful escalation evidence are missing.
9. Synthesize the resulting analysis or execution outputs into one orchestration-owned result and close only through the canonical tracked-flow owners when tracked execution was engaged.

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

-----
artifact_path: config/instructions/instruction-contracts/core.orchestration.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/core.orchestration-protocol.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-11T16:26:38+02:00'
changelog_ref: core.orchestration-protocol.changelog.jsonl
