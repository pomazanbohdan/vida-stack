# VIDA Future Alignment

Purpose: separate out-of-environment work from local platform gaps.

Framework-owned planning/reference surface, not runtime law unless another canonical protocol promotes a section into active execution rules.

## Scope Split

Use this file to keep two categories distinct: `Out of current environment` requires external deployment surfaces, external protocol adoption, remote infrastructure, or organization-level operating model changes not fully realizable inside this repository alone; `Locally remaining` can advance partly or fully inside the current VIDA workspace without Rust work or external control-plane infrastructure.

## Already Present In VIDA

The current workspace already has strong local foundations: route-law orchestration, fail-closed writer authorization, issue-as-contract flow, coach/rework with structured provenance, human approval receipts, framework memory ledger, document lifecycle ledger, problem-party bounded escalation, queue-backed task-state mutation, worker pool/probing/evaluation refresh, operator status, and silent framework diagnosis.

These surfaces shift future work toward platform maturity, interoperability, and stronger runtime evidence loops rather than re-solving orchestration.

## Out Of Current Environment

### 1. Interoperable agent-to-agent surface

Why this is outside the current environment:

1. VIDA currently orchestrates local/internal workers and CLI lanes, not external agent servers with stable network contracts.
2. Full A2A adoption needs externally reachable agent endpoints, discovery documents, authenticated agent identity, and interoperability testing across protocol bindings.

Research anchor:

1. A2A defines Agent Cards, capability validation, authentication/authorization, task operations, and interoperability testing as first-class protocol requirements.
2. Agent Cards may be signed and exposed through a discovery surface such as `/.well-known/agent-card.json`.

Sources:

1. A2A spec overview and required surfaces: https://a2a-protocol.org/dev/specification/
2. Agent Card and signing/auth details: https://a2a-protocol.org/dev/specification/

Future target: external agent registry, typed discovery card per VIDA-exposed agent surface, signed agent identity/capability declaration, task/message bridge from VIDA route receipts to A2A task state.

### 2. Full MCP connector governance beyond local search/tooling

Why this is outside the current environment:

1. The repo already references MCP tooling, but not a full production connector governance plane.
2. Full MCP operating maturity needs external connector deployment, permissioning, remote trust policy, tenancy rules, and lifecycle management for many tools/data sources.

Research anchor:

1. MCP is a standard for connecting AI applications to external systems, including tools, data sources, and workflows.
2. Broad MCP maturity implies not only tool invocation, but connector lifecycle, access boundaries, and trusted client/server relationships.

Source:

1. MCP intro: https://modelcontextprotocol.io/docs/getting-started/intro

Future target: connector registry, connector trust classes, capability-scoped access by route/agent role, freshness/provenance policy for external context, MCP server/client governance docs and receipts.

### 3. Remote operator control plane

Why this is outside the current environment:

1. VIDA already has local operator status surfaces, but not a standalone multi-run, multi-project, remotely accessible control plane.
2. A real control plane needs service deployment, storage, UI/API, historical telemetry aggregation, and cross-workspace visibility.

Future target: route selection audit dashboard, per-lane cost/quality history, anomaly heatmap, approvals queue, lane quarantine/recovery controls, cross-project orchestration metrics.

### 4. Production-grade trace and eval backend

Why this is outside the current environment:

1. VIDA already has local eval artifacts, but not a dedicated external trace/eval platform with scalable grading and dataset management.
2. Full platform maturity needs persistent run storage, grader runs across many tasks, trend analysis, and regression datasets beyond one repository session.

Research anchor:

1. Agent evals should be reproducible and dataset-driven.
2. Trace grading should score full traces, not just final outputs, to identify orchestration-level failures and regressions.

Sources:

1. Agent evals: https://developers.openai.com/api/docs/guides/agent-evals
2. Trace grading: https://developers.openai.com/api/docs/guides/trace-grading

Future target: exported trace corpus, grader registry, replayable run datasets, route-level regression comparisons across framework versions.

### 5. Organization-level human approval network

Why this is outside the current environment:

1. Local approval receipts exist, but organizational approval routing requires identity integration, reviewer directories, notification delivery, SLA policy, and long-wait resumability beyond one terminal session.

Research anchor:

1. Human-in-the-loop systems pause and resume from run state and support approvals across nested tool/agent surfaces.

Source:

1. Human-in-the-loop guide: https://openai.github.io/openai-agents-js/guides/human-in-the-loop/

Future target: named approver groups, approval escalation chains, long-running pending-approval handling, resumable approval callbacks outside the local CLI loop.

## Locally Remaining

These are still within the current workspace and can be implemented without Rust.

### 1. Durable run graph beyond TaskFlow task checkpoints

Current local evidence:

1. boot receipts, TaskFlow checkpoints, and context capsules exist,
2. pack/task lifecycle exists,
3. resumability is still mostly task-centric rather than run-graph-centric.

Local gap:

1. there is no single canonical run-state ledger for one long orchestration run with node-level status, retries, pauses, and resumable pending approvals.

Recommended local next step:

1. add `.vida/state/run-graphs/<task_id>.json`,
2. persist `analysis`, `writer`, `coach`, `verifier`, `approval`, `synthesis` node status,
3. support replay-safe resume after compact or lane failure.

### 2. Stronger local trace/eval loop

Current local evidence:

1. `eval-pack.sh`,
2. `worker-eval-pack.py`,
3. local score refresh in `.vida/state/worker-strategy.json`.

Local gap:

1. evaluation is still task-close oriented and not yet a first-class trace-grading framework for orchestration nodes and route decisions.

Recommended local next step:

1. add canonical trace artifact per routed run,
2. add grader schemas for route correctness, fallback correctness, budget correctness, and approval correctness,
3. add regression dataset export from real framework runs.

### 3. Typed agent capability registry

Current local evidence:

1. `vida.config.yaml` and the agent system carry capability bands and dispatch wiring,
2. routing is still largely config-driven rather than schema-first typed interoperability.

Local gap:

1. there is no canonical capability registry with typed input/output contracts per lane.

Recommended local next step:

1. add framework-owned agent capability schemas,
2. declare required input artifact types and emitted artifact types per task class,
3. validate route compatibility before dispatch.

### 4. Context governance layer

Current local evidence:

1. project overlay and tooling docs define command/context entry surfaces,
2. there is no complete context-governance policy for source trust, freshness, connector class, and role-scoped access.

Local gap:

1. context is still governed mostly by protocol discipline rather than a dedicated context registry.

Recommended local next step:

1. add a framework context-source ledger,
2. classify context sources as `local_repo`, `local_runtime`, `overlay_declared`, `web_validated`, `external_connector`,
3. attach freshness and provenance requirements to each class.

### 5. Problem-party as runtime-selected lane

Current local evidence:

1. problem-party protocol and helper exist,
2. route-visible receipts exist.

Local gap:

1. activation is still mainly orchestrator-driven rather than selected directly by route policy and recorded as a full run-graph node.

Recommended local next step:

1. formalize `problem_party` as a typed route node,
2. connect it to run-graph state,
3. add eval coverage for correct/incorrect admission.

### 6. Operator surface enrichment

Current local evidence:

1. framework operator status exists,
2. worker status already exposes lifecycle and remediation hints.

Local gap:

1. there is still no single local operator summary for:
   - why a route/model/agent was selected,
   - total task cost by lane,
   - repeated anomaly clusters,
   - approval backlog,
   - route-law violations by class.

Recommended local next step:

1. add one consolidated operator report surface,
2. include per-task and per-wave route rationale,
3. export anomaly and cost summaries from historical local artifacts.

## Priority Order

Local non-Rust maturation priority: durable run graph -> local trace grading/eval datasets -> typed capability registry -> context governance layer -> operator surface enrichment -> deeper problem-party runtime integration.

Beyond-current-environment priority: MCP connector governance -> A2A-style interoperable agent registry -> remote control plane -> external trace/eval backend -> organization-level approval network.

## Rule For Future Work

1. Do not silently mix local residual work with out-of-environment roadmap work.
2. Any task taken from `Out of current environment` must first be converted into an explicit implementation slice that states which external dependency, deployment surface, or protocol adoption is being assumed.
3. Any task taken from `Locally remaining` should be tracked through normal framework TaskFlow and verified against current workspace evidence.

-----
artifact_path: config/system-maps/future
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/framework.future-note.md
created_at: '2026-03-08T02:15:22+02:00'
updated_at: 2026-07-03T14:05:00+03:00
changelog_ref: framework.future-note.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: roadmap-prose-compaction+priority-chain-normalization+source-preserve-exact
protocol_compression_baseline_ref: 4aee9451c:vida/config/instructions/system-maps/framework.future-note.md
protocol_compression_audit_at: 2026-07-03T14:05:00+03:00
protocol_compression_before_tokens: 2182
protocol_compression_after_tokens: 2181
protocol_compression_content_sha256: 78a614a913b4f78b1f43c5e9808614600cb760ca0e3338abc027007b4f40ed56
