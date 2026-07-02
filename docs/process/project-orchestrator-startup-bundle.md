# Project Orchestrator Startup Bundle

Status: active project process doc

Purpose: provide one compact project-side startup bundle for routine orchestrator sessions, aggregating the current always-read project control surfaces without replaying every owner document separately.

## Use

Use this bundle after framework bootstrap when the project orchestrator needs the minimum project read set for routine startup, resume, or cheaper orchestration.

This bundle is a routing and compression surface only.
It does not own protocol law.

Owner law remains in:

1. `docs/process/project-orchestrator-operating-protocol.md`
2. `docs/process/project-orchestrator-session-start-protocol.md`
3. `docs/process/project-packet-and-lane-runtime-capsule.md`
4. `docs/process/project-packet-rendering-runtime-capsule.md`
5. `docs/process/command-timing-and-gate-optimization-protocol.md`
6. `docs/process/project-error-search-runtime-diagnostics-protocol.md`

Consult those owner surfaces when an edge case, launch-readiness conflict, or routing ambiguity is not settled by this bundle.

## Bundle Contents

Treat this bundle as the compact project `always_on_core` startup set for routine development orchestration:

1. top-level project routing and anti-stop narrowing from `project-orchestrator-operating-protocol.md`,
2. session-start readiness routing from `project-orchestrator-session-start-protocol.md`,
3. packet and delegated-lane defaults from `project-packet-and-lane-runtime-capsule.md`,
4. packet rendering and prompt-stack interpretation from `project-packet-rendering-runtime-capsule.md`,
5. skill activation routing from `project-skill-initialization-and-activation-protocol.md`,
6. command timing, slow-gate classification, and script/gate optimization defaults from `command-timing-and-gate-optimization-protocol.md`,
7. runtime-defect Error Search routing defaults from `project-error-search-runtime-diagnostics-protocol.md`.
8. runtime development environment skill and issue-processing routing from `vida-runtime-development-environment.md`.
9. wave-first epic optimization, three-step task execution, and post-task
   scorecard/checklist routing from `project-orchestrator-operating-protocol.md`.
10. codebase graph/search routing: use `codebase-memory-mcp` for fresh indexed symbol, call graph, impact, snippet, schema, and ADR work; use `lean-ctx` for current file reads, shell commands, compressed logs, and stale-index fallback.

## Runtime Summary

After reading this bundle, the orchestrator should be able to answer:

1. which bounded unit is active or why it is still ambiguous,
2. whether the next leaf is `delivery_task` or `execution_block`,
3. whether the next move is shape, delegate, verify, or escalate,
4. which proof target closes the next packet,
5. whether the session-start protocol must be expanded for full readiness,
6. whether skill activation is already explicit,
7. whether a command or gate timing must create optimization work,
8. whether a full owner protocol read is required for an edge case,
9. whether a runtime defect must use `META(Error Search)` because authority, ownership, receipt, proof, or routing law is involved.
10. whether `vida-runtime-development` or `vida-github-issues` should be activated for the current bounded step.
11. which wave has the smallest closure distance when the active goal is a
    long-running epic,
12. which executor/validator routing rule was learned from the last comparable
    task,
13. which post-task checklist items must be proven before selecting unrelated
    work.
14. whether codebase discovery should start from `codebase-memory-mcp` or fall back to `lean-ctx` because the index is absent, stale, or contaminated by generated paths.

## Expansion Rule

Use the bundle by default for routine startup.

Expand beyond it only when:

1. the session-start checklist itself is being audited or changed,
2. launch readiness is blocked on an owner-level validation conflict,
3. delegated-lane closure or exception-path law is ambiguous,
4. packet-template or prompt-stack edge cases are not settled by the rendering capsule,
5. skill activation is not settled by the bundle's routing pointer,
6. command timing, slow-gate, or script optimization decisions are not settled by the timing protocol summary,
7. the user explicitly asks for the deeper owner protocol,
8. a runtime defect or multi-defect pool requires the full project Error Search overlay.
9. wave-first closure distance, model-routing optimization, post-task scorecard,
   or publication authorization is unclear.

## Routing

1. for the full session-start checklist, read `docs/process/project-orchestrator-session-start-protocol.md`,
2. for top-level routing and project anti-stop narrowing, read `docs/process/project-orchestrator-operating-protocol.md`,
3. for packet/lane defaults, read `docs/process/project-packet-and-lane-runtime-capsule.md`,
4. for startup readiness, read `docs/process/project-orchestrator-session-start-protocol.md`,
5. for packet rendering and prompt-stack law, read `docs/process/project-packet-rendering-runtime-capsule.md`,
6. for skill activation, read `docs/process/project-skill-initialization-and-activation-protocol.md`,
7. for timing evidence, slow-gate classification, and script/gate optimization, read `docs/process/command-timing-and-gate-optimization-protocol.md`,
8. for runtime blockers, multi-defect pools, ownership conflicts, receipt/proof contradictions, routing blockers, or CI defect clusters, read `docs/process/project-error-search-runtime-diagnostics-protocol.md`.
9. for runtime development environment, project-local skill activation, GitHub issue processing, or operator-efficiency follow-up routing, read `docs/process/vida-runtime-development-environment.md`.
10. for codebase graph discovery, start with `codebase-memory-mcp list_projects` and `index_status`; if the index is ready, use graph tools before broad text search, and if it is stale or contaminated, record the blocker and use `lean-ctx` for current filesystem truth.

-----
artifact_path: process/project-orchestrator-startup-bundle
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-07-01'
schema_version: '1'
status: canonical
source_path: docs/process/project-orchestrator-startup-bundle.md
created_at: '2026-03-13T18:05:15+02:00'
updated_at: 2026-07-01T21:45:00+03:00
changelog_ref: project-orchestrator-startup-bundle.changelog.jsonl
