# Generic Runtime Protocol Promotion Plan

Status: active project process plan

Purpose: define how reusable `vida-stack` project-side workflow rules are promoted into generic VIDA runtime protocols without moving project-local paths, release names, repository maps, or host-specific notes into framework owner law.

## Promotion Boundary

Reusable law is eligible for promotion when it is project-neutral, host-neutral, and enforceable or inspectable through runtime, TaskFlow, DocFlow, carrier, lane, or command-execution surfaces.

Project overlay material must stay in `AGENTS.sidecar.md` or `docs/process/**` when it depends on:

1. concrete `vida-stack` paths,
2. project documentation maps,
3. local release names or historical project phases,
4. local Windows or repository setup notes,
5. temporary operator workarounds,
6. project-specific preferred commands.

## Owner Map

Promote each reusable rule to the narrowest owner:

| Project-side rule family | Generic owner target | Project residue |
| --- | --- | --- |
| Orchestrator anti-stop, final-report, continuation, active-unit binding | `instruction-contracts/core.orchestration-protocol` | active project read paths and local startup bundle links |
| Packet/lane capsule, packet readiness, delegated result synthesis | `instruction-contracts/lane.worker-dispatch-protocol` and `runtime-instructions/lane.agent-handoff-context-protocol` | project packet templates and role-chain preferences |
| TaskFlow task state, continuation, parent/child closure, graph validation | `runtime-instructions/work.taskflow-protocol` and task-state telemetry owners | project TaskFlow task ids, epics, priorities, and conflict domains |
| Command timing, long gates, local proof ladder, CI non-blocking iteration | `runtime-instructions/work.command-execution-discipline-protocol` plus project timing protocol | concrete scripts, local target-dir paths, and CI workflow names |
| Error Search runtime diagnostics | generic Error Search algorithm and runtime diagnosis owners | VIDA-specific command surfaces, blocker vocabulary, and evidence examples |
| Advisory lane and host-agent bridge boundaries | agent-system, capability registry, and handoff owners | configured carrier names and current host environment notes |
| Source-neutral intake for PRs, CI failures, downstream reports, defects, release tasks, and optimization tasks | orchestration plus TaskFlow source-class metadata | project PR protocol, issue tracker use, and release admission runbooks |

## Promotion Order

1. Inventory the project-side rule and name the concrete source document.
2. Decide whether the rule is generic law, project overlay, or mixed.
3. For mixed rules, split the generic invariant from local examples and local command names.
4. Add or update the generic owner protocol with only project-neutral wording.
5. Keep project examples in the mapped project process document.
6. Update bootstrap maps so future agents reach the correct owner first.
7. Add focused local proof: `vida protocol view` for the owner and `vida docflow check-file` for changed docs.

## Local Proof And CI Boundary

Local proof is the development gate for promotion work. GitHub Actions after push are diagnostic signals unless the active bounded task is explicitly release admission, mainline admission, installer validation, or CI architecture repair.

Promotion work must use the cheapest proof that validates the changed owner:

1. docs-only slice: `git diff --check` and `vida docflow check-file <changed-doc>`,
2. runtime-protocol slice: add `vida protocol view <owner-id> --json`,
3. command/script slice: add `scripts/vida-dev-gate.ps1 -Mode script-check -Json`,
4. executable runtime slice: add focused package tests or `scripts/vida-dev-gate.ps1 -Mode quick -Json`,
5. installed-runtime slice: run release install only when the bounded acceptance target is the installed launcher or downstream environment.

Do not wait on GitHub Actions before selecting the next development item after a pushed local-proof-green promotion slice. If CI later reports a failure, classify it as a new source-neutral work item and process it through TaskFlow priority rules.

## Non-Goals

1. Do not convert project runbooks into framework law by copy-paste.
2. Do not promote concrete carrier names, host CLI names, model names, repository paths, or release labels as generic requirements.
3. Do not make CI, installer, or release gates block ordinary local development unless the bounded task owns that admission surface.
4. Do not create a second bootstrap carrier in this plan.

## Acceptance

This plan is satisfied when:

1. every reusable sidecar or process rule under review has a generic owner target,
2. project-local residue remains explicit,
3. promotion order is deterministic,
4. local proof is sufficient for ordinary development continuation,
5. CI after push remains diagnostic unless the active task is an admission task.

-----
artifact_path: process/generic-runtime-protocol-promotion-plan
artifact_type: process_doc
artifact_version: '1'
artifact_revision: 2026-06-02
schema_version: '1'
status: canonical
source_path: docs/process/generic-runtime-protocol-promotion-plan.md
created_at: 2026-06-02T06:25:00+03:00
updated_at: 2026-06-02T06:25:00+03:00
changelog_ref: generic-runtime-protocol-promotion-plan.changelog.jsonl
