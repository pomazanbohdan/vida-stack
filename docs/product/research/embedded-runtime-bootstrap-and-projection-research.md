# Embedded Runtime Bootstrap And Projection Research

Purpose: define the first concrete VIDA-specific vision for how embedded runtime artifacts, bootstrap/init flows, `.vida/**` state, and editable projection/export-import loops cooperate during install, init, and ongoing runtime operation.

## 1. Research Question

How should VIDA combine embedded framework artifacts, project-local `.vida/**` state, root bootstrap carriers, and editable export/import projections so installed runtime stays binary-first while project onboarding and editing remain lawful and understandable?

## 2. Primary Inputs

Product/spec inputs:

1. `docs/product/spec/embedded-runtime-and-editable-projection-model.md`
2. `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`
3. `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
4. `docs/product/spec/release-build-packaging-law.md`
5. `docs/product/spec/runtime-paths-and-derived-cache-model.md`
6. `docs/product/spec/release-1-wave-plan.md`
7. `docs/product/spec/project-activation-and-configurator-model.md`

Research inputs:

1. `docs/product/research/db-authority-and-migration-runtime-research.md`
2. `docs/product/research/runtime-home-and-surface-migration-research.md`
3. `docs/product/research/derived-cache-delivery-and-invalidation-research.md`
4. `docs/product/research/compiled-control-bundle-contract-research.md`

Framework/runtime inputs:

1. `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`
2. `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
3. `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`

## 3. Core Result

The strongest current conclusion is:

1. installed runtime should start from embedded framework artifacts plus `.vida/**` state,
2. root bootstrap carriers remain model-visible entrypoints and therefore stay in the project root,
3. `init` is the lawful cutover from installed shell to project-local runtime home,
4. `project-activator` enriches project-facing routing and onboarding after runtime home exists,
5. editable projections are export/import views over hidden runtime-owned state rather than a second runtime substrate.

## 4. Four Runtime Surface Classes

The strongest practical split is:

1. `embedded framework runtime artifacts`
2. `root bootstrap carriers`
3. `hidden runtime-owned surfaces under .vida/**`
4. `editable projections`

### 4.1 Embedded Framework Runtime Artifacts

Own:

1. sealed framework control bundle baseline,
2. framework instruction/runtime bundle baseline,
3. framework template bundle baseline,
4. framework bootstrap/runtime metadata needed before DB import.

Rule:

1. installed runtime may execute from these plus DB truth,
2. they are not edited in place during ordinary operation.

### 4.2 Root Bootstrap Carriers

Own:

1. model-visible bootstrap routing,
2. project-doc bootstrap routing,
3. onboarding trigger visibility.

Specifically:

1. `AGENTS.md`
2. `AGENTS.sidecar.md`

Rule:

1. they remain in the root,
2. they are not hidden under `.vida/**`,
3. they do not become active runtime config or activation truth.

### 4.3 Hidden Runtime-Owned Surfaces

Own:

1. active runtime config under `.vida/config/**`,
2. project activation surfaces under `.vida/project/**`,
3. DB truth under `.vida/db/**`,
4. cache under `.vida/cache/**`,
5. receipts/runtime ephemeral files under the remaining `.vida/**` families.

### 4.4 Editable Projections

Own:

1. exported config views,
2. exported activation views,
3. exported operator-facing bootstrap/project docs where allowed,
4. explicit import-ready artifacts.

Rule:

1. they are editable,
2. they are not active truth until imported.

## 5. Install Versus Init Versus Activator

The strongest current lifecycle split is:

1. `install`
   - place the runtime shell and embedded baseline
2. `init`
   - create `.vida/**`, seed DB truth, scaffold required bootstrap carriers, and establish runtime-home readiness
3. `project-activator`
   - enrich project-facing onboarding, routing, project structure understanding, and initial task/config import

Compact rule:

1. install gives the shell,
2. init gives the project-local runtime home,
3. activator gives the project a useful operating posture.

## 6. What `init` Must Materialize

The strongest current `init` contract is:

1. create `.vida/**` skeleton,
2. create active runtime config home,
3. create project activation home,
4. seed or migrate authoritative DB state,
5. seed derived cache only after DB authority is lawful,
6. materialize or normalize `AGENTS.md`,
7. materialize or normalize `AGENTS.sidecar.md`,
8. route pending onboarding into `project-activator` when project posture is incomplete.

Rule:

1. `init` may create root bootstrap carriers,
2. but it must not spill active runtime truth back into arbitrary root files.

## 7. What `project-activator` Must Materialize

`project-activator` should materialize only what improves operator/bootstrap understanding, such as:

1. sidecar enrichment,
2. project-map enrichment,
3. environment/runtime summary docs,
4. import-ready task/project payloads,
5. host environment template initialization state.

Rule:

1. activator enriches operator-facing/project-facing surfaces,
2. it does not replace `init` as the owner of runtime-home creation or DB import.

## 8. Binary-First But Not Root-Blind

The strongest current recommendation is:

1. installed runtime should be binary-first,
2. but the project root still matters for model-visible bootstrap carriers and project-facing docs.

Interpretation rule:

1. binary-first does not mean no root files,
2. it means execution does not depend on loose framework source files or root config/activation files as runtime truth.

## 9. Editable Projection Families

The strongest Release-1 projection families are:

1. `config_projection`
2. `activation_projection`
3. `protocol_projection` when explicitly allowed
4. `operator_doc_projection`

Projection rule:

1. export only what has a lawful import path or durable operator value,
2. do not export arbitrary internal runtime state just because it exists.

## 10. Projection Metadata

Every editable projection should carry enough metadata for safe return:

1. projection type,
2. schema version,
3. provenance/source family,
4. exported revision tuple,
5. import class,
6. compatibility status.

Rule:

1. projection import must fail closed if metadata is missing or incompatible.

## 11. Import Cutover Rule

Edited projections become active only after:

1. validation,
2. normalization,
3. compilation when required,
4. DB import,
5. cache rebuild for affected families,
6. import receipt write.

Forbidden pattern:

1. treating the edited file on disk as automatically active because it is newer than DB.

## 12. Empty Project Bootstrap Rule

For an empty project:

1. `init` still creates `.vida/**` and DB truth,
2. `AGENTS.md` and `AGENTS.sidecar.md` still appear as bootstrap carriers,
3. `project-activator` becomes the main follow-up path,
4. project-facing docs/projections may be minimal until the operator provides structure.

## 13. Existing Project Migration Rule

For an existing project:

1. `init` first detects current root/bootstrap/config posture,
2. normalizes root bootstrap carriers,
3. migrates active runtime-owned config and activation into `.vida/**`,
4. seeds DB truth,
5. then sends unresolved project-specific onboarding work to `project-activator`.

Rule:

1. existing project migration is not the same as empty-project bootstrap,
2. but both converge on the same `.vida/**` runtime home and bootstrap-carrier split.

## 14. Runtime Startup Preference Order

The strongest startup preference order is:

1. embedded framework artifacts,
2. authoritative DB truth,
3. derived cache,
4. bounded root bootstrap carriers for routing/orientation,
5. editable projections only when explicitly imported or inspected.

Conflict rule:

1. projection and root bootstrap files never outrank DB truth for execution,
2. embedded baseline and DB truth outrank all projections and bridge files.

## 15. Fail-Close Conditions

Bootstrap or projection flows must fail closed when:

1. embedded artifacts are missing or incompatible,
2. `.vida/**` runtime home cannot be created or validated,
3. DB truth cannot be seeded or migrated,
4. projection metadata is missing or incompatible,
5. the runtime cannot determine whether DB truth or a projection is the lawful active source.

Allowed fallback:

1. bounded remediation, re-init, re-import, or activator routing.

Forbidden fallback:

1. silently downgrade to broad root-file execution.

## 16. Release-1 Practical Recommendation

The strongest practical Release-1 recommendation is:

1. keep packaged runtime binary-first,
2. keep `AGENTS.md` and `AGENTS.sidecar.md` in the root as bootstrap carriers,
3. let `init` own `.vida/**` creation and DB/bootstrap cutover,
4. let `project-activator` own onboarding enrichment and project-facing import preparation,
5. keep exported config/activation files strictly as projections with explicit import cutover,
6. keep installed runtime executable even when loose framework source files are absent.

## 17. Open Questions

The remaining bounded open questions are:

1. the exact command family for export/import of each projection type,
2. the exact packaged asset split once the runtime moves further away from donor layouts,
3. how much project-facing operator documentation should be auto-generated by activator versus manually authored later,
4. whether protocol projections belong in Release 1 or immediately after it.

## 18. Recommended Next Follow-Up

The strongest next bounded follow-up after this research is:

1. `protocol-admission-and-runtime-binding-research`

Reason:

1. bundle contract is framed,
2. DB authority is framed,
3. runtime-home migration is framed,
4. derived cache is framed,
5. embedded bootstrap/projection loop is framed,
6. the next unresolved seam is the exact admission path from known project protocols into executable runtime-bound protocol state.

-----
artifact_path: product/research/embedded-runtime-bootstrap-and-projection-research
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/research/embedded-runtime-bootstrap-and-projection-research.md
created_at: '2026-03-12T23:59:58+02:00'
updated_at: '2026-03-12T23:59:58+02:00'
changelog_ref: embedded-runtime-bootstrap-and-projection-research.changelog.jsonl
