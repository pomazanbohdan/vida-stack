# Vida 3.0 Plugin And Marketplace Specification

Purpose: define the extension model that should arrive only after the binary kernel and daemon runtime are stable.

## Core Principle

Plugins extend Vida around a stable core.

They do not replace:

1. command law,
2. route law,
3. review and approval law,
4. migration law,
5. framework invariants.

## Plugin Package Model

A Vida plugin should be able to ship one or more of the following:

1. flow definitions,
2. instruction parts or capsules,
3. role-specific protocols,
4. artifact templates,
5. validators and auditors,
6. integrations and adapters,
7. renderers,
8. optional retrieval helpers.

## First-Class Plugin Categories

### Flow Packs

These packages extend process flows.

Examples:

1. `SDLC` pack
2. discovery pack
3. incident-response pack
4. release-management pack
5. architecture-review pack

### Role Protocol Packs

These packages provide role-specific instruction and artifact surfaces.

Examples:

1. `PM` pack
2. `BA` pack
3. `SA` pack
4. later `QA`, `Security`, `SRE`, and domain-specific role packs

These packs can provide:

1. role-specific decision cards,
2. artifact schemas,
3. checklists,
4. instruction capsules,
5. verification helpers.

### Domain Packs

Examples:

1. mobile product pack
2. backend/API pack
3. data platform pack
4. AI product pack

### Validator And Auditor Packs

Examples:

1. lifecycle validator,
2. instruction integrity checker,
3. scope-contract auditor,
4. release-readiness validator.

### Integration Packs

Examples:

1. GitHub pack
2. Jira pack
3. Linear pack
4. docs-sync pack

## Example Packs From The Current Design Direction

The earliest marketplace-ready ideas should include:

1. `SDLC` flow pack
   - stage gates,
   - delivery checkpoints,
   - artifact expectations,
   - release and change-management routines
2. `PM` protocol pack
   - prioritization and backlog-shaping instructions,
   - stakeholder decision cards,
   - roadmap and release framing templates
3. `BA` protocol pack
   - discovery questionnaires,
   - scope and requirement decomposition,
   - acceptance and edge-case artifacts
4. `SA` protocol pack
   - architecture-contract instructions,
   - boundary and integration review,
   - NFR and systems-analysis checkpoints

These are strong plugin candidates because they are:

1. valuable,
2. reusable across projects,
3. richer than one-off prompts,
4. not part of the core kernel law.

## Marketplace Contract

The marketplace should distribute packages only on top of a stable extension model.

At minimum, a marketplace package should declare:

1. plugin identity,
2. compatible Vida version range,
3. provided capability types,
4. instruction namespaces,
5. required runtime surfaces,
6. migration implications if any.

## Safety And Governance Rules

Plugins may:

1. add new flow packs,
2. add new instruction layers,
3. add validators and integrations,
4. add role and domain protocols.

Plugins may not:

1. weaken core invariants,
2. bypass review or approval law,
3. replace mandatory route law,
4. silently override framework-owned command semantics,
5. force incompatible migrations without explicit validation.

## Why Plugins Belong In 3.0

Plugins should arrive only after:

1. the command contract is stable,
2. the instruction runtime is stable,
3. the state and memory schemas are stable,
4. migration rules are stable,
5. the daemon runtime is mature enough to host external extension logic safely.

That is why plugins and marketplace belong in `3.0`, not in `1.0`.
