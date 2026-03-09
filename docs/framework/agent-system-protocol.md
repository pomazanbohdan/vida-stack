# Agent System Protocol (ASP)

Purpose: define one generic, portable protocol for agent-system initialization, routing, fallback, and learning.

This file is the canonical agent-system protocol.

Canonical model:

1. `agent system` = orchestration/runtime layer
2. `agent backend` = concrete execution substrate
3. `agent role` = semantic route role
4. `worker packet` = canonical delegated execution artifact

Scope:

1. activation,
2. backend capability detection,
3. mode selection,
4. routing,
5. success/failure scoring,
6. escalation, promotion, and demotion.

Single-dispatch packet contract stays in `docs/framework/worker-dispatch-protocol.md`.

Worker-lane entry contract stays in `docs/framework/WORKER-ENTRY.MD`.

## Modes

Supported system modes:

1. `native`
2. `hybrid`
3. `disabled`

Mode-synced execution rule:

1. `native`
   - internal backends are the first eligible analysis/review lane and the first authorized development-support orchestration lane.
2. `hybrid`
   - external-first routing remains the default for eligible read-only work and the default first hop for development orchestration whenever route policy requires worker-first execution.
3. `disabled`
   - no worker-first requirement; the orchestrator may execute locally.

## Backend Classes

Framework backend classes are generic:

1. `internal`
2. `external_cli`
3. `external_review`

Project docs/config may bind concrete backends to these classes.

## State Ownership

Hard rule:

1. orchestrator owns task state,
2. orchestrator owns TODO lifecycle,
3. orchestrator owns build/close/integration transitions,
4. workers may only return artifacts/results unless explicitly granted bounded repo-write scope.

## Entry Separation

Hard rule:

1. `AGENTS.md` is the orchestrator-only entry contract,
2. external and delegated workers must use `docs/framework/WORKER-ENTRY.MD`,
3. worker packets should optimize for bounded evidence delivery, not meta-orchestration narration.

## Routing Contract

Routing input:

1. task class,
2. activated mode,
3. configured backend order,
4. backend availability,
5. backend score state,
6. optional project overlay model/profile policy,
7. route-level write and verification policy.

Routing output:

1. chosen backend,
2. selected model,
3. selected profile,
4. reason,
5. effective score,
6. fallback backends,
7. effective write scope,
8. verification gate,
9. effective route-law metadata.

## Independent Verification Contract

Independent verification is a first-class runtime artifact, not an ad hoc orchestrator habit.

Minimum contract:

1. eligible non-trivial work should separate authorship and verification when route policy requires it,
2. verification should be selected from a dedicated verification route class when possible,
3. the verifier should differ from the author lane when another eligible verifier exists.
