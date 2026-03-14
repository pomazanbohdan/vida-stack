# Core Protocol Standard Protocol

Purpose: define the canonical framework-level standard for what a `core` protocol must contain, what it may own, what it must not absorb, and how a `core` protocol participates in the bounded `core cluster` without turning one peer protocol into the owner of the others.

## Scope

This protocol applies when the framework:

1. creates a new `core.*-protocol.md` artifact,
2. materially rewrites an existing `core` protocol,
3. audits whether a `core` protocol is overgrown, under-specified, or mislayered,
4. decides whether a concern belongs inside the `core cluster` or in an adjacent framework layer.

This protocol governs only the standard for `core` protocol artifacts.

It does not replace:

1. the cluster stitching and owner split map in `system-maps/framework.core-protocols-map`,
2. the framework-wide layer model in `docs/product/spec/framework-project-documentation-layer-model.md`,
3. the runtime layer matrix in `docs/product/spec/canonical-runtime-layer-matrix.md`,
4. the naming grammar in `instruction-contracts/meta.protocol-naming-grammar-protocol`.

## Core Principle

`core` protocols are peer framework owners for foundational runtime/orchestration concerns.

Therefore:

1. each `core` protocol must own one bounded concern,
2. no `core` peer may become the meta-owner of the others,
3. cluster-wide composition rules must live above the peers in framework-level maps or meta protocols,
4. one `core` protocol may require another peer, but must not absorb that peer's ownership domain.

## When A Concern Belongs In Core

A concern belongs in the `core cluster` only when all are true:

1. it is framework-owned rather than project-owned,
2. it is not bootstrap/environment-only,
3. it is not human-governance-only,
4. it is not backend-specific lifecycle law,
5. it is not a command-layer or operator-catalog concern,
6. it defines a foundational runtime/orchestration/state/evidence boundary that other framework behavior may depend on.

If any item fails, the concern must be placed in its proper adjacent owner layer instead of entering `core`.

## Required Contents Of A Core Protocol

Every `core` protocol must define, in one coherent owner artifact:

1. `Purpose`
   - one bounded owned concern stated explicitly
2. `Scope`
   - what the protocol governs and what it deliberately leaves to adjacent owners
3. `Core Contract`
   - the minimum non-negotiable law for the owned concern
4. `Canonical Artifact` or canonical state surface when applicable
   - only when the concern has durable state, ledger, or canonical machine-readable output
5. `Activation Surface`
   - the trigger conditions that make the protocol active
6. `Primary activating companions`
   - adjacent owners or bridges that lawfully co-activate the concern
7. `Boundary Rule`
   - explicit statement of what the protocol does not own
8. `Operational Proof And Closure`
   - what must be true before the concern is considered closed or proven
9. `Required Core Linkages`
   - only when the concern participates directly in cluster-level peer integration
10. `Runtime Surface Note`
    - only when concrete runtime commands or migration surfaces exist and must be kept outside the owner law

Compact rule:

1. a `core` protocol must be self-sufficient as an owner,
2. but it must not become a cluster map, tool guide, or peer-policy bundle.

## Required Qualities Of A Core Protocol

Every `core` protocol must be:

1. `bounded`
   - one protocol owns one coherent concern rather than a mixed pile of loosely related behavior
2. `framework-owned`
   - the concern must belong to framework canon, not project or local runtime notes
3. `fail-closed`
   - missing preconditions or unproven state must block or escalate rather than silently degrade
4. `activation-visible`
   - the task must be able to determine when the protocol is active
5. `relation-visible`
   - required peer linkages must be explicit enough for canonical discovery
6. `non-duplicative`
   - it must not restate another peer's law body as a second owner
7. `runtime-aware but not runtime-help-shaped`
   - it may govern runtime law, but concrete command syntax belongs elsewhere

## Forbidden Contents Of A Core Protocol

A `core` protocol must not absorb:

1. tooling discovery or operator command catalogs,
2. concrete runtime-family help text or command syntax as the primary law body,
3. backend-specific onboarding, probing, cooldown, recovery, promotion, or retirement mechanics,
4. project-owned environment/process guidance,
5. project/product behavior law,
6. bootstrap carrier responsibilities,
7. human-governance ownership,
8. worker-entry or worker-packet ownership,
9. another `core` peer's bounded owner concern,
10. future-layer or unfinished runtime behavior as if it were already closed law.

If a protocol starts to absorb any of the above, that is `core` drift and must be corrected.

## Peer Ownership Rule

Inside the `core cluster`:

1. `core.orchestration` may integrate the cluster, but it does not own typed admissibility, governed context, or node-level resumability,
2. `core.agent-system` may own generic routing and mode law, but it does not own backend lifecycle, typed admissibility, or resumability,
3. `core.capability-registry` may own typed admissibility, but it does not own generic routing or command catalogs,
4. `core.context-governance` may own provenance/freshness/lane-scoped evidence governance, but it does not own routing, admissibility, or node-level resumability,
5. `core.run-graph` may own node-level routed-run resumability, but it does not own task lifecycle truth, telemetry truth, or governed evidence classification.

Rule:

1. peer protocols may depend on one another,
2. peer protocols must not redefine one another.

## State-Surface Rule

When a `core` concern touches durable state or runtime surfaces:

1. the protocol must name the canonical state surface clearly,
2. it must distinguish that surface from adjacent state surfaces,
3. it must not become a second state engine for a neighbor concern,
4. it must not use vague wording that lets one ledger silently replace another.

Examples of required split:

1. task lifecycle truth may remain in the canonical task-state surface,
2. execution telemetry may remain in the canonical execution-telemetry surface,
3. node-level resumability may remain in `run-graph`,
4. governed evidence classification may remain in `context-governance`.

## Runtime-Surface Separation Rule

When concrete commands or runtime entrypoints exist for a `core` concern:

1. the `core` protocol owns the law and proof conditions,
2. runtime-family maps and runtime help surfaces own the command syntax,
3. migration maps own historical-only or transition-only command references,
4. the presence of CLI examples must not turn the `core` protocol into the canonical operator guide.

## Cluster Integration Rule

If a `core` protocol has required peer dependencies:

1. the protocol must name the peer concern explicitly enough to avoid implied-only coupling,
2. the linkage must preserve peer ownership rather than collapsing the concerns together,
3. cluster stitching must remain consistent with `framework.core-protocols-map.md`,
4. protocol index wiring must expose the linkage for canonical discovery.

## Layer Placement Rule

`core` protocols belong to `Framework Canon`.

They do not belong to:

1. `Agent Role Layer`,
2. `Bootstrap / Environment Layer`,
3. `Human Governance Layer`,
4. `Project Documentation Layer`.

If a `core` protocol begins to carry one of those layer responsibilities, the correction is to move that content to the proper owner layer rather than widening `core`.

## When To Create A New Core Protocol

Create a new `core` protocol only when all are true:

1. the concern is foundational enough to belong to framework canon,
2. the concern has one bounded owner that is not already covered by an existing `core` peer,
3. the concern requires its own activation and closure law,
4. the concern cannot be expressed as a small clarification to an existing peer or to `framework.core-protocols-map.md`.

Do not create a new `core` protocol merely because:

1. a current peer needs stronger wording,
2. a command surface grew large,
3. a project/runtime implementation exists,
4. a map could have been updated instead.

## Audit Questions

When auditing any `core` protocol, answer all of:

1. what exactly is the one bounded concern owned here,
2. what is the minimum hard law of that concern,
3. when does this protocol activate,
4. what state or canonical artifact does it own, if any,
5. what peer protocols does it require,
6. what adjacent concerns does it explicitly not own,
7. does it fail closed when proof is missing,
8. has it accidentally become a tool guide, bootstrap note, or project note,
9. has it duplicated another peer's law,
10. is its discovery wiring visible through canonical maps and protocol index surfaces.

If any answer is missing, the protocol is underspecified or drifting.

## Validation Rule

Changes to a `core` protocol standard or to a `core` protocol under this standard require:

1. `check` on the changed scope,
2. `activation-check` when activation wiring or protocol-bearing coverage changed,
3. `protocol-coverage-check --profile active-canon` when protocol-bearing artifacts or canonical protocol inventory changed,
4. `doctor --profile active-canon-strict`,
5. `proofcheck --profile active-canon-strict`,
6. `readiness-check --profile active-canon` only when readiness-facing law or readiness surfaces changed.

## Routing

1. for cluster composition and required peer edges:
   - continue to `system-maps/framework.core-protocols-map`
2. for owner-layer placement:
   - continue to `system-maps/framework.protocol-layers-map`
3. for filename grammar and artifact naming:
   - continue to `instruction-contracts/meta.protocol-naming-grammar-protocol`
4. for runtime-layer placement of runtime-side `core` concerns:
   - continue to `docs/product/spec/canonical-runtime-layer-matrix.md`

-----
artifact_path: config/instructions/instruction-contracts/meta.core-protocol-standard.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/meta.core-protocol-standard-protocol.md
created_at: '2026-03-11T16:45:00+02:00'
updated_at: '2026-03-12T07:58:34+02:00'
changelog_ref: meta.core-protocol-standard-protocol.changelog.jsonl
