# VIDA Canonical Relation Law

Status: active product law

Purpose: define the canonical relation model for VIDA documentation and inventory so dependency, linkage, and impact analysis can be performed from canonical artifacts without depending on runtime-readiness or runtime-consumption layers.

## 1. Scope

This spec defines:

1. canonical relation surfaces,
2. canonical edge taxonomy,
3. direct and reverse relation semantics,
4. artifact impact semantics,
5. task impact semantics,
6. relation-safe validation expectations.

This spec does not define:

1. runtime-readiness verdicts,
2. bundle compatibility decisions,
3. runtime-owned consumption behavior.

## 2. Canonical Relation Purpose

The canonical relation layer exists to answer:

1. which canonical artifacts point to which other artifacts,
2. which artifacts are referenced by a given artifact,
3. which documentation radius is affected by one artifact change,
4. which documentation radius is affected by one task-scoped change history.

## 3. Canonical Relation Surfaces

The relation layer must work from canonical artifacts only.

The current canonical relation surfaces are:

1. markdown links,
2. footer reference fields,
3. canonical `artifact_path` identity,
4. sidecar task history for task-scoped impact,
5. canonical inventory paths and registry-visible artifact identities.

## 4. Canonical Edge Taxonomy

The minimal canonical edge taxonomy is:

1. `markdown_link`
2. `footer_ref`
3. `artifact_identity`
4. `reverse_reference`
5. `task_touch`

## 5. Direct Relation Rule

Direct relation views must expose:

1. direct markdown links,
2. direct footer references,
3. reverse references to the current artifact,
4. relation outputs without requiring runtime interpretation.

## 6. Artifact Impact Rule

Artifact impact is the set of direct documentation artifacts that reference or depend on one canonical artifact identity.

Rules:

1. artifact impact must be derived from canonical relation surfaces only,
2. impact must be inspectable without runtime execution,
3. impact must distinguish direct references from future indirect or runtime-derived consequences,
4. relation tooling may enrich presentation, but must not silently invent impact edges.

## 7. Task Impact Rule

Task impact is the set of documentation artifacts indirectly implicated by the artifacts touched by one task id.

Rules:

1. task impact begins from task-scoped sidecar history,
2. task impact may expand through canonical artifact relations,
3. task impact must remain documentation-scoped in this layer,
4. task impact must not be used as a readiness or runtime-consumption verdict.

## 8. Validation Rule For Relations

The relation layer must support bounded validation for relation surfaces.

At minimum:

1. broken markdown links are validation defects,
2. broken footer references are validation defects,
3. relation outputs must remain derivable from canonical artifacts and sidecar history,
4. relation validation must not require runtime boot or runtime-readiness logic.

## 9. Completion Proof For Layer 5

Layer 5 is considered closed when all of the following are true:

1. relation semantics are defined in one promoted canonical spec,
2. edge taxonomy is explicit,
3. direct relation views are canonical,
4. artifact impact is canonical,
5. task impact is canonical,
6. relation validation requirements are explicit and bounded.

## 10. Standalone Value

This layer gives VIDA a canonical documentation dependency and impact system without requiring readiness or runtime consumption.

-----
artifact_path: product/spec/canonical-relation-law
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/canonical-relation-law.md
created_at: '2026-03-10T04:45:00+02:00'
updated_at: '2026-03-10T04:45:00+02:00'
changelog_ref: canonical-relation-law.changelog.jsonl
