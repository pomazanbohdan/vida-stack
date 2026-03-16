# Release 1 Schema Versioning And Compatibility Law

Status: active Release-1 contract law

Purpose: define how Release-1 canonical artifacts evolve so schema changes do not silently break runtime families, proofs, or release evidence.

## 1. Scope

This law defines:

1. schema-version rules,
2. compatible vs breaking changes,
3. mixed-version runtime expectations,
4. migration obligations for canonical artifacts.

## 2. Versioning Rule

1. Every canonical artifact schema must carry `schema_version`.
2. Additive optional fields are non-breaking.
3. Removing or renaming required fields is breaking.
4. Changing required field semantics is breaking even if the name stays the same.

## 3. Compatibility Classes

1. `backward_compatible`
   - older readers can safely ignore new optional fields
2. `reader_upgrade_required`
   - required semantics change or new required field added
3. `migration_required`
   - stored artifacts must be migrated before release closure

## 4. Mixed-Version Rule

Release 1 may tolerate mixed versions only when:

1. the active release candidate documents the supported version set,
2. all required readers for closure evidence can interpret the active artifact set,
3. no mixed-version state hides or drops required fields.

## 5. Migration Rule

When a breaking schema change occurs:

1. the owner doc must be updated,
2. the compatibility class must be declared,
3. migration path or blocker posture must be explicit,
4. proof scenarios must include the changed schema where relevant.

## 6. References

1. `docs/product/spec/release-1-canonical-artifact-schemas.md`
2. `docs/product/spec/release-1-closure-contract.md`
3. `docs/product/spec/release-1-proof-scenario-catalog.md`

-----
artifact_path: product/spec/release-1-schema-versioning-and-compatibility-law
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-schema-versioning-and-compatibility-law.md
created_at: 2026-03-16T11:35:00Z
updated_at: 2026-03-16T11:28:19.810983419Z
changelog_ref: release-1-schema-versioning-and-compatibility-law.changelog.jsonl
