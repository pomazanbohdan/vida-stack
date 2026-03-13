# Agent System Protocol (ASP)

Purpose: define one generic, portable protocol for agent-system initialization, routing, fallback, and score-state adjustment.

## Core Contract

Canonical model:

1. `agent system` = orchestration/runtime layer
2. `agent backend` = concrete backend class
3. `agent lane class` = semantic lane class
4. `worker packet` = canonical delegated execution artifact

## Scope

1. activation,
2. backend availability detection and generic backend-class routing,
3. mode selection,
4. routing,
5. success/failure scoring,
6. escalation, promotion, and demotion.

Single-dispatch packet contract stays in `vida/config/instructions/instruction-contracts/lane.worker-dispatch-protocol.md`.

Worker-lane entry contract stays in `vida/config/instructions/agent-definitions/entry.worker-entry.md`.

## Activation Surface

Activate this protocol when at least one is true:

1. worker mode is active for eligible work,
2. generic backend-class routing or mode selection must be resolved,
3. authorship, coach, or verifier lane posture must be selected,
4. fallback or escalation between eligible backend classes must be decided,
5. route policy requires worker-first execution rather than local orchestration.

Primary activating companions:

1. `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
2. `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md`
3. `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`
4. `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`
5. `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md`
6. `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`

## Canonical State-Surface Note

1. `core.agent-system` does not own one standalone durable ledger equivalent to `run-graph` or `context-governance`,
2. its canonical outputs are routing decisions, selected backend-class route posture, and verification routing posture consumed by adjacent owners,
3. durable typed admissibility remains in `core.capability-registry`, and task lifecycle truth remains outside this protocol.

## Boundary Rule

1. backend-specific onboarding, probing, probation, promotion, degradation, cooldown, recovery, and retirement for external CLI backends are owned by `vida/config/instructions/agent-backends/role.backend-lifecycle-protocol.md`,
2. this file keeps the generic agent-system routing and mode law above those backend-specific lifecycle mechanics.
3. typed admissibility remains owned by `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md`,
4. context provenance, freshness, and lane-scoped governed usage remain owned by `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`,
5. this file must not absorb command-level runtime help or backend-specific tool invocation syntax.

## Modes

Supported system modes:

1. `native`
2. `hybrid`
3. `disabled`

Mode-synced execution rule:

1. `native`
   - internal backend classes are the first eligible analysis/review lane and the first authorized development-support orchestration lane.
2. `hybrid`
   - external-first routing remains the default for eligible read-only work and the default first hop for development orchestration whenever route policy requires worker-first execution.
3. `disabled`
   - no worker-first requirement; the orchestrator may execute locally.

## Backend Classes

Framework backend classes are generic:

1. one framework-internal backend class
2. one external execution backend class
3. one external review backend class

Project docs/config may bind concrete backends to these classes.

## State Ownership

Hard rule:

1. orchestrator owns task state,
2. orchestrator owns tracked execution lifecycle,
3. orchestrator owns build/close/integration transitions,
4. workers may only return artifacts/results unless explicitly granted bounded repo-write scope.

## Entry Separation

Hard rule:

1. `AGENTS.md` is the orchestrator-only entry contract,
2. external and delegated workers must use `vida/config/instructions/agent-definitions/entry.worker-entry.md`,
3. worker packets should optimize for bounded evidence delivery, not meta-orchestration narration.

## Routing Contract

Routing input:

1. task class,
2. activated mode,
3. configured backend order,
4. backend availability,
5. backend score state,
6. optional project overlay model/profile policy,
7. route-level write and verification policy,
8. optional project role/skill/profile/flow extension registries and their validation posture,
9. interaction ownership requirement,
10. context-isolation requirement,
11. statefulness requirement,
12. task dependency / parallel-safety posture,
13. required tool and MCP surface fit.

Routing output:

1. chosen backend,
2. selected model,
3. selected profile,
4. reason,
5. effective score,
6. fallback backends,
7. effective write scope,
8. verification gate,
9. effective route-law metadata,
10. effective lane-class source,
11. effective flow-set source.
12. effective route control limits,
13. effective verification posture.
14. selected orchestration pattern,
15. selection basis.

## Agent Selection Doctrine

Agent selection must be explicit about both the chosen lane/backend and the orchestration pattern used to reach it.

Supported selection patterns:

1. `manager_subagent`
   - one orchestrator or manager retains control and invokes specialists for bounded subtasks
2. `handoff`
   - a triage/current agent transfers turn ownership to a specialist for the next interaction step
3. `code_or_router_selected`
   - deterministic code/routing logic classifies the task and picks the next agent/lane explicitly

Selection rules:

1. prefer `manager_subagent` when one orchestrator must own final synthesis, combine outputs from multiple specialists, enforce shared guardrails, or parallelize independent bounded subtasks under centralized control.
2. prefer `handoff` when routing itself is part of the workflow and the chosen specialist should own the next user-facing step with a narrower prompt/state.
3. prefer `code_or_router_selected` when task categories are explicit enough for deterministic classification and you want lower latency, lower cost, or less routing variance than model-only delegation.
4. if the work is multi-domain and file/state-disjoint, `manager_subagent` or `code_or_router_selected` may fan out specialists in parallel; `handoff` should not be the default for parallel consultation.
5. if the specialist must preserve conversation-local state across repeated user turns, prefer `handoff`; if strong context isolation is more important than state reuse, prefer bounded fresh subagents.
6. do not choose a specialist whose allowed tools, MCP servers, or write scope do not match the task's required execution surface.
7. do not use parallel specialist selection when candidates share the same writable scope or resumable state namespace without explicit serialization.

Minimum selection basis:

1. `task_class`
2. `interaction_ownership`
3. `tool_fit`
4. `write_scope_fit`
5. `statefulness_need`
6. `context_isolation_need`
7. `parallel_safety`
8. `verification_posture`

## Route Receipt Minimum Contract

When routing resolves one executable lane for a bounded task or execution slice, the route receipt must expose enough law for downstream execution and recovery owners to operate without reconstruction.

Minimum receipt fields:

1. `task_class`
2. `chosen_backend`
3. `selected_profile`
4. `effective_write_scope`
5. `verification_gate`
6. `verification_route_task_class`
7. `independent_verification_required`
8. `effective_route_control_limits`
   - `max_rounds`
   - `max_stalls`
   - `max_resets`
   - `max_budget_units`
   - `max_total_runtime_seconds`
9. `selected_orchestration_pattern`
10. `selection_basis`
11. `reason`
12. `fallback_backends`

Rules:

1. execution must consume the route receipt as the current control law, not infer missing limits from chat context,
2. if a control limit is omitted by project configuration, the runtime may derive a default, but the receipt must still materialize the effective value,
3. route receipt law must be stable enough for checkpoint/recovery and verification owners to resume without recomputing routing decisions,
4. if the route depends on specialist choice, the receipt must make visible why that agent/lane was lawful for the task instead of leaving agent choice implicit.

Project extension rule:

1. framework lane classes and standard flow sets remain the stable runtime base.
2. project-owned lane classes, skills, profiles, and flow sets may extend that base only through the validated project overlay path.
3. invalid or unresolved project extensions must fail closed rather than silently degrade into ad hoc runtime behavior.
4. project extension activation and validation semantics are governed by `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`.

## Required Core Linkages

1. `core.agent-system` owns generic worker-system routing and mode law only.
2. Before a candidate lane may remain eligible for scoring, this protocol must defer typed admissibility to `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md`.
3. When delegated context or evidence is shaped for a lane, this protocol must respect `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`.
4. This protocol does not own node-level resumability; that remains in `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md`.
5. This protocol is a peer of `core.orchestration`, not a replacement for top-level orchestration law.
6. conversational pre-routing and conversational lane-class selection remain owned by `vida/config/instructions/runtime-instructions/work.agent-lane-selection-protocol.md`.

## Operational Proof And Closure

1. agent-system routing is closed only when the active mode, backend-class route, and verification posture are explicit enough to produce one lawful lane-selection result,
2. when typed admissibility is required, closure depends on `core.capability-registry` proving eligibility before scoring continues,
3. unresolved or invalid project extensions must fail closed rather than silently degrade into ad hoc runtime behavior,
4. when no lawful worker-first path remains and the mode is not `disabled`, escalation must stay explicit rather than collapsing into undocumented local fallback.
5. route closure is incomplete when the effective route control limits or verification posture are still implicit.
6. route closure is incomplete when the chosen orchestration pattern or selection basis is still implicit.

## Runtime Surface Note

1. concrete runtime commands for route inspection, backend availability detection, registry checks, pool/lease handling, or system snapshots belong to runtime-family surfaces rather than this protocol body,
2. this protocol owns generic routing and mode law above those command surfaces,
3. backend-specific CLI behavior remains outside this protocol and belongs to backend-lifecycle or runtime-family owners.

## Independent Verification Contract

Independent verification is a first-class runtime artifact, not an ad hoc orchestrator habit.

Minimum contract:

1. eligible non-trivial work should separate authorship and verification when route policy requires it,
2. verification should be selected from a dedicated verification route class when possible,
3. the verifier should differ from the author lane when another eligible verifier exists.
4. verification-lane semantics are governed by `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md`.

## References

1. `vida/config/instructions/runtime-instructions/work.agent-lane-selection-protocol.md`
2. `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md`
3. `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`
4. `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md`

-----
artifact_path: config/instructions/instruction-contracts/core.agent-system.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/core.agent-system-protocol.md
created_at: '2026-03-09T22:51:59+02:00'
updated_at: '2026-03-13T07:44:24+02:00'
changelog_ref: core.agent-system-protocol.changelog.jsonl
