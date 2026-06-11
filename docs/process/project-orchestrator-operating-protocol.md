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

## Minimum Read Set

For active project development orchestration, the minimum project-side read set after bootstrap is:

1. `docs/process/project-orchestrator-operating-protocol.md`
2. `docs/process/project-packet-and-lane-runtime-capsule.md`
3. `docs/process/generic-runtime-protocol-promotion-plan.md` when a rule may belong in generic runtime owners
4. `docs/product/spec/current-spec-map.md` when product/spec closure context is active

The orchestrator should not widen beyond that set unless a blocker or ambiguity requires it.

Preferred startup helpers:

1. `docs/process/project-orchestrator-startup-bundle.md`
2. `docs/process/project-orchestrator-session-start-protocol.md`
3. `docs/process/project-orchestrator-reusable-prompt.md`
4. `docs/process/project-start-readiness-runtime-capsule.md`
5. `docs/process/project-packet-rendering-runtime-capsule.md`
6. `docs/process/project-skill-initialization-and-activation-protocol.md`
7. `docs/process/project-boot-readiness-validation-protocol.md`

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
7. when `vida.config.yaml -> autonomous_execution.agent_only_development` is true, the project default is VIDA agent orchestration; a current VIDA `agent-init` packet, host-tool bridge request, sticky continuation intent, or visible agent-only runtime policy is not by itself explicit authorization to use a spawn-capable host subagent bridge when the host tool requires separate explicit subagent/delegation permission.

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
3. startup must satisfy `docs/process/project-boot-readiness-validation-protocol.md` before the first write-producing dispatch,
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

1. use `vibe_cli` for bounded read-only pre-analysis, report triage, duplicate-risk review, task-note research, and second-opinion review when no write authority is needed,
2. use `jcode_nim_cli` with `mistralai/mistral-medium-3.5-128b` as a secondary read-only advisory carrier when `vibe_cli` is unavailable, when an independent second opinion is useful, or when root-session token pressure is active,
3. use internal low (`codex_gpt55_low_write`) for one-scope implementation packets with clear owned paths and focused proof,
4. use internal medium (`codex_gpt55_medium_write`) for test authoring, regression shaping, ambiguous but bounded implementation, and coach decisions that require more structure than low,
5. reserve high/xhigh internal profiles for architecture boundary decisions, security/safety review, release readiness, or repeated low/medium failure evidence,
6. keep the root orchestrator focused on binding, packet shaping, synthesis, TaskFlow mutation, final validation, and conflict resolution.

Before starting a new bounded task, the orchestrator should decide:

1. whether `vibe_cli` can prefetch read-only context or review likely risks in parallel with local inspection,
2. whether `jcode_nim_cli` should run the same bounded read-only question as an independent NIM-backed advisory pass,
3. whether the research stage should use `external_readonly_complete`, `external_patch_proposal`, or both in parallel,
4. whether the write lane can be delegated to internal low before medium,
5. whether a medium coach/test-author lane is enough before escalating to high,
6. whether the next command can be replaced by a compact runtime surface, snapshot query, or previously refreshed `.vida/exports/tasks.snapshot.jsonl`,
7. whether similar report items can be batched into one TaskFlow mutation window.

Do not use `vibe_cli` or `jcode_nim_cli` for root-session write authority, task closure, receipt fabrication, or final proof acceptance. Their output is advisory evidence until the orchestrator validates it against source, TaskFlow, runtime receipts, or focused proof. `jcode_nim_cli` remains experimental until `jcode run --json` reports a provider label consistent with `jcode provider current` for the selected NIM model.

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
7. for bounded boot validation, read `docs/process/project-boot-readiness-validation-protocol.md`,
8. for full prompt-stack law, read `docs/process/project-agent-prompt-stack-protocol.md`,
9. for full delegated-lane law and closure edge cases, read `docs/process/team-development-and-orchestration-protocol.md`,
10. for Codex role/runtime posture, read `docs/process/codex-agent-configuration-guide.md`,
11. for release formatting and public release body rules, read `docs/process/release-formatting-protocol.md`,
12. for current product/spec ownership, read `docs/product/spec/current-spec-map.md`.

-----
artifact_path: process/project-orchestrator-operating-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-06-02'
schema_version: '1'
status: canonical
source_path: docs/process/project-orchestrator-operating-protocol.md
created_at: '2026-03-13T18:40:00+02:00'
updated_at: 2026-06-03T15:45:00+03:00
changelog_ref: project-orchestrator-operating-protocol.changelog.jsonl
