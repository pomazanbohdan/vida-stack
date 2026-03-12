# Runtime Paths And Derived Cache Model

Status: active product law

Purpose: define the canonical placement of runtime-owned artifacts under `.vida/`, separate sealed framework runtime state from project-owned runtime activation state, remove active runtime dependence on root project files such as `vida.config.yaml`, and introduce a derived cache layer between authoritative DB state and runtime/CLI consumption.

## 1. Problem

The current repository still mixes several artifact classes:

1. runtime truth,
2. runtime cache,
3. framework runtime artifacts,
4. project-editable source surfaces,
5. transitional donor files and helpers.

Without a stricter path model:

1. runtime truth can drift across DB, root files, and generated snapshots,
2. root project files can be mistaken for active runtime state,
3. cache files can grow into a second truth source,
4. installed runtime and source-mode development remain harder to separate cleanly.

## 2. Goal

The target runtime path model should satisfy all of:

1. one project-local runtime home under `.vida/`,
2. one authoritative DB-first runtime truth root under that home,
3. framework-owned runtime artifacts placed under a framework-specific dot-space,
4. project-owned runtime activation surfaces placed under a project-specific dot-space,
5. a derived cache layer that accelerates CLI/runtime/model-serving hot paths without becoming authority,
6. explicit export/import loops when human-editable projections are needed.

Compact rule:

1. runtime truth lives in `.vida/`,
2. root project files are not the active runtime substrate,
3. cache is derived, never authoritative.

## 3. Canonical `.vida/` Runtime Home

Release-1 and forward runtime should converge on one project-local runtime home:

1. `.vida/config/`
2. `.vida/db/`
3. `.vida/cache/`
4. `.vida/framework/`
5. `.vida/project/`
6. `.vida/receipts/`
7. `.vida/runtime/`
8. `.vida/scratchpad/`

Interpretation rule:

1. these are runtime-owned surfaces,
2. they are distinct from human-facing project source/docs/code surfaces,
3. a runtime family may use narrower subtrees, but it must remain inside this split rather than inventing a parallel placement model.

## 4. Placement Rules

### 4.1 `.vida/config/`

This is the active runtime configuration home.

It should contain:

1. project-level runtime configuration,
2. runtime-family configuration,
3. path overrides,
4. activation/configurator settings,
5. future split config files derived from the current bridge `vida.config.yaml`.

Rule:

1. root `vida.config.yaml` is bridge-only and migration-only,
2. the canonical target runtime location is `.vida/config/**`,
3. installed/runtime mode must not depend on a root config file remaining present.

### 4.2 `.vida/db/`

This is the authoritative runtime truth home.

It should contain:

1. the primary project-local operational database,
2. imported framework-state rows,
3. imported project activation state,
4. protocol-binding state,
5. mutable runtime state, receipts, telemetry, and recovery data.

Rule:

1. DB truth is authoritative for execution,
2. no cache or export file may outrank it,
3. the database remains project-local even when framework artifacts are sealed and embedded.

### 4.3 `.vida/framework/`

This is the framework-runtime artifact home.

It should contain:

1. materialized framework bundle exports,
2. framework template exports,
3. framework runtime snapshots used for bootstrap, audit, or local visibility,
4. future embedded-artifact inspection exports.

Rule:

1. these files are framework-owned runtime artifacts,
2. they are not project-owned source surfaces,
3. runtime may execute from embedded framework artifacts and DB without requiring these files to stay present in installed mode.

### 4.4 `.vida/project/`

This is the project-runtime activation home.

It should contain runtime-owned project activation surfaces such as:

1. roles,
2. skills,
3. profiles,
4. flows,
5. agents,
6. teams,
7. model/backend policy,
8. project protocols,
9. bounded exported activation projections when explicit editing is requested.

Rule:

1. these surfaces are runtime-owned project activation, not root project-source canon,
2. active runtime must not require `docs/process/agent-extensions/**` or similar root-tree registries to remain the live execution source,
3. source-mode registries may still exist for authoring and lineage, but runtime truth remains under `.vida/` plus DB.

### 4.5 `.vida/cache/`

This is the derived serving-cache home.

It should contain only derived artifacts such as:

1. compiled control-bundle snapshots,
2. activation-bundle snapshots,
3. protocol-binding ready snapshots,
4. prompt-prefix partitions,
5. bounded query views for fast CLI/status/doctor rendering,
6. cache manifests and invalidation tuples.

Rule:

1. cache artifacts are always derived from embedded/framework inputs and DB truth,
2. cache artifacts may be deleted and rebuilt without semantic loss,
3. cache artifacts must never become the only authoritative source.

### 4.6 `.vida/receipts/`

This is the durable receipt and migration evidence home when file projection is needed.

It may contain:

1. import receipts,
2. migration receipts,
3. readiness receipts,
4. cache rebuild receipts,
5. export/import operator receipts.

Rule:

1. receipt files are evidence surfaces,
2. they do not replace authoritative DB truth,
3. they must remain bounded and query-safe.

### 4.7 `.vida/runtime/`

This is the runtime-ephemeral operational home.

It may contain:

1. transient traces,
2. session-local runtime files,
3. local sockets or short-lived runtime coordination artifacts,
4. future resumability helpers that are not promoted into DB truth.

Rule:

1. these are operational runtime aids,
2. they are not project source artifacts,
3. they are not a second durable truth model.

## 5. Hidden Runtime Surface Rule

The runtime-owned project configuration and activation surfaces should be hidden from the root project tree by default.

That means:

1. active runtime configuration should move under `.vida/config/**`,
2. active runtime roles/skills/profiles/flows should move under `.vida/project/**`,
3. active runtime execution should not depend on those surfaces appearing as top-level project files,
4. root project tree should remain primarily product/source/documentation space rather than runtime substrate.

Interpretation rule:

1. hidden does not mean inaccessible,
2. hidden means runtime-owned and not part of the default project-source surface.

## 6. Export/Import Rule For Hidden Surfaces

When humans or agents must edit runtime-owned project configuration or activation state, the lawful path is:

1. export from runtime,
2. edit the exported projection,
3. validate and compile,
4. import back into DB truth,
5. rebuild affected derived caches.

Rule:

1. exported files are editable projections,
2. they are not automatically active merely by existing on disk,
3. active truth changes only after successful import.

## 7. Derived Cache Layer

### 7.1 Purpose

The derived cache layer exists to make CLI/runtime/model-serving paths fast without weakening DB-first truth.

It sits between:

1. authoritative DB state and embedded/framework inputs,
2. runtime query/render/serving consumers.

### 7.2 Allowed Cache Families

Release-1 target cache families include:

1. control-bundle snapshots,
2. activation-bundle snapshots,
3. protocol-binding query snapshots,
4. prompt-prefix bundle partitions,
5. bounded status/doctor/readiness query views.

### 7.3 Invalidation Rule

Cache invalidation should be revision-based rather than guess-based.

Minimum invalidation tuple should include:

1. framework artifact revision,
2. project activation revision,
3. runtime config revision,
4. protocol-binding revision,
5. runtime schema version.

Rule:

1. if the tuple is unchanged, cache reuse is allowed,
2. if the tuple changes, the affected cache family must rebuild,
3. stale cache may be tolerated only for explicitly noncritical read-only views if a higher-precedence runtime policy permits it.

### 7.4 Boundary Rule

The derived cache must not:

1. become a second operational truth model,
2. hold the only copy of mutable runtime state,
3. outrank DB-first authority,
4. silently mask missing imports or invalid authoritative state.

## 8. Parameterization Rule

The runtime path model should be parameterizable through runtime configuration.

Minimum path families to parameterize:

1. DB root,
2. cache root,
3. framework export root,
4. project runtime root,
5. receipts root,
6. scratch/runtime temp roots.

Policy families to parameterize:

1. export-on-init,
2. cache rebuild policy,
3. fail-closed cache behavior,
4. allowed stale-read posture for bounded noncritical views,
5. export/import authoring posture.

## 9. Migration Rule

Current root- and donor-era surfaces such as:

1. root `vida.config.yaml`,
2. root-tree project activation registries,
3. `.vida/state/**`,
4. `.vida/data/state/**`,
5. generated payloads outside the canonical `.vida/cache/**` split

should be treated as transitional bridge surfaces.

Migration rule:

1. runtime may read them only through explicit migration or bridge compatibility paths,
2. new runtime law should target the canonical `.vida/` split defined here,
3. compatibility fallbacks must remain bounded and removable.

## 10. Relationship To Other Specs

This model complements, but does not replace:

1. `embedded-runtime-and-editable-projection-model.md`
   - higher-level embedded/runtime/projection shape
2. `project-activation-and-configurator-model.md`
   - DB-first project activation lifecycle and control surface
3. `compiled-autonomous-delivery-runtime-architecture.md`
   - top-level runtime planes and compiled control-bundle direction
4. `taskflow-protocol-runtime-binding-model.md`
   - current protocol-binding bridge authority and DB-backed import path

## 11. Completion Proof

This model is operationally closed enough when:

1. the active runtime configuration can live under `.vida/config/**`,
2. project activation truth can live under `.vida/project/**` plus DB,
3. one project-local DB authority lives under `.vida/db/**`,
4. derived cache artifacts live under `.vida/cache/**` and can be rebuilt safely,
5. root project files are no longer required as the active runtime substrate,
6. export/import remains the lawful edit path for hidden runtime surfaces.

-----
artifact_path: product/spec/runtime-paths-and-derived-cache-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/runtime-paths-and-derived-cache-model.md
created_at: '2026-03-12T20:10:00+02:00'
updated_at: '2026-03-12T20:10:00+02:00'
changelog_ref: runtime-paths-and-derived-cache-model.changelog.jsonl
