# Project TRACE Runtime Diagnostics Protocol

Status: active project process doc

## Purpose

Adapt the generic `TRACE` algorithm to VIDA runtime, TaskFlow, DocFlow, agent-lane,
and operator-surface diagnostics without redefining the framework-owned algorithm.

Owner boundary:

1. Generic algorithm law lives in `vida/config/instructions/instruction-contracts/overlay.step-thinking-protocol.md#section-bug-reasoning`.
2. This document is the `vida-stack` project overlay for applying that algorithm to local runtime work.
3. If this document conflicts with framework instruction law, use the framework owner and repair this project overlay.

## Trigger

Use this protocol for runtime blockers, authority/projection contradictions,
multi-defect families, proof failures, or command/output gates that can hide a
runtime defect.

## Scope

VIDA runtime, TaskFlow, DocFlow, run-graph, recovery, lane, dispatch, receipts,
operator surfaces, and their bounded fixtures/proof artifacts.

## Authority

The framework TRACE overlay owns the generic algorithm; this document owns the
VIDA project authority map, contour evidence, and runtime diagnostic application.

## Inputs

Use current runtime/task state, command evidence, receipts, projections,
persisted artifacts, owned paths, and the active proof target.

## Outputs

Produce an authority-domain map, bounded evidence packet, classified root batch,
proof matrix, residual sweep, and explicit blocker or reset state.

## Rules

Keep authoritative state ahead of projections, preserve missing evidence, and
route shared-boundary defects through the contour gate before writing.

## Forbidden

Do not infer authority from derived output, apply symptom-only fixes after the
two-related-defect trigger, fabricate receipts, or bypass validated proof rows.

## Escalation

Stop and escalate when authority, ownership, receipt, proof, or next-action
validation remains unresolved; see `Stop And Escalate`.

## Validation

Run the bounded focused proof, cross-surface matrix, DocFlow check, and residual
pattern sweep; record exact commands, timing, artifacts, and blockers.

## Token Budget

Keep default diagnostic output compact and artifact-backed while retaining every
field needed for authority, actionability, and closure proof.

## Metadata

Canonical owner: `process/project-error-search-runtime-diagnostics-protocol`.

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

## Authority-Domain Map Before Fix

Before selecting a fix locus, write one bounded authority-domain map. Every
observed symptom must name one primary authority domain and its projection,
persistence, consumers, and proof surface:

| Domain | Canonical authority | Required downstream map |
| --- | --- | --- |
| `config_registry` | project config and registered extensions | resolved defaults, validation, projection, config proof |
| `taskflow_authority` | TaskFlow task/step/run records | status/next-lawful, dependencies, closure proof |
| `persisted_state` | authoritative state store and migrations | adapters, reload/restart, identity/hash proof |
| `runtime_execution` | dispatch/run/lane receipts | consumers, recovery, terminal evidence |
| `public_contract` | CLI/help/output schema owner | default output, explicit JSON, next actions |
| `test_fixture` | fixture/golden/schema builder | unit/integration/smoke coverage and snapshots |

Record `authority_domain`, `authority_ref`, `projection_refs`,
`persistence_refs`, `consumer_refs`, and `proof_refs` before any write. Missing
or conflicting map entries are a blocker; a rendered projection or stale report
cannot be promoted to authority by convenience.

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

When two or more related defects, PR failures, CI failures, runtime blockers, or
operator-surface gaps are present, apply the Two-Defect Deep-Audit Trigger and
the contour pool below before selecting a local fix. More than two independent
events may still be batched when they share an owner, proof window, or release
gate.

1. restore the open family/contour ledger before counting or selecting a fix,
2. cluster by shared invariant before changing code,
3. identify whether failures are one root cause, dependent blockers, or independent slices,
4. pick one bounded root batch with the highest unblocking value,
5. write regression tests for the shared invariant,
6. batch expensive builds/tests after all tightly related fixes are in place,
7. keep unrelated dirty files out of the slice.

Do not run a long full gate after each tiny edit when focused tests can validate the same invariant first.

When closing a coherent batch of tasks, run the runtime self-diagnostic once for
the whole batch after the included tasks are closed and before selecting any
next work. Treat this batch self-diagnostic as closure evidence for the pool,
not as a replacement for per-task proof, and classify any findings before the
pool is considered fully closed.

## Two-Defect Deep-Audit Trigger

This trigger is permanent process law for every defect family; it is not a
one-off session heuristic. Keep a durable, append-only event stream in the
canonical TaskFlow task/pack artifact (or its referenced artifact) and restore
the stream before evaluating a new event. Use a two-stage family identity:

1. the provisional family signature is `(defect_type, owner_domain,
   normalized_transition_pattern, key_version)`; `surface_id` is evidence and
   must not partition related failures before root ownership is confirmed,
2. the confirmed family identity is `(invariant_id, owner_id, key_version)` plus
   the observed `surface_ids`/consumer set; one root cause across surfaces stays
   one family.

The bounded family scope is `(pack_id, epic_id, key_version)`. If either scope
id is missing, keep `scope=missing` and block counting rather than substituting
`session_id`. `session_id` and `worktree_id` remain required provenance, but a
restart or session handoff must not split a restored pack/epic family.
Canonical ids are required; aliases may
resolve to canonical ids only through the persisted alias map and must never
merge two ambiguous owners or surfaces silently.

### Trigger, window, and counting

1. A qualifying event is a fresh, evidence-backed defect or proof failure with
   the same provisional or confirmed family identity. A second qualifying event
   must be consecutive in that identity's bounded pack/epic event stream;
   unrelated keys,
   explicitly excluded events, and a completed reset close the sequence.
2. The first counted event opens the rolling window (`family_count=1`) and keeps the
   normal bounded slice available only while its owner and proof target remain
   clear.
3. The second counted event (`family_count=2`) is the deep-audit trigger. It extends
   the active batch to the shared owner/root boundary, freezes symptom-only
   patches for that family, and requires the audit and proof matrix below before
   closure or reset.
4. A third counted event while the audit is open (`family_count>=3`) raises the family
   to `third_point_fix_forbidden`: no point fix, fixture-only fix, renderer-only
   fix, or local workaround may land. Only the shared-owner root patch (or an
   explicitly recorded independent family) is admissible; the event extends the
   same batch and keeps the trigger open.
5. Count a retry only when it has a new failure shape, command/surface,
   environment/owner, or independently captured proof artifact. Repeated output
   from the same attempt, duplicate issue/task mirrors, stale reports, and
   non-reproducible noise do not increment the count; record the exclusion and
   its evidence instead.
6. A reset is explicit and durable. It requires a completed shared-owner audit,
   passing proof matrix, current freshness classification, no unresolved
   `blocked`/`tampered`/`alias_conflict` state, and a reset event naming the
   closing proof artifact. Session restart, task reassignment, alias renaming,
   or deleting an event is never an implicit reset.

### Contour Analysis Gate

The existing rolling trigger escalates to a contour analysis gate when any one
of these conditions holds:

1. two related defect/proof events are counted for the same canonical family;
2. the planned change crosses shared authority, schema, persistence, or routing;
3. one invariant has three or more downstream consumers or public/live surfaces;
4. a pre-release architectural rewrite, migration, or owner-boundary change is planned.

The gate may be opened earlier by explicit operator direction. It is one
extension of this Two-Defect trigger, not a second counting or reset system.

Two related fresh defects or proof failures are both the contour trigger and
the mandatory pattern-sweep trigger: freeze symptom-only fixes, map the shared
owner, and run the residual same-pattern sweep before reset. A single isolated
event may use ordinary TRACE only when its owner, authority map, and proof target
are explicit.

#### Contour axes

The contour artifact must inspect each applicable axis and mark unavailable
evidence as `missing`:

1. `source_config`: config, registry, defaults, templates, and source inputs;
2. `projection`: derived state, renderers, summaries, TOON/plain, and JSON;
3. `persisted_identity_hash`: persisted ids, keys, hashes, digests, sequence, and alias identity;
4. `adapter_reload`: state-store adapters, migrations, cache boundaries, restart, and reload;
5. `runtime_consumers`: callers, services, lanes, dispatch, recovery, and TaskFlow consumers;
6. `public_live_surfaces`: CLI/help/options, operator output, DocFlow, and live integrations;
7. `fixtures_tests`: fixtures, golden/snapshot data, unit, integration, smoke, and regression tests;
8. `template_schema_docs`: schemas, generated/template sources, docs maps, and runbooks.

#### Required contour artifact

Persist one append-only artifact for the bounded scope. It must contain these
fields (unknown values remain explicit `missing`):

| Field | Required content |
| --- | --- |
| `contour_id`, `scope`, `owner`, `invariant` | Stable contour identity, pack/epic/session/file bounds, accountable owner, and invariant to preserve. |
| `nodes`, `edges` | Axis nodes plus caller, data-flow, dependency, projection, and proof edges; include evidence refs. |
| `duplicate_authorities` | Every competing source, projection, schema, renderer, adapter, or next-action authority and its owner. |
| `confirmed_defects` | Freshness-classified defects/proof failures with event ids, commands, results, and evidence digests. |
| `coverage_gaps` | Uninspected axes, consumers, public surfaces, fixtures, schemas, or docs with the reason and owner. |
| `root_batch` | One shared-owner implementation batch, owned paths, dependent changes, explicit exclusions, and freeze status. |
| `proof_matrix` | Cross-product proof rows for changed axes and consumers, including default output, JSON, help, persisted/reload, and fail-closed cases where applicable. |
| `residual_risks` | Out-of-scope or blocked risks, severity, owner, follow-up TaskFlow id, and closure condition. |
| `reset` | `not_ready`/`ready` state, reset event id, closing proof artifact, and the existing Two-Defect reset evidence. |

#### Gate procedure

1. Create or restore the contour artifact before any write; link the triggering
   event ids and current freshness classifications.
2. Run parallel read-only research lanes by non-overlapping axes and owners;
   lanes produce evidence only and do not edit files, mutate runtime state, or
   close tasks. Consolidate their results into `nodes`, `edges`, defects, and gaps.
3. Freeze point fixes while the contour is open: no symptom-only, fixture-only,
   renderer-only, adapter-only, or local-workaround patch may land. The existing
   `third_point_fix_forbidden` state remains authoritative for a third event.
4. Select one `root_batch` at the shared owner boundary. Implement the invariant
   once, then update only its classified dependents and explicit proof/doc
   artifacts; unrelated owners remain excluded or become follow-up tasks.
5. Run cross-product proof across every changed contour axis and affected
   consumer/public surface. A green point test or one projection is insufficient;
   missing or contradictory rows keep `reset.state=not_ready`.
6. Run a read-only residual sweep for the same pattern across related files,
   helpers, adapters, persisted snapshots, fixtures, operator output, help,
   next-action text, templates, schemas, and docs. Classify every result as
   `eliminated_by_root`, `inside_root_batch`, `independent`, `blocked`, or
   `follow_up`; create/update a follow-up for residuals outside `root_batch`.
7. Set `reset.state=ready` only when the existing Two-Defect reset contract is
   satisfied and the artifact names the closing proof. Append the reset event;
   restart, reassignment, alias changes, or artifact deletion never reset it.

#### Contour Pool Scheduling

Maintain one open contour pool for every provisional or confirmed family in the
current pack/epic scope. Build a small dependency DAG before dispatch:

1. one node represents one open contour family;
2. add an edge when contours share an authority domain, owned path, conflict
   domain, persisted artifact, proof fixture, or dependency;
3. contours without an edge may run read-only axis research concurrently;
4. root-batch writes may run concurrently only after explicit disjoint admission
   proves separate owners, paths, conflict domains, and proof artifacts; any
   edge serializes the root batches;
5. integrate, attach proof, classify residuals, close, and reset contours through
   one serialized owner; a new event joins its existing family before any point
   fix is considered;
6. refresh the pool after every research return and preserve fair progress so an
   older open contour cannot be starved by a newer event.

#### Explicit direct fallback

When the configured runtime/host bridge is blocked, direct fallback is lawful
only after explicit operator authorization and a recorded blocker packet. The
fallback must still complete the contour gate read-only pass first, restrict
writes to `root_batch` owned paths, preserve the same proof matrix, and leave
TaskFlow closure, receipts, runtime-state mutation, release/install, and reset
pending until canonical runtime authority is restored. If the fallback cannot
prove the shared owner or required cross-product rows, remain blocked and
escalate; never use fallback to bypass the contour or fabricate evidence.

#### Proportional bounds and exclusions

1. `isolated`: one owner, at most two consumers, no shared authority/schema/
   persistence/routing change, no pre-release rewrite, and fewer than two
   counted related events. Use ordinary TRACE with one read-only evidence pass
   (10 minutes maximum) and record `no_contour_reason` with the scope,
   exclusions, and proof target in the task artifact.
2. `standard`: any contour trigger without a pre-release rewrite. Time-box
   read-only research to 30 minutes or three parallel lanes, whichever comes
   first; if evidence is incomplete, mark the contour blocked rather than
   weakening the gate.
3. `architectural_release`: pre-release rewrite or shared persistence/schema/
   routing migration. Time-box read-only research to 60 minutes or four lanes,
   require owner/architect review of `root_batch`, and carry every residual risk
   as an explicit follow-up before reset.

Syntax, import, toolchain, flaky/non-reproducible, formatting-only, and
unrelated-owner/invariant events are excluded from contour counting. Record each
excluded event and its `exclusion_reason` in the existing durable event contract;
do not let an exclusion hide a fresh qualifying event.

### Durable event contract

Each event must persist these fields (unknown values are explicit `missing`,
never inferred):

| Field | Requirement |
| --- | --- |
| `schema_version`, `event_id`, `occurred_at` | Stable schema version, unique id, and UTC timestamp. |
| `session_id`, `worktree_id`, `pack_id`, `epic_id`, `bounded_unit_id` | Runtime ownership and bounded-scope identity. |
| `invariant_id`, `owner_id`, `surface_id`, `key_version` | Canonical rolling-family key and versioned scope; aliases include their resolved id and map version. |
| `defect_type`, `owner_domain`, `normalized_transition_pattern`, `confirmed_family_id`, `surface_ids`, `pool_id`, `conflict_domain` | Provisional/confirmed family identity, cross-surface grouping, contour-pool membership, and scheduling conflict evidence. |
| `event_kind`, `failure_class`, `proof_kind`, `freshness`, `classification` | Defect/proof shape and `actual_now`/`partially_fixed`/`superseded`/`merged_into_broader_invariant`/`stale_not_reproduced` result. |
| `sequence_no`, `previous_event_id`, `family_count`, `trigger_state` | Consecutive-window accounting and states `none`, `audit_required`, `audit_active`, `third_point_fix_forbidden`, `reset`. |
| `command`, `exit_code`, `duration_ms`, `status`, `blocker_codes`, `next_actions` | Reproducible command and operator result; missing fields stay visible. |
| `artifact_refs`, `evidence_digest`, `owner_ack` | Receipt/log/proof locations, integrity digest, and shared-owner acknowledgement. |
| `counted`, `exclusion_reason`, `reset_of`, `tamper_reason`, `alias_resolution` | Why the event counted or was excluded, and any reset, integrity, or alias decision. |

### Audit and batch classification

At `family_count>=2`, stop symptom patching and inspect the shared owner/root
boundary plus every affected consumer/caller, command/output schema, CLI
help/next-action text, fixture and persisted-state builder, renderer, docs map,
alias map, and public integration harness. For each candidate change record one
classification: `shared_root`, `dependent`, `independent`, `stale`,
`excluded`, or `blocked`, with the evidence artifact and owner. The batch is
not complete while a `blocked` or unclassified consumer remains. The shared
root patch must preserve one invariant across all classified consumers and
update stale fixtures/docs rather than teaching each surface a new workaround.

### Required proof matrix

The trigger is only reset after this matrix is captured against the same
persisted family event stream:

| Case | Evidence fixture | Required result/public proof |
| --- | --- | --- |
| `default` | One counted defect with valid receipt and canonical key. | `family_count=1`, no audit trigger; default compact output exposes status/blockers/actions/artifacts and allows only a bounded owner-approved slice. |
| `blocked` | Two consecutive related failures where the authoritative command is blocked or proof is missing. | `family_count=2`, `trigger_state=audit_required`/`audit_active`; fail closed, freeze symptom patches, and expose a validated next action. |
| `tampered` | Digest, sequence, receipt, or owner evidence does not match persisted data. | `tampered` blocker; no clean count/reset, no inferred success, and public JSON/default output points to evidence repair. |
| `alias` | Alias is unknown, stale, or resolves to multiple owners/surfaces. | `alias_conflict` blocker; do not merge or increment silently; repair the canonical alias map before continuing. |
| `persisted` | Stop/restart between the first and second events, then reload the same pack/epic/session artifact. | Sequence, count, trigger state, and exclusions survive restart exactly; deletion or reassignment cannot reset the window. |
| `public` | Shared-root fix exercised through `--help`, default TOON/plain, explicit `--json`, and persisted-state/public CLI fixtures. | All surfaces converge on the same invariant, `blocker_codes`, `next_actions`, `artifact_refs`, audit state, and third-event point-fix prohibition. |
| `cross_surface` | Two fresh related events on different surfaces with one provisional signature/root candidate. | One family reaches `family_count=2`; `surface_ids` remain members of the same contour and do not create separate local fixes. |
| `contour_pool` | Two open contours with disjoint versus overlapping conflict domains. | Disjoint contours research in parallel; only explicitly admitted disjoint root batches may write in parallel; overlapping contours serialize. |
| `third_failure` | A third fresh related event arrives while audit is open. | `family_count>=3`, `third_point_fix_forbidden`; batch extends and only shared-owner root remediation remains admissible. |

The matrix is contract proof, not a replacement for per-task regression
tests. Attach the exact command, exit/timing, artifact refs, and event ids for
each row; missing or contradictory evidence keeps the family blocked.

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
artifact_revision: '2026-07-27'
schema_version: '1'
status: canonical
source_path: docs/process/project-error-search-runtime-diagnostics-protocol.md
created_at: 2026-05-26T00:00:00+03:00
updated_at: 2026-07-27T00:00:00+03:00
changelog_ref: project-error-search-runtime-diagnostics-protocol.changelog.jsonl
