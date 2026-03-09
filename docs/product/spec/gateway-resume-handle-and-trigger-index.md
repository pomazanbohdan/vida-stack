# VIDA Gateway Resume Handle And Trigger Index

Status: draft `v1` bounded runtime artifact

Revision: `2026-03-09`

Purpose: define the lawful future shape for resumable gateway handles and indexed trigger matching without promoting vendor bookmark models into root product law.

## 1. Scope

This artifact defines:

1. gateway resume handles,
2. correlation-based resume targeting,
3. trigger index semantics,
4. one-time resume consumption boundaries,
5. distributed resume safety expectations.

It does not define:

1. a vendor bookmark entity model,
2. transport-specific webhook or token middleware,
3. a permanent broad-scan resume fallback,
4. a new canonical machine beyond the current kernel.

## 2. Core Boundary

### 2.1 Gateway Resume Handle

A gateway resume handle is a runtime-owned resumability artifact that identifies a lawful external or human continuation point.

Minimum fields:

1. `handle_id`
2. `gateway_kind`
3. `task_id`
4. `route_ref`
5. `correlation_key` when applicable
6. `resume_target`
7. `consumption_policy`
8. `created_at`

Rule:

1. a gateway handle is not canonical state,
2. a gateway handle is not a receipt,
3. a gateway handle may be referenced by receipts, checkpoints, or projections.

### 2.2 Trigger Index

A trigger index is a runtime-owned lookup surface that maps a normalized trigger key to eligible gateway handles or start/resume candidates.

Rule:

1. trigger matching should prefer indexed lookup over broad scan,
2. trigger indexes are derived/runtime-owned, not product-law entities,
3. index keys must be deterministic across nodes.

### 2.3 Correlation Targeting

Correlation targeting means resume requests should identify the intended continuation point using deterministic data such as:

1. handle id,
2. correlation key,
3. trigger key,
4. exact bookmark-equivalent hash.

Rule:

1. correlation targeting is preferred over runtime-wide search,
2. correlation material must remain stable across resume attempts,
3. payload/schema drift must fail closed rather than match loosely.

### 2.4 Consumption Policy

Gateway handles may be:

1. `single_use`
2. `multi_use`
3. `burn_on_success`
4. `expires_at_boundary`

Rule:

1. one-time handles must become invalid after successful lawful resume,
2. repeated resume attempts against consumed handles must fail clearly,
3. security tokens or URLs are adapters around the handle, not the handle itself.

## 3. Adopted As Future Direction

The kernel accepts as future direction:

1. deterministic correlation hashes for resume targeting,
2. indexed trigger lookup for start/resume,
3. single-use gateway handles with burn-on-success semantics,
4. distributed locking around resume of the same handle or target,
5. idempotent resume handlers for duplicate delivery scenarios.

## 4. Mapping To Existing VIDA Surfaces

1. `route_progression` -> blocked/escalated/manual-intervention gateways
2. `approval_lifecycle` -> governance wait handles
3. `coach_lifecycle` -> optional rework follow-up handles
4. `execution_plan` -> resumable waiting steps
5. `projection-listener-checkpoint-kernel` -> checkpoint and projection references to gateway handles

## 5. Invariants

1. root `vida/config` remains product law
2. gateway handles stay runtime-owned unless explicitly promoted by future spec
3. resume targeting must not silently degrade into broad scan as a default path
4. handle consumption must be inspectable and receipt-backed when decision-relevant
5. `coach`, `verification`, and `approval` remain separate even if all use resumable gateways

-----
artifact_path: product/spec/gateway-resume-handle-and-trigger-index
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/product/spec/gateway-resume-handle-and-trigger-index.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-09T20:28:59+02:00
changelog_ref: gateway-resume-handle-and-trigger-index.changelog.jsonl
