# Vida 1.0 Product Specification

Purpose: define the product boundary for the first full Vida release as a self-hosted local binary.

## Scope Boundary

### In Scope

1. one local Rust binary named `vida`,
2. one embedded SurrealDB-backed runtime,
3. command-first operation for daily framework work,
4. persistent state, memory, instruction, and migration kernels,
5. self-hosted use of Vida for developing Vida,
6. compact TOON-native output for core operator flows.

### Out Of Scope

1. daemon mode,
2. dashboards,
3. resident background workers,
4. watchers and reactive automation loops,
5. vector-search daemon as a required runtime dependency,
6. plugins and marketplace.

## Product Contract

`Vida 1.0` must be a usable local product, not a technology preview.

The binary must let an operator or agent:

1. boot the framework,
2. inspect current operating state,
3. progress task work deterministically,
4. read and write framework memory,
5. validate runtime integrity,
6. upgrade across releases with explicit migration rules.

## Required Operator Surface

The minimum command surface is:

1. `vida boot`
2. `vida task ...`
3. `vida memory ...`
4. `vida status`
5. `vida doctor`

These commands define the first canonical operating surface for daily use.

## Self-Hosting Contract

`Vida 1.0` is considered successful only if Vida can use the binary as its primary local operating surface.

That means:

1. the framework can boot itself through `vida boot`,
2. current work can be inspected through `vida status`,
3. next-step execution can advance through `vida task ...`,
4. framework knowledge can be restored through `vida memory ...`,
5. runtime integrity can be checked through `vida doctor`.

## Runtime Kernels Required By 1.0

The product requires five kernels:

1. state kernel,
2. memory kernel,
3. instruction kernel,
4. command kernel,
5. migration kernel.

Each kernel is required for the release boundary.

## TOON Output Contract

The default operator experience should be compact and structured.

TOON-native output is required for:

1. `vida boot`,
2. `vida task next`,
3. `vida status`,
4. `vida doctor`.

Human-readable output may still exist, but the product should be designed around compact structured interaction rather than verbose prose.

## Non-Goals For 1.0

The first product release should explicitly avoid:

1. trying to solve daemon orchestration too early,
2. trying to solve marketplace extensibility too early,
3. treating plugin architecture as part of the kernel,
4. making vector retrieval a blocking dependency for local use.

## Acceptance Criteria

Vida `1.0` is ready only if all of the following are true:

1. the binary is the primary operator entrypoint,
2. boot, task, memory, status, and doctor are usable daily,
3. core state and instruction migrations are enforced at startup,
4. the framework can operate through the binary without document-first startup,
5. the release boundary does not depend on daemon-only services.

## Relationship To Other Versions

1. `0.1` proves and stabilizes behavior,
2. `1.0` productizes that behavior into the binary,
3. `2.0` adds daemonized control-plane behavior,
4. `3.0` opens plugins and marketplace delivery on top of the stable kernel.
