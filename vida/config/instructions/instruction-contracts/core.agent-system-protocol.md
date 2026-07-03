# Agent System Protocol (ASP)

Purpose: generic portable protocol for agent-system initialization, routing, fallback, and score-state adjustment.

## Core Contract

Canonical model: `agent system` = orchestration/runtime layer; `agent backend` = concrete backend class; `agent lane class` = semantic lane class; `worker packet` = canonical delegated execution artifact.

## Scope

1. activation,
2. backend availability detection and generic backend-class routing,
3. mode selection,
4. routing,
5. success/failure scoring,
6. escalation, promotion, and demotion.

Single-dispatch packet contract stays in `instruction-contracts/lane.worker-dispatch-protocol`.

Worker-lane entry contract stays in `agent-definitions/entry.worker-entry`.

## Activation Surface

Activate this protocol when at least one is true:

1. worker mode is active for eligible work,
2. generic backend-class routing or mode selection must be resolved,
3. authorship, coach, or verifier lane posture must be selected,
4. fallback or escalation between eligible backend classes must be decided,
5. route policy requires worker-first execution rather than local orchestration.

Primary activating companions: `instruction-contracts/core.orchestration-protocol`, `runtime-instructions/core.capability-registry-protocol`, `runtime-instructions/core.context-governance-protocol`, `runtime-instructions/work.project-agent-extension-protocol`, `runtime-instructions/work.verification-lane-protocol`, `instruction-contracts/bridge.instruction-activation-protocol`.

## Canonical State-Surface Note

1. `core.agent-system` does not own one standalone durable ledger equivalent to `run-graph` or `context-governance`,
2. its canonical outputs are routing decisions, selected backend-class route posture, and verification routing posture consumed by adjacent owners,
3. durable typed admissibility remains in `core.capability-registry`, and task lifecycle truth remains outside this protocol.

## Boundary Rule

1. backend-specific onboarding, probing, probation, promotion, degradation, cooldown, recovery, and retirement for external CLI backends are owned by `agent-backends/role.backend-lifecycle-protocol`,
2. this file keeps the generic agent-system routing and mode law above those backend-specific lifecycle mechanics.
3. typed admissibility remains owned by `runtime-instructions/core.capability-registry-protocol`,
4. context provenance, freshness, and lane-scoped governed usage remain owned by `runtime-instructions/core.context-governance-protocol`,
5. this file must not absorb command-level runtime help or backend-specific tool invocation syntax.

## Modes

Supported system modes: `native`, `hybrid`, `disabled`.

Mode-synced execution rule:

1. `native`
   - internal backend classes are the first eligible analysis/review lane and the first authorized development-support orchestration lane.
2. `hybrid`
   - external-first routing remains the default for eligible read-only work and the default first hop for development orchestration whenever route policy requires worker-first execution.
3. `disabled`
   - no worker-first requirement; the orchestrator may execute locally.

Root-lane identity rule:

1. mode selection does not convert the root session into an implementer by default,
2. `native` and `hybrid` preserve orchestrator-first control with delegated execution as the normal posture,
3. `disabled` relaxes worker-first requirement but still does not authorize silent root-session implementation without route law or explicit exception-path evidence.

## Backend Classes

Framework backend classes are generic: one framework-internal backend class, one external execution backend class, one external review backend class.

Project docs/config may bind concrete backends to these classes.

## State Ownership

Hard rule:

1. orchestrator owns task state,
2. orchestrator owns tracked execution lifecycle,
3. orchestrator owns build/close/integration transitions,
4. workers may only return artifacts/results unless explicitly granted bounded repo-write scope.
5. route ownership must not be reinterpreted as implicit write ownership for the root session.

## Entry Separation

Hard rule:

1. `AGENTS.md` is the orchestrator-only entry contract,
2. external and delegated workers must use `agent-definitions/entry.worker-entry`,
3. worker packets should optimize for bounded evidence delivery, not meta-orchestration narration.

## Routing Contract

Routing input: task class, activated mode, configured backend order, backend availability, backend score state, optional project overlay model/profile policy, route-level write and verification policy, optional project role/skill/profile/flow extension registries and validation posture, interaction ownership requirement, context-isolation requirement, statefulness requirement, task dependency / parallel-safety posture, required tool and MCP surface fit.

Routing output: chosen backend, selected model, selected profile, reason, effective score, fallback backends, effective write scope, verification gate, effective route-law metadata, effective lane-class source, effective flow-set source, effective route control limits, effective verification posture, selected orchestration pattern, selection basis.

## Agent Selection Doctrine

Agent selection must explicitly name both chosen lane/backend and orchestration pattern.

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

Minimum selection basis: `task_class`, `interaction_ownership`, `tool_fit`, `write_scope_fit`, `statefulness_need`, `context_isolation_need`, `parallel_safety`, `verification_posture`.

## Route Receipt Minimum Contract

When routing resolves one executable lane for a bounded task/slice, the route receipt must expose enough law for downstream execution and recovery without reconstruction.

Minimum receipt fields: `task_class`, `chosen_backend`, `selected_profile`, `effective_write_scope`, `verification_gate`, `verification_route_task_class`, `independent_verification_required`, `effective_route_control_limits` (`max_rounds`, `max_stalls`, `max_resets`, `max_budget_units`, `max_total_runtime_seconds`), `selected_orchestration_pattern`, `selection_basis`, `reason`, `fallback_backends`.

Rules:

1. execution must consume the route receipt as current control law, not infer missing limits from chat context,
2. if a control limit is omitted by project configuration, the runtime may derive a default, but the receipt must still materialize the effective value,
3. route receipt law must be stable enough for checkpoint/recovery and verification owners to resume without recomputing routing decisions,
4. if route depends on specialist choice, the receipt must show why that agent/lane was lawful instead of leaving choice implicit.

Project extension rule:

1. framework lane classes and standard flow sets remain the stable runtime base.
2. project-owned lane classes, skills, profiles, and flow sets may extend that base only through the validated project overlay path.
3. invalid or unresolved project extensions must fail closed rather than silently degrade into ad hoc runtime behavior.
4. project extension activation and validation semantics are governed by `runtime-instructions/work.project-agent-extension-protocol`.

## Required Core Linkages

1. `core.agent-system` owns generic worker-system routing and mode law only.
2. Before a candidate lane may remain eligible for scoring, this protocol must defer typed admissibility to `runtime-instructions/core.capability-registry-protocol`.
3. When delegated context or evidence is shaped for a lane, this protocol must respect `runtime-instructions/core.context-governance-protocol`.
4. This protocol does not own node-level resumability; that remains in `runtime-instructions/core.run-graph-protocol`.
5. This protocol is a peer of `core.orchestration`, not a replacement for top-level orchestration law.
6. conversational pre-routing and conversational lane-class selection remain owned by `runtime-instructions/work.agent-lane-selection-protocol`.

## Operational Proof And Closure

1. agent-system routing is closed only when the active mode, backend-class route, and verification posture are explicit enough to produce one lawful lane-selection result,
2. when typed admissibility is required, closure depends on `core.capability-registry` proving eligibility before scoring continues,
3. unresolved or invalid project extensions must fail closed rather than silently degrade into ad hoc runtime behavior,
4. when no lawful worker-first path remains and the mode is not `disabled`, escalation must stay explicit rather than collapsing into undocumented local fallback.
5. route closure is incomplete when the effective route control limits or verification posture are still implicit.
6. route closure is incomplete when the chosen orchestration pattern or selection basis is still implicit.
7. route closure is incomplete when root-session local implementation has been assumed without an explicit exception-path receipt or higher-precedence local-law receipt.
8. route closure is incomplete when a read-only discovery lane has found a bounded gap but the lawful writer/coaching/verification cycle has not yet been selected explicitly.
9. route closure is incomplete when a delegated lane or unresolved handoff for the same bounded packet remains open but the root session is being treated as the writer without explicit supersession or hard-blocker evidence.

## Saturation-Recovery Rule

When delegated lane creation fails because of thread, depth, or agent saturation, the orchestrator must run explicit recovery before concluding no worker-first path is available.

Required recovery order:

1. inspect the current delegated-lane inventory and classify each lane as:
   - `active`
   - `waiting`
   - `completed_unsynthesized`
   - `superseded`
   - `blocked`
2. for every `completed_unsynthesized` or `superseded` lane, synthesize or supersede the return first so the orchestrator knows whether the lane still matters,
3. close or reclaim lanes that are already completed and no longer needed for active handoff/verification state,
4. prefer reuse of an existing eligible lane when lawful after that reclaim step,
5. only after inventory review, reclamation, and failed lawful reuse may saturation remain the active reason for an exception path or escalation.

Hard rules:

1. "agent limit reached" is not a sufficient saturation verdict unless the orchestrator first checked whether any completed or superseded delegated lanes can be closed/reclaimed,
2. open handoff state still blocks closure and must be reconciled before a lane is treated as reclaimable,
3. waiting lanes must not be reclaimed merely for convenience if their bounded question is still active,
4. local-only continuation without this saturation-recovery loop is protocol-invalid unless a higher-precedence emergency rule overrides it.

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
4. verification-lane semantics are governed by `runtime-instructions/work.verification-lane-protocol`.

## References

1. `runtime-instructions/work.agent-lane-selection-protocol`
2. `runtime-instructions/core.capability-registry-protocol`
3. `runtime-instructions/core.context-governance-protocol`
4. `runtime-instructions/core.run-graph-protocol`

-----
artifact_path: config/instructions/instruction-contracts/core.agent-system.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/core.agent-system-protocol.md
created_at: '2026-03-09T22:51:59+02:00'
updated_at: 2026-07-03T14:24:00+03:00
changelog_ref: core.agent-system-protocol.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: registry-compaction+route-atom-validation+law-preserve-exact
protocol_compression_baseline_ref: 0d538023e:vida/config/instructions/instruction-contracts/core.agent-system-protocol.md
protocol_compression_audit_at: 2026-07-03T14:24:00+03:00
protocol_compression_before_tokens: 2928
protocol_compression_after_tokens: 2919
protocol_compression_content_sha256: 2d841a73e224e5d397bd98f5e4c069e40734dbdaab387aabe63408165178825b
