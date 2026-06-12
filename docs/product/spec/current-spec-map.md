# VIDA Current Spec Map

Status: active canonical map
Revision: 2026-06-12

Purpose: provide the short routing map for the active current product/spec canon after the detailed catalog was split into a companion document.

Companion rule:

1. Use this map first for product/spec routing.
2. Use [current-spec-catalog.md](current-spec-catalog.md) for the detailed active artifact catalog and config-family notes.
3. Use [current-spec-provenance-map.md](current-spec-provenance-map.md) for source lineage, absorption history, and historical promotion context.
4. Do not expand this map back into a full catalog; register detailed entries in the catalog companion and keep the owning artifact docs authoritative.

## Canonical Entry Points

1. [docs/product/index.md](../index.md)
   - top-level product canon index for the active repository
2. [docs/product/spec/README.md](README.md)
   - spec-lane orientation and local product/spec home
3. [current-spec-catalog.md](current-spec-catalog.md)
   - detailed active product/spec artifact catalog
4. [current-spec-provenance-map.md](current-spec-provenance-map.md)
   - provenance and absorbed-history companion for the active canon
5. [project-documentation-law.md](project-documentation-law.md)
   - project documentation ownership and canonical state law
6. [project-document-naming-law.md](project-document-naming-law.md)
   - project-owned docs naming grammar and owner-directory terminal role rules
7. [canonical-documentation-and-inventory-layer-matrix.md](canonical-documentation-and-inventory-layer-matrix.md)
   - documentation/product alignment matrix
8. [canonical-runtime-layer-matrix.md](canonical-runtime-layer-matrix.md)
   - runtime capability layering matrix
9. [canonical-runtime-readiness-law.md](canonical-runtime-readiness-law.md)
   - runtime readiness gate law used by the project
10. [agent-role-skill-profile-flow-model.md](agent-role-skill-profile-flow-model.md)
   - role, carrier, profile, and flow model
11. [release-1-plan.md](release-1-plan.md)
    - Release 1 planning entrypoint
12. [release-1-current-state.md](release-1-current-state.md)
    - current Release 1 state entrypoint
13. [release-1-capability-matrix.md](release-1-capability-matrix.md)
    - Release 1 capability matrix
14. [release-1-closure-contract.md](release-1-closure-contract.md)
    - Release 1 closure and completion contract
15. [release-build-packaging-law.md](release-build-packaging-law.md)
    - release build and packaging ownership law
16. [taskflow-protocol-runtime-binding-model.md](taskflow-protocol-runtime-binding-model.md)
    - TaskFlow/runtime binding model
17. [compiled-autonomous-delivery-runtime-architecture.md](compiled-autonomous-delivery-runtime-architecture.md)
    - compiled autonomous delivery runtime architecture
18. [bootstrap-carriers-and-project-activator-model.md](bootstrap-carriers-and-project-activator-model.md)
    - bootstrap carrier and project activator model
19. [docs/process/documentation-tooling-map.md](../../process/documentation-tooling-map.md)
    - DocFlow and documentation operator-command map

## Detailed Catalog Companions

1. [current-spec-catalog.md](current-spec-catalog.md)
   - active current canon list, grouped by product/spec area
2. [current-spec-provenance-map.md](current-spec-provenance-map.md)
   - historical source-lineage and promotion context
3. [docs/product/index.md](../index.md)
   - repository-wide product/process/research index
4. [docs/project-root-map.md](../../project-root-map.md)
   - active project root map that routes into this spec map
5. [AGENTS.sidecar.md](../../../AGENTS.sidecar.md)
   - bootstrap-visible project documentation map

## Routing Pointers

1. Documentation ownership, naming, and inventory questions route to [project-documentation-law.md](project-documentation-law.md), [project-document-naming-law.md](project-document-naming-law.md), [canonical-documentation-and-inventory-layer-matrix.md](canonical-documentation-and-inventory-layer-matrix.md), and [docs/process/documentation-tooling-map.md](../../process/documentation-tooling-map.md).
2. Runtime readiness, runtime layering, and operator-surface questions route to [canonical-runtime-readiness-law.md](canonical-runtime-readiness-law.md), [canonical-runtime-layer-matrix.md](canonical-runtime-layer-matrix.md), and the Release 1 contract family.
3. Role, carrier, skill, profile, lane, and flow questions route to [agent-role-skill-profile-flow-model.md](agent-role-skill-profile-flow-model.md) and [docs/process/agent-extensions/README.md](../../process/agent-extensions/README.md).
4. Detailed artifact lookup routes to [current-spec-catalog.md](current-spec-catalog.md); provenance lookup routes to [current-spec-provenance-map.md](current-spec-provenance-map.md).

## Current Rule

1. The current canon is the active product/spec state for the repository.
2. Historical framework-formation evidence is provenance only unless re-promoted into a current canonical owner.
3. New current artifacts must be registered in the owning index/map and, when detailed lookup is needed, in [current-spec-catalog.md](current-spec-catalog.md).
4. Do not use extracted secondary bundles as the default current project surface unless the task explicitly targets them.

## Shared Runtime Spine Rule

1. Shared runtime helpers and executable projections live in [vida/config](../../../vida/config) and the runtime crates.
2. Product/spec docs describe the canonical contract, while executable runtime surfaces prove and enforce it.
3. When docs and executable runtime surfaces disagree, record the disagreement as a bounded runtime/documentation defect instead of silently treating either surface as disposable.

## Project Documentation Rule

1. Project-visible documentation surfaces must stay discoverable from [AGENTS.sidecar.md](../../../AGENTS.sidecar.md), [docs/project-root-map.md](../../project-root-map.md), and [docs/product/index.md](../index.md) when they become bootstrap or product-spec entrypoints.
2. Companion documents may reduce map size, but they must not hide active canonical docs from the owning maps.
3. Changelog JSONL files record map/catalog routing changes as a documentation-system mutation, not as runtime behavior proof.

-----
artifact_path: product/spec/current-spec-map
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-06-12
schema_version: '1'
status: canonical
source_path: docs/product/spec/current-spec-map.md
created_at: '2026-03-10T10:20:00+02:00'
updated_at: 2026-06-12T00:00:00+03:00
changelog_ref: current-spec-map.changelog.jsonl
