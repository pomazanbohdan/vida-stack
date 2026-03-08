# Vida 2.0 Daemon Control Plane Specification

Purpose: define the next product stage after the self-hosted local binary is stable.

## Scope Boundary

### In Scope

1. daemon mode,
2. background workers,
3. watchers and reactive orchestration,
4. richer operator observability,
5. dashboards,
6. vector-search daemon integration when it becomes worth the operational complexity,
7. longer-lived local runtime services.

### Out Of Scope

1. plugin ecosystem as a public product surface,
2. marketplace distribution,
3. treating daemon mode as a replacement for the `1.0` command kernel rather than an expansion of it.

## Core Contract

`2.0` builds on a stable `1.0` binary kernel.

It should not redefine:

1. command law,
2. instruction law,
3. migration law,
4. ownership and governance law.

Instead it should extend the product with longer-lived local behavior.

## Required Expansion Areas

1. resident runtime process,
2. background maintenance loops,
3. richer status and operator visibility,
4. optional semantic retrieval services,
5. stronger automation that no longer requires each action to begin from a cold CLI invocation.

## Relationship To 1.0

`1.0` proves that Vida can operate locally through one binary.

`2.0` proves that the same kernel can stay alive, observe, maintain, and route work over time without forcing the operator to manually restart each loop.

## Relationship To 3.0

Plugins and marketplace features should remain outside `2.0` unless the kernel, daemon runtime, and compatibility model are already stable enough to host external extensions safely.
