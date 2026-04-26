# Release 1 Error And Exception Taxonomy

Status: active Release-1 control law

Purpose: define the canonical error, blocker, and exception vocabulary for `Release 1` so runtime status, proof artifacts, and recovery flows use stable shared semantics.

## 1. Scope

This taxonomy defines:

1. blocker codes,
2. runtime failure codes,
3. policy and approval failure codes,
4. tool and retrieval failure codes,
5. closure and proof failure codes,
6. exception-path codes.

## 2. Naming Rule

1. Codes use lowercase snake_case.
2. One code means one primary cause.
3. Human messages may vary; canonical codes must not.

## 3. Core Blocker Codes

1. `missing_packet`
2. `missing_lane_receipt`
3. `open_delegated_cycle`
4. `exception_path_missing`
5. `closure_evidence_incomplete`
6. `owner_surface_contradiction`

## 4. Policy And Approval Codes

1. `policy_denied`
2. `policy_context_missing`
3. `approval_required`
4. `approval_denied`
5. `approval_expired`
6. `delegation_chain_broken`

## 5. Tool And Retrieval Codes

1. `tool_contract_missing`
2. `tool_contract_incomplete`
3. `tool_execution_failed`
4. `tool_result_unusable`
5. `citation_missing`
6. `source_unregistered`
7. `freshness_policy_missing`
8. `freshness_violation`
9. `acl_context_missing`

## 6. Trace And Proof Codes

1. `trace_incomplete`
2. `trace_missing`
3. `incident_evidence_missing`
4. `rollback_unavailable`
5. `proof_verdict_missing`
6. `metric_gate_failed`
7. `schema_contract_missing`

## 7. Exception Path Codes

1. `timeout_without_takeover_authority`
2. `supersession_without_receipt`
3. `local_takeover_forbidden`
4. `recovery_not_trustworthy`

## 7.1 Execution Preparation And Artifact Query Codes

1. `execution_preparation_artifacts_unavailable`
   - emitted when an execution-preparation artifact query cannot find a final runtime-consumption snapshot with `execution_preparation_artifacts`
2. `missing_execution_preparation_artifact_query_target`
   - emitted when an artifact query names an artifact id outside the current execution-preparation registry

## 8. Usage Rule

1. Every blocked or failed Release-1 control flow must include at least one canonical code.
2. Multiple codes are allowed when multiple primary causes exist.
3. Free-form text does not replace canonical codes.

## 9. Parameterized Blocker Families

The canonical registry also admits the following bounded parameterized blocker families. Their prefixes are part of the owner law; suffixes are constrained to the enumerated domains and may not be improvised ad hoc in runtime code.

1. `missing_<family>_family`
   Allowed `<family>` values:
   `metadata`, `control_core`, `activation_bundle`, `protocol_binding_registry`, `cache_delivery_contract`, `orchestrator_init_view`, `agent_init_view`
2. `missing_cache_key_input:<key>` and `invalid_cache_key_input:<key>`
   Allowed `<key>` values:
   `source_version_tuple`, `project_activation_revision`, `protocol_binding_revision`, `protocol_binding_cache_token`, `startup_bundle_revision`
3. `missing_invalidation_tuple_key:<key>`, `invalid_invalidation_tuple_key:<key>`, and `invalidation_tuple_mismatch:<key>`
   Allowed `<key>` values:
   `framework_revision`, `project_activation_revision`, `protocol_binding_revision`, `protocol_binding_cache_token`, `startup_bundle_revision`
4. `missing_metadata_tuple_key:<key>` and `invalid_metadata_tuple_key:<key>`
   Allowed `<key>` values:
   `framework_revision`, `project_activation_revision`, `protocol_binding_revision`, `protocol_binding_cache_token`
5. `cache_key_mismatch:<key>`
   Allowed `<key>` values:
   `project_activation_revision`, `protocol_binding_revision`, `protocol_binding_cache_token`
6. `missing_triggered_domain_bundle_partition`
7. `cache_registry_contract_missing_triggered_domain_binding`
8. `missing_retrieval_only_optional_context_boundary`
9. `missing_retrieval_optional_boundary_entry:<entry>`
   Allowed `<entry>` values:
   `full_project_owner_protocols`, `non_promoted_project_docs`, `broad_repo_manual_scan`
10. `missing_retrieval_trust_evidence`
11. `missing_retrieval_trust_evidence_field:<field>`
   Allowed `<field>` values:
   `source`, `citation`, `freshness`, `acl`

## 10. References

1. `docs/product/spec/release-1-closure-contract.md`
2. `docs/product/spec/release-1-state-machine-specs.md`
3. `docs/product/spec/release-1-canonical-artifact-schemas.md`

-----
artifact_path: product/spec/release-1-error-and-exception-taxonomy
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-error-and-exception-taxonomy.md
created_at: 2026-03-16T11:35:00Z
updated_at: 2026-04-26T14:58:34.964327145Z
changelog_ref: release-1-error-and-exception-taxonomy.changelog.jsonl
