# Compiled Control Bundle Contract Research

Purpose: define the first concrete VIDA-specific vision for the strict Release-1 compiled control bundle contract, using both framework-owned law and product-owned runtime architecture as inputs.

## 1. Research Question

What exact executable contract should VIDA use for the top-level compiled control bundle in Release 1, and how should that contract separate framework law, project activation, protocol binding, cache delivery, DB truth, and projections?

## 2. Primary Inputs

Framework-owned inputs:

1. `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`
2. `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`
3. `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
4. `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`
5. `vida/config/instructions/diagnostic-instructions/analysis.protocol-consistency-audit-protocol.md`

Product/spec inputs:

1. `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`
2. `docs/product/spec/compiled-runtime-bundle-contract.md`
3. `docs/product/spec/release-1-wave-plan.md`
4. `docs/product/spec/taskflow-protocol-runtime-binding-model.md`
5. `docs/product/spec/embedded-runtime-and-editable-projection-model.md`
6. `docs/product/spec/runtime-paths-and-derived-cache-model.md`
7. `docs/product/spec/project-activation-and-configurator-model.md`

Research inputs:

1. `docs/product/research/runtime-framework-open-questions-and-external-patterns-survey.md`
2. `docs/product/research/instruction-packing-and-caching-survey.md`
3. `docs/product/research/langgraph-runtime-patterns-survey.md`
4. `docs/product/research/execution-approval-and-interrupt-resume-survey.md`

## 3. Framework-To-Bundle Framing

### 3.1 What Must Compile From Framework Law

The following must compile from framework-owned canon into the top-level bundle:

1. sealed runtime-control law from framework-owned instruction/protocol surfaces,
2. activation class and trigger semantics required for runtime consumption,
3. route, gate, packet, handoff, and fail-closed law needed before execution,
4. runtime-family boundary rules and direct runtime-consumption prerequisites,
5. framework-owned protocol-binding requirements where executable enforcement is mandatory.

### 3.2 What Must Not Enter As Raw Framework Content

The bundle must not directly contain:

1. raw markdown protocol bodies,
2. full source-law prose copied as executable truth,
3. framework diagnosis or closure-only texts that are not part of executable control,
4. project-editable source artifacts treated as sealed framework truth.

### 3.3 Framework Effect On Contract Shape

The framework canon forces a shape where:

1. instruction activation remains explicit,
2. runtime consumes a compiled bundle directly,
3. protocol-bearing runtime law is representable as executable contract rows,
4. bundle composition remains inspectable and fail-closed.

Inference:

1. this means the top-level bundle cannot be a generic prompt payload,
2. it must be a typed control object whose sections align with activation, execution, binding, and cache delivery.

## 4. Proposed Contract Vision

The top-level compiled control bundle for Release 1 should be:

1. one strict JSON contract,
2. versioned and revision-bearing,
3. built from sealed framework law plus admitted project runtime posture,
4. compact enough for direct runtime consumption,
5. explicit enough to separate runtime truth from derived cache and from editable projection.

Compact rule:

1. framework law compiles into `control_core`,
2. project runtime posture compiles into `activation_bundle`,
3. executable protocol enforcement compiles into `protocol_binding_registry`,
4. cache-safe delivery boundaries compile into `cache_delivery_contract`.

## 5. Canonical Root Schema

The canonical root schema for Release 1 should be:

1. `metadata`
2. `control_core`
3. `activation_bundle`
4. `protocol_binding_registry`
5. `cache_delivery_contract`

### 5.1 `metadata`

Purpose:

1. identify one compiled bundle instance,
2. bind the bundle to specific framework/project/protocol revisions,
3. support validation, import, and cache invalidation.

Required fields:

1. `bundle_id`
2. `bundle_schema_version`
3. `framework_revision`
4. `project_activation_revision`
5. `protocol_binding_revision`
6. `compiled_at`
7. `binding_status`

Optional fields:

1. `source_mode`
2. `compiler_revision`
3. `runtime_family_scope`
4. `cache_contract_revision`

Must not contain:

1. mutable task state,
2. live receipts,
3. runtime telemetry.

Likely owners:

1. framework runtime bundle law,
2. product compiled runtime bundle contract.

### 5.2 `control_core`

Purpose:

1. carry sealed executable framework control law.

Required fields:

1. `intent_classes`
2. `routing_policy`
3. `gate_chain`
4. `packet_contracts`
5. `runtime_family_branches`
6. `fail_closed_rules`

Optional fields:

1. `lane_boot_views`
2. `route_constraints`
3. `handoff_constraints`

Must not contain:

1. mutable project runtime selections,
2. task-specific dynamic context,
3. receipts or telemetry.

Likely owners:

1. framework-owned protocol canon,
2. runtime-kernel bundle law.

### 5.3 `activation_bundle`

Purpose:

1. carry active project runtime posture admitted for execution.

Required fields:

1. `activation_mode`
2. `enabled_roles`
3. `enabled_skills`
4. `enabled_profiles`
5. `enabled_flow_sets`
6. `active_agents`
7. `model_policy`
8. `backend_policy`
9. `activation_scope`

Optional fields:

1. `active_teams`
2. `enabled_policy_surfaces`
3. `selection_constraints`

Must not contain:

1. sealed framework law,
2. raw editable exports as truth,
3. receipts or telemetry.

Likely owners:

1. project activation/configurator law,
2. project agent extension protocol.

### 5.4 `protocol_binding_registry`

Purpose:

1. carry the active executable protocol-binding and enforcement contract.

Required fields:

1. `protocol_rows`
2. `activation_class`
3. `runtime_owner`
4. `enforcement_type`
5. `required_inputs`
6. `blocker_codes`
7. `expected_receipts`
8. `proof_requirements`
9. `primary_authority`

Optional fields:

1. `admission_class`
2. `cache_scope`
3. `consumption_scope`

Must not contain:

1. prose-only protocol content,
2. detached file-log truth,
3. unresolved owner placeholders.

Likely owners:

1. taskflow protocol-binding model,
2. runtime-bearing protocol families,
3. protocol admission law when promoted.

### 5.5 `cache_delivery_contract`

Purpose:

1. define cache-stable versus task-dynamic delivery boundaries for runtime consumption.

Required fields:

1. `always_on_core`
2. `lane_bundle`
3. `triggered_domain_bundle`
4. `task_specific_dynamic_context`
5. `cache_key_inputs`
6. `invalidation_tuple`

Optional fields:

1. `retrieval_boundary`
2. `provider_cache_hints`
3. `query_view_cache_scope`

Must not contain:

1. sole authoritative copies,
2. dynamic receipts,
3. execution deltas,
4. operator-only evidence.

Likely owners:

1. compiled runtime bundle contract,
2. runtime paths/cache model,
3. instruction-packing research direction.

## 6. Metadata Contract

The minimum valid Release-1 metadata block should be:

1. `bundle_id`
2. `bundle_schema_version`
3. `framework_revision`
4. `project_activation_revision`
5. `protocol_binding_revision`
6. `compiled_at`
7. `binding_status`

Recommended distinctions:

1. `framework_revision`
   - sealed framework law lineage
2. `project_activation_revision`
   - active project runtime posture lineage
3. `protocol_binding_revision`
   - active executable protocol-binding lineage
4. `bundle_schema_version`
   - contract/schema compatibility

Inference:

1. `cache_delivery_contract` may later warrant its own explicit revision field,
2. but Release 1 can likely derive that from the schema version plus the revision tuple.

## 7. Admission And Validation Rules

### 7.1 Bundle Is Valid Only When

1. required framework surfaces resolve,
2. the active project activation posture is valid,
3. every enabled reference resolves,
4. gate and packet obligations are complete,
5. executable protocol rows have runtime owners and enforcement types,
6. cache-delivery partitions do not absorb dynamic truth,
7. revision metadata is complete.

### 7.2 Bundle Is Invalid When

1. required root sections are missing,
2. any mandatory revision field is missing,
3. activation state references unresolved roles/skills/profiles/flows,
4. protocol-binding rows are incomplete,
5. executable project protocols appear without lawful admission,
6. cache-stable partitions include dynamic task evidence or receipts,
7. the runtime cannot determine the authority split safely.

### 7.3 Fail-Close Blockers

Runtime startup or execution must fail closed when:

1. the root schema is incomplete,
2. `control_core` is absent or untrusted,
3. `activation_bundle` is missing for a project-aware run,
4. `protocol_binding_registry` is missing or invalid for executable runtime work,
5. the revision tuple is incompatible with imported DB truth,
6. the cache contract cannot be derived safely.

## 8. Boundary Model

### 8.1 Lives In Bundle

1. executable control contract,
2. admitted project runtime posture,
3. executable protocol-binding registry,
4. cache-safe delivery partitions.

### 8.2 Lives Only In DB

1. authoritative active runtime truth,
2. imported activation state,
3. active protocol-binding rows,
4. receipts,
5. telemetry,
6. execution lineage,
7. waiting/resume state.

### 8.3 Lives Only In Framework / Product Canon

1. human-readable law,
2. full protocol prose,
3. full product architecture explanation,
4. documentation and governance surfaces not yet compiled into executable sections.

### 8.4 Lives Only In Cache / Projection

1. derived prompt-prefix partitions,
2. query-view caches,
3. editable export projections,
4. non-authoritative materialized snapshots.

## 9. Runtime Consumption Model

The primary consumers should be:

1. `orchestrator-init`
2. `agent-init`
3. TaskFlow runtime consumption
4. status/doctor/query surfaces through derived cache views

Consumption split:

1. embedded baseline provides sealed framework `control_core`,
2. DB-backed active state provides current `activation_bundle`,
3. DB-backed active state provides current `protocol_binding_registry`,
4. `.vida/cache/**` provides cache-stable derived views for cheap consumption,
5. direct runtime consumption reads the compiled bundle rather than broad source law.

Inference:

1. the top-level compiled bundle should likely be materialized as one assembled object at runtime startup from embedded baseline + DB-backed active families,
2. even if its sections are persisted or cached separately for efficiency.

## 10. Release-1 Minimum Contract

The smallest practical Release-1 contract is:

1. one strict JSON root object,
2. the five mandatory root sections,
3. minimal revision-bearing metadata,
4. one sealed `control_core` baseline,
5. one DB-backed `activation_bundle`,
6. one DB-backed `protocol_binding_registry`,
7. one explicit `cache_delivery_contract` with:
   - `always_on_core`
   - `lane_bundle`
   - `triggered_domain_bundle`
   - `task_specific_dynamic_context`

## 11. Open Questions

The main remaining open questions are:

1. whether `binding_status` is enough or whether Release 1 needs a stronger import/admission status matrix,
2. whether `active_agents` and `active_teams` belong directly in `activation_bundle` or only as derived runtime views,
3. whether `cache_delivery_contract` needs its own explicit revision field in Release 1,
4. the exact schema of one `protocol_row`,
5. the exact admission lifecycle from `known project protocol` to `compiled executable project protocol`,
6. the exact import/rebuild rule for assembling one runtime-consumable top-level bundle from embedded baseline plus DB-backed active state.

## 12. Result

This research is strong enough to support:

1. one explicit top-level compiled bundle schema,
2. one explicit metadata contract,
3. one explicit authority split between embedded baseline, DB truth, cache, and projection,
4. one fail-closed validation posture for Release 1.

It is not yet enough to claim closure on:

1. exact protocol-row schema,
2. exact promotion/admission lifecycle,
3. final cache manifest contract,
4. final runtime reassembly/import algorithm.

-----
artifact_path: product/research/compiled-control-bundle-contract-research
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/research/compiled-control-bundle-contract-research.md
created_at: '2026-03-12T23:58:00+02:00'
updated_at: '2026-03-12T23:58:00+02:00'
changelog_ref: compiled-control-bundle-contract-research.changelog.jsonl
