# Current Project Root Map

Purpose: provide the default project-document root map for the active `vida-stack` repository without collapsing the active current-project surface and extracted secondary bundles.

## Scope

This map is the default current-project map for:

1. `vida-stack`
2. active project docs under `docs/**`
3. current repository project/process/memory orientation

This map does not cover extracted secondary bundles by default.

## Canonical Entry Points

1. `docs/product/index.md`
   - current product/documentation index for the active project
2. `docs/product/spec/current-spec-map.md`
   - current product spec canon
3. `docs/product/spec/current-spec-provenance-map.md`
   - detailed source-lineage companion for the active product spec canon
4. `docs/process/README.md`
   - project process lane
5. `docs/project-memory/README.md`
   - project-memory lane
6. `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`
   - documentation/product alignment matrix
7. `docs/product/spec/canonical-runtime-layer-matrix.md`
   - runtime capability layering matrix for the active project canon
8. `docs/product/spec/repository-two-project-surface-model.md`
   - active-project vs extracted-bundle boundary law
9. `docs/process/documentation-tooling-map.md`
   - project-owned documentation tooling map
10. `docs/process/agent-extensions/README.md`
   - project-owned agent role/skill/profile/flow extension map

## Activation Triggers

Read this map when:

1. the task needs active current-project understanding,
2. the task needs project/product docs, process docs, or project-memory routing,
3. bootstrap has entered the project-doc layer from `AGENTS.sidecar.md`,
4. the task must stay inside the active `vida-stack` project rather than an extracted secondary bundle.

Do not use this map as the default route for extracted bundles unless the task explicitly targets one.

## Task Routing

1. Product/spec questions:
   - continue to `docs/product/index.md`
   - then to `docs/product/spec/current-spec-map.md`
2. Product/spec provenance or absorbed-history questions:
   - continue to `docs/product/spec/current-spec-provenance-map.md`
3. Documentation alignment / documentation-state questions:
   - continue to `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`
4. Runtime layering / runtime architecture / runtime readiness questions:
   - continue to `docs/product/spec/canonical-runtime-layer-matrix.md`
5. Process/runbook questions for the active project:
   - continue to `docs/process/README.md`
6. Project-memory questions:
   - continue to `docs/project-memory/README.md`
7. Extracted secondary bundle questions:
   - do not continue through active project docs by default
   - enter the named bundle directly under `projects/<name>/**`
8. Documentation tooling / operator-command questions:
   - continue to `docs/process/documentation-tooling-map.md`
9. Project agent-system extension questions:
   - continue to `docs/process/agent-extensions/README.md`

## Boundary Rule

1. `docs/**` remains the active current-project documentation surface for `vida-stack`.
2. `projects/vida-mobile/**` is a preserved extracted bundle, not the active current-project surface.
3. Root `vida.config.yaml` remains the active runtime/config surface for this repository, but it is not itself the project-doc map.

-----
artifact_path: project/root-map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/project-root-map.md
created_at: '2026-03-10T08:45:00+02:00'
updated_at: 2026-03-16T10:05:31.520957193Z
changelog_ref: project-root-map.changelog.jsonl
