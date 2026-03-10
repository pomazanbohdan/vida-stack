# Vida Stack Version Plan

Purpose: define the versioned product path for Vida Stack from the current script-based reference runtime to the future self-hosted binary, daemonized control plane, and extension ecosystem.

This document replaces the older `RELEASE-1-*` framing.

Rule:

1. `README.md` explains the product narrative and current direction.
2. This file defines the versioned scope and transition path.
3. Runtime bootstrap truth remains in `AGENTS.md`, active instruction canon lives in `vida/config/instructions/*`, and active framework program/research layers live in `docs/framework/{plans,research,history}/**`.

Design assumptions:

1. `0.1` remains the behavioral source of truth until the binary reproduces it.
2. `1.0` is a self-hosted local binary, not a daemon release.
3. `2.0` owns daemon mode, richer observability, dashboards, and vector-search daemon integration.
4. `3.0` owns plugins, marketplace, and extension ecosystem concerns.

## Product Direction

Vida Stack now moves through four major product states:

1. `0.1` — Reference Script Runtime
2. `1.0` — Self-Hosted Local Binary
3. `2.0` — Daemonized Control Plane
4. `3.0` — Plugins and Marketplace

The current repository is finishing the `0.1` line while defining the migration path into `1.0`.

## Version 0.1 — Reference Script Runtime

`0.1` is the script-based reference stack.

Its job is to:

1. prove protocol behavior on real work,
2. harden task-state, routing, verification, memory, and lifecycle rules,
3. reduce ambiguity before binary migration,
4. act as the source-of-truth behavior layer for the Rust implementation.

`0.1` is not a toy demo.
It is the last large script/runtime line before binary productization.

Core characteristics:

1. shell and Python runtime adapters,
2. protocol-driven execution through `AGENTS.md`, `vida/config/instructions/*`, and framework plan/research evidence,
3. bounded subagent orchestration,
4. task-state through `br` and TaskFlow,
5. review, approval, and route-law enforcement,
6. framework memory, document lifecycle, and operator status as runtime surfaces,
7. installer and framework-only release packaging.

## Transition Path From 0.1 To 1.0

The transition to `1.0` should move through these internal milestones:

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

## Version 1.0 — Self-Hosted Local Binary

`1.0` is the first full VIDA product release.

It should be:

1. one local Rust binary,
2. one embedded state and memory backend,
3. one command-first runtime surface,
4. one self-hosted operating path for developing VIDA itself.

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

Core architectural expectations:

1. embedded SurrealDB,
2. persistent state kernel,
3. persistent memory kernel,
4. versioned instruction kernel,
5. command kernel over those runtime layers,
6. migration kernel for data and instruction upgrades.

`1.0` is not a daemon release.
It is a usable, self-hosted, local control binary.

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

## What Becomes Repository-Minimal In 1.0

By `1.0`, the repository should no longer be the primary startup surface for framework behavior.

The intended direction is:

1. `AGENTS.md` shrinks toward a bootstrap entrypoint,
2. `vida boot` becomes the primary startup command,
3. framework instructions load from the runtime instruction store,
4. state and memory restore from the embedded backend,
5. the binary provides packed startup guidance instead of requiring document-first reconstruction.

## Current Position

The project is currently:

1. finishing the `0.1` reference stack,
2. reducing ambiguity for semantic freeze,
3. preparing the architectural move into the `1.0` binary line.

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
updated_at: '2026-03-10T03:00:04+02:00'
changelog_ref: VERSION-PLAN.changelog.jsonl
