# Context Governance Protocol (CGP)

Purpose: define one canonical framework-owned ledger for context source classes, provenance, freshness, and lane-scoped usage.

## Core Contract

1. Context must be classified before it becomes execution evidence.
2. Every governed context source must declare:
   - `source_class`
   - `source_locator`
   - `freshness`
   - `provenance`
   - `lane_scope`
3. The canonical source classes are:
   - `local_repo`
   - `local_runtime`
   - `overlay_declared`
   - `web_validated`
   - `external_connector`

## Canonical Artifact

1. the canonical governed-context ledger artifact resolved by the active framework runtime

## Activation Surface

Activate this protocol when at least one is true:

1. execution evidence or delegated context must be classified before use,
2. routed worker context needs provenance, freshness, or lane-scope discipline,
3. runtime or observability surfaces must summarize governed source classes,
4. resumability, replay, or verification work depends on governed input context rather than ad hoc evidence.

Primary activating companions:

1. `instruction-contracts/core.orchestration-protocol`
2. `instruction-contracts/core.agent-system-protocol`
3. `runtime-instructions/lane.agent-handoff-context-protocol`
4. `runtime-instructions/core.run-graph-protocol`
5. `instruction-contracts/bridge.instruction-activation-protocol`

## Freshness Rules

1. `local_repo` and `local_runtime` default to `current` unless a producing runtime marks them otherwise.
2. `web_validated` must be marked `validated` or `current`.
3. `overlay_declared` is reserved for project-declared context resolved by the active overlay.
4. `external_connector` is reserved for connector-backed context outside the local repo/runtime boundary.

## Runtime Integration Rules

1. the active pre-execution owner should record a context-governance summary for the input artifacts it consumes.
2. Operator surfaces may summarize counts by source class, freshness, and recent governed task usage, but the ledger remains canonical.
3. Missing context-source classification is a governance gap, not silent approval to treat all evidence equally.
4. Routed worker context must not inherit broad undeclared chat or local-repo context when a bounded governed context set is required.
5. Governed evidence use is a prerequisite for peer protocols that consume routed context; this protocol does not authorize lane routing, admissibility, or resumability by itself.

## Required Core Linkages

1. `core.orchestration` depends on this protocol when routed execution consumes evidence or delegated context.
2. `core.agent-system` depends on this protocol when worker context must be shaped lawfully by lane scope and freshness.
3. `core.run-graph` may record resumability for a run, but it does not replace governed evidence classification; continuity decisions should keep the two ledgers coherent.

## Boundary Rule

1. this protocol owns governed context classes, provenance, freshness, and lane-scoped usage only,
2. it does not own generic worker routing,
3. it does not own typed admissibility,
4. it does not own node-level resumability state,
5. it must not become a tooling or command-reference artifact.

## Operational Proof And Closure

1. governed context is closed only when the consumed source set is classifiable by `source_class`, `freshness`, `provenance`, and `lane_scope`,
2. missing classification must fail closed or escalate through the owning peer protocol rather than silently degrade,
3. summaries or operator views may project this ledger, but they must not replace it as the canonical evidence-governance source.

## Runtime Surface Note

1. concrete operator commands and runtime entrypoints for recording, validating, or summarizing context governance stay in runtime-family help surfaces,
2. this protocol owns the governance law and closure semantics, not the command syntax.

-----
artifact_path: config/runtime-instructions/context-governance.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/core.context-governance-protocol.md
created_at: '2026-03-08T02:15:22+02:00'
updated_at: '2026-03-11T15:17:28+02:00'
changelog_ref: core.context-governance-protocol.changelog.jsonl
