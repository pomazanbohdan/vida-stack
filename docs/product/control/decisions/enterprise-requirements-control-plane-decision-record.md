# Enterprise Requirements Control Plane Decision Record

Status: accepted decision record

Purpose: record the accepted baseline decisions for the VIDA requirements, documentation, control, and wiki projection model.

## 1. Decision Scope

This decision record covers the initial product direction for:

1. origin-first project inception,
2. requirements and feature decomposition,
3. use case modeling,
4. change request governance,
5. documentation tree authority lanes,
6. project wiki projection,
7. TaskFlow execution authority,
8. DocFlow validation participation,
9. external ALM/wiki adapter posture,
10. first MVP boundary.

It does not define the final database schema, API surface, or web UI layout. Those belong to follow-up implementation specifications after this baseline is accepted.

## 2. Accepted Decisions

### D1. DB Is Primary

VIDA will treat the database as the primary operational truth for requirements, traceability, approvals, waivers, validation verdicts, visibility state, and lifecycle state.

Repository files are synchronized projections and review surfaces, not an equal second authority.

### D2. Repository Projection Is Required

The control plane must still write Markdown and JSONL into the repository:

1. Markdown is the current human-readable body,
2. sibling `*.changelog.jsonl` records artifact-level history,
3. map/catalog/lane-index entries preserve discoverability,
4. Git history preserves review and lineage.

### D3. External Systems Are Adapters

GitHub, Jira, Confluence, Azure DevOps, and Wiki systems are adapter targets by default.

They may mirror or import/export VIDA state, but they do not own canonical truth unless a future project profile explicitly marks a bounded external system as authority for a specific object class.

### D4. TaskFlow Remains Execution Authority

TaskFlow remains the authority for execution work, delivery tasks, dependencies, proof targets, dispatch, and closure.

The first runtime implementation should extend TaskFlow taxonomy and graph linkage before creating a separate tracker.

### D5. DocFlow Is A Validator Participant

DocFlow is not only a Markdown checker. It participates in lifecycle decisions.

DocFlow verdicts gate:

1. baseline activation,
2. change request closure,
3. wiki projection readiness,
4. public visibility readiness,
5. PR readiness when canonical docs are affected,
6. release baseline admission.

### D6. AI Output Is Candidate State

AI agents may classify origins, extract candidate requirements, propose traces, draft specs, draft use cases, and prepare impact analysis.

AI agents may not activate a baseline, approve public visibility, approve waivers, or silently overwrite canonical documentation without approval and validation receipts.

### D7. PBI And CR Are Different Layers

VIDA will not use `PBI` and `CR` as competing universal types.

The internal split is:

1. `FeatureItem` or `RequirementItem` describes product value or requirement content,
2. `UseCaseItem` describes actor/scenario behavior linked to requirements and features,
3. `ChangeRequest` is the governance envelope for changing an approved baseline,
4. `DeliveryItem` or TaskFlow task is the execution unit.

External adapters map these objects to provider terms such as Azure PBI, Jira Story, GitHub Issue, Azure Change Request, or Jira Change.

### D8. Project Inception Is Origin-First

The system must support creating a project baseline from arbitrary origins before any change-request lane or source-document inventory exists.

Valid origins include direct operator entry, direct CR, direct requirement, feature/PBI, use case, risk, decision, baseline, TZ/source document, document set, meeting note, agent-discovered gap, external import, or explicit no-origin rationale.

The initial project lifecycle is:

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

### D9. Gaps Can Be Waived By Profile

Critical gaps are project/profile-defined.

An active baseline may be admitted with gaps only when waiver receipts record:

1. gap id,
2. risk,
3. approver,
4. expiration or review condition,
5. affected baseline,
6. DocFlow verdict context.

### D10. Core 10 Is A Template Library

Core 10 document types are optional templates and not a mandatory checklist.

Missing documents become gaps only when the active project profile or lifecycle gate requires that document type.

### D11. Project Wiki Projection Is Generated

The project wiki is a generated reader-facing projection, not an authority surface.

VIDA will generate:

1. internal wiki under `docs/product/wiki/internal/**`,
2. public wiki under `docs/product/wiki/public/**`.

Public projection requires explicit visibility approval. Delivery status generated from TaskFlow, PR, proof, and release state is internal-only in v1.

### D12. Documentation Tree Uses Authority Lanes

The product documentation tree is optimized for findability by authority lane:

1. `docs/product/spec/**` owns active law,
2. `docs/product/control/**` owns DB-backed control projections and accepted decision records,
3. `docs/product/wiki/**` owns generated reader-facing wiki projections,
4. `docs/product/research/**` owns research inputs.

Existing flat spec docs are grandfathered. Future and migrated spec docs may use catalog-derived domain subfolders through bounded migration waves.

### D13. First MVP Includes Read-Only Web

The first MVP after documentation should include:

1. DB entities for origin/requirements/change/baseline/wiki graph,
2. CLI prototypes for create/read/projection,
3. DocFlow validation verdict integration,
4. repository projection generator,
5. read-only web views for baseline, origin graph, requirements, use cases, gaps, risks, trace graph, approvals, visibility, wiki projection state, and validation state.

## 3. Rejected Alternatives

### A1. Make GitHub Issues The Primary Change Ledger

Rejected because GitHub is a public tracking and discussion surface. It does not replace TaskFlow receipts, DocFlow validation, or DB-first lifecycle state.

### A2. Use PBI As The Universal Internal Type

Rejected because PBI is an Azure/Scrum-specific projection term. VIDA needs provider-neutral object names that can map to Azure, Jira, GitHub, and internal TaskFlow.

### A3. Let AI Extraction Directly Activate Baseline

Rejected because enterprise requirements and documentation changes need provenance, review, validation, and approval receipts.

### A4. Keep Documentation Only In Files

Rejected because the planned web interface, lifecycle state, approvals, waivers, trace graph, wiki projection, and multi-session orchestration require DB-first operational truth.

### A5. Reshuffle All Existing Spec Docs At Once

Rejected because the current flat spec canon is large enough that a mass move would create high link/catalog churn. Migration must happen in bounded ownership waves.

## 4. Consequences

1. Runtime work must add or extend DB-backed object models instead of storing lifecycle only in Markdown.
2. Projection commands must be explicit, auditable, and reversible through receipts.
3. DocFlow must expose verdict objects usable by TaskFlow and release gates.
4. External provider adapters must store provider ids and sync state without changing VIDA object identity.
5. The web UI should read DB state first and use repository files as linked projection evidence.
6. Documentation tree changes must update lane indexes and owner maps, not only leaf documents.

## 5. Follow-Up Design Questions

1. Exact SurrealDB table names and indexes for requirement graph objects.
2. Required trace-link edge kinds and cardinality rules.
3. Minimal CLI command family for origin intake, baseline projection, and wiki publish.
4. DocFlow verdict schema shared between CLI, DB, and web UI.
5. Web UI read-only route shape and permission model.
6. External adapter sync conflict policy for Jira, Azure DevOps, Confluence, GitHub, and Wiki systems.

-----
artifact_path: product/control/decisions/enterprise-requirements-control-plane-decision-record
artifact_type: product_control_doc
artifact_version: '1'
artifact_revision: '2026-06-18'
schema_version: '1'
status: canonical
source_path: docs/product/control/decisions/enterprise-requirements-control-plane-decision-record.md
created_at: '2026-06-18T00:00:00+03:00'
updated_at: 2026-06-18T00:00:00+03:00
changelog_ref: enterprise-requirements-control-plane-decision-record.changelog.jsonl
