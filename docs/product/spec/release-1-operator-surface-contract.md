# Release 1 Operator Surface Contract

Status: active Release-1 implementation law

Purpose: define the stable operator-facing contract for key `Release 1` CLI/runtime surfaces so output shape does not drift while internal refactors continue.

## 1. Scope

This contract covers:

1. `status`
2. `doctor`
3. `consume`
4. `lane`
5. `approval`
6. `recovery`

## 2. Shared Output Rule

All operator surfaces must expose machine-readable output with:

1. `surface`
2. `status`
3. `trace_id` when workflow-bound
4. `workflow_class` when applicable
5. `blocker_codes` when blocked
6. `artifact_refs`
7. `next_actions`

## 3. Surface Contracts

### 3.1 `status`

Must expose:

1. runtime identity
2. open delegated cycle summary
3. closure/gate summary
4. active blocker codes

### 3.2 `doctor`

Must expose:

1. check results
2. failed checks
3. blocker codes
4. remediation hints

### 3.3 `consume`

Must expose:

1. request classification
2. workflow class
3. packet or lane refs
4. downstream required surfaces

### 3.4 `lane`

Must expose:

1. lane id
2. lane role
3. lane status
4. evidence status
5. supersession or exception refs

### 3.5 `approval`

Must expose:

1. approval scope
2. approval status
3. decision reason
4. expiry state

### 3.6 `recovery`

Must expose:

1. incident id
2. recovery stage
3. rollback/restore posture
4. trust reevaluation verdict

## 4. Stability Rule

1. These surfaces may add fields in backward-compatible form.
2. They may not silently remove required shared fields during Release 1.

## 5. References

1. `docs/product/spec/release-1-runtime-enum-and-code-contracts.md`
2. `docs/product/spec/release-1-schema-versioning-and-compatibility-law.md`

-----
artifact_path: product/spec/release-1-operator-surface-contract
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-operator-surface-contract.md
created_at: 2026-03-16T11:45:00Z
updated_at: 2026-03-16T11:34:32.236698562Z
changelog_ref: release-1-operator-surface-contract.changelog.jsonl
