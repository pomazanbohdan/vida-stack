# Project Error Search Runtime Diagnostics Protocol

Status: active project process doc

Purpose: adapt the generic `Error Search / Bug Reasoning` algorithm to VIDA runtime, TaskFlow, DocFlow, agent-lane, and operator-surface diagnostics without redefining the framework-owned algorithm.

Owner boundary:

1. Generic algorithm law lives in `vida/config/instructions/instruction-contracts/overlay.step-thinking-protocol.md#section-bug-reasoning`.
2. This document is the `vida-stack` project overlay for applying that algorithm to local runtime work.
3. If this document conflicts with framework instruction law, use the framework owner and repair this project overlay.

## Bootstrap Rule

This document is mandatory bootstrap context when any active work involves:

1. VIDA runtime blockers,
2. TaskFlow continuation, run-graph, recovery, lane, dispatch, receipt, or closure defects,
3. DocFlow proof/readiness contradictions,
4. multi-session, worktree, or orchestrator ownership conflicts,
5. provider/carrier/model/profile routing blockers,
6. CI clusters, repeated failing tests, or more than two related defects,
7. command timing or slow gate diagnostics that may hide a runtime defect.

For routine startup, read the compact summary in `docs/process/project-orchestrator-startup-bundle.md`. Expand to this document when a runtime defect or multi-defect pool is active.

## VIDA Runtime Error Search Overlay

Use `META(Error Search)` for VIDA runtime defects when any of these are present:

1. framework-owned command behavior,
2. fail-closed law,
3. receipt/proof/closure truth,
4. multi-session or worktree ownership,
5. authoritative-state or projection contradictions,
6. provider/carrier/model/profile admissibility,
7. more than two related failures.

Use plain `Error Search` only for a local, bounded defect whose authoritative state, ownership, and proof law are already clear.

## Required Evidence Packet

Every runtime defect analysis must preserve:

1. exact command or user-visible action,
2. exit code and timing,
3. JSON `status`, `blocker_codes`, `next_actions`, and relevant selected fields,
4. active bounded unit evidence,
5. `why_this_unit`,
6. sequential/parallel posture,
7. root write guard and exception-takeover state when relevant,
8. session/worktree/orchestrator owner evidence when available,
9. dirty worktree summary,
10. proof target that will demonstrate the fix.

If any of those fields are unavailable, record them as missing evidence rather than inferring a clean pass.

## Source-Of-Truth Order

For runtime continuation defects, inspect surfaces in this order unless the active task provides a narrower proof target:

1. `vida status`
2. `vida orchestrator-init`
3. `vida task next-lawful`
4. `vida task show <task-id>` when the active task id is known and the recovery question only needs task metadata, owned paths, or proof target
5. `vida taskflow run-graph status <run-id>`
6. `vida taskflow recovery status <run-id>`
7. `vida lane show <run-id>` when lane receipt/evidence state, exception-takeover state, or lane mutation readiness is specifically needed
8. dispatch result/receipt artifacts referenced by the surfaces
9. TaskFlow task record and dependencies
10. DocFlow proof/check surfaces
11. code-level state-store/projection/command implementation

Derived cache, rendered projection, lane preview, advisory text, and operator summaries are evidence surfaces only. They do not override the authoritative state-store, receipt, proof, or explicit runtime law.

When a session/environment self-diagnostic discovers a new reusable Error Search optimization, update this protocol in the same bounded batch. Current examples include preferring `vida task show <task-id>` over heavier lane/run-graph projections for timeout recovery metadata, and requiring log-backed execution for long proof gates that can exceed host-tool stdout retention.

## Multi-Defect Batch Rule

When more than two defects, PR failures, CI failures, runtime blockers, or operator-surface gaps are present:

1. cluster by shared invariant before changing code,
2. identify whether failures are one root cause, dependent blockers, or independent slices,
3. pick one bounded write slice with the highest unblocking value,
4. write regression tests for the shared invariant,
5. batch expensive builds/tests after all tightly related fixes are in place,
6. keep unrelated dirty files out of the slice.

Do not run a long full gate after each tiny edit when focused tests can validate the same invariant first.

## Fix Locus Guide

Choose the patch layer by the first wrong transition point:

1. selector bug: candidate/default selection chooses the wrong unit before validation,
2. authority bug: derived projection overrides authoritative state,
3. ownership bug: current session/worktree cannot lawfully mutate selected state,
4. receipt bug: execution/closure/proof is inferred without receipt-backed evidence,
5. cache bug: stale or noisy derived cache changes runtime truth,
6. config bug: hardcoded carrier/provider/model/flow name bypasses configured registry,
7. command-surface bug: JSON/operator surface hides the real blocker or recommends an impossible command,
8. test-fixture bug: fixture violates current runtime law and masks the real production behavior.

## Proof Matrix

A VIDA runtime Error Search fix is not ready for commit until the proof matrix covers the claimed blast radius:

1. one focused regression test for the root cause,
2. adjacent contract tests for the affected command family,
3. one debug runtime probe when the defect is observable through a local command,
4. formatting or schema validation for edited source/docs,
5. release build/install only when installed-runtime behavior must be validated or when preparing a push that depends on installed binary behavior,
6. post-pool continuous-improvement diagnostics after the coherent fix pool is proven: command timings, VIDA runtime slow-surface status, token/output reduction opportunities, stage-ordering/parallelism findings, script/gate decisions, command-surface follow-ups, and documentation sync for any new reusable rule.

Record timings for each proof command. If a repeated proof command exceeds the project timing target, create or update an operator-efficiency task.

## Stop And Escalate

Stop local fixing and escalate to project META analysis when:

1. the authoritative state source cannot be identified,
2. the current session cannot own or mutate the selected run/task,
3. proof/receipt/closure law conflicts between surfaces,
4. three hypotheses fail,
5. a fix would hardcode provider, model, CLI, role sequence, flow sequence, or agent identity instead of using configured runtime data,
6. the next recommended operator command cannot be validated against target run, task, receipt, packet, and session evidence.

-----
artifact_path: process/project-error-search-runtime-diagnostics-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-05-26'
schema_version: '1'
status: canonical
source_path: docs/process/project-error-search-runtime-diagnostics-protocol.md
created_at: 2026-05-26T00:00:00+03:00
updated_at: 2026-05-26T00:00:00+03:00
changelog_ref: project-error-search-runtime-diagnostics-protocol.changelog.jsonl
