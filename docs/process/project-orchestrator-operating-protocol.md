# Project Orchestrator Operating Protocol

Status: active project process doc

Purpose: define the project-owned top-level operating protocol for a cheaper but logical orchestrator so the upper lane can route work mechanically through bounded packets instead of relying on deep ad hoc reasoning.

## Scope

This protocol defines:

1. the minimum top-level operating loop for the project orchestrator,
2. the default task-shaping depth,
3. when to delegate, keep work local, or escalate,
4. the minimum project-local packet-routing data needed after generic runtime protocol promotion,
5. the project residue for source intake, proof routing, release impact, and escalation.

This protocol does not define:

1. framework bootstrap law,
2. worker packet schema,
3. product capability ownership,
4. lower-level implementation details for one packet.

## Target Orchestrator

This protocol is designed for an orchestrator that is:

1. cheaper than the strongest available model,
2. still logically disciplined,
3. expected to route and synthesize rather than invent architecture from scratch,
4. successful only when the upper control surfaces are explicit and stable.

Project rule:

1. the orchestrator is not expected to rediscover product structure,
2. the orchestrator is expected to follow the canonical maps, backlog, seams, and packet rules already fixed by project canon.
3. routine packet/lane reminders should come from the compact project packet/lane capsule, with the full team-development protocol reserved for edge cases, closure conflicts, or packet-law audits.
4. codebase graph discovery should use `codebase-memory-mcp` when available and fresh; raw file/shell inspection remains the fallback for current working-tree truth and compressed command output.

## Minimum Read Set

For active project development orchestration, the minimum project-side read set after bootstrap is:

1. `docs/process/project-orchestrator-operating-protocol.md`
2. `docs/process/project-packet-and-lane-runtime-capsule.md`
3. `docs/product/spec/current-spec-map.md` when product/spec closure context is active

The orchestrator should not widen beyond that set unless a blocker or ambiguity requires it.

Preferred startup helpers:

1. `docs/process/project-orchestrator-startup-bundle.md`
2. `docs/process/project-orchestrator-session-start-protocol.md`
3. `docs/process/project-orchestrator-reusable-prompt.md`
4. `docs/process/project-start-readiness-runtime-capsule.md`
5. `docs/process/project-packet-rendering-runtime-capsule.md`
6. `docs/process/project-skill-initialization-and-activation-protocol.md`
7. `docs/process/project-orchestrator-session-start-protocol.md`

## Top-Level Loop

The normal top-level loop is:

1. classify the request,
2. bind it to the active backlog item or bounded ask,
3. inspect the active skill catalog and activate the minimal relevant skill set,
4. choose the decomposition depth,
5. actualize TaskFlow status, parent/child layer, priority, dependencies, owned paths, proof targets, and sequential/parallel posture from current evidence,
6. decide delegated vs escalated handling through the configured agent roles/carriers,
7. shape one lawful packet,
8. dispatch the next configured lane,
9. synthesize the result into TaskFlow and the next bounded step or closure.

For long-running refactor epics, the loop is specialized as a wave-first
optimization loop:

1. select the wave with the smallest verified closure distance,
2. bind only the next child task or bounded child batch inside that wave,
3. run the adaptive routine task loop (`Bind -> Implement -> Focused Proof -> Merge -> Close -> Cleanup`),
4. after each task, update the model-routing scorecard and parent/wave closure
   state before selecting the next child,
5. close the wave parent before moving to an unrelated wave when its children,
   proof, release/install, and diagnostic gates are complete.

## Merge-Before-Close Loop

For every self-contained delivery task, the task-level closure sequence is:

1. verify the bounded worktree commit, focused proof, independent review, and PR state;
2. merge the task branch into the authoritative branch immediately;
3. run focused post-merge proof; do not run release/package/install between sibling tasks;
4. record task proof refs, status/authority smoke, and merge artifact refs;
5. close the child TaskFlow task after merge and focused proof pass;
6. after all coherent runtime tasks are merged, run one `release-package` plus `vida release install --json` final gate and record version/path/fingerprint;
7. close the parent/epic only after the final release gate passes, then remove merged task worktrees and obsolete branches.

A task with a verified but unmerged worktree commit is not closed. Keep it `in_progress` with a `verified_pending_integration` evidence note/label until merge. If merge, focused proof, PR, or runtime authority fails, keep the task `in_progress`/`blocked`, preserve the worktree, and record the blocker. A failed final release gate keeps the parent/epic open. Disjoint implementation may run in parallel, but authoritative merge, final release/install, system-binary update, parent/epic closure, and cleanup are serialized.

## Adaptive Routine Execution Contract

Routine work uses the smallest lawful path:

1. `Bind` one bounded task with invariant, owned paths, proof target, non-goals, and posture.
2. `Implement` with one cheapest capable config-selected worker; do not pre-dispatch duplicate coder/cleaner lanes.
3. `Focused Proof` runs the smallest complete proof bundle for the changed public surface.
4. `Review` or `Architecture` is conditional on runtime-contract impact, cross-boundary ownership, security/data risk, failed proof, or explicit escalation evidence.
5. `Merge`, `Close`, and `Cleanup` are serialized integration gates; a verified unmerged worktree remains open.
6. Use one runtime snapshot per bounded cycle, one consolidated proof bundle per coherent pack, and one final release/install gate after all selected tasks merge.
7. If a host/runtime blocker prevents dispatch and the user explicitly authorizes direct fallback, static bounded edits and script proof may continue without runtime-state mutation; receipt-backed closure remains unavailable until the runtime path is repaired.

Generic owner references:

1. active-unit binding, anti-stop, final-report, and continuation law are owned by `instruction-contracts/core.orchestration-runtime-capsule`, `instruction-contracts/core.orchestration-protocol`, and `runtime-instructions/work.taskflow-protocol`,
2. session ownership, exception takeover, lane identity, packet/handoff evidence, and receipt authority are owned by the runtime, lane handoff, and TaskFlow owner protocols,
3. command timing, fast proof, long-gate classification, and script optimization are owned by `runtime-instructions/work.command-execution-discipline-protocol` plus the project command-timing protocol,
4. source-neutral intake and status actualization are owned by TaskFlow source-class metadata and the project source-specific process maps.

Project residue:

1. keep the current vida-stack active task, parent epic, priority reason, owned/read-only paths, proof target, role chain, and sequential/parallel posture visible before write-producing work,
2. use TaskFlow updates rather than chat-only notes for PRs, downstream reports, runtime defects, CI failures, release tasks, optimizations, documentation/process work, and operator-surface gaps,
3. use `META` for multi-defect or multi-source batch planning before selecting the shared invariant and proof window,
4. prefer local focused proof and documented script modes before expensive workspace, release, installer, or CI gates,
5. treat CI after push as diagnostic unless the active bounded unit is release/mainline/installer/CI architecture admission,
6. keep historical release labels and concrete blocker names as evidence only, not permanent routing law,
7. when `vida.config.yaml -> autonomous_execution.agent_only_development` is true, the project default is VIDA agent orchestration; a current VIDA `agent-init` packet, host-tool bridge request, sticky continuation intent, or visible agent-only runtime policy is not by itself explicit authorization to use a spawn-capable host subagent bridge when the host tool requires separate explicit subagent/delegation permission,
8. when the same runtime defect, blocker code, command-surface failure, lane/receipt/dispatch defect, or session-blocking symptom appears more than once in one session, raise or keep its owning TaskFlow item at priority 1, append recurrence evidence, and route it before unrelated runtime-DX cleanup unless a higher-severity blocker is already active.
9. for codebase search and impact analysis, prefer `codebase-memory-mcp` surfaces in this order: `list_projects`/`index_status` for freshness, `get_graph_schema` for available graph facts, `trace_path` for callers/callees/data-flow, `get_code_snippet` for exact qualified symbols, `detect_changes` for indexed impact, and `manage_adr` for durable architecture decisions. Do not use it as sole authority when the index is stale, contains deleted/generated paths, or the question depends on uncommitted working-tree changes.

Active-epic publication residue:

1. when the current operator explicitly instructs "commit and push after each
   task" or equivalent wording for the active epic, that instruction is the
   current repeatable publication authorization for scoped task commits and
   matching documentation scorecard commits,
2. the authorization remains bounded to the active epic/session pattern and does
   not authorize unrelated publication, broad batch publication, or GitHub
   mutations outside the active TaskFlow item,
3. if the operator pauses, revokes, or narrows publication, return to the generic
   "push only when currently authorized" rule.

The orchestrator must not:

1. begin broad repository exploration when the active maps already answer routing,
2. delegate milestone- or epic-shaped work,
3. keep normal write-producing work local by default,
4. widen one packet into adjacent backlog work without reshaping,
5. skip skill initialization when relevant skills are available for the bounded step,
6. invent packet shape or prompt precedence ad hoc when the canonical packet or prompt-stack protocols already answer it.

## Default Decomposition Rule

The default decomposition leaf is `delivery_task`.

Stop at `delivery_task` when all are true:

1. one owner,
2. one dominant goal,
3. one bounded write scope or one bounded read-only scope,
4. one proof target or verification command,
5. one bounded lane cycle can decide closure.

Split further to `execution_block` only when at least one is true:

1. the candidate task still crosses more than one mutable contract,
2. the candidate task crosses more than one crate or owner boundary,
3. the candidate task mixes refactor and feature closure,
4. the candidate task mixes implementation and seam/proof closure,
5. `definition_of_done` is still too broad for one bounded cycle.

Hard stop:

1. never delegate `epic`,
2. never delegate `milestone`,
3. never delegate a backlog paragraph.

Launch-readiness rule:

1. a launch-ready queue must be shaped to lawful `delivery_task` leaves,
2. launch readiness does not require the full backlog to be pre-split into `execution_block` leaves,
3. `execution_block` is a just-in-time refinement for the next active item or near-critical-path item only.

## Source-Neutral Work Rule

PR processing is a source-specific protocol under the project law, not the project law itself.

Before any bounded work source moves into implementation, the orchestrator must:

1. classify the source type,
2. consult the mapped spec or process surface for that source,
3. create or update the owning TaskFlow item with source evidence, priority reason, dependencies, owned/read-only paths, proof target, role chain, and sequential/parallel posture,
4. route through the configured role chain when VIDA dispatch is available,
5. record a separate runtime defect and use bounded Defective Runtime Emulation Mode when VIDA cannot execute that chain,
6. apply the source's own closure evidence without weakening the generic proof gate.

## Delegation And Local Handling Overlay

Generic delegation, packet, lane, exception, and host-agent bridge law is owned by the runtime/agent-system protocols. This project process doc keeps only the vida-stack routing overlay:

1. normal write-producing work is shaped as one bounded TaskFlow packet and routed through the configured role chain when the runtime can execute it,
2. the project-preferred chain is analyst -> test_author when needed -> coach_test_gate when needed -> developer/implementer -> coach_implementation_gate -> duplication_reviewer -> tester/prover -> release_closure -> orchestrator synthesis,
3. role, carrier, model, cost, reasoning effort, host CLI, and worktree decisions come from `vida.config.yaml`, agent-extension registries, and runtime assignment evidence,
4. keep work local only for shaping, read-only analysis, proof-only checks, or a recorded bounded recovery/exception path,
5. if runtime delegation is blocked, record the runtime defect and use the defective-runtime overlay only until canonical dispatch/continuation is restored,
6. do not treat a visible host subagent, explicit process-carrier execution, activation view, known patch location, dirty tree, or advisory draft as receipt-backed execution evidence,
7. before escalating, verify packet boundaries, write-scope collisions, architecture conflicts, and repeated rework evidence against the mapped owner protocols,
8. host-tool contracts that require explicit subagent/delegation permission are not satisfied by repository policy, `agent_only_development`, sticky continuation, VIDA dispatch, or a host bridge request alone; the host tool remains an independent approval boundary for spawn-capable adapters,
9. before spawning host subagents, fail closed unless the current user instruction or host approval surface explicitly authorizes the host subagent/delegation path for the bounded work; visible agent-only runtime policy may only shape VIDA routing and cannot substitute for that authorization.

## Packet Readiness Rule

Before dispatch, the orchestrator must ensure the active packet satisfies the template-specific minimum for its `packet_template_kind`.

Minimum by packet family:

1. `delivery_task_packet` and `execution_block_packet` must include `goal`, `scope_in`, `owned_paths` or `read_only_paths`, `definition_of_done`, `verification_command`, `proof_target`, `stop_rules`, and one `blocking_question`,
2. `coach_review_packet` must include `review_goal`, `owned_paths` or `read_only_paths`, `definition_of_done`, `proof_target`, and one `blocking_question`,
3. `verifier_proof_packet` must include `proof_goal`, `verification_command`, `proof_target`, `owned_paths` or `read_only_paths`, and one `blocking_question`,
4. `escalation_packet` must include `decision_needed`, `options`, `constraints`, and one `blocking_question`.

If any mandatory field for the active packet family is missing, the packet is not ready and must be reshaped first.

Interpretation rule:

1. packet fields must be rendered and checked through `docs/process/project-development-packet-template-protocol.md`,
2. prompt-layer precedence must follow `docs/process/project-agent-prompt-stack-protocol.md`,
3. startup must satisfy `docs/process/project-orchestrator-session-start-protocol.md` before the first write-producing dispatch,
4. runtime surfaces such as `vida taskflow consume final`, dispatch-packet persistence, resume, and `vida agent-init` must fail closed when the active packet template minimum is missing.
5. for `tracked_flow_packet` handoffs, raw `create_command` is initial materialization evidence only; once the tracked task id already exists, continue through the runtime-provided ensure/reuse command instead of retrying duplicate creation.

## Continuation, Exception, And Saturation Owner Pointers

Do not duplicate generic continuation, exception, open-delegation, or saturation law here. Resolve the full rules through:

1. `instruction-contracts/core.orchestration-runtime-capsule`,
2. `instruction-contracts/core.orchestration-protocol`,
3. `runtime-instructions/work.taskflow-protocol`,
4. `instruction-contracts/lane.worker-dispatch-protocol`,
5. `runtime-instructions/lane.agent-handoff-context-protocol`.

Project residue:

1. interim synthesis may describe state but must not end an active cycle while a lawful same-unit continuation is evidenced,
2. one green proof closes only its bounded proof target, not the whole development session,
3. local orchestrator writes require the generic owner-law evidence plus a project-visible bounded unit, path scope, proof target, and TaskFlow note,
4. agent/thread saturation must inspect delegated lane state, synthesize closeable returns, reclaim only closeable lanes, and retry lawful dispatch before any fallback is recorded.

## Top-Level Routing Table

Use this table by default:

| Work shape | Default depth | Default lane sequence | Notes |
|---|---|---|---|
| bounded read-only analysis | `delivery_task` | orchestrator or verifier-only | keep local when no writer is needed |
| one coherent write packet | `delivery_task` | orchestrator -> analyst -> test_author when required -> coach_test_gate when required -> developer/implementer -> coach_implementation_gate -> duplication_reviewer -> tester -> prover -> release_closure | normal path |
| broad backlog item with one clear owner but unclear done | split to `delivery_task` first | shaping only until lawful | do not dispatch yet |
| one delivery task still crossing multiple mutable contracts | `execution_block` | orchestrator -> analyst -> bounded lane chain after split | split before dispatch |
| seam or closure bottleneck | `delivery_task` or `execution_block` | orchestrator -> implementer/verifier -> synthesis | choose by contract tightness |
| unresolved architecture conflict | no normal leaf yet | escalation | do not push an invalid packet downstream |

## Cheap-Orchestrator Safety Rule

To keep a cheaper orchestrator effective:

1. prefer explicit maps over broad inference,
2. prefer routing tables over free-form planning,
3. prefer one packet at a time over speculative multi-step trees,
4. prefer shallow lawful decomposition over premature micro-splitting,
5. prefer escalation over invented structure when canonical boundaries are unclear.

## Token And Carrier Economy Rule

The orchestrator must treat root-session tokens and paid/high-reasoning model calls as a scarce sprint resource during long refactor epics.

Default routing:

1. use internal read-only or medium lanes for bounded pre-analysis, report triage, duplicate-risk review, task-note research, and second-opinion review by default,
2. use `jcode_nim_cli` with `mistralai/mistral-medium-3.5-128b` as a secondary read-only advisory carrier when an independent second opinion is useful or when root-session token pressure is active,
3. use internal low (`codex_gpt55_low_write`) for one-scope implementation packets with clear owned paths and focused proof,
4. use internal medium (`codex_gpt55_medium_write`) for test authoring, regression shaping, ambiguous but bounded implementation, and coach decisions that require more structure than low,
5. reserve high/xhigh internal profiles for architecture boundary decisions, security/safety review, release readiness, or repeated low/medium failure evidence,
6. keep the root orchestrator focused on binding, packet shaping, synthesis, TaskFlow mutation, final validation, and conflict resolution.

Before starting a new bounded task, the orchestrator should decide:

1. whether an internal read-only/medium lane can prefetch read-only context or review likely risks in parallel with local inspection,
2. whether `jcode_nim_cli` should run the same bounded read-only question as an independent NIM-backed advisory pass,
3. whether the research stage should use `external_readonly_complete`, `external_patch_proposal`, or both in parallel,
4. whether the write lane can be delegated to internal low before medium,
5. whether a medium coach/test-author lane is enough before escalating to high,
6. whether the next command can be replaced by a compact runtime surface, snapshot query, or previously refreshed `.vida/exports/tasks.snapshot.jsonl`,
7. whether similar report items can be batched into one TaskFlow mutation window.

Do not use `vibe_cli` or `jcode_nim_cli` for root-session write authority, task closure, receipt fabrication, or final proof acceptance. Their output is advisory evidence until the orchestrator validates it against source, TaskFlow, runtime receipts, or focused proof. `jcode_nim_cli` remains experimental until `jcode run --json` reports a provider label consistent with `jcode provider current` for the selected NIM model.

## Wave-First Epic Optimization Rule

The active refactor epic is optimized for wave closure, not leaf-task count.

Before selecting the next work item, compute or inspect closure distance for the
candidate waves:

1. open child count,
2. blocked child count,
3. missing proof or closure-ready blockers,
4. dirty-file overlap and staging risk,
5. same-file or same-contract conflict domains,
6. expected debug/build/test cost,
7. release/install cost if the wave can close,
8. open PR or GitHub mutation coupling,
9. runtime ambiguity or ownership blockers,
10. validator residual risk from the last similar task.

Select the wave with the smallest closure distance unless current user wording,
runtime binding, open PR priority, or a critical blocker explicitly requires a
different bounded unit.

Do not optimize for closed-task percentage alone. The progress report must
include closed waves over total waves whenever the question is about epic
completion.

## Three-Step Task Optimization Loop

Every task in a long-running refactor epic uses:

1. `Bind`
2. `Delegate`
3. `Close`

`Bind` must record:

1. active task id and parent/wave,
2. why this task is selected now,
3. exact invariant or documentation contract,
4. owned paths and out-of-scope paths,
5. dirty-worktree hunks that must remain unstaged,
6. proof bundle,
7. sequential versus parallel posture,
8. model-routing expectation for executor and validator,
9. stop condition and blocked fallback.

When a new TaskFlow child/step/follow-up is needed and the metadata is already
known, `Bind` must create it in one `vida task create` command with parent,
description or notes, owned paths, acceptance target, proof target, and execution
semantics instead of creating the item first and immediately updating the same
record. Use `vida task update` for later state changes or genuinely unknown
metadata discovered after creation.

`Delegate` must:

1. use the cheapest capable executor for the bounded packet,
2. use the lowest-cost eligible capability pattern for hunk classification,
   preflight, docs/source fidelity, test-only patches, and one-file
   implementation when the proof bundle is explicit,
3. use the next eligible capability pattern for focused rework after timeout,
   under-coverage, or validator rejection,
4. use the strongest eligible capability pattern for TaskFlow, host-bridge,
   receipt authority, path policy, public CLI, release, or wave-closure gates;
   resolve model, role, and reasoning values from configuration and telemetry,
   never from hardcoded runtime branches,
5. keep validator prompts short: invariant gap, false-green tests, missing
   proof, unrelated hunks, residual risk,
6. classify every agent return before launching a replacement for the same
   stage.

`Close` must:

1. run the focused proof bundle and the declared debug build,
2. run `vida task validate-graph --json`,
3. run `vida task closure-ready <task-id> --json`,
4. close the TaskFlow item only when closure-ready passes,
5. commit only scoped task files and push when the active publication pattern
   authorizes it,
6. record the optimization scorecard in the TaskFlow closure evidence,
7. run DocFlow/diff checks for documentation changes,
8. commit and push scoped documentation updates under the same publication pattern,
9. run Post-Task Self-Analysis and update instructions again when the analysis
   changes the operating rule,
10. check parent/wave closure readiness,
11. run runtime self-diagnostic when the task is architectural/process-shaped or
    closes a wave,
12. run runtime self-diagnostic when closing a coherent task batch; the
    diagnostic covers the whole batch after all included task closes and before
    unrelated work selection,
13. actualize project skill creation or updates through
    `docs/process/agent-skill-learning-protocol.md`: collect close,
    self-analysis, user-correction, and diagnostic events, classify whether a
    skill update is required, record `no_skill_update_reason` when not required,
    and stage or validate any bounded skill proposal before TaskFlow
    next-selection,
14. actualize TaskFlow last, immediately before deciding what to take into work
    next: refresh task status, parent/child layer, priority, dependencies,
    owned paths, proof targets, execution semantics, and sequential/parallel
    posture from current evidence,
15. run one release-package/install and installed-binary smoke gate only after
    all selected runtime tasks in the coherent pack or epic are merged and
    focused proof is green; do not install between sibling tasks,
16. for a runtime defect that blocked the current session, continuation, lane
    execution, receipt/proof truth, or installed CLI behavior, keep focused
    proof and debug smoke at task level and defer installed-runtime validation
    to the final coherent pack/epic gate; if that final gate cannot run, keep
    the parent/epic closure blocked or record the proof blocker,
17. select the next task using the updated scorecard, self-analysis, and
    closure-distance data.

## Post-Task Self-Analysis Gate

Post-Task Self-Analysis is a STOP gate after every closed task and before
selecting unrelated work. The next unrelated task is blocked until the
orchestrator records the base fields, checks all 20 fixed criteria below,
applies or records the meta-analysis remediation, creates or updates TaskFlow
implementation tasks for every actionable self-analysis or self-diagnostic
finding, cites those task ids or an explicit `no_task_reason`, and then
completes the final checklist item: the dynamic-criteria STOP point for the
just-finished session segment. The fixed list is the baseline only; it is not a
substitute for generating at least one new additional session-derived criterion
every time.

Base fields:

1. `worked`: which executor, validator, proof, prompt, or local step produced
   useful progress,
2. `waste`: which commands, waits, scans, repeated checks, or agent prompts
   were avoidable,
3. `risk`: which false-green, dirty-hunk, runtime ambiguity, missing proof,
   stale TaskFlow, timeout, or publication risk appeared,
4. `next_change`: what changes in model choice, reasoning effort, prompt shape,
   proof bundle, staging, or parallel/sequential posture for the next task,
5. `docs_update`: whether the finding requires updating project instructions,
   scorecard templates, prompt templates, scripts, code, tests, or TaskFlow
   optimization defects, and which TaskFlow implementation task id owns each
   actionable follow-up,
6. `workflow_score_10`: orchestrator process score considering cost, tool calls,
   proof strength, rework, elapsed time, and closure quality.

Twenty fixed required criteria (baseline list, not the final dynamic step):

1. Active bounded unit was explicit before write-producing work.
2. Wave/parent closure distance improved or the task had a documented reason to
   run before the current wave.
3. Task scope, non-goals, and owned paths stayed stable.
4. Dirty worktree hunks were classified and preserved or converted to follow-up
   tasks.
5. Executor model choice was the cheapest capable option for the bounded risk.
6. Validator model choice matched authority/risk level.
7. Agent prompts had one task id, one goal, one proof bundle, one stop condition,
   and telemetry requirements.
8. Agent handles were closed/deleted or cleanup blockers were recorded.
9. Token, tool-call, step, and wait costs were recorded or explicitly marked
   `not_exposed_by_host`.
10. Avoidable shell/read/status/doctor/build commands were identified.
11. Proof bundle covered the claimed behavior rather than a narrow false-green.
12. Public-surface proof, JSON/default output proof, help proof, or release proof
    was included when the task type required it.
13. Debug build or declared build substitute was run and recorded.
14. TaskFlow graph, closure-ready, and close state were current.
15. Commit staging was by invariant, not by whole dirty file.
16. Push/publication matched the active authorization pattern.
17. TaskFlow closure scorecard or relevant process-doc scorecard was updated before unrelated work.
18. Parent/wave closure-ready state and epic task/wave metrics were refreshed.
19. New runtime/tooling/process defects discovered during the task were created,
    updated, or explicitly deferred with reason.
20. The next task routing rule changed when the evidence justified changing it,
    or explicitly remained unchanged with reason.

Final dynamic criteria STOP point:

1. Run this step last, after the base fields, all 20 fixed criteria, and
   meta-analysis remediation have been recorded.
2. Analyze the session segment from the previous task closure to the current
   task closure, including user feedback, agent returns, command delays,
   proof failures, dirty-tree surprises, and documentation changes.
3. Create at least one new additional dynamic criterion that captures failure
   modes, waste patterns, proof gaps, agent behavior, runtime/tooling friction,
   user feedback, or documentation drift observed in that exact session segment
   and not already captured by the fixed list or prior dynamic criteria.
4. Each dynamic criterion must be actionable and testable in the next task, with
   an expected evidence source or stop condition.
5. Record which dynamic criteria become one-time checks for the next task and
   which should be promoted into the fixed checklist, prompt template, script,
   code, test, or project documentation.
6. Record each dynamic criterion in a lightweight registry line or table in the
   scorecard. Each entry must include `criterion_id`, `owner`, `expected_evidence`,
   `task_ref` or `no_task_reason`, `promotion_decision` (`one_time`, `promoted`,
   `scripted`, `converted_to_task`, `duplicate`, or `retired`), and
   `duplicate_of` when the criterion reuses an earlier rule.
7. This is the last required checklist item for every task closure. The next
   task remains blocked until this step creates and records at least one new
   criterion for the current segment.
8. Do not close the STOP gate by reusing only fixed criteria or previous dynamic
   criteria. If the session segment appears fully covered, create a narrower
   criterion from the strongest observed risk, delay, proof gap, or user
   correction in that segment and mark whether it stays one-time or is promoted.

Meta-analysis remediation:

1. For every `waste` item, choose one remediation: remove the redundant step,
   batch it, replace it with a compact command, script it, document it, or create
   a runtime/operator-efficiency defect.
2. For every `risk` item, choose one remediation: add/adjust proof, update a
   prompt/checklist, create a follow-up TaskFlow task, update code/tests/scripts,
   update documentation/instructions, or record why no action is required.
3. For every actionable finding from Post-Task Self-Analysis, runtime
   self-diagnostic, release/install diagnostic, DocFlow check, TaskFlow check,
   agent return, or user correction, create or update a concrete TaskFlow
   implementation task unless the finding is already fully fixed inside the
   just-closed task, is a duplicate of an existing TaskFlow task, or is tracked
   only in an upstream issue with a recorded project-local no-task reason.
4. Every created or updated follow-up must include acceptance criteria, proof
   target, owner surface or owned path, and the source finding. Cite its task id
   in the scorecard and final dynamic criteria section. A prose log entry alone
   does not satisfy this gate.
5. If the task touched, reviewed, cited, or depended on GitHub/open-source
   intake, record open PR handling and processed issue closure state in the
   scorecard. Open PRs must be processed, explicitly marked `no_open_prs` or
   `not_applicable`, or kept open with `left_open_reason`. Processed issues must
   be closed, explicitly marked `no_processed_issues` or `not_applicable`, or
   kept open with `kept_open_reason`/`no_task_reason`.
6. If remediation changes project behavior, update the relevant instruction,
   process doc, script, code, test, or TaskFlow defect before unrelated work.
7. If remediation cannot be completed inside the just-closed task, create or
   update a follow-up with acceptance criteria and cite it in the scorecard.

Development-time defect batching:

1. Defects found while closing or proving another task must be classified before
   creating a new leaf task:
   - `blocks_current_closure`: prevents the current task, commit, release,
     install, or proof from completing; fix inside the current task when it fits
     the same ownership boundary, or create one blocker task if ownership differs.
   - `same_surface_batch`: shares the same command family, output contract,
     runtime gate, proof script, or recovery path with other findings; add it to
     the current open batch task for that surface instead of creating one leaf per
     symptom.
   - `later_evidence_only`: useful evidence but not blocking and not enough to
     define acceptance; record it in the batch task notes or scorecard until a
     concrete invariant is clear.
2. A batch task is the default container for related runtime-DX defects discovered
   during development. Name the invariant, not the first symptom. Examples:
   default output/actionability, proof/build gate friction, agent/exception/lane
   recovery, PR intake closeout, and release/install feedback.
3. A batch task must carry an acceptance matrix: affected commands or surfaces,
   default TOON/plain output, explicit JSON parity when applicable, help or
   next-action text, proof artifacts, and release/install impact. Close the batch
   only after the matrix is green or after out-of-scope rows are split with a
   recorded reason.
4. Create a separate new leaf task only when the defect has a different owner
   surface, blocks the current closure independently, crosses a security/data
   migration boundary, needs a separate release/install proof, or would make the
   batch too broad to validate in one proof cycle.
5. If a batch already exists, update that task with the new reproduction,
   acceptance row, and proof target. Do not inflate epic totals by creating a new
   task for every small default-output, option-help, lock, or recovery-friction
   symptom.
6. Commit and release per batch, not per small symptom, unless the symptom is a
   `blocks_current_closure` fix. The scorecard must report batch membership,
   newly added rows, rows closed, and whether total epic scope grew.
7. Epic progress reports after a defect-finding segment must include both raw
   TaskFlow percent and scope delta: tasks closed, tasks created, net open
   change, and whether new items were batch rows or new leaf tasks.

Coherent work-pool execution:

1. When several ready or in-progress tasks are adjacent or same-surface, the
   orchestrator must inspect the whole candidate pool before selecting the
   implementation route. This applies to runtime-DX work, docs/process work,
   test-only work, operator surfaces, and other project-local task clusters.
2. Classify the pool before write-producing work:
   - `parallel_coherent_pack`: allowed only when all included tasks are ready in
     the same dependency layer, have no direct or transitive dependency edges
     between them, are `parallel_safe` or explicitly non-conflicting, share a
     compatible owner surface, command family, proof family, dirty-worktree
     boundary, or shared invariant, have compatible owned paths and conflict
     domains, and can be represented by one acceptance matrix and consolidated
     proof bundle.
   - `sequential_coherent_pack`: required when tasks are adjacent or
     same-surface but ordered by dependency edges, shared contracts, sequential
     execution semantics, conflict domains, proof gates, release/install gates,
     or acceptance criteria. This posture still keeps one shared analysis,
     acceptance matrix, proof plan, and closeout bundle, but implementation,
     focused proof, and closure move layer by layer.
   - `not_packable`: required when a candidate crosses an incompatible security
     boundary, data migration, owner surface, owned-path boundary, release gate,
     or proof target too broad for one cycle.
3. Before write-producing work, bind the pool explicitly in TaskFlow evidence:
   included task ids, dependency graph and closure order, posture classification,
   shared invariant, owned paths, conflict domains, non-goals, dirty-worktree
   exclusions, proof bundle, release/install decision, and whether the next work
   is simultaneous, staged, or blocked. Create or mark the TaskFlow task and
   execution step for each included slice before its mutation.
4. For `parallel_coherent_pack`, analyze the whole pool first, then implement
   code, tests, and docs for all selected slices in one coordinated write
   window. Write the test matrix as a batch before broad verification; use
   focused proof only while shaping the pool, then run the consolidated proof
   bundle after the pack is complete.
5. For `sequential_coherent_pack`, analyze the whole pool first and write the
   shared proof matrix up front, but edit only the next dependency layer. Run
   focused proof for that layer, refresh assumptions and conflict evidence, then
   proceed to the next layer. Do not describe this as simultaneous
   implementation. Run the consolidated pack proof after all layers are complete.
6. If the user asks for simultaneous pack work but the dependency or conflict
   evidence requires staged execution, report the downgrade to
   `sequential_coherent_pack` before coding. Never silently downgrade a requested
   parallel pack into task-by-task execution.
7. Close included tasks sequentially in dependency order, reusing the same pool
   proof where it satisfies each task's acceptance target. If one row fails,
   split only that row or task with a recorded reason and keep passing rows
   attached to the pool evidence.
8. After the included tasks are closed, run the runtime self-diagnostic once for
   the whole pool and classify findings before selecting unrelated work.
9. Actualize project skills through `docs/process/agent-skill-learning-protocol.md`
   before final TaskFlow actualization: collect batch close, self-analysis,
   diagnostic, and user-correction events; decide whether a skill update is
   needed; record `no_skill_update_reason` or stage/validate a bounded proposal.
10. Actualize TaskFlow last, immediately before next-work analysis, so candidate
   tasks are ordered from current status, dependencies, proof targets,
   execution semantics, and sequential/parallel posture.
11. Commit and push per task or coherent pool as authorized; release and install
   once at the final coherent pack/epic gate rather than per small task. The closure scorecard must
   report pool membership, posture, tasks closed, rows split, proof commands
   reused, operations saved, residual risks, and the next routing rule.

## Post-Task Optimization Checklist

After every task, the orchestrator must track:

1. TaskFlow state updated and closed/blocker recorded.
2. Declared proof bundle result.
3. Debug build result.
4. Scoped commit hash and push result.
5. TaskFlow closure scorecard evidence and any relevant process-doc commit hash/push result.
6. Completed agent handles closed or cleanup blocker recorded.
7. Executor model, reasoning effort, score, tokens, tool calls, wait cost, agent
   count, and rework count. Each telemetry field must be marked `exact`,
   `approximate`, or `not_exposed_by_host`; approximate fields must name the
   observable basis.
8. Validator model, reasoning effort, score, tokens, tool calls, wait cost, and
   residual risk, using the same `exact` / `approximate` /
   `not_exposed_by_host` telemetry shape.
9. Post-Task Self-Analysis recorded with base fields, all 20 fixed criteria,
   dynamic criteria created from the latest session segment, and meta-analysis
   remediation outcomes.
10. Implementation follow-up task ids recorded for every actionable
    self-analysis or self-diagnostic finding, or a specific `no_task_reason`
    recorded for non-actionable, duplicate, or upstream-only findings.
11. False-green risks found and whether they became tests or follow-up tasks.
12. Dirty hunks preserved or follow-up task created for adjacent useful hunks.
13. Parent/wave closure readiness and remaining child count.
14. Epic task percentage and wave percentage.
15. Runtime friction or slow-command defects created/updated when observed.
16. PR/open-source intake state recorded: open PRs processed or explicitly kept
    open with reason, and processed issues closed or explicitly kept open with
    reason.
17. Project-skill learning actualization completed through
    `docs/process/agent-skill-learning-protocol.md`, with accepted proposal,
    staged proposal, rejection, or `no_skill_update_reason` recorded.
18. Final TaskFlow actualization completed after skill-learning actualization
    and before next-work selection.
19. Next routing rule written in the TaskFlow closure scorecard or relevant process-doc evidence.

If any checklist item cannot be proven, keep the current task or a follow-up
TaskFlow item open instead of silently moving to unrelated work.

Research-stage delegation modes:

1. `external_readonly_complete` lets an external carrier fully complete analysis, specification, review, or proof-diagnosis work and return a structured report with evidence refs, risks, and a recommended next step. The root orchestrator validates that report before mutating TaskFlow or accepting closure.
2. `external_patch_proposal` lets an external carrier prepare a patch proposal, proposed diff or file plan, verification commands, and rollback notes. The root orchestrator applies or rejects the diff, runs proof, commits, performs only publication authorized by a current explicit operator instruction, and closes TaskFlow.
3. Every task research intake must record both mode decisions: `external_readonly_complete` as the read-only evidence path and `external_patch_proposal` as the patch/diff proposal path. A mode may be marked `not_run` only with a reason such as no useful independent question, no lawful owned paths, unavailable carrier, duplicate output, or task risk too low for the extra attempt.

Premature micro-splitting rule:

1. do not convert the whole backlog into `execution_block` trees up front,
2. keep future work at `delivery_task` depth until dispatch is near,
3. refine only the next active item or the smallest near-critical-path set needed to keep work flowing.

## Bootstrap Visibility Rule

After bootstrap, the orchestrator should be able to answer immediately:

1. what backlog unit is active,
2. whether `delivery_task` is enough or `execution_block` is required,
3. whether the next step is local shaping, delegation, or escalation,
4. which proof target closes the next packet,
5. which map owns the current seam or release slice.

If those answers are not visible from the minimum read set, do not continue into write-producing work until the packet is reshaped.

## Routing

1. for the compact project startup read set, read `docs/process/project-orchestrator-startup-bundle.md`,
2. for repeated session startup, read `docs/process/project-orchestrator-session-start-protocol.md`,
3. for reusable upper-lane wording, read `docs/process/project-orchestrator-reusable-prompt.md`,
4. for mandatory skill activation, read `docs/process/project-skill-initialization-and-activation-protocol.md`,
5. for routine packet rendering and prompt-stack interpretation, read `docs/process/project-packet-rendering-runtime-capsule.md`,
6. for full packet-template law, read `docs/process/project-development-packet-template-protocol.md`,
7. for bounded boot validation, read `docs/process/project-orchestrator-session-start-protocol.md`,
8. for full prompt-stack law, read `docs/process/project-agent-prompt-stack-protocol.md`,
9. for full delegated-lane law and closure edge cases, read `docs/process/team-development-and-orchestration-protocol.md`,
10. for Codex role/runtime posture, read `docs/process/codex-agent-configuration-guide.md`,
11. for release formatting and public release body rules, read `docs/process/release-formatting-protocol.md`,
12. for current product/spec ownership, read `docs/product/spec/current-spec-map.md`.

-----
artifact_path: process/project-orchestrator-operating-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-06-12'
schema_version: '1'
status: canonical
source_path: docs/process/project-orchestrator-operating-protocol.md
created_at: '2026-03-13T18:40:00+02:00'
updated_at: 2026-07-01T21:45:00+03:00
changelog_ref: project-orchestrator-operating-protocol.changelog.jsonl
