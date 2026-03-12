# Compiled Runtime Bundle Contract

Status: active product law

Purpose: define the canonical machine-readable runtime bundle contract that compiles framework law, project activation, protocol binding, and cache-delivery boundaries into executable runtime control payloads for Release 1.

## 1. Why This Contract Exists

The runtime must not require large repeated markdown rereads in order to act lawfully.

Instead:

1. canonical law stays human-readable in specs and protocol surfaces,
2. active runtime control is compiled into one bounded machine-readable bundle,
3. orchestration consumes that bundle directly,
4. invalid or incomplete activation must fail closed before execution begins.

## 2. Top-Level Bundle Families

Release 1 recognizes these top-level compiled bundle families:

1. `control_core`
   - sealed framework-owned executable control law
2. `activation_bundle`
   - active project runtime posture admitted for execution
3. `protocol_binding_registry`
   - fail-closed protocol-binding and enforcement registry
4. `cache_delivery_contract`
   - explicit cache-safe delivery partitions and invalidation inputs

Family rule:

1. these families are the canonical top-level split of the Release-1 compiled control bundle,
2. runtime-specific views such as orchestrator-init, agent-init, and future team/runtime-family views are built from these families rather than replacing them,
3. no family may smuggle in authority that belongs to another family.

## 3. Runtime Views Over The Bundle Families

The top-level bundle families do not eliminate runtime-specific views.

Release 1 still needs at least:

1. `framework baseline view`
   - the minimum executable framework control slice built primarily from `control_core`
2. `orchestrator-init view`
   - a startup/runtime-consumption slice built from `control_core`, the active `activation_bundle`, the active `protocol_binding_registry`, and the relevant `cache_delivery_contract` partitions
3. `agent-init view`
   - a bounded execution slice built from the same top-level families, narrowed to one selected role/profile/skill/flow posture
4. `team/runtime-family view`
   - optional later specializations built from the same top-level families when team or runtime-family-specific composition becomes active

View rule:

1. runtime views are consumption projections,
2. the top-level families remain the canonical compiled contract.

## 4. Source Inputs

### 4.1 Always-Compiled Framework Inputs

These inputs compile into every executable bundle:

1. framework core/system protocols,
2. lane/role coordination law,
3. gate and proof obligations,
4. packet and handoff law,
5. route constraints,
6. runtime-family boundary rules.

### 4.2 Selected Project Inputs

These inputs compile when active and valid:

1. selected role class,
2. selected project role override or extension,
3. selected profile,
4. enabled skills,
5. selected flow set,
6. model/backend policy,
7. approval/escalation posture,
8. project output/render posture,
9. promoted project protocols admitted for execution.

## 5. Minimum Root Bundle Shape

Every Release-1 compiled control bundle must expose at least:

1. `metadata`
2. `control_core`
3. `activation_bundle`
4. `protocol_binding_registry`
5. `cache_delivery_contract`

### 5.1 `metadata`

The root `metadata` block must expose at least:

1. `bundle_id`
2. `bundle_schema_version`
3. `framework_revision`
4. `project_activation_revision`
5. `protocol_binding_revision`
6. `compiled_at`
7. `binding_status`

### 5.2 `control_core`

`control_core` must expose at least:

1. intent classes,
2. routing policy,
3. gate chain,
4. packet contracts,
5. runtime-family branches,
6. fail-closed rules.

### 5.3 `activation_bundle`

`activation_bundle` must expose at least:

1. activation mode,
2. enabled roles,
3. enabled skills,
4. enabled profiles,
5. enabled flow sets,
6. active agents,
7. active teams when present,
8. model policy,
9. backend policy,
10. activation scope.

### 5.4 `protocol_binding_registry`

`protocol_binding_registry` must expose at least:

1. protocol identifiers,
2. activation classes or triggers,
3. runtime owners,
4. enforcement types,
5. blocker codes,
6. expected receipts,
7. proof requirements,
8. primary authority or import class.

### 5.5 `cache_delivery_contract`

`cache_delivery_contract` must expose at least:

1. `always_on_core`,
2. `lane_bundle`,
3. `triggered_domain_bundle`,
4. `task_specific_dynamic_context`,
5. cache-key inputs,
6. invalidation tuple,
7. retrieval-only optional-context boundary.

Root-shape rule:

1. the runtime may materialize additional bounded detail inside these sections,
2. but it must not omit the minimum root shape and still claim lawful execution.

## 6. Family Boundaries

### 6.1 `control_core`

Framework-owned law compiles always.

That includes:

1. core law,
2. orchestration shell law,
3. runtime-family execution law needed by the active release slice,
4. safety boundaries that remain sealed from project mutation.

`control_core` must not contain:

1. mutable project activation truth,
2. runtime receipts or telemetry,
3. task-specific dynamic evidence.

### 6.2 `activation_bundle`

Project-owned activation inputs compile selectively.

That means:

1. project roles, skills, profiles, and flows may compile when enabled and valid,
2. known project protocols do not automatically become executable,
3. only promoted project protocols admitted by the project protocol promotion rule may enter executable bundles.

`activation_bundle` must not contain:

1. sealed framework law,
2. raw editable exports as runtime truth,
3. runtime receipts or telemetry.

### 6.3 `protocol_binding_registry`

Protocol-binding state compiles or imports selectively from already admitted runtime-bearing protocol inputs.

That means:

1. prose-only protocol text does not enter this family,
2. detached file-log truth does not satisfy this family,
3. missing runtime owners or missing enforcement fields must fail admission,
4. script-era deterministic payloads may act as bounded bridge inputs until the stronger runtime-binding layer closes.

### 6.4 `cache_delivery_contract`

Cache-delivery state is always derived from the other families.

That means:

1. it exists only to accelerate runtime/query/model-serving consumption,
2. it must not become a second truth model,
3. dynamic evidence and receipts stay outside the cache-stable sections,
4. cache partitions must remain explicit and inspectable.

## 7. Bundle Compilation Rule

Compilation is valid only when:

1. all required framework surfaces resolve,
2. selected project activation state resolves,
3. every enabled reference is valid,
4. gate and packet obligations are complete,
5. promotion/admission rules for project protocols are satisfied,
6. protocol-binding rows are complete for the active executable protocol set,
7. cache delivery partitions can be derived without smuggling dynamic truth into cache-stable sections.

## 8. Validation And Failure

Bundle compilation is valid only when:

1. all required framework surfaces resolve,
2. selected project activation state resolves,
3. every enabled reference is valid,
4. gate and packet obligations are complete,
5. promotion/admission rules for project protocols are satisfied.

Fail-closed rule:

1. if required data is missing or inconsistent, compilation must stop,
2. runtime must not silently drop invalid project inputs and continue with a weaker bundle.

## 9. Storage And Authority Split

The compiled bundle families must preserve this storage/authority split:

1. embedded or packaged framework artifacts may carry the sealed `control_core` baseline,
2. DB-first runtime truth owns the active `activation_bundle`,
3. DB-first runtime truth owns the active `protocol_binding_registry`,
4. `.vida/cache/**` may hold derived snapshots of the compiled bundle families and cache-delivery partitions,
5. export or projection surfaces may expose editable project inputs, but they do not become runtime truth until validated and re-imported.

## 10. Orchestrator And Agent Initialization

Release 1 must expose at least two initialization paths:

1. `orchestrator-init`
   - builds or loads the active orchestrator bundle
2. `agent-init`
   - builds or loads one bounded agent bundle for the selected execution posture

Initialization rule:

1. init paths must be inspectable,
2. init output must tell the runtime what law/policy was compiled,
3. init output must remain usable by an LLM orchestrator without broad manual repo traversal.

## 11. Inspection Surfaces

Release-1 operator/runtime surfaces must support:

1. bundle summary,
2. bundle validation result,
3. bundle source inputs,
4. effective `control_core` summary,
5. effective `activation_bundle` summary,
6. effective `protocol_binding_registry` summary,
7. effective `cache_delivery_contract` summary,
8. effective role/profile/skill/flow composition,
9. effective gate/policy summary.

Inspection rule:

1. bundle inspection exists for proof and debugging,
2. it must not become a second human-owned product-law source.

## 12. Boundary Rule

1. the bundle is executable composition, not the owner of product law,
2. canonical law remains in specs and framework protocols,
3. project activation remains DB-first truth,
4. the bundle is the compact runtime consumption surface built from that truth.

## 13. Release-1 First Bundle Map

The recommended first Release-1 bundle map is:

1. root compiled control bundle with:
   - `metadata`
   - `control_core`
   - `activation_bundle`
   - `protocol_binding_registry`
   - `cache_delivery_contract`
2. first cache partitions inside `cache_delivery_contract`:
   - `always_on_core`
   - `lane_bundle`
   - `triggered_domain_bundle`
   - `task_specific_dynamic_context`
3. first runtime views:
   - `orchestrator-init`
   - `agent-init`
4. first authority split:
   - embedded baseline for sealed framework control,
   - DB truth for activation and protocol binding,
   - derived cache for delivery acceleration,
   - projection only for editable exports/imports.

## 14. Completion Proof

This contract is operationally closed enough for Release 1 when:

1. framework law compiles into bounded executable `control_core`,
2. project activation compiles into `activation_bundle` when valid,
3. protocol-binding state compiles or imports into `protocol_binding_registry`,
4. cache-delivery partitions derive lawfully from the other families,
5. invalid inputs fail closed,
6. bundle inspection can show the effective runtime posture by family,
7. init paths can bootstrap the orchestrator and bounded agents lawfully.

-----
artifact_path: product/spec/compiled-runtime-bundle-contract
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/compiled-runtime-bundle-contract.md
created_at: '2026-03-11T23:01:49+02:00'
updated_at: '2026-03-12T23:10:00+02:00'
changelog_ref: compiled-runtime-bundle-contract.changelog.jsonl
