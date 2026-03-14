# Capability Registry Protocol (CRP)

Purpose: define a framework-owned typed capability registry for agent lanes and a deterministic compatibility gate between route task classes and candidate workers.

## Core Contract

1. Route selection may use scoring and cost heuristics.
2. Compatibility must be checked before scoring can authorize a lane.
3. A candidate that fails compatibility is ineligible, not merely low-ranked.
4. Compatibility is a fail-closed gate, not an advisory preference.
5. The registry is the canonical typed capability source for routed worker admissibility; route heuristics must not silently override it.

## Canonical Artifact

1. the canonical typed capability-registry artifact resolved by the active framework runtime

Artifact rule:

1. the generated registry must remain machine-readable and deterministic for the same capability inputs,
2. route compatibility decisions must resolve against the current generated registry artifact rather than ad hoc worker descriptions,
3. if the artifact is missing, stale, or invalid for the active check, routed worker eligibility remains unproven and must fail closed.

## Typed Task-Class Requirements

Registry must define requirement groups for at least:

1. `analysis`
2. `coach`
3. `verification`
4. `verification_ensemble`
5. `review_ensemble`
6. `problem_party`
7. `read_only_prep`
8. `implementation`

Each group must declare:

1. `allowed_write_scopes`
2. `required_capability_any`
3. `required_artifacts`
4. `forbidden_capabilities`

Compatibility interpretation rule:

1. `allowed_write_scopes`
   - defines the maximum lawful repo-write surface for the task class.
2. `required_capability_any`
   - at least one declared capability from this set must be present for the candidate to remain eligible.
3. `required_artifacts`
   - required runtime or protocol-facing artifacts must be available before eligibility is proven.
4. `forbidden_capabilities`
   - any match disqualifies the candidate even if all positive requirements pass.

## Activation Surface

Activate this protocol when at least one is true:

1. worker routing must filter candidate lanes before scoring or cost comparison,
2. task-class to worker compatibility must be proven for delegated execution,
3. project or framework capability declarations changed and the typed registry must be refreshed or checked,
4. lane-class selection or agent-system routing work needs explicit admissibility evidence rather than heuristic lane selection.

Primary activating companions:

1. `instruction-contracts/core.orchestration-protocol`
2. `instruction-contracts/core.agent-system-protocol`
3. `runtime-instructions/work.agent-lane-selection-protocol`
4. `instruction-contracts/bridge.instruction-activation-protocol`

Boundary rule:

1. this protocol owns typed compatibility gating only,
2. generic route selection, fallback policy, and mode law stay in `agent-system-protocol.md`,
3. overlay lane-class and conversational mode selection stay in `agent-lane-selection-protocol.md`,
4. top-level orchestration and writer ownership stay in `core.orchestration-protocol.md`,
5. this file must not become a second generic routing protocol or a command catalog.

## Required Core Linkages

1. `core.agent-system` depends on this protocol before eligible routed candidates may remain in scoring scope,
2. `core.orchestration` depends on this protocol when delegated lane admissibility must be proven before route authorization,
3. this protocol does not replace either peer's routing law; it supplies the fail-closed typed gate they consume.

## Operational Proof And Closure

Minimum proof conditions:

1. the registry artifact exists and is readable,
2. the active task class resolves to one declared requirement group,
3. the candidate worker can be checked against the group with an explicit pass/fail result,
4. ineligible candidates are excluded before any ranking/scoring step continues.

Closure rule:

1. routed worker eligibility is closed only when compatibility was checked against the canonical registry artifact,
2. a successful route may proceed only after at least one candidate remains eligible,
3. if no candidate remains eligible, the route must fail closed or escalate through the owning routing protocol instead of silently widening admissibility.

Discoverability note:

1. canonical discovery lives in `system-maps/protocol.index`,
2. activation binding lives in `instruction-contracts/bridge.instruction-activation-protocol`,
3. absence from either surface is activation/discoverability drift and should be treated as blocking for canonical coverage claims.

## Runtime Surface Note

1. concrete operator commands and migrated runtime entrypoints for registry build/check stay in the execution runtime-family surfaces, not in this owner protocol,
2. this protocol owns the compatibility law and proof conditions; runtime-family maps and runtime help surfaces own the concrete command syntax,
3. migration-only command history belongs in `system-maps/migration.runtime-transition-map`.

-----
artifact_path: config/runtime-instructions/capability-registry.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md
created_at: '2026-03-08T02:15:22+02:00'
updated_at: '2026-03-11T16:26:38+02:00'
changelog_ref: core.capability-registry-protocol.changelog.jsonl
