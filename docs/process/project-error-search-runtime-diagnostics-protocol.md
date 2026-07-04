# Project TRACE Runtime Diagnostics Protocol

Status: active project process doc

Purpose: adapt the generic `TRACE` algorithm to VIDA runtime, TaskFlow, DocFlow, agent-lane, and operator-surface diagnostics without redefining the framework-owned algorithm.

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
7. command timing or slow gate diagnostics that may hide a runtime defect,
8. oversized command output, token-heavy JSON, or artifact payloads that make orchestration expensive or break completion surfaces.

For routine startup, read the compact summary in `docs/process/project-orchestrator-startup-bundle.md`. Expand to this document when a runtime defect or multi-defect pool is active.

## VIDA Runtime TRACE Overlay

Use `META(TRACE)` for VIDA runtime defects when any of these are present:

1. framework-owned command behavior,
2. fail-closed law,
3. receipt/proof/closure truth,
4. multi-session or worktree ownership,
5. authoritative-state or projection contradictions,
6. provider/carrier/model/profile admissibility,
7. more than two related failures.

Use plain `TRACE` only for a local, bounded defect whose authoritative state, ownership, and proof law are already clear.

## Required Evidence Packet

Every runtime defect analysis must preserve:

1. exact command or user-visible action,
2. exit code and timing,
3. output economy evidence: default output byte/line estimate, model-visible truncation state, artifact refs for full logs, and whether a smaller selector existed,
4. JSON `status`, `blocker_codes`, `next_actions`, and relevant selected fields,
5. active bounded unit evidence,
6. `why_this_unit`,
7. sequential/parallel posture,
8. root write guard and exception-takeover state when relevant,
9. session/worktree/orchestrator owner evidence when available,
10. dirty worktree summary,
11. proof target that will demonstrate the fix.

If any of those fields are unavailable, record them as missing evidence rather than inferring a clean pass.

## Source-Of-Truth Order

For runtime continuation defects, inspect surfaces in this order unless the active task provides a narrower proof target:

1. `vida status --json`
2. `vida orchestrator-init --json`
3. `vida task next-lawful --json`
4. `vida task show <task-id> --json` when the active task id is known and the recovery question only needs task metadata, owned paths, or proof target
5. `vida taskflow run-graph status <run-id> --json`
6. `vida taskflow recovery status <run-id> --json`
7. `vida lane show <run-id> --json` when lane receipt/evidence state, exception-takeover state, or lane mutation readiness is specifically needed
8. dispatch result/receipt artifacts referenced by the surfaces
9. TaskFlow task record and dependencies
10. DocFlow proof/check surfaces
11. code-level state-store/projection/command implementation

Derived cache, rendered projection, lane preview, advisory text, and operator summaries are evidence surfaces only. They do not override the authoritative state-store, receipt, proof, or explicit runtime law.

When a session/environment self-diagnostic discovers a new reusable TRACE optimization, update this protocol in the same bounded batch. Current examples include preferring `vida task show <task-id> --json` over heavier lane/run-graph projections for timeout recovery metadata, and requiring log-backed execution for long proof gates that can exceed host-tool stdout retention.

## Output Economy Diagnostic Rule

Runtime diagnostics must evaluate command output economy alongside duration. A command is not adequate just because it exits quickly; it is also a defect when it emits more model-visible output than the operator needs to decide the next action.

Adequate output criteria:

1. default output is the smallest sufficient operator summary: status, blocker codes, next actions, and artifact refs;
2. full JSON/log output is opt-in, artifact-backed, and reachable by an explicit full-output command or selector;
3. large outputs must expose bounded selectors, field filters, head/tail/range views, or compact summaries before requiring raw reads;
4. repeated need for raw reruns, client-side JSON unwrapping, or reading megabyte artifacts is `output_economy_defect` evidence;
5. when two commands prove the same fact, prefer the one with fewer model-visible tokens and the same or stronger proof value;
6. command output that exceeds host/tool retention, crashes compression, or blocks runtime completion is a hard runtime defect even if the underlying operation succeeded.

## Runtime-First Diagnostic Rule

1. Before local source edits for a task, attempt the configured VIDA runtime path: `orchestrator-init`, team dispatch preview, run-graph dispatch init, `agent-init --execute-dispatch`, host-bridge request rendering, host adapter execution, and receipt-backed completion.
2. Treat activation views, missing execution evidence, impossible next actions, host-bridge submit-result contradictions, stale downstream routes, and open delegated-cycle write guards as runtime blockers, not as silent permission to write locally.
3. Manual repair is lawful only after the blocker packet records active bounded unit, command, exit result, `blocker_codes`, artifact paths, root write guard or delegated-cycle state, and the runtime defect task that owns the blocker.
4. In manual repair, keep the same configured flow as evidence: analyst result, test-author proof, implementation, coach/review, verifier proof, PR protocol, commit/push, and release/system-binary policy.
5. If the same runtime blocker appears again in the session, raise or keep priority at the highest active level, add release-required evidence when the fix must unblock the installed binary, and do not close the task without a system-binary update decision.

## Multi-Defect Batch Rule

When more than two defects, PR failures, CI failures, runtime blockers, or operator-surface gaps are present:

1. cluster by shared invariant before changing code,
2. identify whether failures are one root cause, dependent blockers, or independent slices,
3. pick one bounded write slice with the highest unblocking value,
4. write regression tests for the shared invariant,
5. batch expensive builds/tests after all tightly related fixes are in place,
6. keep unrelated dirty files out of the slice.

Do not run a long full gate after each tiny edit when focused tests can validate the same invariant first.

When closing a coherent batch of tasks, run the runtime self-diagnostic once for
the whole batch after the included tasks are closed and before selecting any
next work. Treat this batch self-diagnostic as closure evidence for the pool,
not as a replacement for per-task proof, and classify any findings before the
pool is considered fully closed.

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

A VIDA runtime TRACE fix is not ready for commit until the proof matrix covers the claimed blast radius:

1. one focused regression test for the root cause,
2. adjacent contract tests for the affected command family,
3. one debug runtime probe when the defect is observable through a local command,
4. formatting or schema validation for edited source/docs,
5. release build/install according to the System Binary Update Policy in `docs/process/command-timing-and-gate-optimization-protocol.md`,
6. post-pool continuous-improvement diagnostics after the coherent fix pool is proven: command timings, VIDA runtime slow-surface status, token/output reduction opportunities, stage-ordering/parallelism findings, script/gate decisions, command-surface follow-ups, and documentation sync for any new reusable rule.
7. project-skill creation or update actualization through `docs/process/agent-skill-learning-protocol.md`: collect the close/self-analysis/diagnostic events, classify whether a project skill update is required, record `no_skill_update_reason` when not required, and stage or validate skill proposals before TaskFlow next-work selection.
8. final TaskFlow actualization after skill-learning actualization and immediately before deciding what to take into work next: refresh status, parent/child layer, priority, dependencies, owned paths, proof targets, execution mode, order bucket, parallel group, conflict domain, and sequential/parallel posture from current evidence.

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
