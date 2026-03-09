# VIDA Receipt And Proof Taxonomy

Status: draft `v1` receipt/proof law

Revision: `2026-03-09`

Purpose: define the canonical boundary between events, receipts, proof attachments, checkpoints, and audit records for the partial development kernel.

## 1. Core Distinction

### 1.1 Event

A domain fact about state progression or machine activity.

### 1.2 Receipt

An immutable runtime record that a lawful transition, assignment, handoff, escalation, or decision occurred.

### 1.3 Proof

An immutable evidence artifact that supports a guard, review, verification verdict, approval, doctor verdict, or closure decision.

### 1.4 Checkpoint

A resumability artifact derived from canonical state, receipts, and projections. A checkpoint may carry a resume cursor, route/task references, and gateway posture, but it is not itself a state transition record.

### 1.5 Audit Record

An operational trace for observability, latency, cost, adapter diagnostics, or compliance reporting. Audit is not source-of-truth state law.

## 2. Canonical Receipt Classes

### 2.1 Transition Receipts

1. `task_state_changed_receipt`
2. `execution_plan_state_changed_receipt`
3. `route_state_changed_receipt`
4. `coach_state_changed_receipt`
5. `verification_state_changed_receipt`
6. `approval_state_changed_receipt`
7. `boot_state_changed_receipt`

### 2.2 Assignment And Routing Receipts

1. `route_resolved_receipt`
2. `assignment_requested_receipt`
3. `agent_assigned_receipt`
4. `lease_issued_receipt`
5. `dispatch_started_receipt`
6. `fallback_selected_receipt`
7. `escalation_triggered_receipt`
8. `assignment_exhausted_receipt`
9. `manual_intervention_required_receipt`

### 2.3 Execution Receipts

1. `execution_started_receipt`
2. `execution_completed_receipt`
3. `execution_failed_receipt`
4. `execution_paused_receipt`
5. `execution_resumed_receipt`
6. `rework_requested_receipt`
7. `checkpoint_written_receipt`

### 2.4 Coach Receipts

1. `coach_requested_receipt`
2. `coach_feedback_issued_receipt`
3. `coach_rework_receipt`

### 2.5 Verification Receipts

1. `verification_requested_receipt`
2. `verifier_assigned_receipt`
3. `verification_partial_receipt`
4. `verification_aggregate_receipt`
5. `verification_passed_receipt`
6. `verification_failed_receipt`
7. `verification_inconclusive_receipt`

### 2.6 Approval Receipts

1. `approval_requested_receipt`
2. `approval_received_receipt`
3. `approval_rejected_receipt`
4. `approval_expired_receipt`
5. `approval_escalated_receipt`

### 2.7 Boot And Migration Receipts

1. `compat_checked_receipt`
2. `migration_required_receipt`
3. `migration_applied_receipt`
4. `doctor_verdict_receipt`
5. `boot_allowed_receipt`
6. `boot_blocked_receipt`

## 3. Mandatory Receipt Fields

Every lawful receipt must carry:

1. `receipt_id`
2. `receipt_type`
3. `entity_type`
4. `entity_id`
5. `machine`
6. `event`
7. `actor`
8. `timestamp`
9. `config_artifact`
10. `config_revision`
11. `prior_state`
12. `new_state`

When applicable, receipts must also carry:

1. `route_ref`
2. `instruction_bundle_ref`
3. `proof_refs`
4. `metadata`

### 3.4 Checkpoint Receipt Minimum

Checkpoint-related receipts must include:

1. `checkpoint_kind`
2. `cursor_or_position`
3. `resume_target` when resumable
4. `checkpoint_group` when multiple grouped projections advance together

### 3.1 Assignment Receipt Minimum

Assignment-related receipts must include:

1. `required_role`
2. `required_capability`
3. `selection_strategy`
4. `chosen_agent`
5. `independence_check`
6. `lease_terms` when leased

### 3.2 Verification Receipt Minimum

Verification-related receipts must include:

1. `verifier_set`
2. `aggregation_policy`
3. `proof_category_coverage`
4. `verdict`

### 3.3 Approval Receipt Minimum

Approval-related receipts must include:

1. `approver_identity_or_class`
2. `approval_disposition`
3. `expiry_metadata` when pending or expiring

## 4. Optional Receipt Fields

Optional only:

1. performance telemetry,
2. token or resource usage,
3. verbose debug trace,
4. advisory notes,
5. intermediate summaries.

## 5. Canonical Proof Categories

1. `artifact_manifest`
2. `test_report`
3. `diff_summary`
4. `review_findings`
5. `verification_evidence`
6. `approval_note`
7. `doctor_output`
8. `migration_report`
9. `execution_log_excerpt`
10. `policy_evaluation_snapshot`
11. `execution_snapshot`

### 5.1 Proof Rule

Policies and machines must ask for proof categories, not helper-specific file paths.

## 6. Receipt Versus Proof

Examples:

1. `approval_received_receipt` is not the approval note itself.
2. `verification_passed_receipt` is not the test report itself.
3. `doctor_verdict_receipt` is not the raw doctor output.
4. `coach_feedback_issued_receipt` is not the critique payload itself.

Rule:

1. receipts prove a lawful step happened,
2. proofs support why the step was allowed or what it concluded,
3. checkpoints preserve where resumable work can continue,
4. checkpoints may reference receipts and proofs but must not satisfy receipt or proof obligations by themselves.

Checkpoint rule:

1. `checkpoint_written_receipt` proves that a durability boundary was written,
2. `execution_snapshot` is the immutable snapshot artifact itself,
3. projections derived from that checkpoint remain projections, not proof,
4. future replay or fork from checkpoint must create new runtime lineage, not rewrite the original receipt chain.

## 7. Audit Boundary

Audit may store:

1. adapter logs,
2. latency,
3. cost,
4. execution trace detail.

Audit must not replace:

1. state,
2. receipts,
3. proof artifacts.

## 8. Initial Machine-To-Receipt Mapping

### 8.1 task_lifecycle

Required receipts:

1. `task_state_changed_receipt`

Typical proof dependencies:

1. `artifact_manifest`
2. `verification_evidence`
3. `approval_note` when required

### 8.2 execution_plan

Required receipts:

1. `execution_started_receipt`
2. `execution_completed_receipt`
3. `execution_failed_receipt`
4. `execution_paused_receipt`
5. `execution_resumed_receipt`
6. `checkpoint_written_receipt`

Typical proof dependencies:

1. `artifact_manifest`
2. `execution_log_excerpt`
3. `execution_snapshot` when an execution-time checkpoint is required

### 8.3 route_progression

Required receipts:

1. `route_resolved_receipt`
2. `assignment_requested_receipt`
3. `agent_assigned_receipt`
4. `lease_issued_receipt`
5. `dispatch_started_receipt`
6. `fallback_selected_receipt`
7. `escalation_triggered_receipt`

Typical proof dependencies:

1. `policy_evaluation_snapshot`
2. future `execution_snapshot` or equivalent checkpoint proof when route replay/debug is explicitly required

### 8.4 coach_lifecycle

Required receipts:

1. `coach_requested_receipt`
2. `coach_feedback_issued_receipt`
3. `coach_rework_receipt`

Typical proof dependencies:

1. `review_findings`
2. `diff_summary`

### 8.5 verification_lifecycle

Required receipts:

1. `verification_requested_receipt`
2. `verifier_assigned_receipt`
3. `verification_aggregate_receipt`
4. one of:
   - `verification_passed_receipt`
   - `verification_failed_receipt`
   - `verification_inconclusive_receipt`

Typical proof dependencies:

1. `verification_evidence`
2. `test_report`
3. `artifact_manifest`

### 8.6 approval_lifecycle

Required receipts:

1. `approval_requested_receipt`
2. one of:
   - `approval_received_receipt`
   - `approval_rejected_receipt`
   - `approval_expired_receipt`
   - `approval_escalated_receipt`

Typical proof dependencies:

1. `approval_note`
2. `verification_evidence`

### 8.7 boot_migration_gate

Required receipts:

1. `compat_checked_receipt`
2. `migration_required_receipt` when applicable
3. `doctor_verdict_receipt` when applicable
4. one of:
   - `boot_allowed_receipt`
   - `boot_blocked_receipt`

Typical proof dependencies:

1. `doctor_output`
2. `migration_report`
3. `policy_evaluation_snapshot`
