# VIDA Current Spec Map

Status: active canonical map
Revision: 2026-06-21

Purpose: provide the short routing map for the active current product/spec canon after the detailed catalog was split into a companion document.

Companion rule:

1. Use this map first for product/spec routing.
2. Use [index.md](index.md) for local product/spec orientation.
3. Use [current-spec-catalog.md](current-spec-catalog.md) for the detailed active artifact catalog and config-family notes.
4. Do not expand this map back into a full catalog; register detailed entries in the catalog companion and keep the owning artifact docs authoritative.

## Canonical Entry Points

1. [docs/product/index.md](../index.md)
   - top-level product canon index for the active repository
2. [docs/product/spec/index.md](index.md)
   - spec-lane orientation and local product/spec home
3. [current-spec-catalog.md](current-spec-catalog.md)
   - detailed active product/spec artifact catalog
4. [project-documentation-law.md](project-documentation-law.md)
   - project documentation ownership and canonical state law
5. [project-document-naming-law.md](project-document-naming-law.md)
   - project-owned docs naming grammar and owner-directory terminal role rules
6. [canonical-documentation-and-inventory-layer-matrix.md](canonical-documentation-and-inventory-layer-matrix.md)
   - documentation/product alignment matrix
7. [canonical-runtime-layer-matrix.md](canonical-runtime-layer-matrix.md)
   - runtime capability layering matrix
8. [canonical-runtime-readiness-law.md](canonical-runtime-readiness-law.md)
   - runtime readiness gate law used by the project
9. [agent-role-skill-profile-flow-model.md](agent-role-skill-profile-flow-model.md)
   - role, carrier, profile, and flow model
10. [release-build-packaging-law.md](release-build-packaging-law.md)
    - release build and packaging ownership law
11. [taskflow-protocol-runtime-binding-model.md](taskflow-protocol-runtime-binding-model.md)
    - TaskFlow/runtime binding model
12. [compiled-autonomous-delivery-runtime-architecture.md](compiled-autonomous-delivery-runtime-architecture.md)
    - compiled autonomous delivery runtime architecture
13. [bootstrap-carriers-and-project-activator-model.md](bootstrap-carriers-and-project-activator-model.md)
    - bootstrap carrier and project activator model
14. [docs/process/documentation-tooling-map.md](../../process/documentation-tooling-map.md)
    - DocFlow and documentation operator-command map
15. [typed-transition-state-store-extraction-contract.md](typed-transition-state-store-extraction-contract.md)
    - typed transition/state-store extraction design contract for the active shared extraction epic

## Detailed Catalog Companions

1. [current-spec-catalog.md](current-spec-catalog.md)
   - active current canon list, grouped by product/spec area
2. [docs/product/index.md](../index.md)
   - repository-wide product/process/research index
3. [docs/project-root-map.md](../../project-root-map.md)
   - active project root map that routes into this spec map
4. [AGENTS.sidecar.md](../../../AGENTS.sidecar.md)
   - bootstrap-visible project documentation map

## Routing Pointers

1. Documentation ownership, naming, and inventory questions route to [project-documentation-law.md](project-documentation-law.md), [project-document-naming-law.md](project-document-naming-law.md), [canonical-documentation-and-inventory-layer-matrix.md](canonical-documentation-and-inventory-layer-matrix.md), and [docs/process/documentation-tooling-map.md](../../process/documentation-tooling-map.md).
2. Runtime readiness, runtime layering, and operator-surface questions route to [canonical-runtime-readiness-law.md](canonical-runtime-readiness-law.md), [canonical-runtime-layer-matrix.md](canonical-runtime-layer-matrix.md), and the active runtime contract/profile specs.
3. Role, carrier, skill, profile, lane, and flow questions route to [agent-role-skill-profile-flow-model.md](agent-role-skill-profile-flow-model.md) and [docs/process/agent-extensions/index.md](../../process/agent-extensions/index.md).
4. Detailed artifact lookup routes to [current-spec-catalog.md](current-spec-catalog.md).

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
artifact_revision: 2026-06-21
schema_version: '1'
status: canonical
source_path: docs/product/spec/current-spec-map.md
created_at: '2026-03-10T10:20:00+02:00'
updated_at: 2026-06-13T00:00:00+03:00
changelog_ref: current-spec-map.changelog.jsonl
