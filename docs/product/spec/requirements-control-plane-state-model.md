# Requirements Control Plane State Model

Status: active implementation spec

Purpose: define the DB-first state model for the `vida product` requirements, change-control, baseline, project wiki, DocFlow validation, TaskFlow binding, repository projection, and external adapter control plane.

## 1. Scope

This document owns the v1 state model for the product control plane defined by [requirements-documentation-control-plane.md](requirements-documentation-control-plane.md) and implemented by [requirements-control-plane-runtime-implementation-model.md](requirements-control-plane-runtime-implementation-model.md).

The model is DB-first:

1. lifecycle state, relation state, approvals, waivers, validation verdicts, projection receipts, and adapter mappings live in the VIDA state spine;
2. repository Markdown and JSONL are projections and review surfaces;
3. TaskFlow remains the execution authority for work;
4. DocFlow remains the validator participant for documentation, baseline, projection, and release readiness;
5. external ALM/wiki systems mirror or import/export state through adapters.

Non-goals for v1:

1. no external system as canonical authority by default,
2. no silent DB/filesystem bidirectional merge,
3. no mandatory source-document inventory for project inception,
4. no migration that renames existing TaskFlow records into product-control records.

## 2. Identity And Authority

Every control-plane row uses these common fields unless the table is explicitly a receipt table:

1. `id`
2. `project_id`
3. `record_kind`
4. `status`
5. `created_at`
6. `created_by`
7. `updated_at`
8. `updated_by`
9. `baseline_id`
10. `origin_ids`
11. `docflow_verdict_ids`
12. `taskflow_ids`
13. `projection_receipt_ids`
14. `external_mapping_ids`

Authority rules:

1. DB rows own current lifecycle and relation truth.
2. Git commits own review lineage only.
3. Markdown files under `docs/product/control/**` and `docs/product/wiki/**` are generated projections unless explicitly marked otherwise.
4. Human approval is represented by `ApprovalReceipt`, `VisibilityApprovalReceipt`, or `WaiverRecord`; free text in a projection is not approval truth.
5. A controlled item can enter an active baseline only when it links to at least one `OriginRecord` or carries an approved no-origin rationale.

## 3. Table Set

The v1 Surreal state-spine table set is:

```txt
control_project
control_origin
control_source_document
control_requirement_candidate
control_requirement
control_feature
control_use_case
control_change_request
control_delivery_item
control_trace_link
control_decision
control_gap
control_risk
control_waiver
control_approval_receipt
control_visibility_approval_receipt
control_docflow_validation_verdict
control_project_wiki_projection
control_wiki_page_projection
control_baseline
control_release_baseline
control_external_adapter_mapping
control_projection_receipt
```

The tables are schemaless in the Surreal v1 migration posture, but runtime structs and validators must treat the required fields below as the typed contract.

## 4. Core Records

### ProjectRecord

Represents one product/project control-plane scope.

Required fields:

1. `id`
2. `title`
3. `status=new|intake|review|active_baseline|active_with_waivers|archived`
4. `profile_id`
5. `active_baseline_id`
6. `default_visibility=internal|public`
7. `wiki_projection_ids`
8. `adapter_policy_id`

### OriginRecord

Represents provenance for direct input, source files, imported external objects, meeting notes, agent-discovered gaps, or approved no-origin rationale.

Required fields:

1. `id`
2. `project_id`
3. `origin_kind=direct_entry|direct_cr|direct_pbi|direct_requirement|direct_use_case|direct_risk|direct_decision|direct_baseline|source_document|document_set|meeting_note|agent_gap|external_import|no_origin_rationale`
4. `title`
5. `summary`
6. `source_ref`
7. `trust_level=untrusted|operator_provided|reviewed|approved`
8. `review_status=registered|extracted|reviewed|accepted|rejected|superseded`
9. `source_document_ids`
10. `external_mapping_ids`

### SourceDocumentRecord

Represents an optional carrier for an origin.

Required fields:

1. `id`
2. `project_id`
3. `origin_id`
4. `document_kind=tz|brief|spec|meeting_note|wiki_page|confluence_page|azure_wiki_page|github_issue|jira_issue|custom`
5. `source_uri`
6. `content_hash`
7. `ingest_status=registered|parsed|extracted|blocked|superseded`
8. `source_locations`

Source documents are optional. Direct CR, PBI, requirement, use case, risk, decision, and baseline rows are valid when their `OriginRecord` captures provenance or approved no-origin rationale.

### RequirementCandidate

Represents extracted or drafted candidate content before approval.

Required fields:

1. `id`
2. `project_id`
3. `origin_id`
4. `candidate_kind=requirement|feature|use_case|risk|gap|decision`
5. `statement`
6. `source_location`
7. `extractor=operator|agent|adapter`
8. `confidence`
9. `review_status=proposed|in_review|accepted|rejected|merged|superseded`
10. `accepted_record_id`

## 5. Controlled Product Objects

### RequirementItem

Represents an approved functional or non-functional requirement.

Required fields:

1. `id`
2. `project_id`
3. `kind=functional|nfr`
4. `statement`
5. `rationale`
6. `acceptance_criteria`
7. `priority=p0|p1|p2|p3`
8. `status=draft|proposed|approved|active|superseded|deprecated|rejected`
9. `baseline_id`
10. `origin_ids`
11. `trace_link_ids`
12. `owner`

### FeatureItem

Represents a product value unit.

Required fields:

1. `id`
2. `project_id`
3. `level=capability|feature|pbi`
4. `title`
5. `user_value`
6. `status=draft|proposed|approved|active|superseded|deprecated|rejected`
7. `priority=p0|p1|p2|p3`
8. `parent_feature_id`
9. `requirement_ids`
10. `use_case_ids`
11. `delivery_item_ids`

### UseCaseItem

Represents an actor/scenario behavior description linked to requirements and features.

Required fields:

1. `id`
2. `project_id`
3. `title`
4. `actor`
5. `trigger`
6. `preconditions`
7. `main_flow`
8. `alternate_flows`
9. `postconditions`
10. `status=draft|proposed|approved|active|superseded|deprecated|rejected`
11. `requirement_ids`
12. `feature_ids`

### DecisionRecord

Represents an accepted, rejected, or superseded product/control decision.

Required fields:

1. `id`
2. `project_id`
3. `decision`
4. `context`
5. `accepted_by`
6. `decision_status=proposed|accepted|rejected|superseded`
7. `affected_record_ids`
8. `baseline_id`

## 6. Change And Delivery Objects

### ChangeRequest

Represents the governance envelope for changing an approved baseline.

Required fields:

1. `id`
2. `project_id`
3. `requested_change`
4. `business_reason`
5. `status=created|impact_analysis|affected_items_linked|approval_requested|approved|rejected|taskflow_execution|docflow_validation|repository_projection|baseline_updated|closed`
6. `affected_record_ids`
7. `impact_analysis`
8. `approval_receipt_ids`
9. `waiver_ids`
10. `target_baseline_id`
11. `taskflow_ids`
12. `docflow_verdict_ids`

### DeliveryItem

Represents work that binds product/control state to TaskFlow execution.

Required fields:

1. `id`
2. `project_id`
3. `delivery_kind=implementation|verification|documentation|migration|release`
4. `status=planned|taskflow_bound|in_execution|proof_ready|verified|closed|blocked`
5. `taskflow_ids`
6. `proof_targets`
7. `proof_receipt_ids`
8. `change_request_id`
9. `affected_record_ids`

`DeliveryItem` is not a TaskFlow replacement. It records product-control linkage to TaskFlow tasks, run graphs, receipts, and closure evidence.

## 7. Relation Graph

`control_trace_link` owns explicit graph edges between controlled records.

Required fields:

1. `id`
2. `project_id`
3. `source_type`
4. `source_id`
5. `target_type`
6. `target_id`
7. `relation_kind=derives_from|satisfies|implements|verifies|blocks|supersedes|duplicates|depends_on|documents|projects_to|maps_to_external`
8. `direction=source_to_target|bidirectional`
9. `status=proposed|active|superseded|rejected`
10. `baseline_id`
11. `origin_id`
12. `created_by`

Required graph paths:

1. `OriginRecord -> RequirementCandidate -> RequirementItem|FeatureItem|UseCaseItem`
2. `RequirementItem -> FeatureItem -> UseCaseItem -> DeliveryItem`
3. `ChangeRequest -> affected controlled items -> DeliveryItem -> TaskFlow task`
4. `Baseline -> included controlled items -> DocFlowValidationVerdict`
5. `Controlled item -> ProjectionReceipt -> repository path`
6. `Controlled item -> ExternalAdapterMapping -> provider object`

## 8. Baseline, Gap, Risk, And Waiver

### Baseline

Required fields:

1. `id`
2. `project_id`
3. `baseline_kind=inception|change|release`
4. `status=draft|validation_requested|approval_requested|active|active_with_waivers|superseded|archived`
5. `included_record_ids`
6. `open_gap_ids`
7. `waiver_ids`
8. `docflow_verdict_ids`
9. `activated_by`
10. `activated_at`

### GapRecord

Required fields:

1. `id`
2. `project_id`
3. `gap_kind=missing_origin|missing_requirement|missing_use_case|missing_acceptance_criteria|missing_validation|missing_projection|missing_visibility_approval|profile_required_document_missing|custom`
4. `severity=critical|high|medium|low`
5. `status=open|waived|resolved|superseded`
6. `affected_record_ids`
7. `baseline_id`

### RiskItem

Required fields:

1. `id`
2. `project_id`
3. `risk_statement`
4. `impact`
5. `probability`
6. `mitigation`
7. `status=open|accepted|mitigated|closed|superseded`
8. `affected_record_ids`

### WaiverRecord

Required fields:

1. `id`
2. `project_id`
3. `waived_gap_id`
4. `risk_id`
5. `reason`
6. `approver`
7. `expires_at`
8. `review_condition`
9. `baseline_id`
10. `docflow_verdict_id`

Critical gaps may enter a baseline only through an active waiver record.

## 9. Validation And Projection

### DocFlowValidationVerdict

Required fields:

1. `id`
2. `project_id`
3. `subject_type`
4. `subject_id`
5. `gate=origin|candidate|requirement|use_case|change_request|baseline|repository_projection|wiki_projection|public_visibility|pr_readiness|release_baseline`
6. `status=pass|blocked|waived|not_applicable`
7. `blocker_codes`
8. `evidence_refs`
9. `profile_id`
10. `baseline_id`
11. `projection_receipt_id`
12. `checked_at`
13. `checked_by`

DocFlow verdict rows are queryable by TaskFlow, web UI, release gates, and external adapters.

### ProjectionReceipt

Required fields:

1. `id`
2. `project_id`
3. `projection_kind=control_markdown|wiki_internal|wiki_public|adapter_export|changelog`
4. `source_record_ids`
5. `target_path`
6. `content_hash`
7. `projection_mode=generated`
8. `edit_authority=db`
9. `docflow_verdict_id`
10. `projected_at`

Repository projection targets:

1. `ProjectRecord`, `Baseline`, `DecisionRecord`, `ChangeRequest`, `RequirementItem`, `FeatureItem`, `UseCaseItem`, `GapRecord`, `RiskItem`, and `WaiverRecord` project to `docs/product/control/**`.
2. `ProjectWikiProjection` and `WikiPageProjection` project to `docs/product/wiki/internal/**` or `docs/product/wiki/public/**`.
3. Every generated Markdown projection has a sibling `*.changelog.jsonl` or an owning projection receipt that names the generated changelog path.
4. Manual edits to generated projection files must be imported through explicit reconcile commands before DB state changes.

## 10. Wiki Projection Records

### ProjectWikiProjection

Required fields:

1. `id`
2. `project_id`
3. `view=internal|public`
4. `status=draft|validation_requested|ready|published|blocked|superseded`
5. `source_baseline_id`
6. `page_ids`
7. `visibility_receipt_id`
8. `projection_receipt_ids`
9. `external_mapping_ids`

### WikiPageProjection

Required fields:

1. `id`
2. `project_id`
3. `projection_id`
4. `page_kind=overview|requirements|features|use_cases|change_log|risks|decisions|delivery_status|release_baseline|custom`
5. `visibility=internal|public`
6. `source_record_ids`
7. `target_path`
8. `status=draft|ready|published|blocked|superseded`

Public wiki pages require `VisibilityApprovalReceipt`. Internal pages may include TaskFlow delivery state; public pages must not include delivery status in v1.

## 11. External Adapter Mapping

`control_external_adapter_mapping` owns provider identity and sync state.

Required fields:

1. `id`
2. `project_id`
3. `provider=github|jira|confluence|azure_boards|azure_wiki|github_wiki|custom`
4. `provider_object_type`
5. `provider_object_id`
6. `provider_url`
7. `vida_object_type`
8. `vida_object_id`
9. `sync_direction=import_only|export_only|bidirectional_mirror`
10. `sync_state=mapped|sync_planned|sync_ready|synced|conflict|suspended`
11. `last_sync_at`
12. `conflict_state`

Adapter mapping rules:

1. provider-specific terms are not VIDA owner types;
2. Azure PBI, Jira Story, and GitHub Issue may map to `FeatureItem(level=pbi)`;
3. Azure Change Request, Jira Change, and GitHub CR issue may map to `ChangeRequest`;
4. Confluence, Azure Wiki, and GitHub Wiki pages may map to `WikiPageProjection`;
5. conflicts block outbound writes until a human decision or adapter-specific reconcile receipt exists.

## 12. Indexes And Queries

Minimum v1 indexes:

1. `project_id`
2. `status`
3. `record_kind`
4. `baseline_id`
5. `origin_ids`
6. `source_document_ids`
7. `parent_feature_id`
8. `change_request_id`
9. `taskflow_ids`
10. `docflow_verdict_ids`
11. `provider + provider_object_id`
12. `vida_object_type + vida_object_id`
13. `source_type + source_id`
14. `target_type + target_id`
15. full-text fields: `title`, `statement`, `summary`, `requested_change`, `business_reason`, and `acceptance_criteria`

Future vector search may be added for semantic discovery, but lifecycle gates must continue to use typed state and relation rows rather than embedding similarity.

## 13. Lifecycle Gates

Inception baseline path:

```txt
new_project
-> origin_intake_received
-> origin_records_registered
-> candidate_graph_created
-> human_review_in_progress
-> gap_assessment_created
-> docflow_validation_requested
-> baseline_approval_requested
-> active_baseline|active_with_waivers
```

Post-baseline change path:

```txt
change_request_created
-> impact_analysis
-> affected_items_linked
-> approval_or_waiver
-> taskflow_execution
-> docflow_validation
-> repository_projection
-> pull_request_or_release_proof
-> baseline_updated
-> closed
```

Fail-closed states:

```txt
rejected_intake
blocked_by_critical_gaps
blocked_by_docflow
blocked_by_missing_origin
blocked_by_projection_drift
blocked_by_external_conflict
superseded_draft
archived_project
```

## 14. Migration Posture

The first implementation migration must:

1. add the control-plane tables behind a schema-versioned state-spine migration;
2. update the state-spine manifest with the new control-plane entity surfaces;
3. keep TaskFlow task, dependency, attempt, run-graph, receipt, and scheduler rows backward-compatible;
4. avoid backfilling product-control rows from existing TaskFlow rows unless an explicit import task is created;
5. write projection receipts for generated repository files;
6. leave filesystem-only historical docs as source/projection evidence until imported through `vida product` commands.

Compatibility rule:

1. old TaskFlow tasks remain valid execution records;
2. product-control records link to TaskFlow by `taskflow_ids`;
3. no TaskFlow command becomes a product-control command by alias;
4. `vida product` is the public v1 namespace for product-control operations.

## 15. Related Documents

1. [requirements-documentation-control-plane.md](requirements-documentation-control-plane.md) owns the product law.
2. [requirements-control-plane-runtime-implementation-model.md](requirements-control-plane-runtime-implementation-model.md) owns CLI/API, web, rollout, and runtime integration posture.
3. [project-documentation-law.md](project-documentation-law.md) owns documentation authority lanes.
4. [operational-state-and-synchronization-model.md](operational-state-and-synchronization-model.md) owns DB/filesystem/Git synchronization law.
5. [taskflow-protocol-runtime-binding-model.md](taskflow-protocol-runtime-binding-model.md) owns TaskFlow runtime binding.
6. [canonical-relation-law.md](canonical-relation-law.md) owns canonical relation posture.

-----
artifact_path: product/spec/requirements-control-plane-state-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-06-18'
schema_version: '1'
status: canonical
source_path: docs/product/spec/requirements-control-plane-state-model.md
created_at: '2026-06-18T00:00:00+03:00'
updated_at: 2026-06-18T00:00:00+03:00
changelog_ref: requirements-control-plane-state-model.changelog.jsonl
