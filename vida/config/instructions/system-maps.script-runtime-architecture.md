# Script Runtime Architecture

Purpose: define the canonical transition from legacy `docs/framework/history/_vida-source/scripts/*` wrappers to the `vida-v0` transitional runtime package.

## Core Rule

Current transitional runtime model:

1. `vida-v0/*` is the active transitional runtime surface,
2. legacy `docs/framework/history/_vida-source/scripts/*.sh` and `docs/framework/history/_vida-source/scripts/*.py` are migration-source wrappers only,
3. project delivery scripts stay in `scripts/` and are not part of this framework contract.

Rule:

1. do not keep two active canonical runtime surfaces,
2. when a legacy wrapper has a `vida-v0` equivalent, `vida-v0` becomes canonical immediately,
3. legacy wrappers remain only until the equivalent behavior is either migrated or explicitly retired.

## Ownership Split

`vida-v0` is responsible for:

1. canonical CLI ergonomics for transitioned surfaces,
2. parsing, validation, routing, scoring, and state derivation,
3. transitional runtime consumption of `vida/config/**`,
4. executable proof that the replacement surface still works.

Historical wrappers are responsible only for:

1. preserving source trail during cutover,
2. supporting unmigrated commands until retired,
3. never reasserting themselves as the active canonical runtime surface.

## Current Framework Examples

Already transitioned or transition-ready in `vida-v0`:

1. boot packet/profile/snapshot,
2. task store and `br` compatibility,
3. TaskFlow/readiness views,
4. run-graph,
5. route resolution,
6. kernel config introspection,
7. worker packet gate,
8. execution authorization and coach gates,
9. spec-intake/spec-delta/draft-execution-spec surfaces,
10. worker registry/system/leases/pool.

Canonical reference map lives in `vida/config/instructions/system-maps.runtime-transition-map.md`.

## Migration Rules

When migrating a framework script to `vida-v0`:

1. identify the canonical caller surface first,
2. preserve CLI arguments and exit-code semantics,
3. move parsing/derivation/validation logic into `vida-v0`,
4. demote the old wrapper to historical-only status or delete it,
5. re-run consumer tests that depend on the command,
6. update this document and all linked framework references in the same change.

## Verification Expectations

Minimum proof for script-runtime migrations:

1. `nim c vida-v0/src/vida.nim` passes,
2. direct `vida-v0` command smoke or targeted test passes,
3. at least one real framework consumer path is verified after migration,
4. old wrapper references are either removed or marked historical-only.

## Boundary Rule

This document covers framework-owned runtime script architecture only.

Do not place here:

1. project build commands,
2. app-specific delivery scripts,
3. product runtime runbooks,
4. worker/model choices for project overlay.

-----
artifact_path: config/system-maps/script-runtime.architecture
artifact_type: system_map
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/system-maps.script-runtime-architecture.md
created_at: 2026-03-09T20:28:59+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: system-maps.script-runtime-architecture.changelog.jsonl
