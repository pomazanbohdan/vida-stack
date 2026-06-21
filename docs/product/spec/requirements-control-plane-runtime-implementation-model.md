# Requirements Control Plane Runtime Implementation Model

Status: active implementation model

Purpose: define the first runtime implementation model for the VIDA DB-first requirements, change-control, DocFlow validation, TaskFlow execution, project wiki projection, external adapter, and web interface control plane.

## 1. Scope

This model implements the product law in [requirements-documentation-control-plane.md](requirements-documentation-control-plane.md). The DB-first entity graph, required record fields, lifecycle states, indexes, projection receipts, validation verdict rows, and migration posture are owned by [requirements-control-plane-state-model.md](requirements-control-plane-state-model.md).

The v1 implementation scope is:

1. store controlled requirements and documentation state in the VIDA state spine first,
2. project reviewable Markdown and JSONL artifacts into the repository,
3. keep TaskFlow as the execution authority,
4. keep DocFlow as the documentation and baseline validation participant,
5. expose read-only web views before write-capable web workflows,
6. support GitHub, Jira, Azure DevOps, Confluence, and wiki systems through adapters instead of replacing VIDA authority.

Non-goals for v1:

1. no silent bidirectional merge between filesystem and DB,
2. no external ALM system as canonical authority by default,
3. no write-capable web UI until read-only state, projection, and validation are proven,
4. no mandatory source-document checklist for project inception.

## 2. Research Basis

External practice points used by this model:

1. requirements management is a trace graph, not a document pile;
2. ALM systems separate epics, features, backlog items, tasks, bugs, risks, approvals, and change records;
3. modern AI-agent frameworks separate agents, tools, handoffs, durable state, guardrails, human review, and trace evidence;
4. documentation systems work best when current pages, decision records, reference pages, and change history have separate authority;
5. public wiki and internal delivery wiki views require separate visibility policy;
6. graph, full-text, and vector retrieval should enrich discovery without replacing lifecycle authority.

VIDA already has matching primitives:

1. TaskFlow tasks, dependencies, run graph, dispatch packets, receipts, and closure evidence;
2. DocFlow artifact registry, relations, readiness rows, closeout verdicts, and changelog checks;
3. Surreal-backed state-spine bootstrap contracts;
4. VIDA operation registry posture classes for read, plan, apply, and admin operations;
5. repository projection through Markdown, JSONL changelogs, maps, catalogs, and Git history.

## 3. Authority Stack

The runtime authority order is:

```txt
VIDA DB state
-> TaskFlow execution state
-> DocFlow validation verdicts
-> repository projection
-> Git review and history
-> external adapter mirrors
```

Rules:

1. DB records own lifecycle state, approvals, waivers, trace links, visibility, validation verdicts, and adapter mappings.
2. Filesystem projection owns human review shape, changelog sidecars, and import/export artifacts.
3. Git owns reviewable diff and historical reconstruction, not live lifecycle authority.
4. External tools own their local mirrored objects only.

## 4. Runtime Layers

### Layer 1: Control Domain

Owns DB entities, lifecycle state, relation graph, approvals, waivers, validation verdicts, and external mapping records.

### Layer 2: Extraction And Intake

Owns direct operator intake, direct CR creation, optional source-document registration, candidate extraction, source-location provenance, and AI-generated candidate proposals.

### Layer 3: Review And Baseline

Owns human review state, gap assessment, waiver legality, baseline approval, baseline activation, and supersession.

### Layer 4: Execution Binding

Owns TaskFlow-linked delivery work, proof targets, run-graph dispatch, closure evidence, and release handoff.

### Layer 5: Projection

Owns repository Markdown, changelog sidecars, control projections, wiki projections, and external adapter packaging.

### Layer 6: Validation

Owns DocFlow validation verdicts for baseline readiness, CR closure, projection correctness, PR readiness, release admission, and public visibility.

### Layer 7: Web Interface

Owns read-only operator and stakeholder views first, then write-capable workflows after the read model is stable.

## 5. Core Entity Types

The first runtime entity set is:

1. `ProjectRecord`
2. `OriginRecord`
3. `SourceDocumentRecord`
4. `RequirementCandidate`
5. `RequirementItem`
6. `FeatureItem`
7. `UseCaseItem`
8. `ChangeRequest`
9. `DeliveryItem`
10. `TraceLink`
11. `DecisionRecord`
12. `GapRecord`
13. `RiskItem`
14. `WaiverRecord`
15. `ApprovalReceipt`
16. `VisibilityApprovalReceipt`
17. `DocFlowValidationVerdict`
18. `ProjectWikiProjection`
19. `WikiPageProjection`
20. `Baseline`
21. `ReleaseBaseline`
22. `ExternalAdapterMapping`
23. `ProjectionReceipt`

`SourceDocumentRecord` is optional. Every active controlled item must link to at least one `OriginRecord` or carry an approved no-origin rationale.

## 6. Entity Classes

### OriginRecord

Represents the provenance root for direct input, meeting notes, source files, adapter imports, agent-discovered gaps, or explicit no-origin rationale.

Minimum fields:

1. `id`
2. `project_id`
3. `origin_kind`
4. `title`
5. `summary`
6. `source_ref`
7. `created_by`
8. `created_at`
9. `trust_level`
10. `review_status`

### RequirementItem

Represents an approved functional or non-functional requirement.

Minimum fields:

1. `id`
2. `project_id`
3. `kind=functional|nfr`
4. `statement`
5. `rationale`
6. `acceptance_criteria`
7. `priority`
8. `status`
9. `baseline_id`
10. `origin_ids`

### FeatureItem

Represents a product value unit.

Minimum fields:

1. `id`
2. `project_id`
3. `level=capability|feature|pbi`
4. `title`
5. `user_value`
6. `status`
7. `priority`
8. `parent_feature_id`
9. `requirement_ids`
10. `delivery_item_ids`

### ChangeRequest

Represents a governance envelope for changing an approved baseline.

Minimum fields:

1. `id`
2. `project_id`
3. `requested_change`
4. `business_reason`
5. `affected_item_ids`
6. `impact_analysis`
7. `approval_state`
8. `target_baseline`
9. `taskflow_ids`
10. `docflow_verdict_ids`

### ProjectWikiProjection

Represents a generated reader-facing wiki view.

Minimum fields:

1. `id`
2. `project_id`
3. `view=internal|public`
4. `status=draft|ready|published|blocked`
5. `visibility_receipt_id`
6. `source_baseline_id`
7. `page_ids`
8. `generated_at`
9. `projection_root`
10. `external_mapping_ids`

## 7. State Spine Tables

The Surreal state-spine table set is owned by [requirements-control-plane-state-model.md](requirements-control-plane-state-model.md). The first runtime implementation should extend the state spine with:

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

Indexes:

1. `project_id`
2. lifecycle `status`
3. `baseline_id`
4. `origin_ids`
5. `external_adapter_mapping.provider`
6. `external_adapter_mapping.external_id`
7. relation source and target ids
8. full-text fields for title, statement, summary, and acceptance criteria
9. future vector embeddings for semantic discovery

State migration rule:

1. add tables behind a schema-versioned migration,
2. keep existing TaskFlow tables backward-compatible,
3. do not rename existing task records into control-plane records,
4. connect records through explicit IDs and relation edges.
5. keep table names and record fields aligned with the state model owner doc.

## 8. Relation Graph

Relation edges must support:

1. `originates_from`
2. `refines`
3. `satisfies`
4. `implements`
5. `verifies`
6. `documents`
7. `affects`
8. `depends_on`
9. `supersedes`
10. `migrates_from`
11. `blocks`
12. `waives`
13. `approved_by`
14. `visible_in`
15. `projected_to`
16. `mapped_to_external`

DocFlow artifact relation kinds may be reused for filesystem artifacts. Control-plane relation edges should stay explicit DB records so web, CLI, DocFlow, and adapters can query the same graph.

## 9. TaskFlow Integration

TaskFlow remains execution authority.

Task taxonomy should be extended with provider-neutral work item kinds:

1. `requirement`
2. `feature`
3. `pbi`
4. `use_case`
5. `change_request`
6. `decision_record`
7. `risk`
8. `baseline`
9. `wiki_projection`

Binding rule:

1. `pbi`, `change_request`, `delivery_item`, `wiki_projection`, and `baseline` may become TaskFlow-bindable.
2. `origin`, `source_document`, `requirement_candidate`, `approval_receipt`, and `validation_verdict` are not execution work by default.
3. Existing task records remain valid.
4. Provider mappings must preserve external issue type and VIDA canonical kind.

## 10. DocFlow Integration

DocFlow is a validator participant, not a passive scanner.

DocFlow must emit or project `DocFlowValidationVerdict` records for:

1. origin and provenance completeness,
2. profile-required document inventory,
3. candidate extraction traceability,
4. requirement and use-case completeness,
5. gap and waiver legality,
6. CR impact-analysis completeness,
7. repository projection metadata and footer posture,
8. sibling changelog sidecars,
9. map and catalog registration,
10. project wiki projection readiness,
11. public visibility approval readiness,
12. PR readiness when controlled docs change,
13. release baseline admission.

DocFlow verdicts must be queryable from TaskFlow, web UI, release gates, and external adapters.

## 11. CLI And Operation Surface

The first operator surface should use a new `vida product` family:

```txt
vida product origin create|list|show
vida product source import|list|show
vida product candidate extract|list|approve|reject
vida product requirement create|list|show|approve|supersede
vida product feature create|list|show|link
vida product use-case create|list|show|link
vida product cr create|impact|approve|close
vida product baseline plan|activate|show
vida product wiki project|publish|show
vida product adapter map|sync-plan|show
```

Namespace rule:

1. `vida product` is the v1 public namespace for DB-first requirements, CR, baseline, wiki, and external adapter operations.
2. `vida project` remains the project registry, activation, resolution, and status surface; it must not absorb product requirements, CR, baseline, wiki, or adapter control operations in v1.
3. Runtime/domain internals may keep `control_*` table names and `control_plane` module naming until a separate implementation migration intentionally renames them.
4. `vida control` is not a v1 alias unless a future accepted change request adds it with compatibility and help-text rules.

Operation posture:

1. `list`, `show`, and `sync-plan` are read-only.
2. `impact`, `project`, and `baseline plan` are plan-only.
3. `create`, `approve`, `activate`, `close`, and `publish` are apply operations with receipts.
4. adapter import/export and schema migration are admin operations.

## 12. Web Interface

The first web interface is read-only.

Required views:

1. Control dashboard
2. Origin intake and provenance graph
3. Requirements list and detail
4. Features/PBI hierarchy
5. Use cases
6. Change requests
7. Gaps and risks
8. Baselines
9. DocFlow validation verdicts
10. TaskFlow delivery links
11. Wiki projections
12. External adapter mappings

Write-capable web workflows are a later phase and must use the same apply operations, receipts, validation gates, and visibility approvals as CLI.

## 13. Repository And Wiki Projection

Projection outputs:

1. canonical product/spec docs under `docs/product/spec/**`,
2. DB-backed control projections under `docs/product/control/**`,
3. generated wiki pages under `docs/product/wiki/internal/**` and `docs/product/wiki/public/**`,
4. sibling `*.changelog.jsonl`,
5. map and catalog updates,
6. projection receipts.

Generated files must carry:

```yaml
projection_mode: generated
edit_authority: db
```

Manual edits to generated projection files must be reconciled through an explicit import/reconcile command. Silent merge is forbidden.

## 14. External Adapter Contract

Every external mapping must store:

1. `provider`
2. `external_id`
3. `external_url`
4. `provider_object_type`
5. `vida_object_type`
6. `vida_object_id`
7. `sync_direction`
8. `last_sync_state`
9. `last_sync_at`
10. `conflict_state`

Default adapter order:

1. GitHub Issues, Projects, Releases, and Wiki mirror,
2. Azure Boards and Azure Wiki,
3. Jira and Confluence,
4. custom ALM/wiki adapters.

External systems may initiate intake, but VIDA must record an `OriginRecord` and preserve VIDA object identity before activation.

## 15. Lifecycle Gates

Project inception:

```txt
origin_intake_received
-> candidate_graph_created
-> human_review_in_progress
-> gap_assessment_created
-> docflow_validation_requested
-> baseline_approval_requested
-> active_baseline
```

Post-baseline change:

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
```

Publish gate:

```txt
wiki_projection_created
-> internal_projection_ready
-> visibility_review
-> visibility_approval_receipt
-> public_projection_published
```

## 16. AI Agent Boundary

AI agents may:

1. classify origins,
2. extract candidates,
3. draft requirements, use cases, and acceptance criteria,
4. propose trace links,
5. propose gaps and risks,
6. draft impact analysis,
7. draft projections.

AI agents may not:

1. activate baselines,
2. approve waivers,
3. approve public visibility,
4. silently change canonical DB state,
5. close lifecycle gates without receipts,
6. treat untrusted source text as authority without provenance and review.

## 17. Rollout Plan

Slice 1: contracts and schema.

1. add control-plane contract structs,
2. add state-spine table constants,
3. add migration bootstrap tests,
4. add provider-neutral taxonomy kinds.

Slice 2: CLI read model.

1. add `vida product list/show` read-only commands,
2. expose DB records and trace graph,
3. add JSON and compact output tests.

Slice 3: DocFlow verdict integration.

1. emit `DocFlowValidationVerdict`,
2. gate baseline and CR closure,
3. validate generated projection footer posture.

Slice 4: repository/wiki projection.

1. generate control projections,
2. generate internal wiki pages,
3. require visibility receipt for public projection.

Slice 5: web read-only interface.

1. expose dashboard and detail pages,
2. show graph, validation, delivery, and adapter state,
3. keep writes disabled until apply operations are proven.

Slice 6: write workflows and external sync.

1. add apply flows with receipts,
2. add GitHub adapter first,
3. add Jira/Azure/Confluence adapters after sync conflict rules are proven.

## 18. Proof Targets

Required proof for implementation:

1. contract serialization round-trip tests,
2. Surreal bootstrap schema tests,
3. TaskFlow taxonomy mapping tests,
4. DocFlow validation verdict tests,
5. CLI read-only JSON and compact output tests,
6. projection generation tests,
7. map/catalog/changelog DocFlow checks,
8. web read-only route smoke tests when the web surface exists,
9. adapter mapping conflict tests.

## 19. Related Documents

1. [requirements-documentation-control-plane.md](requirements-documentation-control-plane.md) owns the product law.
2. [requirements-control-plane-state-model.md](requirements-control-plane-state-model.md) owns the DB-first state model.
3. [operational-state-and-synchronization-model.md](operational-state-and-synchronization-model.md) owns DB/filesystem/Git synchronization law.
4. [taskflow-protocol-runtime-binding-model.md](taskflow-protocol-runtime-binding-model.md) owns TaskFlow runtime binding.
5. [project-documentation-law.md](project-documentation-law.md) owns documentation authority lanes.
6. [project-document-naming-law.md](project-document-naming-law.md) owns naming and tree rules.
7. [canonical-relation-law.md](canonical-relation-law.md) owns canonical artifact relation posture.
8. This document owns the first runtime implementation model for the requirements control plane.

-----
artifact_path: product/spec/requirements-control-plane-runtime-implementation-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-06-18'
schema_version: '1'
status: canonical
source_path: docs/product/spec/requirements-control-plane-runtime-implementation-model.md
created_at: '2026-06-18T00:00:00+03:00'
updated_at: 2026-06-18T00:00:00+03:00
changelog_ref: requirements-control-plane-runtime-implementation-model.changelog.jsonl
