# Enterprise Requirements Control Plane Research

Status: promoted research input

Purpose: summarize external requirements-management, documentation, and AI-agent practices that inform the VIDA requirements and documentation control plane without making this research document itself product law.

## 1. Research Question

VIDA needs an enterprise-grade path from arbitrary origins to requirements, use cases, product backlog items, controlled changes, implementation work, proof, project wiki projection, and release baselines.

The research asks:

1. which external practices are useful for requirements and documentation governance,
2. how modern AI-agent frameworks model agents, tools, state, handoffs, guardrails, and traces,
3. how those practices map to VIDA's existing DB-first runtime, TaskFlow, DocFlow, receipts, and filesystem projection model.

## 2. Requirements And ALM Practices

The useful pattern across ISO/IEC/IEEE 29148, BABOK, Azure DevOps, Jira-style workflows, and ReqIF/OSLC interoperability is a graph rather than a document pile.

VIDA should model:

1. origin records,
2. optional source documents,
3. extracted requirement candidates,
4. approved requirement items,
5. feature or backlog items,
6. first-class use cases,
7. non-functional requirements,
8. risks and gaps,
9. change requests,
10. decision records,
11. trace links,
12. approvals and waivers,
13. validation verdicts,
14. wiki projections,
15. implementation and proof references.

The key distinction is that an item describing product value is not the same as the governance envelope that allows an approved baseline to change.

## 3. Documentation Practices

Diataxis supports a clear documentation split:

1. tutorials for learning,
2. how-to guides for task execution,
3. reference for exact technical facts,
4. explanation for reasoning and context.

ADR practice adds the complementary rule that durable decisions should capture context, decision, alternatives, consequences, and status.

For VIDA this means:

1. product/spec law stays in `docs/product/spec/**`,
2. DB-backed control projections live in `docs/product/control/**`,
3. generated project wiki views live in `docs/product/wiki/**`,
4. research stays in `docs/product/research/**` until promoted,
5. process guidance stays in `docs/process/**`,
6. accepted decision records live under `docs/product/control/decisions/**`,
7. Markdown and sibling `*.changelog.jsonl` remain the filesystem projection for human review and Git audit.

## 4. Agent Framework Practices

OpenAI Agents SDK, Microsoft Agent Framework and Foundry Agent Service, Google ADK, MCP, and A2A converge on the same product concepts:

1. agents have instructions, tools, state, outputs, and optional handoffs,
2. orchestration chooses whether a specialist takes ownership or acts as a bounded tool,
3. guardrails and human review gate risky or authoritative actions,
4. traces record model calls, tool calls, handoffs, guardrails, and custom spans,
5. MCP standardizes tools and resources as explicit protocol surfaces,
6. A2A standardizes agent capability discovery, task lifecycle, long-running collaboration, and artifacts.

VIDA already has matching native concepts:

1. TaskFlow owns execution authority,
2. DocFlow validates documentation state,
3. receipts and checkpoints preserve proof,
4. runtime dispatch packets shape bounded work,
5. DB-first state provides durable session and task truth,
6. filesystem projection and Git provide reviewable external surfaces.

## 5. AI Risk And Governance Practices

NIST AI RMF, NIST Generative AI Profile, and OWASP LLM/agentic security guidance point to the same control need: AI output should be treated as proposed evidence until a controlled system validates it.

For VIDA:

1. AI extraction creates `candidate` requirements, not approved requirements,
2. generated trace links require provenance and review,
3. tools that mutate baseline state require human approval or policy-backed authorization,
4. prompt injection and untrusted-document input must be handled as source risk,
5. safety and correctness evidence must be attached as receipts rather than informal chat claims.

## 6. VIDA Mapping

The best fit is a DB-first control plane with synchronized repo projection:

1. DB stores the requirements graph, lifecycle state, approvals, waivers, trace links, validation verdicts, and external adapter ids.
2. Repository Markdown/JSONL stores the current human-readable projection.
3. TaskFlow remains the authority for execution and delivery work.
4. DocFlow becomes a first-class validator participant for baseline activation, change closure, PR readiness, and release baseline admission.
5. Project wiki is a generated projection with internal and public views.
6. GitHub, Jira, Confluence, Azure DevOps, and Wiki systems are adapters or mirrors unless a future project profile explicitly delegates an external authority role.

## 7. Recommended VIDA Object Split

Use VIDA-native object names internally and map them outward:

1. `RequirementItem`
2. `FeatureItem`
3. `UseCaseItem`
4. `DeliveryItem`
5. `ChangeRequest`
6. `OriginRecord`
7. `SourceDocument`
8. `TraceLink`
9. `Gap`
10. `RiskItem`
11. `Waiver`
12. `ApprovalReceipt`
13. `VisibilityApprovalReceipt`
14. `DocFlowValidationVerdict`
15. `ProjectWikiProjection`
16. `WikiPageProjection`
17. `Baseline`
18. `ReleaseBaseline`

External mapping:

1. `FeatureItem` maps to Azure PBI, Jira Story, or GitHub Issue.
2. `ChangeRequest` maps to Azure Change Request, Jira Change, or GitHub CR issue.
3. `DeliveryItem` maps to Azure Task, Jira Sub-task, GitHub Issue, or TaskFlow task.

## 8. Research Conclusion

VIDA should not copy Confluence, Jira, or Azure DevOps. It should own the canonical requirements and documentation graph, then project into those systems through adapters.

The first implementation should therefore create a documentation-backed control-plane spec before runtime schema migration:

1. research summary,
2. decision record,
3. active product/spec law,
4. DocFlow validation role,
5. TaskFlow taxonomy and DB graph expansion plan,
6. project wiki projection scope,
7. read-only web interface scope.

## 9. References

1. ISO/IEC/IEEE 29148:2018, Systems and software engineering requirements engineering.
2. IIBA BABOK, Requirements Life Cycle Management.
3. Azure DevOps process and CMMI work item documentation.
4. ReqIF, Requirements Interchange Format.
5. OSLC Requirements Management.
6. W3C PROV-DM.
7. Diataxis documentation framework.
8. arc42 architecture documentation template.
9. C4 model for software architecture documentation.
10. Architecture Decision Records.
11. GitHub Wiki documentation.
12. Azure DevOps Wiki documentation.
13. Atlassian product requirements documentation.
14. OpenAI Agents SDK documentation.
15. Microsoft Agent Framework and Foundry Agent Service documentation.
16. Model Context Protocol specification.
17. Agent2Agent protocol.
18. NIST AI RMF and Generative AI Profile.
19. OWASP LLM and Agentic AI security guidance.

-----
artifact_path: product/research/enterprise-requirements-control-plane-research
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-06-18'
schema_version: '1'
status: canonical
source_path: docs/product/research/enterprise-requirements-control-plane-research.md
created_at: '2026-06-18T00:00:00+03:00'
updated_at: 2026-06-18T00:00:00+03:00
changelog_ref: enterprise-requirements-control-plane-research.changelog.jsonl
