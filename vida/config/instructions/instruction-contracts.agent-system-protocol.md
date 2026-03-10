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

Single-dispatch packet contract stays in `vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md`.

Worker-lane entry contract stays in `vida/config/instructions/agent-definitions.worker-entry.md`.

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
2. orchestrator owns TaskFlow lifecycle,
3. orchestrator owns build/close/integration transitions,
4. workers may only return artifacts/results unless explicitly granted bounded repo-write scope.

## Entry Separation

Hard rule:

1. `AGENTS.md` is the orchestrator-only entry contract,
2. external and delegated workers must use `vida/config/instructions/agent-definitions.worker-entry.md`,
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
8. optional project role/skill/profile/flow extension registries and their validation posture.

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
10. effective role source,
11. effective flow-set source.

Project extension rule:

1. framework roles and standard flow sets remain the stable runtime base.
2. project-owned roles, skills, profiles, and flow sets may extend that base only through the validated project overlay path.
3. invalid or unresolved project extensions must fail closed rather than silently degrade into ad hoc runtime behavior.
4. project extension activation and validation semantics are governed by `vida/config/instructions/runtime-instructions.project-agent-extension-protocol.md`.

## Independent Verification Contract

Independent verification is a first-class runtime artifact, not an ad hoc orchestrator habit.

Minimum contract:

1. eligible non-trivial work should separate authorship and verification when route policy requires it,
2. verification should be selected from a dedicated verification route class when possible,
3. the verifier should differ from the author lane when another eligible verifier exists.
4. verification-lane semantics are governed by `vida/config/instructions/runtime-instructions.verification-lane-protocol.md`.

-----
artifact_path: config/instructions/instruction-contracts/agent-system.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts.agent-system-protocol.md
created_at: '2026-03-09T22:51:59+02:00'
updated_at: '2026-03-10T15:41:04+02:00'
changelog_ref: instruction-contracts.agent-system-protocol.changelog.jsonl
