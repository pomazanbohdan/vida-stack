# Product Wiki Lane

Purpose: provide the root entrypoint for generated VIDA project wiki projections without making reader-facing wiki pages canonical product law.

This lane contains generated wiki views built from DB state, approved control projections, product/spec law, and validated documentation inputs.

Rules:

1. `docs/product/wiki/internal/**` is the internal delivery wiki view for product, BA, engineering, and agents.
2. `docs/product/wiki/public/**` is the public reader-facing wiki view.
3. Public wiki content requires explicit visibility approval before projection.
4. Generated delivery status from TaskFlow, PR, proof, and release state is internal-only in v1.
5. Wiki files are generated-only; edits must go through DB-backed web, CLI, CR, or reconcile flows.
6. Generated wiki pages must carry `projection_mode: generated` and `edit_authority: db` in footer metadata.
7. GitHub Wiki or Pages is the first external mirror target; Confluence and Azure Wiki are adapter backlog targets.

Current state:

1. this lane root is canonical,
2. generated `internal/**` and `public/**` page trees are reserved for the runtime projection implementation,
3. no generated wiki pages are materialized by this documentation slice.

-----
artifact_path: product/wiki/index
artifact_type: product_wiki_doc
artifact_version: '1'
artifact_revision: '2026-06-18'
schema_version: '1'
status: canonical
source_path: docs/product/wiki/index.md
created_at: '2026-06-18T00:00:00+03:00'
updated_at: 2026-06-18T00:00:00+03:00
changelog_ref: index.changelog.jsonl
