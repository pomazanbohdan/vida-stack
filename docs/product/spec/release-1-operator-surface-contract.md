# Release 1 Operator Surface Contract

Status: active Release-1 implementation law

Purpose: define the stable root operator-facing contract for key `Release 1` CLI/runtime surfaces so output shape does not drift while internal refactors continue.

## 1. Scope

This contract covers:

1. `vida status`
2. `vida doctor`
3. `vida consume`
4. `vida lane`
5. `vida approval`
6. `vida recovery`

Rule:

1. these are the canonical root operator surfaces for Release 1,
2. nested compatibility paths may exist temporarily,
3. nested compatibility paths must not replace the root contract as owner law.

TaskFlow diagnostic/query adjunct rule:

1. Release-1 shared-envelope parity also applies to promoted TaskFlow diagnostic adjuncts that operators use as proof surfaces,
2. current promoted adjuncts are `vida taskflow config-actuation census` and `vida taskflow artifacts {list,show}`,
3. those adjuncts may remain under the `taskflow` family, but their JSON must still expose `status`, `blocker_codes`, `next_actions`, `artifact_refs`, `shared_fields`, and `operator_contracts`.

## 2. Shared Envelope Rule

All operator surfaces must expose machine-readable output with the shared envelope:

1. `surface`
2. `status`
3. `trace_id`
4. `workflow_class`
5. `risk_tier`
6. `artifact_refs`
7. `next_actions`
8. `blocker_codes`

Envelope rule:

1. `surface` must report the canonical root surface id even when the request reached the runtime through a compatibility alias,
2. `blocker_codes` must always be present as an array, even when empty,
3. `next_actions` must always be present as an array, even when empty,
4. `artifact_refs` must always be present as a machine-readable collection, even when empty,
5. `trace_id` may be `null` when no bounded workflow trace exists for the current surface instance,
6. workflow-bound fields may be `null` when not applicable, but they must not silently disappear from one surface while remaining required on another.

## 3. Root Surface Rule

Release 1 must expose the root surfaces above directly on `vida`.

Temporary compatibility rule:

1. `vida taskflow consume` may exist as a compatibility alias behind the canonical root `vida consume` surface,
2. `vida taskflow recovery` may exist as a compatibility alias behind the canonical root `vida recovery` surface,
3. `vida lane` and `vida approval` may currently exist as explicit fail-closed reserved root surfaces before their family-owned implementations are promoted,
4. unsupported root surfaces must fail closed with an explicit machine-readable unsupported envelope rather than being omitted from the operator model.

## 4. Surface Contracts

### 4.1 `status`

Must expose:

1. runtime identity
2. open delegated cycle summary
3. closure/gate summary
4. active blocker codes
5. activation truth posture
6. checkpoint/recovery summary
7. selected host/carrier-system posture

### 4.2 `doctor`

Must expose:

1. check results
2. failed checks
3. blocker codes
4. remediation hints
5. compatibility posture
6. operator-contract conformance status

### 4.3 `consume`

Must expose:

1. request classification
2. workflow class
3. risk tier
4. packet or lane refs
5. runtime/carrier assignment
6. downstream required surfaces
7. closure-relevant proof surfaces when the request enters a closure path

### 4.4 `lane`

Must expose:

1. lane id
2. lane role or runtime role
3. lane status
4. selected carrier/backend id
5. evidence status
6. supersession or exception refs
7. receipt refs when the lane is blocked, completed, superseded, or taken over

### 4.5 `approval`

Must expose:

1. approval scope
2. approval status
3. gate level
4. decision reason
5. expiry state
6. approval evidence refs

### 4.6 `recovery`

Must expose:

1. run id or incident id
2. recovery stage
3. checkpoint or replay lineage refs
4. rollback/restore posture
5. trust reevaluation verdict
6. recovery blockers and remediation path

### 4.7 TaskFlow Diagnostic Adjuncts

`config-actuation census` must expose:

1. routed lane coverage,
2. config key,
3. validator,
4. runtime consumer,
5. proof status,
6. model-profile readiness audit when model selection is in scope.

`artifacts list/show` must expose:

1. execution-preparation artifact registry entries,
2. required/missing/materialized posture,
3. source runtime-consumption snapshot pointer,
4. source snapshot operator-contract evidence,
5. fail-closed Release-1 blocker codes when query evidence is unavailable or the requested artifact id is unknown.

## 5. Unsupported And Missing Surface Rule

1. if a surface is invoked before it is fully implemented, the runtime must respond with one explicit fail-closed unsupported envelope,
2. the unsupported envelope must still honor the shared envelope fields,
3. silent fallback into another surface is forbidden,
4. shell-local embedded summaries do not satisfy the root operator-surface contract for `lane`, `approval`, or `recovery`.

## 6. Stability Rule

1. These surfaces may add fields in backward-compatible form.
2. They may not silently remove required shared fields during Release 1.
3. Compatibility aliases may remain temporarily, but the root surface ids above remain canonical.
4. Tests and golden fixtures must assert canonical root contracts rather than compatibility-only nested paths.

## 7. References

1. `docs/product/spec/release-1-runtime-enum-and-code-contracts.md`
2. `docs/product/spec/release-1-schema-versioning-and-compatibility-law.md`
3. `docs/product/spec/release-1-unsupported-surface-contract.md`
4. `docs/product/spec/release-1-state-machine-specs.md`

-----
artifact_path: product/spec/release-1-operator-surface-contract
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-08
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-operator-surface-contract.md
created_at: 2026-03-16T11:45:00Z
updated_at: 2026-04-26T14:58:34.964502972Z
changelog_ref: release-1-operator-surface-contract.changelog.jsonl
