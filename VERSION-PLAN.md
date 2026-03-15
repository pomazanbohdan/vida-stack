# Vida Stack Version Plan

Purpose: define the versioned product path for Vida Stack from the current transitional runtime substrates to the future compiled autonomous delivery runtime, daemonized control plane, and extension ecosystem.

This document stays above the detailed `Release 1` program package and defines the versioned product path rather than the inner execution program.

Rule:

1. `README.md` explains the product narrative and current direction.
2. This file defines the versioned scope and transition path.
3. Runtime bootstrap truth remains in `AGENTS.md`, active instruction canon lives in `vida/config/instructions/**`, and active product-law direction lives in `docs/product/spec/**`.
4. The bounded Release-1 working entrypoint is `docs/product/spec/release-1-program-map.md`, not this version ladder.

Design assumptions:

1. the active repository execution line is the `0.9.0` transition slice and it is the only active planning baseline in this document.
2. `Release 1` and `1.0` point at the first CLI-first compiled autonomous delivery runtime, not at a daemon release.
3. the public runtime surface is `vida taskflow` plus `vida docflow`; implementation ownership lives in the Rust runtime line.
4. `2.0` owns daemon mode, richer observability, dashboards, and vector-search daemon integration.
5. `3.0` owns plugins, marketplace, and extension ecosystem concerns.

## Product Direction

Vida Stack now moves through four active product states:

1. `0.9.0` — Self-Hosting Transition and Runtime Hardening
2. `1.0` / `Release 1` — CLI-First Compiled Autonomous Delivery Runtime
3. `2.0` — Daemonized Control Plane
4. `3.0` — Plugins and Marketplace

The current repository is operating in the `0.9.0` transition line, with active work focused on runtime hardening, DB-first closure gaps, and `Release 1` / `1.0` readiness.

## Transition Path From 0.9.0 To 1.0 / Release 1

The active path to `1.0` / `Release 1` is:

1. stabilize the runtime operational spine (`boot/status/doctor/taskflow/docflow`) on fresh and migrated state roots,
2. close DB-first authority for activation entities (roles, skills, profiles, flows) with YAML as projection/import-export only,
3. complete configurator lifecycle surfaces (`status/import/export/sync/reconcile/restore`) and their receipts,
4. close protocol/proof surface parity so documented operator paths and runtime command surfaces match fail-closed behavior,
5. finalize release hardening and closure gates for `1.0.0`.

## Version 0.9.0 — Self-Hosting Transition and Runtime Hardening

`0.9.0` is the active transition line.

Its job is to close the highest-risk runtime gaps before `1.0.0`:

1. runtime stability hardening across `boot/status/doctor/taskflow` surfaces,
2. schema and state compatibility hardening for fresh and migrated state roots,
3. removal of transition-only fallback behavior and stale aliases,
4. documentation/runtime status alignment so canonical docs reflect real operator behavior,
5. explicit readiness reporting for remaining DB-first and configurator-lifecycle gaps.

Current observed closure state for `0.9.0`:

1. installed and source `vida` binaries pass bounded post-fix smoke for core non-destructive operator paths,
2. `status` and `doctor` no longer fail on fresh booted state because missing run-graph dispatch receipt schema is now repaired by canonical bootstrap plus `open_existing` schema reconciliation,
3. open gap to `1.0.0` remains in DB-first authority closure for project activation entities (roles/skills/profiles/flows), where YAML projections are still active runtime inputs.

## Version 1.0 / Release 1 — CLI-First Compiled Autonomous Delivery Runtime

`1.0` / `Release 1` is the first full VIDA product release.

It is the point where the active public runtime moves from the current transition runtime line to the compiled Rust runtime line.

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

1. operating on the `0.9.0` transition line,
2. hardening runtime reliability and closing high-risk transition drifts before `1.0.0`,
3. using `vida taskflow` and `vida docflow` as the active public runtime-family surfaces,
4. preparing the architectural move into the `1.0` / `Release 1` compiled runtime line through active parallel Rust implementation tracks.

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
artifact_revision: '2026-03-15'
schema_version: '1'
status: canonical
source_path: VERSION-PLAN.md
created_at: '2026-03-10T00:30:00+02:00'
updated_at: '2026-03-15T09:05:34+02:00'
changelog_ref: VERSION-PLAN.changelog.jsonl
