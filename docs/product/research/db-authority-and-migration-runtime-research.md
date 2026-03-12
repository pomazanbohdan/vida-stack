# DB Authority And Migration Runtime Research

Purpose: define the first concrete VIDA-specific vision for authoritative DB state, import and migration order, receipt requirements, and fail-closed runtime readiness around `.vida/db/**`.

## 1. Research Question

How should VIDA turn embedded framework artifacts, project activation state, protocol binding, memory, and operational runtime state into one authoritative project-local DB truth, and what migration/import lifecycle is needed for lawful Release-1 startup?

## 2. Primary Inputs

Product/spec inputs:

1. `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`
2. `docs/product/spec/release-1-wave-plan.md`
3. `docs/product/spec/embedded-runtime-and-editable-projection-model.md`
4. `docs/product/spec/runtime-paths-and-derived-cache-model.md`
5. `docs/product/spec/project-activation-and-configurator-model.md`
6. `docs/product/spec/taskflow-protocol-runtime-binding-model.md`
7. `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
8. `docs/product/spec/compiled-runtime-bundle-contract.md`

Research inputs:

1. `docs/product/research/runtime-framework-open-questions-and-external-patterns-survey.md`
2. `docs/product/research/compiled-control-bundle-contract-research.md`
3. `docs/product/research/runtime-memory-state-and-retrieval-research.md`

Framework/runtime inputs:

1. `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`
2. `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
3. `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`

## 3. Core Result

The strongest current conclusion is:

1. VIDA needs one project-local authoritative DB truth under `.vida/db/**`,
2. the DB must contain imported framework/runtime state rather than merely mirror files,
3. import and migration must be receipt-bearing lifecycle transitions,
4. cache, projections, and embedded artifacts do not outrank DB truth after lawful import,
5. runtime must fail closed when authoritative imported state is incomplete or invalid.

## 4. Proposed Authority Model

The authoritative runtime DB should contain at least these logical state families:

1. `framework_state`
2. `project_activation_state`
3. `protocol_binding_state`
4. `memory_state`
5. `runtime_operational_state`

### 4.1 `framework_state`

Owns:

1. imported sealed framework baseline needed for runtime execution,
2. active framework bundle lineage,
3. imported framework-side runtime rows needed after init.

Must not own:

1. project activation choices,
2. task-specific operational receipts,
3. cache snapshots.

### 4.2 `project_activation_state`

Owns:

1. active roles, skills, profiles, flows, agents, teams,
2. model/backend policy,
3. active project-owned configuration posture.

Must not own:

1. sealed framework law,
2. raw export projections as truth,
3. cache-only query views.

### 4.3 `protocol_binding_state`

Owns:

1. active executable protocol rows,
2. runtime owners,
3. enforcement types,
4. blocker codes,
5. proof/receipt expectations.

Must not own:

1. prose protocol documents,
2. detached file-log truth,
3. cache-only delivery partitions.

### 4.4 `memory_state`

Owns:

1. runtime memory truth,
2. validity/supersession data,
3. ordinary searchable memory records for Release 1.

Must not own:

1. cache-stable prompt partitions,
2. daemon-stage semantic/vector indexing as Release-1 truth,
3. sealed framework law.

### 4.5 `runtime_operational_state`

Owns:

1. receipts,
2. readiness,
3. waits/resume state,
4. checkpoints,
5. telemetry,
6. run/session lineage.

Must not own:

1. the sealed framework baseline itself,
2. editable projection sources,
3. cache-only snapshots.

## 5. Revision Tuple

The minimum active runtime revision tuple should include:

1. `framework_revision`
2. `project_activation_revision`
3. `protocol_binding_revision`
4. `bundle_schema_version`
5. `memory_contract_revision` when memory becomes active for the project runtime

Rule:

1. this tuple is the minimum authority pointer for startup, readiness, and cache invalidation,
2. runtime must not claim green readiness if the tuple is incomplete or incompatible.

## 6. Canonical Import And Migration Order

The canonical import/migration order should be:

1. `migration_preflight`
   - path/layout check
   - DB schema compatibility
   - embedded artifact availability
   - runtime version compatibility
2. `framework_import`
   - import sealed framework baseline and related executable framework rows
3. `framework_import_receipt`
4. `project_activation_import`
   - import current active project runtime posture
5. `project_activation_import_receipt`
6. `protocol_binding_import`
   - import executable protocol-binding rows
7. `protocol_binding_import_receipt`
8. `memory_state_import_or_seed`
   - initialize or migrate the project-local memory state family when required
9. `memory_import_receipt`
10. `cache_seed`
   - build derived cache only after authoritative state is lawful
11. `cache_seed_receipt`
12. `activation_ready_evaluation`
13. `activation_ready_receipt`

Compact rule:

1. DB truth first,
2. cache second,
3. execution last.

## 7. Required Receipts

Minimum receipts for Release 1 should be:

1. `migration_preflight_receipt`
2. `framework_import_receipt`
3. `project_activation_import_receipt`
4. `protocol_binding_import_receipt`
5. `memory_import_receipt`
6. `cache_seed_receipt`
7. `activation_ready_receipt`
8. `migration_failure_receipt` when the process fails closed

Receipt rule:

1. a successful runtime state transition must be provable through receipts,
2. lack of receipt means the transition is not trusted as complete.

## 8. Fail-Close Rule

Runtime startup or non-bootstrap execution must fail closed when any of the following is true:

1. required DB schema or runtime authority tables are missing,
2. required `framework_state` is missing or invalid,
3. required `project_activation_state` is missing for a non-empty project,
4. required `protocol_binding_state` is missing or invalid,
5. required memory family initialization is missing when the active runtime depends on memory,
6. the revision tuple is incomplete or incompatible,
7. cache can only be built from ambiguous or stale authority.

Allowed surfaces before readiness:

1. `init`
2. `doctor`
3. `status`
4. bounded `repair`
5. bounded `import` or `re-import`
6. bounded cache rebuild and remediation commands

## 9. Re-Init And Migration Modes

The lifecycle should distinguish:

1. `init`
   - first-time creation of `.vida/db/**`
2. `migrate`
   - schema/version forward movement
3. `re-import`
   - authoritative state refresh from newer embedded/project inputs
4. `repair`
   - bounded recovery from broken but salvageable state
5. `reset`
   - explicit destructive path only

Rule:

1. `re-init` must not silently discard authoritative runtime truth,
2. forward migration is preferred over blind recreation,
3. superseded imported states should remain inspectable until explicit cleanup policy says otherwise.

## 10. Empty-Project Rule

An empty project still needs runtime authority.

That means:

1. `framework_state` is still mandatory,
2. `project_activation_state` may exist in minimal scaffold form,
3. `protocol_binding_state` must still cover minimum executable framework/runtime paths,
4. `memory_state` may be initialized as empty but lawful,
5. readiness may report `project_empty_but_runtime_ready` rather than `uninitialized`.

## 11. Storage And Path Recommendation

The recommended canonical path is:

1. `.vida/db/primary/**`

Interpretation rule:

1. one authoritative project-local DB root,
2. future internal tables/collections may evolve inside it,
3. cache, receipts, and projections remain outside it.

## 12. Result

This research is strong enough to support the following proposal now:

1. one project-local authoritative DB root,
2. five runtime state families,
3. one explicit import/migration order,
4. mandatory receipts,
5. revision-tuple readiness,
6. fail-closed execution gating.

It is not yet strong enough to close:

1. exact table/collection names,
2. exact receipt payload schema,
3. exact rollback/supersession retention policy,
4. exact migration command family names,
5. exact memory-state import behavior when memory is optional for a given project.

-----
artifact_path: product/research/db-authority-and-migration-runtime-research
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/research/db-authority-and-migration-runtime-research.md
created_at: '2026-03-12T23:59:40+02:00'
updated_at: '2026-03-12T23:59:40+02:00'
changelog_ref: db-authority-and-migration-runtime-research.changelog.jsonl
