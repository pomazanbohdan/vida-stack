# Vida Roadmap Reframe Research

Purpose: capture the product-direction reasoning behind the move from the old `RELEASE-1` framing to a versioned Vida product line.

## Core Conclusions

1. `0.1` should be treated as the reference script runtime, not a throwaway demo.
2. `1.0` should be a self-hosted local binary, not a daemon-first release.
3. `2.0` should own daemonized runtime behavior, richer observability, dashboards, and vector-search daemon integration.
4. `3.0` should own plugins, marketplace, and ecosystem extensibility after the kernel is stable.

## Why `0.1` Stays Important

The current stack already contains the behavioral truth for:

1. orchestration,
2. task-state law,
3. review and approval gates,
4. framework memory,
5. lifecycle and operator surfaces.

That means `0.1` is the source-of-truth behavior layer that the binary must reproduce before it can replace the script runtime as the main operator surface.

## Why `1.0` Is Binary-First

The next product step should be:

1. one local Rust binary,
2. one embedded operational backend,
3. one command-first UX,
4. one self-hosted path for developing Vida through Vida.

This is preferred over a daemon-first release because it reduces risk and creates a usable product earlier:

1. no resident service lifecycle is required,
2. no background worker and watcher complexity is required,
3. self-hosting can start through direct commands,
4. migration and upgrade law can be stabilized before background automation is introduced.

## Why Embedded SurrealDB Fits

The binary direction benefits from an embedded backend because Vida needs more than a flat task store.

The product already points toward:

1. workflow state,
2. framework memory,
3. instruction runtime,
4. approvals and receipts,
5. lifecycle and operator status.

An embedded SurrealDB-backed model supports that richer runtime without forcing an external database deployment for `1.0`.

## Why Instructions Move Into Runtime

The future product should not keep the primary startup surface in a large repository bootloader.

The binary direction therefore assumes:

1. framework-owned instructions become versioned runtime data,
2. command behavior is assembled from ordered instruction parts or capsules,
3. project and user overlays extend the runtime within validated boundaries,
4. release upgrades update core instructions through migrations.

This is required for:

1. self-hosted startup,
2. stable upgrades,
3. compatibility checks,
4. reducing markdown-first startup friction.

## Why TOON Matters

Vida wants compact command-first interaction rather than verbose startup prose and JSON-heavy operator surfaces.

TOON is a strong fit for:

1. `vida boot`,
2. `vida task next`,
3. `vida status`,
4. `vida doctor`,
5. packed startup guidance.

It should therefore be treated as part of the `1.0` operator contract, not as a later cosmetic improvement.

## Why Vector Search Moves To `2.0`

Semantic retrieval is valuable, but daemon-dependent vector search should not become a hard dependency for the first binary release.

`1.0` can remain correct and useful with:

1. exact lookup,
2. metadata filters,
3. explicit links,
4. durable memory records.

Vector-search daemon integration can then land in `2.0`, where a longer-lived local runtime already exists.

## Why Plugins And Marketplace Move To `3.0`

Plugins should not arrive before the binary kernel, instruction model, migration rules, and command contracts are stable.

Otherwise the ecosystem would amplify instability instead of extending a mature core.

That means:

1. `1.0` builds the kernel,
2. `2.0` matures the control plane,
3. `3.0` opens structured extensibility.

## Early Plugin Ideas Worth Preserving

The most natural plugin examples for Vida are not novelty tools but workflow and instruction packs.

Examples:

1. `SDLC` flow packs
2. `PM` protocol packs
3. `BA` protocol packs
4. `SA` protocol packs
5. later `QA`, `Security`, `SRE`, and domain-specific packs

These packs can ship:

1. flow definitions,
2. role-specific instruction parts,
3. decision-card templates,
4. artifact schemas,
5. validators and checklists,
6. integration adapters when needed.

## Decision Summary

The roadmap should therefore be read as:

1. prove the logic in `0.1`,
2. productize it into a self-hosted local binary in `1.0`,
3. daemonize the stable kernel in `2.0`,
4. open plugin and marketplace extensibility in `3.0`.
