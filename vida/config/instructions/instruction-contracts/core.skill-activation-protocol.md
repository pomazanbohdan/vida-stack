# Core Skill Activation Protocol

Purpose: define the framework-owned rule for discovering available skills, activating the minimal relevant skill set, and failing closed when a bounded lane materially depends on a missing skill.

## Scope

This protocol defines:

1. the mandatory skill-discovery step for orchestrator and worker lanes,
2. when a skill must be activated,
3. how active skills relate to bounded packets,
4. the fail-closed rule for missing required skills.

This protocol does not define:

1. one specific skill body,
2. one project-specific skill catalog,
3. one specific packet instance,
4. one model/provider policy.

## Core Rule

When an active skill catalog exists, skill inspection is mandatory before bounded work begins.

Framework rule:

1. do not assume a skill exists unless it is visible through the active bootstrap/runtime catalog,
2. activate only the minimal relevant skill set for the current bounded work,
3. if no skill applies, make `no_applicable_skill` explicit,
4. do not silently skip an applicable skill because the work looks familiar.

## Activation Triggers

Activate this protocol when at least one is true:

1. a visible skill catalog exists for the current lane,
2. the user names one or more skills,
3. the bounded packet clearly matches a visible skill description,
4. a role prompt or packet declares a skill dependency.

## Discovery Rule

1. treat the visible session skill list as the current source of available skills,
2. read the relevant `SKILL.md` files before bounded packet work begins,
3. keep skill loading sparse and bounded to the current step.

## Activation Rule

Activate a skill when either is true:

1. the user explicitly names it,
2. the bounded work clearly matches the skill description.

Activation discipline:

1. activate the minimal relevant set,
2. re-evaluate the active skill set when the bounded unit changes materially,
3. do not carry unrelated skills forward by inertia.

## Lane Rule

### Orchestrator

The orchestrator must:

1. inspect the active skill catalog during bootstrap or packet shaping,
2. identify the relevant skill set for the next bounded step,
3. include that expectation in the dispatched packet when it matters materially.
4. when packet dispatch is the known next lawful step, complete skill activation before progress-only reporting or route suspension.

### Worker Lanes

Worker lanes must:

1. inspect the visible skill catalog as part of lane bootstrap,
2. read the relevant `SKILL.md` files before bounded work begins,
3. fail closed when a named or clearly required skill is missing and the packet depends on it materially.

## Packet Rule

When a packet depends materially on one or more skills, it should identify:

1. the relevant skill name or names,
2. whether the skill is `required` or `optional`,
3. the bounded reason the skill matters.

Silence rule:

1. if a packet is silent but the skill match is obvious from the visible catalog, the receiving lane must still activate the relevant skill.

## Fail-Closed Rule

Do not begin bounded work when:

1. a named required skill is missing,
2. a clearly required visible skill has not been activated,
3. the lane cannot determine whether the packet is `skill-bound` or `no_applicable_skill`.
4. the next lawful step is worker dispatch but the required skill check for that packet has not been completed and recorded.

## Related

1. `instruction-contracts/core.orchestration-protocol`
2. `instruction-contracts/lane.worker-dispatch-protocol`
3. `system-maps/protocol.index`

-----
artifact_path: config/instructions/instruction-contracts/core.skill-activation-protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/core.skill-activation-protocol.md
created_at: '2026-03-13T22:00:00+02:00'
updated_at: '2026-03-13T22:00:00+02:00'
changelog_ref: core.skill-activation-protocol.changelog.jsonl
