# Product Control Lane

Purpose: provide the root entrypoint for DB-backed VIDA control-plane projections without making generated object projections compete with product/spec law.

This lane contains controlled object projections derived from the VIDA requirements and documentation control plane.

Rules:

1. DB state remains the operational authority for lifecycle, traceability, approvals, waivers, visibility, and validation verdicts.
2. Files in this lane are repository projections for human review, Git audit, and external adapter packaging.
3. Accepted decision records live under `docs/product/control/decisions/**`.
4. Future typed object folders are reserved for:
   - `origins`
   - `requirements`
   - `features`
   - `use-cases`
   - `changes`
   - `decisions`
   - `risks`
   - `baselines`
   - `gaps`
   - `approvals`
5. Generated projections must carry `projection_mode: generated` and `edit_authority: db` in their footer metadata.
6. This lane does not replace `docs/product/spec/**`; active product law remains in the spec lane.

Current entrypoints:

1. [decisions/enterprise-requirements-control-plane-decision-record.md](decisions/enterprise-requirements-control-plane-decision-record.md)

-----
artifact_path: product/control/index
artifact_type: product_control_doc
artifact_version: '1'
artifact_revision: '2026-06-18'
schema_version: '1'
status: canonical
source_path: docs/product/control/index.md
created_at: '2026-06-18T00:00:00+03:00'
updated_at: 2026-06-18T00:00:00+03:00
changelog_ref: index.changelog.jsonl
