# Release 1 Fixture And Golden Data Contract

Status: active Release-1 proof law

Purpose: define the minimum fixture, golden-data, and canonical example set required for `Release 1` proof, regression, and compatibility work.

## 1. Scope

This contract defines:

1. canonical fixture families,
2. golden scenario artifacts,
3. minimum sample datasets for proof and evaluation.

## 2. Required Fixture Families

1. workflow classification fixtures
2. lane lifecycle fixtures
3. approval lifecycle fixtures
4. tool contract fixtures
5. retrieval citation/freshness fixtures
6. incident and recovery fixtures
7. schema compatibility fixtures

## 3. Golden Scenario Rule

Each proof scenario in the proof catalog should have:

1. one happy-path golden fixture where applicable,
2. one negative/blocking fixture where applicable,
3. canonical expected artifacts,
4. canonical expected blocker codes when negative.

## 4. Compatibility Fixture Rule

Schema-evolution proofs must include:

1. backward-compatible fixture sample
2. reader-upgrade-required fixture sample
3. migration-required fixture sample

## 5. References

1. `docs/product/spec/release-1-proof-scenario-catalog.md`
2. `docs/product/spec/release-1-schema-versioning-and-compatibility-law.md`

-----
artifact_path: product/spec/release-1-fixture-and-golden-data-contract
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-fixture-and-golden-data-contract.md
created_at: 2026-03-16T11:45:00Z
updated_at: 2026-03-16T11:34:32.256232757Z
changelog_ref: release-1-fixture-and-golden-data-contract.changelog.jsonl
