# Runtime Home And Surface Migration Research

Purpose: define the first concrete VIDA-specific migration contract from bridge-era root/runtime surfaces into the canonical `.vida/**` runtime home, while preserving source-mode authoring and keeping runtime truth DB-first.

## 1. Research Question

How should VIDA migrate from bridge-era root files and mixed donor/runtime placements into one canonical `.vida/**` runtime home without losing operator clarity, source-mode authoring, or lawful DB-first authority?

## 2. Primary Inputs

Product/spec inputs:

1. `docs/product/spec/runtime-paths-and-derived-cache-model.md`
2. `docs/product/spec/embedded-runtime-and-editable-projection-model.md`
3. `docs/product/spec/project-activation-and-configurator-model.md`
4. `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
5. `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`
6. `docs/product/spec/release-1-plan.md`
7. `docs/product/spec/compiled-runtime-bundle-contract.md`
8. `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`

Research inputs:

1. `docs/product/research/runtime-framework-open-questions-and-external-patterns-survey.md`
2. `docs/product/research/db-authority-and-migration-runtime-research.md`
3. `docs/product/research/compiled-control-bundle-contract-research.md`
4. `docs/product/research/runtime-memory-state-and-retrieval-research.md`

Framework/runtime inputs:

1. `runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`
2. `runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
3. `runtime-instructions/bridge.project-overlay-protocol.md`
4. `instruction-contracts/bridge.instruction-activation-protocol.md`

External grounding already promoted through upstream survey:

1. XDG Base Directory Specification
2. Microsoft declarative/runtime artifact separation patterns
3. OpenAI and Anthropic cache/config separation patterns

## 3. Core Result

The strongest current conclusion is:

1. `.vida/**` must become the canonical runtime home,
2. root project files remain project/source/bootstrap surfaces only,
3. root bridge files such as `vida.config.yaml` are migration-only after cutover,
4. runtime-owned project activation surfaces must move under `.vida/project/**`,
5. export/import is the lawful path for editing hidden runtime-owned surfaces,
6. installed/runtime mode must not depend on root files remaining present after successful migration.

## 4. Canonical Runtime Home Split

The canonical runtime home remains:

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
2. source-mode project files remain outside this tree,
3. a runtime family may narrow usage inside this split but must not invent a parallel home.

## 5. Bridge-Era Source Surfaces To Migrate

The main bridge-era surfaces that should no longer remain active runtime truth are:

1. root `vida.config.yaml`,
2. root or source-tree activation registries,
3. runtime-generated JSON payloads outside `.vida/**`,
4. root project bootstrap carriers used as if they were live runtime configuration,
5. donor-era state/cache/log files treated as execution truth.

Rule:

1. these surfaces may remain as migration aids, source-mode authoring aids, or export views,
2. they must not remain the active runtime substrate after lawful migration.

## 6. Surface Ownership After Migration

### 6.1 Root Project Tree

The root project tree should keep:

1. source code,
2. project docs/specs/process docs,
3. framework bootstrap carriers:
   - `AGENTS.md`
   - `AGENTS.sidecar.md`
4. optional exported projections when explicitly materialized for editing.

It must not keep active runtime truth for:

1. runtime config,
2. active roles/skills/profiles/flows,
3. active protocol-binding state,
4. durable memory truth,
5. runtime cache truth.

### 6.2 `.vida/config/**`

Owns active runtime config:

1. project runtime config,
2. runtime-family config,
3. path overrides,
4. activation/configurator settings,
5. future split config files replacing the bridge root config.

### 6.3 `.vida/project/**`

Owns runtime-owned project activation surfaces:

1. roles,
2. skills,
3. profiles,
4. flows,
5. agents,
6. teams,
7. model/backend policy,
8. known project protocols as runtime-owned activation inventory,
9. explicit exported projections when edit/import is requested.

### 6.4 `.vida/framework/**`

Owns framework-runtime materializations:

1. bundle inspection exports,
2. framework template exports,
3. local framework snapshot views,
4. embedded-artifact inspection outputs.

### 6.5 `.vida/db/**`

Owns authoritative runtime truth:

1. imported framework state,
2. imported project activation state,
3. protocol binding state,
4. memory state,
5. runtime operational state.

### 6.6 `.vida/cache/**`

Owns derived delivery/query caches only.

### 6.7 `.vida/receipts/**`

Owns file-projected receipts and migration evidence only.

## 7. Canonical Migration Order

The strongest current migration order is:

1. detect current runtime-home posture,
2. detect bridge-era root/runtime surfaces,
3. classify each found surface:
   - `active-bridge`
   - `source-only`
   - `runtime-generated`
   - `stale-donor`
4. create canonical `.vida/**` skeleton if absent,
5. migrate config into `.vida/config/**`,
6. migrate runtime-owned project activation into `.vida/project/**`,
7. import authoritative state into `.vida/db/**`,
8. rebuild derived cache under `.vida/cache/**`,
9. write migration receipts,
10. downgrade root bridge surfaces to migration-only or export-only status,
11. mark the project as `runtime-home-green`.

Compact rule:

1. classify first,
2. migrate authority second,
3. only then deprecate bridge surfaces.

## 8. Root Bootstrap Carrier Rule

Root bootstrap carriers are a special case.

They must remain in the root:

1. `AGENTS.md`
2. `AGENTS.sidecar.md`

Reason:

1. they are model-visible bootstrap carriers rather than hidden runtime config,
2. they are part of project/bootstrap discoverability,
3. they are not substitutes for active runtime truth in `.vida/**`.

Interpretation rule:

1. keep them in the root,
2. keep them bootstrap-only,
3. do not let them become shadow runtime config or shadow project activation registries.

## 9. `vida.config.yaml` Migration Rule

`vida.config.yaml` should become bridge-only.

The target law is:

1. active runtime configuration lives under `.vida/config/**`,
2. root `vida.config.yaml` may exist only as:
   - a migration source,
   - an install/init seed,
   - an explicit export projection,
3. installed/runtime mode must not require root `vida.config.yaml` to remain present after successful import.

Migration rule:

1. if root `vida.config.yaml` is present, load it as bridge input,
2. normalize it into `.vida/config/**`,
3. record a migration receipt,
4. treat the root file as no longer authoritative.

## 10. Runtime-Owned Project Activation Migration Rule

Roles, skills, profiles, flows, agents, teams, and project protocol inventory should not remain as active root-tree runtime truth.

Target law:

1. active runtime posture lives under `.vida/project/**` and `.vida/db/**`,
2. root-tree/source-mode registries remain authoring lineage only,
3. runtime execution must consume DB truth plus derived cache, not broad root-tree reads.

Migration rule:

1. read current source-mode activation inventory,
2. normalize into runtime-owned activation form,
3. import the normalized activation into DB truth,
4. keep exported/source registries only as projection or lineage when needed.

## 11. Projection And Authoring Rule

Hidden runtime-owned surfaces do not remove human editability.

The lawful edit path remains:

1. export projection,
2. edit projection,
3. validate,
4. compile if required,
5. import into DB truth,
6. rebuild affected cache families.

Interpretation rule:

1. projections are editable,
2. projections are not automatically authoritative,
3. import is the cutover point.

## 12. Source Mode Versus Installed Mode

### 12.1 Source Mode

Source mode may still keep:

1. framework canon in repo,
2. project docs/specs in repo,
3. source-mode activation lineage in repo,
4. bridge root files for development compatibility during the migration window.

Rule:

1. source mode may expose more files,
2. but active runtime truth still belongs in `.vida/**` plus DB.

### 12.2 Installed Mode

Installed/runtime mode should assume:

1. sealed framework artifacts may be embedded,
2. runtime starts from `.vida/**` and DB truth,
3. root project tree does not need to contain active runtime config or activation registries.

Rule:

1. installed mode must be stricter than source mode,
2. if installed mode works only when root bridge files are still present, the migration is incomplete.

## 13. Path Precedence Rule

The recommended precedence is:

1. authoritative DB truth in `.vida/db/**`,
2. canonical runtime-owned config and activation under `.vida/config/**` and `.vida/project/**`,
3. derived cache under `.vida/cache/**`,
4. exported projections and bridge root files as migration-only or edit-only surfaces.

Conflict rule:

1. if a root bridge file disagrees with imported runtime truth after successful migration, DB truth wins,
2. the root file is drift to reconcile, not a second valid source.

## 14. Migration Receipts

The minimum receipt families should include:

1. `runtime_home_detection_receipt`
2. `bridge_surface_classification_receipt`
3. `config_migration_receipt`
4. `project_activation_surface_migration_receipt`
5. `db_import_receipt`
6. `cache_rebuild_receipt`
7. `runtime_home_ready_receipt`
8. `migration_failure_receipt`

Rule:

1. successful path migration must be inspectable,
2. lack of receipts means the new runtime-home status is not trusted as complete.

## 15. Fail-Close Conditions

Migration or startup must fail closed when:

1. `.vida/**` skeleton cannot be created or validated,
2. runtime config cannot be normalized into `.vida/config/**`,
3. runtime-owned activation cannot be normalized or imported,
4. authoritative DB import is missing or invalid,
5. cache can only be seeded from ambiguous sources,
6. the system still depends on bridge root files for normal runtime execution after claiming migration closure.

Allowed surfaces before full closure:

1. `init`
2. `doctor`
3. `status`
4. bounded migration/repair/import/export commands

## 16. Release-1 Practical Recommendation

The strongest practical Release-1 recommendation is:

1. keep `AGENTS.md` and `AGENTS.sidecar.md` in the root as bootstrap carriers,
2. move active runtime config under `.vida/config/**`,
3. move runtime-owned project activation under `.vida/project/**`,
4. keep authoritative truth in `.vida/db/**`,
5. keep derived query/delivery cache in `.vida/cache/**`,
6. treat root `vida.config.yaml` and source registries as bridge or projection surfaces after lawful import,
7. require explicit import before any edited projection becomes active truth.

## 17. Open Questions

The remaining bounded open questions are:

1. the exact split of files inside `.vida/config/**`,
2. the exact projection schema for editable activation exports,
3. the exact migration UI/operator contract for source-mode repositories with large existing root registries,
4. whether known project protocols first land in `.vida/project/**`, DB, or both during the migration window,
5. the exact cleanup policy for superseded bridge files after successful migration.

## 18. Recommended Next Follow-Up

The strongest next bounded follow-up after this research is:

1. `derived-cache-delivery-and-invalidation-research`

Reason:

1. runtime-home placement is now framed,
2. DB authority is already framed,
3. the next unresolved seam is how `.vida/cache/**` is built, invalidated, and queried without becoming authority.

-----
artifact_path: product/research/runtime-home-and-surface-migration-research
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/research/runtime-home-and-surface-migration-research.md
created_at: '2026-03-12T23:59:50+02:00'
updated_at: 2026-03-16T10:02:36.814104963Z
changelog_ref: runtime-home-and-surface-migration-research.changelog.jsonl
