# Team Development And Orchestration Protocol

Status: active project process doc

## Purpose

Define the project-owned operating protocol for manager-led multi-agent development so `.codex`, VIDA orchestration, and Release-1 execution all use the same bounded delivery-task model.

## Trigger

Apply this protocol before shaping, dispatching, reviewing, proving, or closing any project development, defect, CI, release, optimization, documentation, process, diagnostics, or operator-surface packet.

## Scope

This protocol defines:

1. the project-level team topology for development work,
2. the packet shape used for delegated development lanes,
3. how one backlog item is decomposed into delivery-task packets,
4. how orchestrator, implementer, coach, verifier, and escalation cooperate,
5. closure rules for packet-level work,
6. how packet shapes, prompt-stack precedence, and boot-readiness rules stay explicit,
7. how orchestrators phrase delegated packets so low-cost agents preserve source facts, scope, and proof evidence on the first attempt,
8. how delegated lanes report usage, proof, commit, and operator-gated publication or wave-close follow-up obligations.
9. how long-running epics optimize executor/validator routing after every task.

This protocol does not define:

1. framework bootstrap law,
2. framework-owned worker dispatch law,
3. product-law semantics for Release 1 capability or seam ownership,
4. Codex runtime schema itself.

## Authority

1. Option and capability authority is the master template `docs/framework/templates/vida.config.yaml.template` under `dev_team.authority_catalog`.
2. Selection authority is project `vida.config.yaml` under `dev_team.authority_selection`.
3. The machine contract is `vida/config/schemas/team-flow-authority.schema.json`; typed runtime projection and receipts outrank process prose.
4. Packet shape authority is `docs/process/project-development-packet-template-protocol.md`; public command routing is indexed by `docs/process/runtime-command-authority-inventory.md`.

## Inputs

1. source-classified request and active TaskFlow unit,
2. selected TeamFlow configuration, registries, typed lane projection, and command/profile references,
3. canonical packet template, owned/read-only paths, acceptance and proof targets,
4. runtime initialization, dispatch, handoff, receipt, and blocker evidence.

## Outputs

1. one bounded packet and selected-lane handoff schema,
2. receipt-backed lane result with verification, blockers, and residual risks,
3. TaskFlow actualization and selected-flow transition evidence,
4. focused proof/closure result or an explicit rework, escalation, or runtime-blocker state.

## Rules

The detailed rules below remain canonical in `Core Rule`, `TeamFlow Configuration Authority`, `Canonical Work Unit`, `Packet Contract`, runtime-first/parallel-pack rules, and `Closure Rule`. This index does not create a second role, flow, profile, or command catalog.

## Forbidden

1. Do not reconstruct active role, carrier, model, reasoning, flow, edge, or command authority from prose, order, or familiar identifiers.
2. Do not bypass packet ownership, receipt requirements, proof gates, or the root write guard.
3. Do not duplicate the master option catalog in this document or treat stable operator examples as TeamFlow authority.

## Escalation

The canonical escalation conditions remain under `Lane Responsibilities` / `### Escalation`; this top-level block is the authoring-contract pointer and does not add a second escalation policy.

## Validation

Validate packet readiness through `docs/process/project-development-packet-template-protocol.md`, selected-flow and receipt integrity through the TeamFlow schema/runtime, and closure through the declared proof target, TaskFlow evidence, and DocFlow checks.

## Token Budget

Carrier, model, reasoning, timeout, and cost selection come only from `vida.config.yaml`, registries, and runtime admission/telemetry. This protocol contains no fixed model or reasoning defaults; command batching follows `docs/process/command-timing-and-gate-optimization-protocol.md`.

## Core Rule

Project development runs as:

1. orchestrator-led,
2. delivery-task shaped,
3. delegation-first,
4. system-analysis-first for complex or write-producing work,
5. duplication-review-backed before final coach review,
6. coach-separated,
7. verification-backed,
8. skill-aware before bounded work begins,
9. fail-closed on missing packet data or shared-scope ambiguity.
10. explorer/read-only findings feed packet shaping, not root-session write ownership.
11. session-scoped: one blocked orchestrator session must not block another session's disjoint task in the same project root.
12. test-first for runtime/operator defect remediation, with the configured test-authoring lane before the implementation lane.
13. TaskFlow-actualized at every layer: new evidence must update task status, parent/child placement, priority, dependencies, proof targets, execution semantics, and sequential/parallel posture before the next lane is dispatched.
14. source-neutral: pull requests, defects, external downstream reports, CI failures, release tasks, optimization tasks, documentation/process tasks, operator-surface gaps, diagnostics, and newly discovered work all follow the same spec-first intake, TaskFlow actualization, configured-role chain, proof, and closure discipline.
15. runtime-first: every executable task must attempt the configured VIDA runtime flow before root-session implementation; manual execution is allowed only after a recorded runtime blocker prevents the lawful lane path.

## Team Topology

The active project development team is the runtime-selected projection from the configured TeamFlow authority:

1. one root orchestrator session owns framing, decomposition, packet routing, synthesis, and closure decisions,
2. configured carrier tiers provide bounded write, analysis/test-authoring, independent verification, and architecture-escalation capabilities,
3. runtime roles, carrier identities, and admissibility are resolved from the selected typed projection rather than from this document.

Multiple orchestrator sessions:

1. each root orchestrator session is a separate controller over shared DB-first project truth,
2. each controller must hold claims for its active planning, dispatch, write, proof, or recovery work,
3. parallel orchestrators are lawful only when task/run identity, owned paths, and exclusive conflict domains do not overlap,
4. foreign blocked sessions remain visible to the team but are not inherited as the current session's active bounded unit.

The configured development chain is the selected flow's explicit ordered lane projection. The process describes lane capabilities without owning their active identifiers:

1. an analysis/specification lane researches the bounded task, contracts, architectural context, acceptance targets, owned paths, and duplication risks before implementation,
2. a test-authoring lane writes or specifies the failing regression proof before implementation for test-first defect work,
3. a pre-implementation quality gate confirms that proof matches the spec/runtime evidence and is not a weak fixture,
4. a bounded write lane implements only after required upstream handoffs are present,
5. a post-implementation quality gate reviews the result against the brief, accepted test, spec, acceptance targets, and expected handoff,
6. an independent reuse review checks existing framework/runtime contracts, duplicate semantics/operator surfaces, and dead or unwired helpers,
7. independent verification/proof lanes gate release closure when the selected flow includes them.

Runtime-first execution rule:

1. For each task or subtask, the orchestrator must bind the active bounded unit, resolve the selected flow's startup/dispatch command references, and execute the next included lane through the registry-mapped runtime surfaces. Public command families are indexed in `docs/process/runtime-command-authority-inventory.md`; this protocol must not hardcode a TeamFlow command sequence.
2. The configured runtime sequence is authoritative. If a project pack exposes four runtime roles but expands the worker role into multiple lane steps, the orchestrator follows the lane sequence returned by runtime instead of assuming a hardcoded count.
3. Lanes run sequentially unless TaskFlow execution semantics, dispatch preview, owned paths, and conflict domain all report parallel-safe admission.
4. Host-tool subagents are adapter carriers for runtime packets. Their result is not complete until the parent orchestrator submits or records a runtime receipt through the host-bridge/lane surface.
5. If runtime cannot execute the configured chain, the orchestrator records the failing command, `blocker_codes`, artifact paths, and active task/run identity before entering bounded Defective Runtime Emulation Mode.
6. Defective Runtime Emulation Mode must preserve every included configured lane, evidence requirement, proof gate, approval pause, command mapping, and terminal condition; it must not substitute a conventional lane sequence.
7. Manual emulation never converts into permission to skip the runtime defect. The blocking runtime issue must be created or updated, prioritized, and linked to the current task before unrelated implementation continues.

Parallel pack execution rule:

1. A parallel pack is one explicit `order_bucket` with every included task marked `execution_mode=parallel_safe`, the same `parallel_group`, and a distinct `conflict_domain`.
2. The pack is analyzed as one unit before dispatch: owned paths, proof targets, expected shared contracts, and likely integration conflicts must be known before the first write lane starts.
3. Write lanes may run at the same time only when their owned paths and conflict domains are disjoint. TaskFlow graph mutation, proof attachment, task close, release install, and git publication remain sequential.
4. The orchestrator must not implement one task, test it, close it, then start the next task when the operator requested simultaneous pack development. The intended order is pack analysis, parallel lane dispatch, one root integration batch, one focused proof batch, one broad diagnostic gate, then pack closeout.
5. Completed lane returns are synthesized together. Root integration may fix cross-pack regressions, but those fixes must stay inside the pack's shared invariant or become a separate TaskFlow item.
6. Pack closeout requires structured proof evidence on each task before the selected TaskFlow closure transition. Close reason text is not proof authority.
7. After all task closes, run the selected flow's configured post-pack reconciliation and orchestrator-readiness gates through their registry command references. A remaining `closed_task_active_run_projection_mismatch` is a runtime blocker or follow-up, not a clean close.
8. If the broad package/workspace gate fails after focused proof is green, classify the failures as `inside_pack`, `adjacent_regression`, or `outside_pack_residual` before closeout. Close only the tasks whose acceptance and focused proof are satisfied, and record the residual count and boundary in structured proof evidence.

## TeamFlow Configuration Authority

1. The exhaustive option and capability/admissibility catalog is `docs/framework/templates/vida.config.yaml.template -> dev_team.authority_catalog`.
2. The machine contract is `vida/config/schemas/team-flow-authority.schema.json`; project `vida.config.yaml -> dev_team.authority_selection` may select only catalog-declared options.
3. The selected flow, explicit edges, approval policy, rework/resume targets, terminal declarations, role/profile authority, and command refs are config/registry facts. Runtime and process prose must not reconstruct them from ids, order, or conventions.
4. The compiled bundle binds roles, skills, profiles, flows, packs, commands, and dispatch aliases before producing deterministic component and aggregate authority identities.
5. Each resolved lane must preserve the complete typed projection required by the schema. Missing, malformed, conflicting, or unresolved authority yields a typed blocker; legacy fallback lane shapes are not executable.
6. Team role/profile authority is distinct from the runtime-selected model profile. A carrier/model change does not rewrite flow authority.
7. Terminal closure requires a config-declared terminal edge. Approval pending, approval rejection, and rework request are distinct non-success outcomes.
8. Duplicate or shadowing aliases and conflicting explicit edges fail closed. Source order never decides authority.
9. Other docs may explain this process but must not copy the option catalog and become a second source.

## Canonical Work Unit

The canonical delegated work unit is one `delivery_task` or one `execution_block` packet.

It must map to:

1. one owner,
2. one dominant goal,
3. one bounded write scope or one bounded read-only scope,
4. one proof target,
5. one explicit stop condition.

Forbidden shapes:

1. one packet for a whole feature,
2. one packet spanning unrelated frontend/backend/schema/infra changes,
3. one packet with no explicit done rule,
4. one packet that depends on a later slice to justify its own closure.

## Packet Contract

Every delegated development packet must include:

1. `packet_id`
2. `backlog_id`
3. `release_slice`
4. `owner`
5. `closure_class`
6. `goal`
7. `non_goals`
8. `scope_in`
9. `scope_out`
10. `owned_paths` or `read_only_paths`
11. `definition_of_done`
12. `verification_command`
13. `proof_target`
14. `stop_rules`
15. `blocking_question`
16. `handoff_target`

TaskFlow actualization rule:

1. every packet must either create, update, or cite the active TaskFlow item that owns it,
2. lane returns must update task notes or a linked artifact with role, evidence, blockers, proof result, and next lane,
3. new blockers or defects discovered by any lane must be placed under the correct parent/epic before the next write-producing lane starts,
4. after each update, the orchestrator must re-evaluate priority, dependencies, parent/child layer, conflict domain, and sequential/parallel admissibility,
5. diagnostic findings, including global-goal happy-path progress failures, must become task updates or child tasks before the next write-producing lane starts,
6. stale task ordering, stale dependencies, missing proof targets, or unrecorded lane handoffs are process defects.

Source-neutral intake rule:

1. PR processing is only one concrete work-source workflow; it does not weaken the same requirements for non-PR work.
2. Every bounded source must be classified before write-producing work: `pull_request`, `external_downstream_report`, `runtime_defect`, `ci_failure`, `release_task`, `optimization`, `documentation_process`, `operator_surface_gap`, or a more specific project-approved source type.
3. The orchestrator must consult the mapped canonical spec or process surface for that source class before implementation, then record the consulted surface, expected behavior, acceptance target, proof target, priority reason, role chain, and sequential/parallel posture on the TaskFlow item or linked artifact.
4. If the mapped spec is missing, contradictory, or too weak to define acceptance, create or update a specification-clarification task before changing implementation code.
5. Source-specific protocols add source evidence and closure details only; they do not replace the generic intake, role-chain, proof, or closure law.
6. Defects, downstream reports, CI failures, release tasks, optimization work, documentation/process tasks, diagnostics, and operator-surface gaps must be handled with the same TaskFlow and configured-role discipline as PRs.
7. When VIDA cannot execute the configured lane projection because the runtime itself is defective, record or update that runtime defect separately and enter bounded Defective Runtime Emulation Mode while preserving every selected lane, evidence requirement, proof gate, approval pause, rework transition, command mapping, and terminal condition manually.
8. No defect, downstream report, PR finding, CI/release signal, optimization idea, command-surface gap, or process correction may remain only as chat/session memory when TaskFlow mutation is available.

Readiness rule:

1. readiness is template-specific and must follow `docs/process/project-development-packet-template-protocol.md`,
2. `delivery_task_packet` and `execution_block_packet` are invalid if any of `goal`, `scope_in`, `owned_paths` or `read_only_paths`, `definition_of_done`, `verification_command`, `proof_target`, `stop_rules`, or `blocking_question` is missing,
3. `coach_review_packet` is invalid if any of `review_goal`, `owned_paths` or `read_only_paths`, `definition_of_done`, `proof_target`, or `blocking_question` is missing,
4. `verifier_proof_packet` is invalid if any of `proof_goal`, `verification_command`, `proof_target`, `owned_paths` or `read_only_paths`, or `blocking_question` is missing,
5. `escalation_packet` is invalid if any of `decision_needed`, `options`, `constraints`, or `blocking_question` is missing,
6. invalid packets must be reshaped by the orchestrator before delegation.

Workflow-spec annex rule:

1. when a task needs multi-lane or multi-agent work, the orchestrator must attach a `workflow_spec` annex before dispatch,
2. the annex must name DAG steps, dependencies, output schemas, proof gates, readiness, cost budget, write scope, sandbox, `fanout_min_results`, merge policy, retry policy, timeout policy, and partial-result disclosure,
3. the configured lane projection remains authoritative; the annex may specialize steps but must not silently skip required analysis, test-authoring, review, verification, reuse-review, or closure gates required by runtime state,
4. parallel fanout is lawful only when TaskFlow scheduling, owned paths, conflict domains, and the annex graph all agree,
5. per-lane attempts and consolidators use configured carrier/model profiles selected by runtime admission; hardcoded provider or model authority in packet prose is invalid,
6. explorer/read-only results, external CLI outputs, or independent model reports are lane outputs to synthesize, not root-session permission to write,
7. synthesis must update TaskFlow with the accepted result, conflicts, partial evidence, or blocker before the next write-producing lane starts.

Template rule:

1. render project packets using `docs/process/project-development-packet-template-protocol.md`,
2. do not treat prose-only delegation as a valid substitute for the canonical packet family.

## Delegated Prompt Quality Rule

When a task is delegated to a low-cost or small implementation carrier, the orchestrator must bias the packet toward exactness rather than broad interpretation.

Small-agent packet requirements:

1. name exactly one bounded task id and title,
2. name the model or carrier constraint only when the operator explicitly requires it or runtime selection already selected it,
3. list the complete write scope and say that all other files are out of scope,
4. name the source-of-truth file and required source sections or line ranges,
5. state whether the lane should copy source wording closely, summarize, refactor, or implement new behavior,
6. include source-fidelity rules for copied commands, fixture specs, blocker codes, paths, and negative assertions,
7. include forbidden shorthand when compression would damage the artifact, such as `cur`, `src`, `req`, `impl`, `cmd`, `cfg`, or unexplained single-letter markers,
8. require honest status language for unexecuted work, such as `pending`, `not run`, or `not created`,
9. include a self-check block that the lane must run before its final response,
10. require a final response with changed files, exact summary, verification, remaining gaps, token usage, step count, and tool-call count,
11. include the intended validation lane and the rework trigger before dispatch, so the small executor knows that partial output is acceptable only as evidence and not as self-approval.

Small-agent reasoning rule:

1. when the operator or runtime selects a cheap executor profile for development or test-authoring packets, dispatch it with that profile's configured reasoning effort,
2. do not lower a selected executor profile's configured reasoning effort for runtime authority, TaskFlow, DocFlow, host-bridge, path-policy, receipt, or release-gate work unless the operator explicitly overrides the bounded packet,
3. a higher reasoning budget does not authorize self-approval; root orchestration and an independent validator remain required for authority-sensitive work.

Dynamic model-routing rule:

1. keep `vida.config.yaml`, registry identity, and runtime assignment as the carrier/profile owner; concrete model names and reasoning levels are never permanent framework law,
2. select the cheapest eligible configured profile for read-only decomposition, PR/task mapping, source-sync documentation, and exact test-only patches when the write scope is one small surface and the proof command is explicit,
3. dispatch the selected profile with a hard timeout, one expected artifact, and a required final-report telemetry block; timeout, shutdown, empty artifact, missing telemetry, or self-approval without proof is `process_failure`,
4. reassign implementation to the next admissible configured profile when a cheaper attempt times out, shuts down, under-covers acceptance criteria, touches production/runtime authority code, or a validator rejects closure,
5. select one stronger configured validator for authority-sensitive runtime, TaskFlow, DocFlow, host-bridge, path-policy, receipt, release, or public operator-surface work,
6. use only one strong validator by default; add a focused cheap validator only
   for one named risk, and add parallel or triple validation only when the patch
   changes production authority paths, validators disagree, the first validator
   reports medium-high residual risk, or the packet closes a wave, epic, or
   release gate,
7. close or delete each completed or failed agent handle immediately after
   classifying it as accepted evidence, partial evidence, process failure,
   false-green, or stale,
8. score every executor and validator attempt on a 10-point scale after
   synthesis, record the reason, and use the score only as local routing evidence
   for future packets, not as proof that the current task is closed.

Wave-first routing rule:

1. in long-running refactor epics, model routing is evaluated inside the current
   wave first; do not select unrelated ready leaves only because they are cheap,
2. before launching a task, inspect whether finishing it moves a wave closer to
   closure, removes a parent blocker, or only increases leaf percentage,
3. prefer the task that reduces wave closure distance when proof risk and write
   scope are comparable,
4. record the updated task percent and wave percent after closure,
5. when a wave reaches closure-ready, stop selecting leaf work and close the
   wave parent with release/install and self-diagnostic evidence.

Three-step task execution rule:

1. Delegate and self-proof:
   - dispatch one bounded executor lane with exact task id, owned paths, invariant,
     non-goals, proof bundle, and final-report telemetry requirements,
   - wait long enough for cheap models before classifying timeout,
   - require the executor to run the focused proof bundle before returning.
2. Bundle validation:
   - the orchestrator runs one compact local proof bundle instead of repeated
     per-command micro-gates,
   - run one stronger validator over the diff and proof evidence,
   - if the validator rejects, send one exact blocker-focused rework packet
     instead of reopening broad research.
3. Close and publish:
   - when proof and validator pass, record TaskFlow evidence, close the task,
     commit only scoped files, push, update PR state, and update agent evaluation
     documentation in one continuous pass,
   - do not add extra exploratory checks unless new evidence contradicts closure.

Escalate out of the three-step loop only when validators disagree, the same
invariant is rejected twice, dirty-file overlap blocks scoped commits, the
executor skipped required public proof, or the task needs a shared architectural
decision. The three-step loop preserves quality gates while reducing
root-session micro-operations.

Post-task optimization rule:

After every task packet, before selecting the next task, the orchestrator must
complete or explicitly block this checklist:

1. TaskFlow item updated, closed, or left open with an exact blocker.
2. Focused proof bundle and debug build result recorded.
3. Scoped code/test/doc commit created and pushed when the active publication
   pattern authorizes it.
4. Executor/validator scorecard recorded in TaskFlow closure evidence or the
   relevant process document and committed/pushed when that document changed
   under the same publication pattern.
5. Completed host-agent handles closed or deleted; cleanup failures recorded by
   handle id and blocker.
6. Executor and validator scored on a 10-point scale with token/tool-call
   telemetry or `not_exposed_by_host`.
7. The canonical Post-Task Self-Analysis STOP gate from
   `docs/process/project-orchestrator-operating-protocol.md` passed, including
   base fields, 20 fixed criteria, dynamic criteria created from the latest
   session segment, and meta-analysis remediation outcomes.
8. Rework count, false-green risk, residual risk, and next model-routing rule
   recorded.
9. Dirty-worktree hunks outside the bounded invariant preserved unstaged or moved
   to a follow-up TaskFlow item.
10. Parent and wave `closure-ready` checked when the task changed closure
    distance.
11. Epic progress reported as both task percent and wave count.
12. Any slow command, missing CLI option, runtime ambiguity, or documentation
    inconsistency classified as a TaskFlow optimization/runtime defect.

The checklist is a closure gate, not a final-report template. Missing checklist
items require rework, follow-up, or an explicit blocker before unrelated work is
selected.

Optimized packet launch rule:

1. every small-agent packet must include exactly one task id, one goal, one
   owned/read-only scope, one proof target, one timeout, one artifact schema, and
   one stop condition,
2. prompts must say whether the expected output is `read_only_analysis`,
   `test_patch`, `implementation_patch`, `validation_report`, or
   `pr_disposition_report`,
3. prompts must explicitly forbid closure authority unless the runtime packet is
   a closure lane,
4. prompts must require `tokens_used`, `steps_taken`, `tool_calls_used`, changed
   files or reviewed scope, proof status, blockers, and residual risks,
5. the orchestrator must classify each return as `accepted_evidence`,
   `partial_evidence`, `rework_required`, `process_failure`, `false_green`, or
   `stale` before launching another agent for the same stage.
6. when a mini attempt is still consuming tokens, producing progress, or can be
   resumed by the host, do not close it on the first short wait timeout; wait at
   least one longer interval or send a compact final-report request before
   classifying `process_failure`.

Delegated final-report telemetry rule:

1. every delegated lane final report must include `tokens_used`, `steps_taken`, and `tool_calls_used`,
2. if the host runtime exposes exact token usage, report the exact value,
3. if exact token usage is not exposed, write `tokens_used: not_exposed_by_host` and do not invent a number,
4. `steps_taken` must count meaningful reasoning/action stages completed by the lane,
5. `tool_calls_used` must count shell, read/search, edit, test, build, VCS, browser, MCP, or host-tool calls made by that lane,
6. every delegated lane final report must also include `changed_files` or
   reviewed scope, `verification`, `gaps`, blockers, and residual risks,
7. missing telemetry makes the lane report incomplete and requires rework or
   orchestrator notation before the result can be accepted.

Source-derived documentation packets must add an acceptance gate that names mandatory copied lines. For example, if the source contains proof commands or fixture invariants, the packet must list the exact commands, paths, blocker codes, and negative assertions that must appear in the target artifact. A delegated lane that omits any mandatory copied line is not closure-ready; route it back as rework instead of accepting a polished summary.

Use this compact packet addendum when shaping small-agent documentation or spec-sync work:

```text
Source fidelity:
- Prefer exact copying from the source document over summarizing.
- Preserve command order and fixture structure.
- Preserve Surface, Setup, Expected, blocker codes, paths, and negative assertions.
- Do not invent labels, severities, or terminology unless the packet explicitly asks for derived tracking labels.
- Use normal professional wording. Forbidden shorthand: cur, src, req, impl, cmd, cfg, C from source.

Acceptance gate:
- Required copied lines:
  - <exact command, path, blocker code, or invariant>
  - <exact command, path, blocker code, or invariant>
- If any required line is missing, fix before final.

Self-check before final:
1. Search the changed files for forbidden shorthand.
2. Verify every required copied line exists in the target.
3. Run `git diff --check -- <changed_files>`.
4. Read the final changed sections and compare them with the named source range.

Final response:
- files changed
- exact changes made
- verification commands and results
- remaining gaps or risks
- changed_files or reviewed scope
- verification
- gaps
- tokens_used
- steps_taken
- tool_calls_used
- whether the lane considers the packet closure-ready or partial evidence only
```

Use this compact packet addendum when shaping cheap executor work:

```text
Execution packet controls:
- One task id:
- One goal:
- Owned paths:
- Out-of-scope paths:
- Required source facts:
- Required proof command:
- Stop immediately if:
  - the proof target requires unrelated files,
  - the task needs a broader authority decision,
  - the patch changes outside owned paths,
  - or the proof fails for a reason not explained by the patch.

Validation contract:
- The executor must not self-close the task.
- The orchestrator will validate locally and route one focused validator.
- For authority-sensitive work the validator is the runtime-selected admissible profile.
- Partial output may be accepted as evidence, rejected, or sent to the next configured rework profile.
- Timeout, missing artifact, missing telemetry, or proof-free self-approval is a
  process failure, not partial success.
- After synthesis, close the host-agent handle before dispatching a replacement
  for the same packet stage.
```

## Decomposition Rule

Backlog decomposition must proceed in this order:

1. `Release slice`
2. `owner`
3. `layer` or `seam segment`
4. `closure class`
5. `delivery_task`
6. `execution_block` when further split is needed

Project rule:

1. do not delegate directly from epic or milestone wording,
2. do not give the implementer a packet still shaped like a backlog paragraph,
3. if one packet still crosses multiple mutable contracts, split again.

## Default Decomposition Depth

The default stopping point for project task shaping is `delivery_task`.

Use `delivery_task` as the delegated leaf when all are true:

1. one dominant goal exists,
2. one owner exists,
3. one bounded write scope or one bounded read-only scope exists,
4. one verification command or proof target is sufficient for closure,
5. one configured test-first lane cycle can judge closure without further subdivision.

Split further into `execution_block` only when at least one is true:

1. the candidate task still spans more than one mutable contract,
2. the candidate task crosses more than one crate or owner boundary,
3. the candidate task mixes refactor and feature closure in one packet,
4. the candidate task mixes implementation and seam/proof closure in one packet,
5. `definition_of_done` is still too broad to be judged in one bounded lane cycle.

Depth rule:

1. do not default to `execution_block` for every task,
2. do not stop at `milestone` or `epic` for delegated work,
3. use the shallowest lawful leaf that still preserves one-owner closure,
4. if a `delivery_task` still sounds like a feature paragraph, it is not ready.

Just-in-time split rule:

1. project launch readiness requires a lawful `delivery_task` queue, not a backlog-wide `execution_block` tree,
2. split a `delivery_task` into `execution_block` only when that task becomes the next active item or a near-critical-path item that is about to be dispatched,
3. do not pre-split the whole backlog into `execution_block` leaves by default,
4. if a future backlog item is still far from execution, keep it at `delivery_task` until real dispatch shaping begins.

Size heuristic:

1. prefer packets with one dominant change and one proof target,
2. prefer one bounded writable cluster, usually within 1-3 directly related files,
3. allow up to 5 related files only when they still form one coherent owner surface,
4. split again when neighboring file changes represent separate closure classes rather than one coherent packet.

## Lane Responsibilities

### Orchestrator

The orchestrator must:

1. classify the active request,
2. bind it to the active backlog item or bounded execution unit,
3. initialize the available skill catalog and activate the relevant skill set,
4. shape one lawful packet,
5. choose the lane sequence,
6. keep writer ownership singular,
7. synthesize coach and verifier returns,
8. decide closure or rework.
9. reroute partial implementer returns instead of absorbing the same write scope locally by inertia.
10. keep delegated lane state explicit and avoid closure-style final reporting while delegated work remains open.
11. preserve the full write-producing lane cycle after bounded read-only findings instead of collapsing directly into local patching.
12. treat project-delegated execution as the runtime-selected lane flow; host executor subagent APIs remain optional carrier details and are not the canonical packet-dispatch surface.

The orchestrator must not:

1. act as the default local writer for normal development work,
2. delegate a packet with ambiguous writable scope,
3. skip coach or verifier when the packet still requires them,
4. stop after an interim report while the bounded packet still has an owed next step,
5. enter local write work without an explicit recorded exception path.
6. treat a partial implementer return as implicit permission to finish the packet locally in the same write scope.
7. emit final closure for the packet while implementer/coach/verifier/escalation handoff state is still open or unsynthesized.
8. treat explorer findings as implicit permission to skip implementer/coach/verifier routing for normal write-producing work.
9. treat delayed or hanging delegated lanes as permission to absorb the packet locally while the delegated cycle still remains open.
10. silently replace the active packet with the first locally failing test or compile error and then treat that narrower symptom fix as packet closure.
11. treat a dirty worktree, same-scope partial diff, or partially applied delegated patch as implicit transfer of writer ownership back to the root session.
12. treat a worker wait timeout, empty poll result, or late implementer response as permission to collapse the packet into one generic development lane or root-session self-development.

### Implementer

The implementer must:

1. execute one packet,
2. activate the relevant skills before packet work begins,
3. stay inside assigned write scope,
4. return changed files, verification result, blockers, and residual risks.
5. make partial or unresolved state explicit when the packet is not closure-ready.

The implementer must not:

1. widen the packet,
2. self-approve closure,
3. silently absorb neighboring backlog work.

### Coach

The coach must:

1. review the packet result against the approved spec, acceptance criteria, and `definition_of_done`,
2. activate the relevant skills before packet review begins,
3. identify rework signals,
4. return bounded corrective guidance or explicit forward approval.

The coach must not:

1. replace the verifier,
2. convert review into milestone-wide architecture scope,
3. silently accept missing proof.
4. silently widen a stalled implementer packet into generic development or root-session coding.

### Verifier

The verifier must:

1. run or assess the declared proof target,
2. activate the relevant skills before proof work begins,
3. judge closure readiness of the packet,
4. fail closed on missing evidence.

The verifier must not:

1. act as a second coach,
2. widen into implementation unless explicitly rerouted,
3. treat neighboring packet evidence as implicit proof.

### Escalation

Escalation is lawful only when:

1. write scopes collide,
2. packet boundaries cannot be made coherent,
3. architecture conflict blocks normal closure,
4. repeated rework still leaves one unresolved design decision.

## Default Lane Sequence

For write-producing packets, the lane sequence is the selected configured flow's explicit ordered projection. Conditional lanes are evaluated through the configured inclusion rule and remain visible with their inclusion result. The process protocol does not define a fallback role sequence.

The orchestrator shapes the packet before the first included lane and synthesizes results after the configured terminal/admission conditions are satisfied. Those orchestration responsibilities do not add implicit TeamFlow edges.

Explorer-to-writer rule:

1. when explorer or other read-only lanes find a bounded writable gap, that result feeds the next packet,
2. the next lawful write-producing sequence still remains the configured test-first agent chain unless a recorded exception path says otherwise,
3. “the gap is already obvious” is not a valid reason to collapse into local patching.

Partial-return reroute rule:

1. if implementer returns non-closure-ready state, the next step is orchestrator reroute rather than implicit root-session writing,
2. reroute may produce:
   - fresh implementer rework packet,
   - coach review packet when bounded critique is the blocker,
   - verifier/escalation packet when closure law requires it,
   - or explicit exception-path receipt for local repair
3. same-scope local completion by the root session is forbidden unless that exception path is recorded first.
4. if coach/review/verifier then finds a concrete compile blocker in those same mutated files, the packet still remains under reroute law; the finding does not silently transfer repair authority to the root session.
5. "leave no broken packet behind" is a valid risk observation but not a substitute for an explicit exception-path receipt.

Delegated-closure rule:

1. if a delegated lane is still active or its handoff remains unresolved, packet closure is not ready,
2. local workaround work does not by itself close the delegated lane,
3. final packet closure requires either:
   - synthesized delegated returns,
   - explicit supersession/redirection receipt,
   - or explicit blocker/escalation state
4. open delegated state also blocks root-session takeover for the same packet unless supersession or hard-blocker evidence is recorded first,
5. a pre-write exception-path receipt alone does not bypass an otherwise still-lawful delegated cycle,
6. progress reporting during rework or post-dispatch in-flight state remains non-blocking only,
7. when one packet closes and synthesized team evidence already names the next lawful packet, use that result for immediate rerouting/continuation rather than closure-style reporting.
8. after each closure-ready task packet, the orchestrator must run the declared
   debug build/proof gate, update TaskFlow state, commit, and push when the
   operator has authorized the current publication pattern; failed or blocked
   packets still require TaskFlow state update and handle cleanup.
9. after a wave parent closes, release-build and install the system `vida`
   binary through the normal release path, then smoke the PATH-resolved `vida`
   before treating the wave as operationally closed.
10. open pull requests are source-neutral work items; process them in parallel to
   epic work only when they are bound to an explicit TaskFlow item and their
   write scopes, GitHub mutations, and proof gates do not conflict with the
   active epic packet.

For read-heavy or proof-only packets, the orchestrator may use:

1. orchestrator shaping
2. verifier only
3. orchestrator synthesis

or

1. orchestrator shaping
2. coach
3. verifier
4. orchestrator synthesis

## Agent Engagement Rule

Use delegated agents by default for write-producing work.

Default engagement policy:

1. the orchestrator owns shaping, routing, synthesis, and closure decisions,
2. the selected test-authoring lane and its resolved carrier own the bounded failing-test/regression authoring packet when test-first proof is required,
3. the runtime-selected bounded-write lane owns one implementation packet,
4. the runtime-selected quality-gate lane owns bounded test and implementation review,
5. the runtime-selected verification lane owns independent proof and closure readiness,
6. the exceptional architecture-escalation lane activates only when normal packet closure cannot be made coherent.

Host-tool permission bridge:

1. this project's default development posture is VIDA agent orchestration when `vida.config.yaml -> autonomous_execution.agent_only_development` is true,
2. a VIDA `agent-init` dispatch packet, downstream packet, or host-tool bridge request may select an admissible configured carrier for that bounded lane,
3. if the host agent API has its own "use only when explicitly requested" rule, this project policy, `agent_only_development`, sticky continuation, and the current bounded VIDA dispatch do not satisfy that host-tool explicit-request requirement by themselves,
4. host subagent/delegation bridge permission requires an explicit current user instruction or host approval surface authorizing that host path for the bounded work, and then authorizes only that requested bounded lane without permitting broad subagent spawning, overlapping write scopes, root-session implementation, or closure without receipt-backed evidence,
5. when the current conversation also contains user direction such as `agent-first`, `parallel agents`, `продовжи агентами`, or sprint-wide orchestrator mode, treat that direction as sticky for VIDA TaskFlow routing until the user explicitly stops or narrows it, but do not treat ambiguous continuation or project policy as permission to bypass host-tool subagent approval gates.

Agent-init interpretation rule:

1. The configured lane-init/packet-consumption surface is not by itself an execution-complete receipt; public command routing is indexed in `docs/process/runtime-command-authority-inventory.md`,
2. A lane-init dispatch packet or downstream packet does not transfer writer ownership back to the root session,
3. if the activated packet is still a `tracked_flow_packet`, that lane is shaping/materialization-only until runtime emits a concrete write-producing packet with bounded ownership,
4. absence of that later write-producing packet is a blocker/reroute condition, not implicit permission for local root-session patching.

Local orchestrator-only work is lawful only for:

1. shaping or reshaping packets,
2. bounded read-only analysis,
3. proof-only checks,
4. recorded saturation or escalation exceptions.

Clarification:

1. "very small one-file fix" is not by itself a lawful reason to bypass delegation, exception-path gating, or open delegated-cycle law,
2. if the work is write-producing, local root-session handling still needs the same exception/supersession gates as any other packet.

Forbidden shortcut:

1. do not keep work local merely because the backlog item is familiar,
2. do not skip delegation for normal write-producing work once the packet is lawful,
3. do not use multi-agent fanout for one packet with overlapping writable scope.
4. do not treat backlog-wide `execution_block` pre-splitting as a substitute for lawful just-in-time packet shaping.

## Closure Rule

A packet is closure-ready only when:

1. the assigned lane completed its bounded scope,
2. `definition_of_done` is satisfied,
3. the declared verification command or proof target passed,
4. no unresolved scope widening occurred,
5. residual risks are recorded explicitly.

After each closure-ready task:

1. update the TaskFlow task state with the completion, rework, or blocker result,
2. run the declared debug build for the touched workspace or the broader workspace when no narrower debug build is declared,
3. commit only the bounded closed scope after the task state update and debug build pass,
4. push the task commit when a current explicit operator instruction authorizes
   that specific task, publication batch, or repeatable publication pattern; the
   active epic instruction to push after every task is such a repeatable pattern
   until the operator pauses, revokes, or narrows it,
5. record the executor/validator scorecard and next routing rule in TaskFlow
   closure evidence or the relevant process document,
6. commit and push any changed scorecard-bearing document under the same active
   publication pattern,
7. leave unfinished red-test or rework files unstaged unless they are the explicit closed scope.

After each closure-ready wave:

1. close or update every child TaskFlow item before closing the wave parent,
2. run the declared wave proof and debug build,
3. build and install the release binary through the current project-approved release operator surface so the system `vida` on the normal PATH matches the closed wave,
4. smoke-check the installed binary through the normal PATH with the wave-declared installed-runtime operator surface,
5. push the wave closure state only when a current explicit operator instruction authorizes that specific wave, publication batch, or repeatable publication pattern; wave closure and a clean commit are not authorization by themselves,
6. run `docs/process/github-pr-processing-protocol.md` from wave closure only when a current explicit operator instruction authorizes that specific PR-processing batch or repeatable PR-processing pattern.

If any of those fail:

1. return to rework,
2. re-shape the packet,
3. or escalate.

## Mapping To Current Release-1 Work

For the active Release-1 execution line:

1. `R1-Bxx` backlog items are not yet delegated packets,
2. each `R1-Bxx` must first be split into one or more delivery-task packets,
3. only those packet leaves may be delegated into `.codex` agent lanes.

## Bootstrap Rule

After bootstrap, development agents must know immediately:

1. project-local TaskFlow runtime is entered through `vida taskflow`,
2. task lifecycle truth is the config/runtime-resolved authoritative state root,
3. lifecycle/task mutation uses the project-local TaskFlow command surface selected
   by current runtime/help authority, not a legacy subcommand assumption,
4. JSONL is bounded import/export only,
5. delivery-task packets are the only lawful delegated write unit,
6. the default `vida taskflow` surface is expected to resolve to the project-local runtime path for this repository rather than an installed shim rooted elsewhere.
7. the default decomposition leaf is `delivery_task`, with `execution_block` reserved for packets that still violate one-owner bounded closure.
8. delegated agents are the normal path for write-producing work once a lawful packet exists.
9. packet interpretation follows the project prompt-stack protocol rather than ad hoc precedence guesses.
10. no session is write-ready until the project session-start readiness gate passes.
11. host subagent bridge execution is not default-authorized merely because a bounded VIDA dispatch exists or agent-only development is enabled; when the host API requires explicit subagent/delegation permission, agents must obtain current user or host-surface authorization before spawning host subagents.

## Routing

1. for project top-level orchestrator routing, read `docs/process/project-orchestrator-operating-protocol.md`,
2. for repeatable orchestrator startup, read `docs/process/project-orchestrator-session-start-protocol.md`,
3. for reusable upper-lane wording, read `docs/process/project-orchestrator-reusable-prompt.md`,
4. for mandatory skill activation, read `docs/process/project-skill-initialization-and-activation-protocol.md`,
5. for canonical packet templates, read `docs/process/project-development-packet-template-protocol.md`,
6. for prompt-stack precedence, read `docs/process/project-agent-prompt-stack-protocol.md`,
7. for bounded boot validation, read `docs/process/project-orchestrator-session-start-protocol.md`,
8. for project Codex configuration, read `docs/process/codex-agent-configuration-guide.md`,
9. for project agent-system posture, read `docs/process/agent-system.md`,
10. for project role/skill/profile/flow registries, read `docs/process/agent-extensions/index.md`,
11. for canonical spec-to-task decomposition law, read `command-instructions/planning.form-task-protocol.md`,
12. for delegated packet invariants, read `instruction-contracts/lane.worker-dispatch-protocol.md`,
13. for Release-1 execution-program ownership, read `active runtime contract/profile specs`.

## Metadata

1. Canonical artifact identity, status, source path, revision, and update time are recorded in the footer below.
2. Revision history is recorded only in `docs/process/team-development-and-orchestration-protocol.changelog.jsonl`.
3. This document points to configuration/schema/runtime owners and does not become a second option catalog.

-----
artifact_path: process/team-development-and-orchestration-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-07-27'
schema_version: '1'
status: canonical
source_path: docs/process/team-development-and-orchestration-protocol.md
created_at: '2026-03-13T17:00:00+02:00'
updated_at: 2026-07-27T00:00:00+03:00
changelog_ref: team-development-and-orchestration-protocol.changelog.jsonl
