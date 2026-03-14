# Vida Stack Version Plan

Purpose: define the versioned product path for Vida Stack from the current transitional runtime substrates to the future compiled autonomous delivery runtime, daemonized control plane, and extension ecosystem.

This document stays above the detailed `Release 1` program package and defines the versioned product path rather than the inner execution program.

Rule:

1. `README.md` explains the product narrative and current direction.
2. This file defines the versioned scope and transition path.
3. Runtime bootstrap truth remains in `AGENTS.md`, active instruction canon lives in `vida/config/instructions/**`, and active product-law direction lives in `docs/product/spec/**`.
4. The bounded Release-1 working entrypoint is `docs/product/spec/release-1-program-map.md`, not this version ladder.

Design assumptions:

1. `0.2.x` remains the active semantic-freeze and proving line built on bounded proof runtimes, with `v0.2.1` as the latest published hotfix and `v0.2.2` as the current protocol-binding bridge slice now built and installer-proven locally before publication.
2. `Release 1` and `1.0` point at the first CLI-first compiled autonomous delivery runtime, not at a daemon release.
3. the public runtime surface is `vida taskflow` plus `vida docflow`; implementation ownership lives in the Rust runtime line.
4. `2.0` owns daemon mode, richer observability, dashboards, and vector-search daemon integration.
5. `3.0` owns plugins, marketplace, and extension ecosystem concerns.

## Product Direction

Vida Stack now moves through four major product states:

1. `0.2.x` — Semantic-Freeze And Proving Line
2. `1.0` / `Release 1` — CLI-First Compiled Autonomous Delivery Runtime
3. `2.0` — Daemonized Control Plane
4. `3.0` — Plugins and Marketplace

The current repository is operating in the `0.2.x` proving line, with `v0.2.1` as the latest published hotfix and `v0.2.2` as the active next slice for TaskFlow protocol-binding closure, now locally built and temp-install proven while semantic-freeze and closure work continue toward `Release 1` / `1.0`.

## Version 0.2.x — Semantic-Freeze And Proving Line

`0.2.x` is the active semantic-freeze and proving stack. The current published hotfix on that line is `v0.2.1`, and the active next bounded slice is `v0.2.2`, now validated as a local release candidate.

Its job is to:

1. prove protocol behavior on real work,
2. harden task-state, routing, verification, memory, and lifecycle rules,
3. reduce ambiguity before binary migration,
4. act as the source-of-truth behavior layer for the next compiled runtime implementation.

`0.2.x` is not a toy demo.
It is the semantic-freeze and proving release that bridges the current bounded proof runtimes into the compiled autonomous delivery runtime line.

Core characteristics:

1. bounded proof runtimes through `vida taskflow` and `vida docflow`,
2. protocol-driven execution through `AGENTS.md`, `vida/config/instructions/**`, and active product/spec canon,
3. source-of-truth authority in `docs/product/spec/**`, `vida/config/**`, and `vida/config/instructions/**`,
4. bounded subagent orchestration,
5. review, approval, and route-law enforcement,
6. framework memory, document lifecycle, and operator status as runtime surfaces,
7. installer and framework-only release packaging,
8. active parallel Rust `taskflow` / `docflow` implementation tracks for `Release 1`, not the current public runtime.

Current `0.2` priorities:

1. freeze the system vocabulary:
   - commands,
   - states,
   - approvals,
   - receipts,
   - memory kinds
2. close canonical maps and instruction canon so runtime is not the source of meaning,
3. collect golden fixtures and parity artifacts for the binary kernel path,
4. avoid a forbidden middle path where `1.0` is silently hidden inside the old shell/runtime stack,
5. prepare a clean `taskflow` / `docflow` / `vida` crate split for the next release line.
6. bind the first TaskFlow execution path to canonical protocol activation and enforcement without using file-log-only truth.

Current `v0.2.2` target:

1. introduce the TaskFlow protocol-binding bridge as a bounded subrelease,
2. keep protocol-binding authority on DB/taskflow runtime state,
3. materialize compiled protocol-binding JSON from the canonical seed and import it into `taskflow-state.db`,
4. prove script-era bridge outputs, installer bootstrap, and later Rust parity against that same authoritative state,
5. avoid a false closure path where exported file logs are treated as runtime truth.

## Transition Path From 0.2.x To 1.0 / Release 1

The transition to `1.0` / `Release 1` moves through these internal milestones, with the repository currently inside the `0.2.x` semantic-freeze and closure phase:

1. `0.2` — semantic freeze
   - freeze command model,
   - freeze state vocabulary,
   - freeze approval, lifecycle, and memory kinds,
   - remove critical ambiguity before binary migration
2. `0.3` — migration specification
   - define binary command tree,
   - define SurrealDB state model,
   - define instruction/runtime storage model,
   - define data and instruction migration rules,
   - define cap-based instruction composition and effective command capsules,
   - define project and user instruction overlay rules without weakening framework law
3. `0.4` — binary foundation
   - create Rust workspace,
   - split `taskflow` and `docflow` into separate crates from the start,
   - add embedded SurrealDB,
   - add fast build profile,
   - ship minimal `vida` binary shell,
   - add migration runner and version checks at startup
4. `0.5` — state, memory, and instruction kernel
   - persistent state,
   - framework memory,
   - project memory foundation,
   - versioned instruction storage and loading,
   - instruction parts and command capsules,
   - effective instruction composition from core plus project plus user layers
5. `0.6` — task engine and TOON-native command contracts
   - `vida task ...`,
   - structured compact command outputs,
   - deterministic task progression
6. `0.7` — boot, status, and doctor surfaces
   - `vida boot`,
   - `vida status`,
   - `vida doctor`,
   - packed startup instructions from real runtime state
7. `0.8` — governance and migrations
   - approval and verification law in the binary,
   - lifecycle validation,
   - schema and instruction migrations,
   - fail-closed upgrade path
8. `0.9` — self-hosting transition
   - VIDA uses `vida` as its primary operating surface,
   - script runtime becomes reference and migration support

## Version 1.0 / Release 1 — CLI-First Compiled Autonomous Delivery Runtime

`1.0` / `Release 1` is the first full VIDA product release.

It is the point where the active public runtime moves from the `0.2.x` proof runtimes to the compiled Rust runtime line.

It should be:

1. one local Rust binary,
2. one embedded state and memory backend,
3. one command-first runtime surface,
4. one self-hosted operating path for developing VIDA itself,
5. one compiled autonomous delivery runtime that consumes compiled law/config bundles instead of re-reading raw canon on every step.

Required `1.0` capabilities:

1. `vida boot`
2. `vida task ...`
3. `vida memory ...`
4. `vida status`
5. `vida doctor`
6. versioned migrations for state and instructions
7. TOON-native compact output for core operator flows
8. self-hosting viability for daily framework work
9. hard startup checks for schema, instruction, and compatibility migrations
10. framework-owned instruction updates with compatibility validation for project and user overlays
11. `taskflow` available as a reusable library crate and as its own CLI surface
12. `docflow` available as a reusable library crate and as its own CLI surface
13. the top-level `vida` CLI composed over those crates rather than replacing them

Core architectural expectations:

1. embedded SurrealDB,
2. persistent state kernel,
3. persistent memory kernel,
4. versioned instruction kernel,
5. command kernel over those runtime layers,
6. migration kernel for data and instruction upgrades.
7. `taskflow` and `codex` must remain separate bounded crates in the same workspace.
8. each must be usable independently as a library and independently as a CLI tool.
9. the `vida` binary may compose them, but it must not collapse their boundaries.
10. direct runtime consumption of canonical inventory, readiness, bundles, and projections is blocked until `taskflow` becomes the primary runtime engine for that path; `codex` alone cannot close that layer.

`1.0` / `Release 1` is not a daemon release.
It is a usable, self-hosted, CLI-first local control runtime.

Instruction-runtime expectations for `1.0`:

1. core instructions are stored and versioned in the runtime, not only in repo docs,
2. core instructions can be updated by release-driven migrations,
3. command behavior is assembled from ordered instruction parts or capsules,
4. project and user instruction layers can extend the runtime within validated boundaries,
5. user overlays may not weaken framework invariants, review law, approval law, or route law.

## Version 1.0 Non-Goals

The following should stay out of `1.0`:

1. background daemon services,
2. dashboards and rich control-plane UIs,
3. resident background workers,
4. watchers and reactive automation loops,
5. marketplace and public plugin ecosystem,
6. vector-search daemon dependence as a required core runtime path.

Notes:

1. lightweight exact lookup, filters, and explicit links are acceptable in `1.0`,
2. vector-search integration can wait until `2.0` because it benefits from a longer-lived daemonized runtime.

## Version 2.0 — Daemonized Control Plane

`2.0` should add long-lived local control-plane behavior on top of the stable binary kernel.

Target areas:

1. daemon mode,
2. background workers,
3. watchers and reactive orchestration,
4. richer observability and dashboards,
5. vector-search daemon integration when it is worth the operational cost,
6. more autonomous background maintenance and runtime services.

`2.0` expands the local product.
It does not redefine the core model established in `1.0`.

## Version 3.0 — Plugins and Marketplace

`3.0` should add ecosystem extensibility after the binary kernel and daemon runtime are stable.

Target areas:

1. plugin system,
2. marketplace,
3. flow packs such as `SDLC`,
4. role protocol packs such as `PM`, `BA`, and `SA`,
5. validators and auditors,
6. import/export adapters,
7. project integrations,
8. retrieval and renderer extensions,
9. optional DB-side extension use where beneficial.

The plugin layer should build on stable:

1. command contracts,
2. instruction contracts,
3. state and memory schemas,
4. migration rules,
5. ownership and governance law.

Plugin package examples worth preserving:

1. `SDLC` flow pack,
2. `PM` protocol pack,
3. `BA` protocol pack,
4. `SA` protocol pack.

## What Becomes Repository-Minimal In 1.0 / Release 1

By `1.0` / `Release 1`, the repository should no longer be the primary startup surface for framework behavior.

The intended direction is:

1. `AGENTS.md` shrinks toward a bootstrap entrypoint,
2. `vida boot` becomes the primary startup command,
3. framework instructions load from the runtime instruction store,
4. state and memory restore from the embedded backend,
5. the binary provides packed startup guidance instead of requiring document-first reconstruction.

## Current Position

The project is currently:

1. operating on the `0.2.x` proving line,
2. reducing ambiguity for semantic freeze and Release-1 closure,
3. using `vida taskflow` and `vida docflow` as the current public proof runtimes,
4. treating canonical specs, config, and instruction canon as the source of truth,
5. preparing the architectural move into the `1.0` / `Release 1` compiled runtime line through active parallel Rust implementation tracks.

## Core Principle

Vida Stack should evolve in this order:

1. prove the logic,
2. freeze the semantics,
3. productize the binary,
4. daemonize the runtime,
5. open the extension ecosystem.

-----
artifact_path: project/repository/version-plan
artifact_type: repository_doc
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: VERSION-PLAN.md
created_at: '2026-03-10T00:30:00+02:00'
updated_at: '2026-03-13T08:47:25+02:00'
changelog_ref: VERSION-PLAN.changelog.jsonl
