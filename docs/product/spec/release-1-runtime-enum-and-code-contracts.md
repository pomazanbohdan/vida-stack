# Release 1 Runtime Enum And Code Contracts

Status: active Release-1 implementation law

Purpose: define the canonical enum/value vocabulary that runtime code, receipts, metrics, and operator surfaces must share during `Release 1`.

## 1. Scope

This contract defines canonical values for:

1. `workflow_class`
2. `risk_tier`
3. `lane_status`
4. `approval_status`
5. `gate_level`
6. `blocker_code`
7. `compatibility_class`

## 2. Workflow Class Values

Allowed values:

1. `informational_answer`
2. `retrieval_grounded_answer`
3. `documentation_mutation`
4. `internal_state_mutation`
5. `delegated_development_packet`
6. `tool_assisted_read`
7. `tool_assisted_write`
8. `memory_write`
9. `identity_or_policy_change`
10. `incident_response_or_recovery`

## 3. Risk Tier Values

Allowed values:

1. `R0`
2. `R1`
3. `R2`
4. `R3`
5. `R4`

## 4. Lane Status Values

Allowed values:

1. `packet_ready`
2. `lane_open`
3. `lane_running`
4. `lane_blocked`
5. `lane_completed`
6. `lane_superseded`
7. `lane_exception_takeover`

## 5. Approval Status Values

Allowed values:

1. `approval_not_required`
2. `approval_required`
3. `waiting_for_approval`
4. `approved`
5. `denied`
6. `expired`

## 6. Gate Level Values

Allowed values:

1. `block`
2. `warn`
3. `observe`

## 7. Compatibility Class Values

Allowed values:

1. `backward_compatible`
2. `reader_upgrade_required`
3. `migration_required`

## 8. Blocker Code Values

Allowed values include at minimum:

1. `missing_packet`
2. `missing_lane_receipt`
3. `open_delegated_cycle`
4. `exception_path_missing`
5. `closure_evidence_incomplete`
6. `owner_surface_contradiction`
7. `policy_denied`
8. `approval_required`
9. `approval_denied`
10. `approval_expired`
11. `delegation_chain_broken`
12. `tool_contract_missing`
13. `tool_contract_incomplete`
14. `tool_execution_failed`
15. `citation_missing`
16. `source_unregistered`
17. `freshness_policy_missing`
18. `freshness_violation`
19. `trace_incomplete`
20. `incident_evidence_missing`
21. `rollback_unavailable`
22. `proof_verdict_missing`
23. `metric_gate_failed`
24. `schema_contract_missing`
25. `timeout_without_takeover_authority`
26. `supersession_without_receipt`
27. `local_takeover_forbidden`
28. `recovery_not_trustworthy`

## 9. Code Contract Rule

1. Release-1 Rust code must not invent alternative enum names for these values.
2. User-facing labels may differ, but canonical machine values must not.
3. Any expansion requires updating this contract first.

## 10. References

1. `docs/product/spec/release-1-error-and-exception-taxonomy.md`
2. `docs/product/spec/release-1-state-machine-specs.md`
3. `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
4. `docs/product/spec/release-1-schema-versioning-and-compatibility-law.md`

-----
artifact_path: product/spec/release-1-runtime-enum-and-code-contracts
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-runtime-enum-and-code-contracts.md
created_at: 2026-03-16T11:45:00Z
updated_at: 2026-03-16T11:34:32.217623693Z
changelog_ref: release-1-runtime-enum-and-code-contracts.changelog.jsonl
