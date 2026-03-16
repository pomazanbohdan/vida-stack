# Release 1 Unsupported Surface Contract

Status: active Release-1 scope law

Purpose: define the explicit unsupported and architecture-reserved surfaces for `Release 1` so implementation does not widen support by accident.

## 1. Scope

This contract defines:

1. unsupported workflow classes,
2. architecture-reserved but not broadly supported surfaces,
3. required operator denial posture.

## 2. Unsupported Or Reserved Surface

Release 1 does not claim broad support for:

1. unbounded `R4` production workflows
2. unrestricted external `tool_assisted_write`
3. unrestricted `identity_or_policy_change`
4. unrestricted sensitive `memory_write`
5. unsupported workflow classes not listed in the workflow matrix

## 3. Denial Rule

When an unsupported surface is requested, runtime must:

1. classify it explicitly,
2. return the applicable blocker codes,
3. explain whether the surface is unsupported or only tightly bounded,
4. avoid silent fallback to a less safe mode.

## 4. References

1. `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
2. `docs/product/spec/release-1-closure-contract.md`

-----
artifact_path: product/spec/release-1-unsupported-surface-contract
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-unsupported-surface-contract.md
created_at: 2026-03-16T11:45:00Z
updated_at: 2026-03-16T11:34:32.249131088Z
changelog_ref: release-1-unsupported-surface-contract.changelog.jsonl
