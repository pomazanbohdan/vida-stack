# Framework Core Protocols Map

Purpose: provide one bounded framework-owned map for the `core cluster` of protocols so agents can orient the cluster as one package without turning the core protocols themselves into tooling catalogs or mixed topology documents.

## Scope

This map covers only the framework `core cluster`:

1. `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
2. `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`
3. `vida/config/instructions/instruction-contracts/core.skill-activation-protocol.md`
4. `vida/config/instructions/instruction-contracts/core.packet-decomposition-protocol.md`
5. `vida/config/instructions/instruction-contracts/core.agent-prompt-stack-protocol.md`
6. `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md`
7. `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`
8. `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md`

This map does not own:

1. tooling/operator command discovery,
2. project/environment notes,
3. backend lifecycle specifics,
4. worker-packet law,
5. execution-substrate detail outside the bounded core-cluster routing/state split.

## Activation Triggers

Read this map when:

1. the task asks how the `core` protocols fit together as one cluster,
2. a task needs the bounded owner split inside the `core cluster`,
3. a task asks for cross-core consistency, needs coverage, or required linkages,
4. a task asks what the `core` protocols must not absorb.

## Core Cluster Model

The `core cluster` is the bounded framework package for:

1. top-level orchestration posture and execution gates,
2. agent-system routing, mode, and verification posture,
3. skill activation when a visible skill catalog exists,
4. bounded packet decomposition and just-in-time deeper refinement,
5. prompt-stack precedence between bootstrap, role prompt, packet, skill overlay, and runtime state,
6. typed worker admissibility before scoring,
7. context provenance/freshness governance,
8. node-level routed-run resumability.

Cluster reading rule:

1. `core` is the structural cluster label,
2. each file under the cluster remains the canonical owner of its own bounded family/function,
3. this map is a stitching and routing surface only, not a second law-bearing owner.
4. artifact-standard requirements for what a `core` protocol must contain live in `vida/config/instructions/instruction-contracts/meta.core-protocol-standard-protocol.md`.

## Canonical Owner Split

1. `core.orchestration`
   - owns top-level orchestration law, execution authorization posture, writer ownership, and route-stage decomposition
2. `core.agent-system`
   - owns generic worker-system mode, backend-class routing, and verification routing posture
3. `core.skill-activation`
   - owns visible-skill discovery, minimal relevant skill activation, and fail-closed missing-skill behavior
4. `core.packet-decomposition`
   - owns bounded packet law, default leaf selection, and just-in-time deeper refinement
5. `core.agent-prompt-stack`
   - owns prompt-layer precedence between bootstrap, role prompt, packet, skill overlay, and runtime state
6. `core.capability-registry`
   - owns typed admissibility and fail-closed compatibility before scoring
7. `core.context-governance`
   - owns context-source classes, provenance, freshness, and lane-scoped evidence governance
8. `core.run-graph`
   - owns node-level routed-run resumability state beyond task lifecycle truth and execution telemetry

## Required Cluster Boundaries

The `core cluster` must not absorb:

1. tooling discovery or operator command catalogs,
2. backend-specific onboarding and recovery law,
3. project-owned environment/process guidance,
4. worker-entry or worker-packet artifact law,
5. detailed runtime-family command routing that belongs to execution or documentation runtime-family maps.

## Required Linkages

Current required package-level linkages are:

1. `core.orchestration -> core.agent-system`
   - required and explicit
2. `core.orchestration -> core.skill-activation`
   - required when a visible skill catalog exists
3. `core.orchestration -> core.packet-decomposition`
   - required and explicit
4. `core.orchestration -> core.agent-prompt-stack`
   - required and explicit
5. `core.agent-system -> core.capability-registry`
   - required and explicit
6. `core.orchestration -> core.capability-registry`
   - required at package level, currently mostly indirect through `core.agent-system`
7. `core.orchestration -> core.context-governance`
   - required at package level for governed evidence/context usage
8. `core.agent-system -> core.context-governance`
   - required at package level for routed worker context discipline
9. `core.orchestration -> core.run-graph`
   - required at package level for routed-run resumability
10. `core.context-governance -> core.run-graph`
   - desirable continuity link, but not the primary blocking edge today

## Package Rule

The `core cluster` is intended to remain:

1. package-level stitched through explicit peer linkages,
2. semantically law-bearing at the owner level,
3. separate from concrete command syntax, which remains in runtime-family and migration surfaces,
4. bounded against worker-entry law, backend lifecycle law, and project/process ownership.

## Routing

1. top-level routed execution, writer ownership, and execution authorization questions:
   - continue to `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
2. worker mode, backend-class routing, fallback, and verification-lane posture:
   - continue to `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`
3. visible skill discovery, activation, and missing-skill fail-closed questions:
   - continue to `vida/config/instructions/instruction-contracts/core.skill-activation-protocol.md`
4. bounded packet shaping, default leaf depth, and JIT deeper refinement:
   - continue to `vida/config/instructions/instruction-contracts/core.packet-decomposition-protocol.md`
5. prompt-stack precedence between bootstrap, role prompt, packet, skill overlay, and runtime state:
   - continue to `vida/config/instructions/instruction-contracts/core.agent-prompt-stack-protocol.md`
6. typed admissibility and fail-closed compatibility before scoring:
   - continue to `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md`
7. context-source provenance, freshness, and lane-scoped evidence governance:
   - continue to `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`
8. routed-run resumability and node-level stage state:
   - continue to `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md`
9. activation and canonical coverage questions for this cluster:
   - continue to `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`
   - then to `vida/config/instructions/system-maps/protocol.index.md`
10. runtime-layer placement questions for the runtime-side `core` protocols:
   - continue to `docs/product/spec/canonical-runtime-layer-matrix.md`
11. artifact-standard and boundary questions for what a `core` protocol must contain:
   - continue to `vida/config/instructions/instruction-contracts/meta.core-protocol-standard-protocol.md`

-----
artifact_path: config/system-maps/framework.core-protocols-map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/framework.core-protocols-map.md
created_at: '2026-03-11T15:35:00+02:00'
updated_at: '2026-03-11T16:26:38+02:00'
changelog_ref: framework.core-protocols-map.changelog.jsonl
