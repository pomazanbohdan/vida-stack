# VIDA Project Documentation Law

Status: active canonical rule

Purpose: define how project-owned documentation is organized, versioned, and made operational for systematic work before the full instruction/runtime store owns latest resolution automatically.

## Documentation Layers

1. Repository product narrative lives in root documents such as `README.md`, `CONTRIBUTING.md`, and `VERSION-PLAN.md`.
2. Product prose canon lives in `docs/product/spec/**`.
3. Product map and entrypoint docs live in `docs/product/index.md`.
4. Product research staging lives in `docs/product/research/**`.
5. Project process docs live in `docs/process/**`.
6. Project-memory source docs live in `docs/project-memory/**`.

## Related Owner Documents

Detailed owner surfaces derived from this system rule:

1. `docs/product/spec/project-document-naming-law.md`
   - canonical naming law for project-owned `docs/**` artifacts
2. `docs/product/spec/github-public-repository-law.md`
   - canonical GitHub-native public repository and community-surface law

## Current Operating Rule

1. Active project documentation is markdown-first.
2. Only the latest active revision should exist as the canonical markdown body.
3. Historical revisions should not remain as parallel active markdown copies.
4. Historical change evidence belongs in sidecar changelog files and git history.
5. Active canonical documents are not assumed to be final unless a stricter rule says so.
6. Current project/product/framework documentation is expected to keep evolving during work on VIDA `0.2.0` and VIDA `1.0.0`.
7. This in-progress maturity state belongs in policy/spec documents, not as repetitive status markers in every sidecar changelog.

## Required Metadata Footer

Each active canonical markdown document must end with a YAML footer after `-----`.

Required fields:

1. `artifact_path`
2. `artifact_type`
3. `artifact_version`
4. `artifact_revision`
5. `schema_version`
6. `status`
7. `source_path`
8. `created_at`
9. `updated_at`
10. `changelog_ref`

## Sidecar Changelog Rule

1. Each active canonical markdown document must have a sibling `*.changelog.jsonl`.
2. The sidecar records latest known initialization and revision events for the canonical artifact.
3. During the markdown-first phase, the sidecar is the lightweight operational history store.
4. When future runtimes own full document lineage, sidecar contents may be projected into runtime storage, but the canonical markdown footer contract must remain machine-readable.

## Artifact Classes

1. `repository_doc` for root narrative/governance files.
2. `product_index` for product entrypoint maps.
3. `product_spec` for promoted product law.
4. `product_research_doc` for promoted research staging.
5. `process_doc` for project process/runbook docs.
6. `project_memory_doc` for project-memory source docs.

## Promotion Rule

1. If a document defines stable product law, it belongs in `docs/product/spec/**`.
2. If a document is project-specific operating process, it belongs in `docs/process/**`.
3. If a document captures project-memory source material, it belongs in `docs/project-memory/**`.
4. If a document is only narrative or contribution framing for the repo, it remains at root.

## Documentation Standard Precedence Rule

When an active documentation task writes or reshapes an artifact body, formatting authority is resolved in this order:

1. a currently active skill-specific artifact format when the skill explicitly governs that artifact family,
2. an explicit project-owned documentation standard for that artifact family,
3. promoted product-law requirements for canonical documentation and instruction artifacts,
4. transitional `DocFlow` fallback behavior.

Rules:

1. `DocFlow` is not the authority for project-specific artifact shape when a higher-precedence standard already exists.
2. A skill-owned artifact format may refine the body structure of a document, but it must not remove canonical metadata/footer/sidecar/validation requirements.
3. If no higher-precedence artifact standard exists, `DocFlow` fallback behavior is allowed.

## Deduplication Rule

1. Active canonical law must not live in multiple parallel documents without an explicit primary owner.
2. Maps, summaries, and reference surfaces may compress or point to canonical law, but they must not become duplicate law-bearing sources.
3. If documentation work reveals duplicated active law, the bounded change should reduce that duplication when safe and scope-bounded.

## Runtime Transition Rule

1. Before VIDA `0.2.0` and VIDA `1.0` fully own latest document resolution, humans and transitional tooling read the canonical markdown files directly.
2. The runtime-facing future model should resolve latest documentation from structured metadata and sidecar lineage, not from filename chronology.
3. This rule allows the current repo to stay operational without introducing legacy duplicate files.
4. Root-level repository markdown files may temporarily use a bootstrap exception path where missing footer metadata is tolerated and sidecar changelog handling remains available.
5. This root-level metadata exception must be carried through one canonical policy layer, not repeated as ad hoc hardcoded exceptions.
6. This root-level metadata exception is transitional only and must be removed by VIDA `1.0.0`.
7. By VIDA `1.0.0`, root-level markdown files must obey the same canonical metadata contract as the rest of the active documentation surface unless a stricter replacement bootstrap mechanism supersedes them.
8. Concrete operator-command discovery for documentation work must route through `docs/process/documentation-tooling-map.md`, not through bootstrap carriers or mixed product-spec bodies.

## Initialization Rule

1. Project initialization must begin with an automatic latest-state read of active project documentation and instruction canon before broad manual inspection.
2. The canonical documentation system must provide:
   - one-command overview reads for current document health and totals,
   - one-layer bounded doctor and proof paths when the work is constrained to one canonical layer,
   - current catalog/status summaries,
   - materialized registry snapshots when downstream automation needs a frozen inventory,
   - a canonical registry write path when automation requires one shared latest inventory file,
   - a canonical readiness write path when downstream automation needs one shared latest readiness report,
   - per-artifact history reads,
   - task-scoped change aggregation,
   - artifact-impact and task-impact tracing for change radius analysis,
   - canonical protocol-index and activation coverage checks for protocol-bearing artifacts,
   - bounded readiness checks for tuples, projections, bundles, compatibility classes, and boot-gate artifact presence,
   - one grouped proof command that can close the active documentation work cycle with bounded checks,
   - link inventories and lawful link migration,
   - consistency checks over metadata, sidecars, and references.
3. Tool names may evolve, but this initialization capability is mandatory for the active architecture.

-----
artifact_path: product/spec/project-documentation-law
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/project-documentation-law.md
created_at: '2026-03-10T00:00:00+02:00'
updated_at: '2026-03-12T07:58:34+02:00'
changelog_ref: project-documentation-law.changelog.jsonl
