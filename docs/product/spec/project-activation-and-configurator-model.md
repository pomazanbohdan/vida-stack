# Project Activation And Configurator Model

Status: active product law

Purpose: define the canonical DB-first project activation model and configurator behavior for Release 1, including project-owned runtime entities, explicit and automatic activation modes, lifecycle operations, and filesystem/Git synchronization boundaries.

## 1. Operational Truth Model

Project activation uses one authoritative operational state model:

1. `SurrealDB` is the primary operational truth,
2. filesystem artifacts are synchronized editable projections,
3. `Git` preserves filesystem history and backup lineage,
4. runtime executes against database truth rather than against raw files directly.

Compact rule:

1. DB first,
2. filesystem mirrored,
3. Git historical.

Placement rule:

1. active runtime configuration and project activation surfaces should converge under `.vida/**`,
2. root project files may remain source-mode or export-mode surfaces,
3. they are not the long-term active runtime authority.

## 2. Configurator Purpose

The configurator exists to manage which runtime behavior is active for one project without requiring direct mutation of sealed framework law.

It owns:

1. project activation state,
2. project-owned runtime entities,
3. import/export/sync/reconcile flows,
4. explicit and automatic activation posture,
5. controlled lifecycle changes for project-owned surfaces.

Primary runtime homes:

1. `.vida/config/**`
2. `.vida/project/**`
3. `.vida/db/**`

## 3. Minimum Release-1 Entity Set

Release 1 must support all of:

1. roles,
2. skills,
3. profiles,
4. flow sets,
5. agents,
6. teams,
7. model classes,
8. backend classes,
9. policy surfaces,
10. project protocols.

Release rule:

1. if this pool is materially incomplete, the result is pre-release rather than Release 1.

## 4. Framework Versus Project Split

### 4.1 Framework-Owned

Framework owns:

1. sealed system protocols,
2. core/system runtime safety law,
3. baseline role classes,
4. framework gate and packet rules,
5. framework orchestration and bundle compilation rules.

### 4.2 Project-Owned

Project owns:

1. enabled role subset,
2. project roles,
3. project skills,
4. project profiles,
5. project flow sets,
6. project agents,
7. project teams,
8. project model/backend policy,
9. known project protocols,
10. promoted executable project protocols.

Runtime-home rule:

1. active runtime-owned project roles, skills, profiles, flows, agents, teams, model/backend policy, and project protocols should live under `.vida/project/**` plus DB truth,
2. root-tree registries may remain source-mode authoring or export/import surfaces, but they are not the target installed/runtime truth model.

## 5. Activation Modes

### 5.1 Explicit Mode

In explicit mode:

1. the operator/project declares the active runtime composition directly,
2. enabled roles, profiles, skills, flows, teams, and adjacent policy are selected in configuration,
3. runtime must respect this selection exactly unless validation fails.

### 5.2 Automatic Mode

In automatic mode:

1. the runtime may dynamically choose from the enabled role catalog and adjacent project activation state,
2. the automatic choice must still obey explicit disablement and project policy,
3. the runtime must not auto-enable a project surface that the project has disabled.

## 6. Lifecycle Operations

The configurator must support a consistent lifecycle for project-owned activation surfaces:

1. `import`
2. `activate`
3. `update`
4. `replace`
5. `disable`
6. `restore`

Lifecycle rule:

1. exact per-entity permission details may differ,
2. but the configurator must expose one coherent lifecycle model across the main project-owned classes.

## 7. Sync And Reconciliation

Release 1 requires bidirectional sync under DB-first authority.

That means:

1. DB changes may project into export files,
2. filesystem export changes may be imported back into DB,
3. runtime must detect drift and reconcile it lawfully,
4. Git receives the filesystem projection as backup/history.

Conflict rule:

1. conflicts must not be silently solved by a model,
2. runtime may use point changes and bounded tools in either direction,
3. conflict handling must stay explicit, inspectable, and fail closed.

## 8. Release-1 CLI Requirements

Release 1 must expose CLI control over at least:

1. initialization/bootstrap,
2. configuration inspection,
3. import/export,
4. sync/reconcile,
5. activation changes,
6. restore,
7. bounded query/status retrieval for project activation state.

## 9. Queryability Rule

Each major activation/configuration surface must be queryable through bounded operator paths.

Examples:

1. active roles,
2. active skills,
3. active profiles,
4. active flows,
5. active teams,
6. active model/backend policy,
7. project protocol registration/promotion state,
8. sync/reconcile status.

Model-facing summaries may be used, but:

1. they must be built on explicit query paths,
2. they must not replace the underlying operator-accessible state surfaces.

## 10. Boundary Rule

1. framework-owned protocol state must enter the operational database only through migration/init paths,
2. project-owned runtime state may evolve through the lawful configurator lifecycle,
3. the configurator must not become a bypass around sealed framework law,
4. root `vida.config.yaml` and root-tree activation registries are bridge-compatible only and must not remain the final active runtime placement.

## 11. Completion Proof

This model is closed enough for Release 1 when:

1. DB-first project activation state is operational,
2. filesystem projection and Git lineage are both present,
3. explicit and automatic modes both exist,
4. lifecycle operations are coherent across project-owned classes,
5. sync/reconcile remains lawful and fail-closed under conflict.

Operator-flow note:

1. the user-facing sequencing of this activation/configurator model is owned by `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`,
2. this document remains the domain owner for activation/configurator law itself.

-----
artifact_path: product/spec/project-activation-and-configurator-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/project-activation-and-configurator-model.md
created_at: '2026-03-11T23:01:49+02:00'
updated_at: '2026-03-12T21:55:00+02:00'
changelog_ref: project-activation-and-configurator-model.changelog.jsonl
