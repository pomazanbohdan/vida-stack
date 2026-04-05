# Project Orchestrator Operating Protocol

Status: active project process doc

Purpose: define the project-owned top-level operating protocol for a cheaper but logical orchestrator so the upper lane can route work mechanically through bounded packets instead of relying on deep ad hoc reasoning.

## Scope

This protocol defines:

1. the minimum top-level operating loop for the project orchestrator,
2. the default task-shaping depth,
3. when to delegate, keep work local, or escalate,
4. the minimum packet-routing data needed for normal Release-1 work,
5. the anti-stop and exception-path rules needed to keep orchestration in control after interim reports.

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
3. `docs/product/spec/release-1-plan.md` when Release-1 work is active
4. `docs/product/spec/release-1-seam-map.md` when closure or handoff work is active

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
5. decide local vs delegated vs escalated handling,
6. shape one lawful packet,
7. dispatch the packet,
8. synthesize the result into the next bounded step or closure.

Intent-binding clarification:

1. `continue development` authorizes orchestrator-led continuation only after the active bounded unit is explicitly bound,
2. `continue the next task` or equivalent ordinal wording does not by itself authorize choosing the first ready TaskFlow/backlog candidate,
3. if the user did not name the bounded unit and the runtime does not show one uniquely evidenced active continuation unit, fail closed to clarification or explicit ambiguity report before shaping/dispatch,
4. `продовжи агентами`, `continue by agents`, and equivalent delegated-continuation wording sets sticky orchestration intent for the active session until the user explicitly requests stop/final closure.

Loop binding rule:

1. once steps 3-7 become lawful for the active bounded unit, the orchestrator must continue through them in the same active cycle unless a real blocker appears,
2. commentary/progress visibility between those steps does not authorize stopping the cycle,
3. context gathering that already answers packet ownership and next route is not a closure point,
4. interim status summaries must remain commentary-style while continuation intent is active,
5. final closure wording/reporting is forbidden while continuation intent is active unless the user explicitly asks to end/finalize the session.
6. before emitting any final report, the orchestrator must pass a pre-response gate:
   - `active delegated agents == 0`,
   - delegated handoff state is resolved,
   - no ready continuation item exists in TaskFlow unless the user explicitly requests stop/closure.
7. if any pre-response gate check fails, continue orchestration and reporting through commentary updates only.
8. after any green proof/build/test result, finished delegated lane, or intermediate report, the orchestrator must immediately bind the next lawful continuation item in the same cycle when one is already evidenced.
9. “report now, continue later” is forbidden when the next lawful item is already known from TaskFlow, recovery state, delegated evidence, or the just-finished proof result.
10. when the user gives an explicit ordered sequence, that sequence is the controlling execution contract; the orchestrator must not reorder it because some adjacent cleanup or broader technical program looks more complete.
11. scope expansion is forbidden unless the current bounded step cannot be completed without it or the user explicitly authorizes the broader track.
12. if closure-style wording is emitted by mistake during active continuation intent, the immediate recovery step is to return to commentary mode and bind the already-evidenced next lawful continuation item in the same cycle.
13. when shell commands record backlog notes or similar free text, prefer file-backed arguments such as `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.

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

## Delegation Rule

For normal write-producing work, delegation is the default.

Default route:

1. orchestrator shapes,
2. implementer writes,
3. coach reviews,
4. verifier proves,
5. orchestrator synthesizes.

Full-orchestration rule:

1. when normal write-producing work is in scope, the full delegated cycle remains the default even after bounded read-only findings or explorer-discovered gaps,
2. discovering the exact patch location does not by itself authorize local orchestrator patching,
3. local departure from this full cycle requires an explicit recorded exception path.
4. a worker wait timeout, empty poll result, or slow delegated response does not compress the full cycle into one generic development lane or root-session coding; the next lawful step is renewed waiting, bounded inspection, reuse, reroute, or explicit escalation.
5. in this project, the canonical delegated execution surface for that cycle is the runtime lane flow through `vida agent-init`; host-tool-specific subagent APIs are backend details and must not be treated as the primary legality gate for project delegation.

Keep work local only when:

1. the work is shaping only,
2. the work is bounded read-only analysis,
3. the work is proof-only and cheaper to verify directly,
4. a recorded saturation or exception path is active.

Clarification:

1. a "very small one-file fix" is not an independent bypass around worker-first or open-delegation law,
2. if the work is still write-producing, local handling must still satisfy the exception-path and delegated-cycle gates.
3. local shell access, `apply_patch`, or any other host-tool write affordance is not by itself a legality signal for root-session implementation.

Lane-identity rule:

1. the root session remains the orchestrator throughout normal development orchestration,
2. resumed execution intent does not convert the root session into a local implementer,
3. local root-session writing requires an explicit recorded exception path,
4. absent that receipt, the next lawful action is shaping, delegation, verification routing, or escalation.
5. if the delegated lane still exists, a same-turn timeout summary is not a supersession receipt.

Escalate only when:

1. packet boundaries cannot be made coherent,
2. write scopes collide,
3. architecture conflict blocks lawful closure,
4. repeated rework still leaves one unresolved design decision.

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

## Anti-Stop Rule

Framework anti-stop, reporting-boundary, continuation, and final-report law is owned by:

1. `instruction-contracts/core.orchestration-runtime-capsule`
2. `instruction-contracts/core.orchestration-protocol`
3. `runtime-instructions/work.taskflow-protocol`

Project narrowing:

1. interim synthesis may summarize state, but it must not end the active cycle while `in_work=1`,
2. do not treat one closed `execution_block` or `delivery_task` as development-session closure when lawful continuation still exists,
3. after one bounded item closes, rebuild the parent bounded unit and either:
   - shape the next lawful leaf/item,
   - or emit an explicit blocker/escalation receipt
4. `continue development` must not collapse into a symptom-only proof step such as "fix the first failing test" unless the active packet already names that symptom as the current leaf,
5. a green local proof command closes only its bounded proof target,
6. if the next lawful item is already known from TaskFlow, verifier, subagent, or continuation evidence, that signal is a trigger to continue routing rather than a closure-style reporting boundary.

## Recorded Exception Path Rule

Framework exception-path and open-delegation law is owned by:

1. `instruction-contracts/core.orchestration-runtime-capsule`
2. `instruction-contracts/core.orchestration-protocol`
3. `runtime-instructions/work.taskflow-protocol`

Project narrowing:

1. local orchestrator write work is lawful only under an explicit pre-write exception path,
2. narrow that path to one bounded `delivery_task` or `execution_block`,
3. return to normal orchestrator posture immediately after the bounded local fix/proof step,
4. before local fix/proof work, be able to name the active parent bounded unit and active packet/leaf from TaskFlow or packet receipts,
5. do not treat `continue development`, implementer delay, known patch location, or self-diagnosis pressure as substitutes for exception-path or supersession law,
6. if live runtime still reports blocked takeover for an open delegated cycle, remain in diagnosis/orchestration posture and surface the conflict rather than repair locally.
7. do not treat a dirty worktree, already-present same-scope diff, or a partially applied delegated patch as implicit permission for local completion; those remain reroute/blocker/supersession evidence until a lawful exception path is recorded before mutation.

## Saturation Recovery Rule

When worker-first execution hits agent or thread limits, the orchestrator must run saturation recovery before declaring local fallback or exception path.

Required sequence:

1. inspect currently delegated lanes,
2. identify lanes that are `completed_unsynthesized`, `superseded`, `still_waiting`, or `still_active`,
3. synthesize or supersede any completed returns first,
4. close/reclaim lanes that no longer carry an open handoff or proof obligation,
5. retry lawful reuse or fresh dispatch,
6. only then record saturation as still active if no lawful delegated path remains.

Hard rule:

1. "agent limits" without this inspection/reclaim sequence is not a sufficient reason for root-session coding,
2. completed delegated lanes must be checked for closeability before any local fallback,
3. a completed lane with unsynthesized handoff is not closeable yet and must be reconciled first.

## Top-Level Routing Table

Use this table by default:

| Work shape | Default depth | Default lane sequence | Notes |
|---|---|---|---|
| bounded read-only analysis | `delivery_task` | orchestrator or verifier-only | keep local when no writer is needed |
| one coherent write packet | `delivery_task` | orchestrator -> implementer -> coach -> verifier | normal path |
| broad backlog item with one clear owner but unclear done | split to `delivery_task` first | shaping only until lawful | do not dispatch yet |
| one delivery task still crossing multiple mutable contracts | `execution_block` | orchestrator -> implementer -> coach/verifier | split before dispatch |
| seam or closure bottleneck | `delivery_task` or `execution_block` | orchestrator -> implementer/verifier -> synthesis | choose by contract tightness |
| unresolved architecture conflict | no normal leaf yet | escalation | do not push an invalid packet downstream |

## Cheap-Orchestrator Safety Rule

To keep a cheaper orchestrator effective:

1. prefer explicit maps over broad inference,
2. prefer routing tables over free-form planning,
3. prefer one packet at a time over speculative multi-step trees,
4. prefer shallow lawful decomposition over premature micro-splitting,
5. prefer escalation over invented structure when canonical boundaries are unclear.

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
11. for Release-1 execution ownership, read `docs/product/spec/release-1-plan.md`,
12. for Release-1 closure bottlenecks, read `docs/product/spec/release-1-seam-map.md`.

-----
artifact_path: process/project-orchestrator-operating-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-orchestrator-operating-protocol.md
created_at: '2026-03-13T18:40:00+02:00'
updated_at: 2026-04-04T20:12:10.232383544Z
changelog_ref: project-orchestrator-operating-protocol.changelog.jsonl
