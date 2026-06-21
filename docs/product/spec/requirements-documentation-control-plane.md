# Requirements Documentation Control Plane

Status: active product law

Purpose: define the VIDA product model for DB-first requirements, documentation, baseline, change-control, validation, and repository projection.

## 1. Core Rule

VIDA owns a requirements and documentation control plane.

The control plane must keep:

1. database truth as the operational authority,
2. repository Markdown and JSONL as the human-readable projection,
3. TaskFlow as execution authority,
4. DocFlow as validation authority for documentation and baseline readiness,
5. external ALM/wiki systems as adapters unless explicitly promoted by project profile.

## 2. State Authority

The authoritative state order is:

1. DB state for lifecycle, graph, approvals, waivers, traceability, validation verdicts, visibility, and external adapter mappings,
2. filesystem projection for Markdown specs, control records, wiki pages, changelog sidecars, maps, catalogs, and import/export artifacts,
3. Git lineage for review, audit, and historical reconstruction.

Filesystem edits may be imported back into DB state only through explicit sync/reconcile commands with receipts.

Silent bidirectional merge is forbidden.

## 3. Object Model

The first canonical object set is:

1. `Project`
2. `OriginRecord`
3. `SourceDocument`
4. `RequirementCandidate`
5. `RequirementItem`
6. `FeatureItem`
7. `UseCaseItem`
8. `DeliveryItem`
9. `ChangeRequest`
10. `TraceLink`
11. `Gap`
12. `RiskItem`
13. `Waiver`
14. `ApprovalReceipt`
15. `VisibilityApprovalReceipt`
16. `DocFlowValidationVerdict`
17. `ProjectWikiProjection`
18. `WikiPageProjection`
19. `Baseline`
20. `ReleaseBaseline`

Provider-specific terms are adapter projections, not VIDA owner types.

## 4. PBI, CR, NFR, And Use Case Rule

`PBI` and `CR` are not competing names for the same thing.

VIDA internal terms are:

1. `FeatureItem`
   - product value unit with `level=capability|feature|pbi`,
2. `RequirementItem`
   - approved functional or non-functional requirement with `kind=functional|nfr`,
3. `UseCaseItem`
   - actor/scenario/flow description linked to requirements and features,
4. `ChangeRequest`
   - governance envelope for changing an already approved baseline,
5. `DeliveryItem`
   - executable implementation or verification work, normally bound to TaskFlow.

Adapter examples:

1. `FeatureItem(level=pbi)` maps to Azure PBI, Jira Story, or GitHub Issue.
2. `ChangeRequest` maps to Azure Change Request, Jira Change, or GitHub CR issue.
3. `DeliveryItem` maps to Azure Task, Jira Sub-task, GitHub Issue, or TaskFlow task.

## 5. Origin-First Inception Lifecycle

The system must support starting a project from any origin, not only from a document set.

Valid inception origins include:

1. direct operator entry,
2. direct `ChangeRequest`,
3. direct requirement, feature, PBI, use case, risk, decision, or baseline item,
4. uploaded or repository-provided source documents,
5. a single TZ or arbitrary document set,
6. meeting notes or stakeholder input,
7. agent-discovered gaps,
8. imported GitHub, Jira, Azure DevOps, Confluence, Wiki, or custom adapter items,
9. explicit no-origin rationale approved for the project profile.

The baseline inception lifecycle is:

```txt
new_project
-> origin_intake_received
-> origin_records_registered
-> candidate_graph_created
-> human_review_in_progress
-> gap_assessment_created
-> docflow_validation_requested
-> baseline_approval_requested
-> active_baseline
```

Alternative states:

```txt
rejected_intake
blocked_by_critical_gaps
active_with_waivers
superseded_draft
archived_project
```

No origin intake becomes active baseline until review, DocFlow validation, and approval or waiver receipts exist.

## 6. Post-Baseline Change Lifecycle

After an active baseline exists, baseline-changing work flows through `ChangeRequest`.

The change lifecycle is:

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

A `ChangeRequest` may create or update `OriginRecord`, `RequirementItem`, `FeatureItem`, `UseCaseItem`, `DeliveryItem`, `Gap`, `RiskItem`, `Waiver`, `TraceLink`, and projection artifacts.

## 7. Origin And Source Rule

`OriginRecord` is the root provenance layer.

`SourceDocument` is optional and is only one possible origin carrier.

Rules:

1. Every activated controlled item must link to at least one `OriginRecord` or carry an approved no-origin rationale.
2. Directly created CR, requirement, feature, PBI, use case, risk, decision, and baseline items are valid without a source document.
3. Imported source files create registered `SourceDocument` records before extraction.
4. Extracted candidates must retain provenance to the origin, source location when available, extractor, extraction time, confidence, and review state.
5. Missing Core 10 document types become `Gap` records only when the active project profile or lifecycle gate requires that document type.

## 8. Document Type Library Rule

The Core 10 document types are a template library, not a mandatory project checklist.

The v1 library is:

1. project charter,
2. source inventory,
3. glossary,
4. requirement specification,
5. feature/capability/PBI specification,
6. non-functional requirement specification,
7. risk register,
8. change request,
9. decision record,
10. baseline.

Project profiles and lifecycle gates decide which types are required for a given baseline.

## 9. Project Wiki Projection Rule

`ProjectWikiProjection` is a generated reader-facing output layer.

Rules:

1. The primary wiki projection lives under `docs/product/wiki/**`.
2. Internal and public wiki views are separate generated views.
3. Internal wiki may include delivery status generated from TaskFlow, PR, proof, and release state.
4. Public wiki must not expose delivery status unless a later explicit public-status policy allows a bounded summary.
5. Default visibility is `internal`.
6. Public projection requires `VisibilityApprovalReceipt`.
7. Wiki pages are generated from DB state and approved projections; manual changes must go through web, CLI, CR, or DB-backed reconcile flow.
8. GitHub Wiki or Pages is the first external mirror target; Azure Wiki, Confluence, and other wiki systems are adapter backlog targets.

## 10. Documentation Tree Rule

VIDA product documentation is organized by authority lane first.

Rules:

1. `docs/product/spec/**` owns active product law, contracts, models, maps, templates, and matrices.
2. `docs/product/control/**` owns DB-backed controlled object projections, including accepted decision records.
3. `docs/product/wiki/**` owns generated reader-facing internal and public wiki projections.
4. `docs/product/research/**` owns research input and does not own decisions after they are accepted.
5. Domain grouping inside `docs/product/spec/**` follows the current spec catalog groups for future and bounded migration work.
6. Existing flat spec files are grandfathered until moved by bounded ownership-group migration waves.
7. Superseded active markdown copies are not retained in an archive lane; history belongs in Git and changelog sidecars.

Generated projection files must expose their projection posture in footer metadata with:

```yaml
projection_mode: generated
edit_authority: db
```

## 11. AI Agent Rule

AI agents may:

1. classify origins and source carriers,
2. extract requirement candidates,
3. propose feature and PBI decomposition,
4. draft use cases and acceptance criteria,
5. propose trace links,
6. identify gaps and risks,
7. prepare change impact analysis,
8. draft repository and wiki projections.

AI agents may not:

1. activate baseline without approval,
2. approve waivers,
3. approve public visibility,
4. silently overwrite canonical projection,
5. close lifecycle gates without DocFlow and TaskFlow receipts,
6. treat external untrusted source text as authority without provenance.

## 12. DocFlow Validator Participant Rule

DocFlow is a first-class validator participant.

DocFlow must be able to emit or project `DocFlowValidationVerdict` records for:

1. project inception baseline readiness,
2. origin and provenance completeness,
3. profile-required source inventory completeness,
4. canonical Markdown metadata and footer shape,
5. generated projection footer posture,
6. sibling changelog sidecar presence,
7. map/catalog/lane-index registration,
8. traceability completeness,
9. gap and waiver legality,
10. change request closure readiness,
11. wiki projection readiness,
12. public visibility approval readiness,
13. PR readiness when canonical docs are affected,
14. release baseline admission.

Verdicts must be usable by TaskFlow and release gates.

## 13. TaskFlow Execution Rule

TaskFlow remains the execution authority.

Requirements, change records, use cases, wiki projection records, and decision records may reference TaskFlow items, but they do not replace TaskFlow execution state.

TaskFlow must own:

1. delivery tasks,
2. dependencies and blockers,
3. dispatch and lane state,
4. proof targets,
5. closure evidence,
6. execution receipts.

## 14. Repository Projection Rule

Repository projection must produce or update:

1. canonical Markdown documents,
2. sibling `*.changelog.jsonl`,
3. owning lane indexes, maps, and catalogs,
4. control projections under `docs/product/control/**`,
5. wiki projections under `docs/product/wiki/**`,
6. projection receipts or validation evidence,
7. PR-ready diffs for human review.

Projection is complete only when DocFlow can validate the projected artifacts against the active baseline graph.

## 15. External Adapter Rule

External adapters must preserve VIDA object identity.

Adapters may store:

1. provider name,
2. provider object id,
3. provider object type,
4. last sync state,
5. last sync time,
6. conflict state,
7. external URL,
8. projection direction.

Adapters must not rename VIDA internal types to provider terms inside the canonical DB model.

## 16. First MVP Boundary

The first implementation after this spec should include:

1. DB graph entities for project, origin, source document, requirement, feature, use case, change request, trace link, gap, risk, waiver, approval, visibility approval, validation verdict, wiki projection, and baseline.
2. TaskFlow taxonomy extension without breaking existing task records.
3. CLI prototype for origin intake, candidate graph review, baseline activation, and projection.
4. DocFlow verdict integration for baseline and wiki readiness.
5. Repository projection generator for Markdown and changelog sidecars.
6. Read-only web views over baseline state, origin graph, requirements, use cases, gaps, risks, trace graph, approvals, visibility, wiki projection state, and validation verdicts.

Write-capable web UI flows come after the read-only baseline is proven.

## 17. Relationships

1. `operational-state-and-synchronization-model.md` owns DB-first state and filesystem/Git synchronization law.
2. `taskflow-protocol-runtime-binding-model.md` owns TaskFlow execution/runtime binding.
3. `feature-design-and-adr-model.md` owns bounded design document and ADR split.
4. `canonical-inventory-law.md` owns canonical artifact inventory shape.
5. `canonical-relation-law.md` owns relation and traceability edge posture.
6. `project-documentation-law.md` owns project documentation law.
7. `project-document-naming-law.md` owns documentation tree and filename law.
8. This document owns the requirements, documentation, control, and wiki projection product model.

-----
artifact_path: product/spec/requirements-documentation-control-plane
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-06-18'
schema_version: '1'
status: canonical
source_path: docs/product/spec/requirements-documentation-control-plane.md
created_at: '2026-06-18T00:00:00+03:00'
updated_at: 2026-06-18T00:00:00+03:00
changelog_ref: requirements-documentation-control-plane.changelog.jsonl
