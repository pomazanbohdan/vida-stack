# Release 1 Canonical Artifact Schemas

Status: active Release-1 contract law

Purpose: define the minimum canonical artifact schemas required by `Release 1` so traces, approvals, tool contracts, evaluation runs, incidents, and memory-governance records are implemented with stable machine-readable contracts rather than ad hoc JSON.

## 1. Scope

This document defines the minimum field contracts for:

1. `trace_event`
2. `policy_decision`
3. `approval_record`
4. `tool_contract`
5. `lane_execution_receipt`
6. `evaluation_run`
7. `feedback_event`
8. `incident_evidence_bundle`
9. `memory_record`
10. `closure_admission_record`

This document does not define:

1. final wire format per crate,
2. storage backend layout,
3. every optional field future releases may add.

Schema rule:

1. fields listed here are the minimum required shape,
2. implementations may extend them,
3. implementations may not remove or silently rename the required fields while this Release-1 contract is active.

## 2. Shared Contract Fields

All Release-1 canonical artifacts must contain:

1. `artifact_id`
2. `artifact_type`
3. `schema_version`
4. `created_at`
5. `updated_at`
6. `status`
7. `owner_surface`
8. `trace_id` when a workflow trace exists
9. `workflow_class` when the artifact is workflow-bound

## 3. Trace Event

Required fields:

1. `artifact_id`
2. `artifact_type = "trace_event"`
3. `trace_id`
4. `span_id`
5. `parent_span_id`
6. `workflow_class`
7. `workflow_run_id`
8. `actor_kind`
9. `actor_id`
10. `event_type`
11. `started_at`
12. `ended_at`
13. `outcome`
14. `side_effect_class`
15. `related_artifact_ids`
16. `policy_decision_ids`
17. `approval_record_ids`

## 4. Policy Decision

Required fields:

1. `artifact_id`
2. `artifact_type = "policy_decision"`
3. `policy_id`
4. `policy_version`
5. `trace_id`
6. `workflow_class`
7. `actor_id`
8. `subject_id`
9. `decision`
10. `reason_codes`
11. `constraints_applied`
12. `created_at`
13. `expires_at`

## 5. Approval Record

Required fields:

1. `artifact_id`
2. `artifact_type = "approval_record"`
3. `approval_id`
4. `trace_id`
5. `workflow_class`
6. `approval_scope`
7. `requested_by`
8. `approved_by`
9. `decision`
10. `decision_at`
11. `decision_reason`
12. `expires_at`
13. `related_policy_decision_ids`

## 6. Tool Contract

Required fields:

1. `artifact_id`
2. `artifact_type = "tool_contract"`
3. `tool_id`
4. `tool_version`
5. `tool_name`
6. `operation_class`
7. `side_effect_class`
8. `auth_mode`
9. `approval_required`
10. `idempotency_class`
11. `retry_posture`
12. `rollback_posture`
13. `input_schema_ref`
14. `output_schema_ref`
15. `policy_hook_ids`
16. `observability_requirements`

## 7. Lane Execution Receipt

Required fields:

1. `artifact_id`
2. `artifact_type = "lane_execution_receipt"`
3. `run_id`
4. `packet_id`
5. `lane_id`
6. `lane_role`
7. `carrier_id`
8. `lane_status`
9. `evidence_status`
10. `started_at`
11. `finished_at`
12. `result_artifact_ids`
13. `supersedes_receipt_id`
14. `exception_path_receipt_id`

## 8. Evaluation Run

Required fields:

1. `artifact_id`
2. `artifact_type = "evaluation_run"`
3. `evaluation_id`
4. `evaluation_profile`
5. `target_surface`
6. `workflow_class`
7. `dataset_or_sample_window`
8. `metric_results`
9. `regression_summary`
10. `decision`
11. `decision_reason`
12. `run_at`
13. `trace_sample_refs`

## 9. Feedback Event

Required fields:

1. `artifact_id`
2. `artifact_type = "feedback_event"`
3. `feedback_id`
4. `workflow_class`
5. `trace_id`
6. `source_kind`
7. `severity`
8. `feedback_type`
9. `summary`
10. `linked_defect_or_remediation_id`
11. `created_at`

## 10. Incident Evidence Bundle

Required fields:

1. `artifact_id`
2. `artifact_type = "incident_evidence_bundle"`
3. `incident_id`
4. `workflow_class`
5. `trace_ids`
6. `trigger_reason`
7. `impact_summary`
8. `side_effect_summary`
9. `rollback_or_restore_actions`
10. `recovery_outcome`
11. `root_cause_status`
12. `opened_at`
13. `closed_at`

## 11. Memory Record

Required fields:

1. `artifact_id`
2. `artifact_type = "memory_record"`
3. `memory_id`
4. `memory_class`
5. `subject_scope`
6. `origin_trace_id`
7. `origin_workflow_class`
8. `sensitivity_level`
9. `consent_basis`
10. `ttl_policy`
11. `deletion_or_correction_ref`
12. `approval_record_ids`
13. `created_at`

## 12. Closure Admission Record

Required fields:

1. `artifact_id`
2. `artifact_type = "closure_admission_record"`
3. `release_scope`
4. `supported_workflow_classes`
5. `closure_decision`
6. `decision_at`
7. `decision_owner`
8. `evidence_bundle_refs`
9. `open_risk_acceptance_ids`
10. `blocked_by`

## 13. Contract Rule

Release-1 implementation is not schema-complete unless:

1. these artifact types exist as explicit contracts,
2. the required fields above are materially represented,
3. runtime-family code and owner docs use the same artifact names,
4. proof/evaluation surfaces can point to these artifacts directly.

## 14. References

1. Airtable `Vida` base, `Spec` table, refreshed `2026-03-16`
2. `docs/product/spec/release-1-plan.md`
3. `docs/product/spec/release-1-closure-contract.md`
4. `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
5. `docs/product/spec/release-1-control-metrics-and-gates.md`

-----
artifact_path: product/spec/release-1-canonical-artifact-schemas
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-canonical-artifact-schemas.md
created_at: 2026-03-16T11:16:55.806302398Z
updated_at: 2026-03-16T11:22:20.731260586Z
changelog_ref: release-1-canonical-artifact-schemas.changelog.jsonl
