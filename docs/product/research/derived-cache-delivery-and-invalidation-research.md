# Derived Cache Delivery And Invalidation Research

Purpose: define the first concrete VIDA-specific vision for the derived cache layer under `.vida/cache/**`, including cache families, invalidation rules, stale-read policy, and the boundary between authoritative DB truth, prompt delivery cache, query views, and runtime memory retrieval.

## 1. Research Question

How should VIDA build and invalidate derived cache artifacts so CLI/runtime/model-serving hot paths stay fast, while DB truth remains authoritative and cache never becomes a second truth model?

## 2. Primary Inputs

Product/spec inputs:

1. `docs/product/spec/runtime-paths-and-derived-cache-model.md`
2. `docs/product/spec/compiled-runtime-bundle-contract.md`
3. `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`
4. `docs/product/spec/release-1-plan.md`
5. `docs/product/spec/project-activation-and-configurator-model.md`
6. `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`

Research inputs:

1. `docs/product/research/instruction-packing-and-caching-survey.md`
2. `docs/product/research/compiled-control-bundle-contract-research.md`
3. `docs/product/research/db-authority-and-migration-runtime-research.md`
4. `docs/product/research/runtime-home-and-surface-migration-research.md`
5. `docs/product/research/runtime-memory-state-and-retrieval-research.md`

Framework/runtime inputs:

1. `runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`
2. `runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
3. `instruction-contracts/bridge.instruction-activation-protocol.md`

## 3. Core Result

The strongest current conclusion is:

1. `.vida/cache/**` must remain a derived serving layer only,
2. all cache families rebuild from authoritative revision tuples rather than file timestamps or guesswork,
3. prompt delivery cache and CLI/query-view cache may share one root invalidation model while still keeping family-specific freshness rules,
4. dynamic receipts, telemetry, waits, and operator deltas must stay out of cache-stable prompt prefixes,
5. ordinary memory retrieval contributes only to task-dynamic context, not to cache-stable bundle truth.

## 4. Cache Purpose

The derived cache layer exists to accelerate:

1. CLI startup and status rendering,
2. doctor/readiness inspection,
3. runtime bundle delivery to orchestrator and agents,
4. provider-facing cache-stable prompt prefix generation,
5. repeated query paths that would otherwise re-walk DB and compiled bundle sources on every call.

It must not exist to:

1. replace authoritative DB state,
2. store the only active copy of runtime posture,
3. bypass import/readiness gates,
4. silently continue from stale state without policy permission.

## 5. Allowed Cache Families

The strongest Release-1 cache family split is:

1. `control_bundle_cache`
2. `activation_bundle_cache`
3. `protocol_binding_query_cache`
4. `prompt_prefix_cache`
5. `query_view_cache`
6. `cache_manifests`

### 5.1 `control_bundle_cache`

Owns:

1. compiled `control_core` snapshots,
2. framework delivery views,
3. lane boot views derived from sealed framework/runtime law.

### 5.2 `activation_bundle_cache`

Owns:

1. derived activation snapshots,
2. effective role/skill/profile/flow views,
3. bundle-ready project activation projections for runtime consumption.

### 5.3 `protocol_binding_query_cache`

Owns:

1. query-safe protocol-binding summaries,
2. blocker and proof expectation views,
3. operator/status inspection surfaces over executable binding rows.

### 5.4 `prompt_prefix_cache`

Owns:

1. `always_on_core`
2. `lane_bundle`
3. `triggered_domain_bundle`

Rule:

1. `task_specific_dynamic_context` is not part of the stable prefix cache family.

### 5.5 `query_view_cache`

Owns:

1. bounded status summaries,
2. doctor summaries,
3. readiness summaries,
4. active init/activation/runtime posture views.

### 5.6 `cache_manifests`

Owns:

1. invalidation tuples,
2. family freshness status,
3. last successful build receipts,
4. family-specific rebuild policy hints.

## 6. Canonical Invalidation Tuple

The minimum root invalidation tuple should include:

1. `framework_revision`
2. `project_activation_revision`
3. `runtime_config_revision`
4. `protocol_binding_revision`
5. `bundle_schema_version`
6. `memory_contract_revision` when memory retrieval is enabled for the runtime

Interpretation rule:

1. this is the shared root tuple,
2. each cache family may derive a narrower family tuple from it,
3. a family must rebuild when any dependency in its family tuple changes.

## 7. One Manifest Or Many

The strongest current recommendation is:

1. one root cache manifest,
2. plus one family manifest per cache family.

Reason:

1. one root manifest keeps the authority picture simple,
2. family manifests avoid over-rebuilding unrelated cache families,
3. one global manifest alone would be too coarse,
4. completely separate manifest systems would drift.

## 8. Family Dependency Model

### 8.1 `control_bundle_cache` Depends On

1. `framework_revision`
2. `bundle_schema_version`

### 8.2 `activation_bundle_cache` Depends On

1. `project_activation_revision`
2. `runtime_config_revision`
3. `bundle_schema_version`

### 8.3 `protocol_binding_query_cache` Depends On

1. `protocol_binding_revision`
2. `bundle_schema_version`

### 8.4 `prompt_prefix_cache` Depends On

1. `framework_revision`
2. `project_activation_revision`
3. `protocol_binding_revision`
4. `runtime_config_revision`
5. `bundle_schema_version`

### 8.5 `query_view_cache` Depends On

1. the relevant authority revisions above,
2. bounded operational-state freshness where the specific query family includes readiness or wait summaries.

Rule:

1. `query_view_cache` is the only family that may need small operational-state projections,
2. even then, it must remain rebuildable from DB truth.

## 9. Stale-Read Policy

The strongest current recommendation is:

1. no stale reads for execution-bearing cache families,
2. bounded stale reads allowed only for noncritical read-only query views when policy explicitly permits it.

That means:

1. `control_bundle_cache` must not serve stale data,
2. `activation_bundle_cache` must not serve stale data,
3. `protocol_binding_query_cache` must not serve stale data for execution or gating,
4. `prompt_prefix_cache` must not serve stale data once the family tuple changes,
5. `query_view_cache` may serve briefly stale summaries only for noncritical views such as informational status snapshots if the runtime labels them clearly.

## 10. Prompt Delivery Boundary

The prompt delivery cache should keep the partition already promoted in canon:

1. `always_on_core`
2. `lane_bundle`
3. `triggered_domain_bundle`
4. `task_specific_dynamic_context`

Rule:

1. only the first three are cache-stable prefix candidates,
2. `task_specific_dynamic_context` is always task-dynamic,
3. receipts, telemetry, operator notes, current waits, and current memory retrieval results belong only in `task_specific_dynamic_context`.

## 11. Query Cache Versus Prompt Cache

The strongest current VIDA-specific split is:

1. `prompt_prefix_cache` serves model/runtime delivery,
2. `query_view_cache` serves CLI/operator inspection.

They should share:

1. root invalidation tuple,
2. family manifests,
3. rebuild receipts,
4. DB-first authority rules.

They should not be merged into one undifferentiated cache family because:

1. prompt delivery has strict prefix-stability rules,
2. query views have different freshness and rendering concerns,
3. prompt families and query families drift differently.

## 12. Memory Boundary

For Release 1:

1. memory is authoritative DB state,
2. ordinary search results may feed task-dynamic context,
3. memory retrieval results do not belong in cache-stable prompt prefix bundles,
4. memory search indexes beyond ordinary search remain deferred until daemon/reactive stages.

Interpretation rule:

1. memory participates in runtime context,
2. memory does not become a stable prompt cache baseline.

## 13. Rebuild Triggers

The strongest rebuild triggers are:

1. successful DB import or migration,
2. activation/config update,
3. protocol-binding re-import,
4. bundle-schema version change,
5. runtime-home migration that changes source authority posture,
6. explicit operator rebuild command,
7. cache corruption or missing family manifest.

## 14. Rebuild Order

The strongest current rebuild order is:

1. validate root manifest dependencies,
2. rebuild `control_bundle_cache` if needed,
3. rebuild `activation_bundle_cache` if needed,
4. rebuild `protocol_binding_query_cache` if needed,
5. rebuild `prompt_prefix_cache` if needed,
6. rebuild `query_view_cache` last,
7. write family receipts,
8. update root manifest.

Rule:

1. execution-bearing families rebuild before query-decoration families,
2. root manifest updates only after successful family rebuilds.

## 15. Required Receipts

The minimum receipt families should include:

1. `cache_manifest_validation_receipt`
2. `cache_family_rebuild_receipt`
3. `cache_root_manifest_update_receipt`
4. `cache_rebuild_failure_receipt`

These receipts may be stored in:

1. DB operational state,
2. `.vida/receipts/**` projection surfaces when file evidence is needed.

## 16. Fail-Close Conditions

Cache must fail closed when:

1. a required execution-bearing family is missing after readiness claims green state,
2. the invalidation tuple is incomplete or incompatible,
3. cache artifacts claim freshness without a valid manifest,
4. prompt delivery would require stale or ambiguous execution-bearing cache content,
5. cache rebuild tries to outrank authoritative DB truth.

Allowed fallback:

1. rebuild from DB truth,
2. then continue.

Forbidden fallback:

1. silently continue from unknown stale cache,
2. silently treat cache artifacts as authority.

## 17. Release-1 Practical Recommendation

The strongest practical Release-1 recommendation is:

1. keep one root manifest plus family manifests,
2. keep prompt delivery cache and query-view cache as separate families with one shared invalidation model,
3. allow stale cache only for explicitly noncritical read-only query views,
4. keep memory retrieval out of cache-stable prompt prefixes,
5. rebuild from authoritative DB truth on every execution-bearing cache miss or invalidation.

## 18. Open Questions

The remaining bounded open questions are:

1. the exact JSON schema for root and family cache manifests,
2. the exact operator command/query set for cache inspection and forced rebuilds,
3. whether readiness summaries should read directly from DB or exclusively through `query_view_cache`,
4. how much of provider-specific cache hints should be exposed in Release 1 versus deferred.

## 19. Recommended Next Follow-Up

The strongest next bounded follow-up after this research is:

1. `embedded-runtime-bootstrap-and-projection-research`

Reason:

1. compiled bundle contract is now framed,
2. DB authority is framed,
3. runtime-home migration is framed,
4. derived cache behavior is framed,
5. the next unresolved seam is how embedded runtime artifacts and editable projections cooperate during install/init/bootstrap.

-----
artifact_path: product/research/derived-cache-delivery-and-invalidation-research
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/research/derived-cache-delivery-and-invalidation-research.md
created_at: '2026-03-12T23:59:55+02:00'
updated_at: 2026-03-16T10:02:36.795557275Z
changelog_ref: derived-cache-delivery-and-invalidation-research.changelog.jsonl
