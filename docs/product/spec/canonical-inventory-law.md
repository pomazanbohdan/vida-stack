# VIDA Canonical Inventory Law

Status: active product law

Purpose: define the canonical inventory model for VIDA so the active canon can be discovered, materialized, validated, and consumed without relying on ad hoc filesystem assumptions.

## 1. Scope

This spec defines:

1. the canonical inventory surface,
2. the canonical registry structure,
3. inventory coverage rules,
4. source and projection linkage rules,
5. version-tuple visibility rules,
6. the canonical registry write path.

This spec does not define:

1. dependency or impact semantics,
2. runtime-readiness verdict semantics,
3. direct runtime consumption behavior.

Those belong to later layers.

## 2. Canonical Inventory Purpose

The canonical inventory exists to provide one authoritative map of active VIDA artifacts across:

1. canonical markdown authoring artifacts,
2. executable product-law artifacts under `vida/config/**`,
3. canonical registries and manifests that describe the instruction and projection surfaces.

The inventory must be sufficient to answer:

1. what active canonical artifacts exist,
2. where their authority lives,
3. which artifact family each artifact belongs to,
4. which version tuple is currently active,
5. which source and projection surfaces are linked canonically.

## 3. Active Inventory Scope

The active canonical inventory must cover all active artifact families that participate in the current canon.

The minimum scope is:

1. root repository canonical markdown documents,
2. `docs/product/spec/**`,
3. `docs/product/index.md`,
4. `docs/product/research/**` when active,
5. `docs/process/**`,
6. `docs/project-memory/**`,
7. `vida/config/instructions/*.md`,
8. `vida/config/instructions/*.yaml`,
9. active machine-readable product-law families under `vida/config/**`.

Framework plans and research remain active framework inputs and must be inventory-visible when they are active canonical inputs, but they do not become product canon merely by being inventoried.

## 4. Canonical Registry Structure

The canonical registry is the materialized inventory artifact for the active canon.

Each canonical registry row must expose enough information to identify and classify one active artifact.

Required canonical inventory fields:

1. `artifact_path`
2. `artifact_type`
3. `artifact_version`
4. `artifact_revision`
5. `status`
6. `source_path`
7. `changelog_ref` when the artifact is markdown-canonical
8. `compatibility_class` when the artifact family supports compatibility semantics

Additional fields may be present, but they must not weaken or contradict the canonical fields above.

## 5. Canonical Registry Write Path

The canonical registry write path for the current architecture is:

1. `vida/config/codex-registry.current.jsonl`

Rules:

1. this path is canonical for the current transitional architecture,
2. registry materialization must be deterministic,
3. writing the canonical registry must not create a second competing registry authority,
4. if future runtime-owned inventory replaces this path, that replacement must be promoted through product law before becoming authoritative.

## 6. Coverage Rule

Coverage must be explainable from the active canonical tree plus the canonical inventory rules in this spec.

Coverage requirements:

1. all active markdown-canonical artifacts must be inventory-visible,
2. all active machine-readable law families that belong to the active canon must be inventory-visible,
3. instruction inventory artifacts such as `instruction_catalog` and `projection_manifest` must be inventory-visible,
4. inventory coverage must not depend on hidden heuristics,
5. inventory omission of an active canonical family is a defect.

## 7. Markdown Authoring Artifact Rule

For markdown-canonical artifacts:

1. the latest active markdown revision is the canonical body,
2. footer metadata is part of inventory authority,
3. sibling `*.changelog.jsonl` is part of lineage authority,
4. historical markdown duplicates are forbidden in the active tree.

Inventory must therefore expose the markdown artifact as an active canonical object rather than treating the filesystem path alone as sufficient identity.

## 8. Machine-Readable Artifact Rule

For machine-readable law under `vida/config/**`:

1. inventory must expose the artifact even when no markdown sidecar exists,
2. inventory must preserve the artifact family and canonical config home,
3. machine-readable law must not be omitted simply because it is not markdown-first,
4. machine-readable inventory must remain distinct from binary packaging contents.

## 9. Source and Projection Linkage Rule

Where canonical source or projection relationships are defined, the inventory must preserve those relationships as inventory-visible facts.

This includes:

1. authoring markdown to projection-manifest linkage,
2. authoring markdown to machine-readable projection linkage where canonically defined,
3. canonical source references for instruction artifacts and instruction families,
4. default bundle and activation-policy linkage where canonically defined.

Inventory may expose these relationships directly or through linked canonical registry and manifest artifacts, but the relationships must remain inspectable without ad hoc inference.

## 10. Version Tuple Visibility Rule

The inventory must preserve active version-tuple visibility for all active artifact families that participate in versioned canon.

Rules:

1. active version tuple means at minimum `artifact_version` plus `artifact_revision`,
2. unresolved version tuple behavior is blocking where canonical law says so,
3. inventory must expose version tuples rather than forcing consumers to infer them from filenames or timestamps,
4. inventory must not silently collapse two distinct active tuples into one row.

## 11. Authority Rule

The canonical inventory is a map of authority, not a second source of competing law.

Therefore:

1. inventory reflects canonical docs and canonical config,
2. inventory must not redefine the meaning of an artifact outside its canonical source,
3. if registry output conflicts with product law or canonical metadata, the law wins and the inventory must be corrected.

## 12. Completion Proof For Layer 2

Layer 2 is considered closed when all of the following are true:

1. the active canon has one documented canonical registry structure,
2. the active canon has one documented canonical registry write path,
3. coverage rules are explicit for markdown and machine-readable active families,
4. source/projection linkage expectations are explicit,
5. version-tuple visibility is explicit,
6. no additional undocumented inventory authority is required to explain the active canon.

## 13. Standalone Value

This layer gives VIDA one authoritative inventory map of the active canon without requiring later relation, readiness, or runtime-consumption layers.

-----
artifact_path: product/spec/canonical-inventory-law
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/canonical-inventory-law.md
created_at: '2026-03-10T04:25:00+02:00'
updated_at: '2026-03-10T04:25:00+02:00'
changelog_ref: canonical-inventory-law.changelog.jsonl
