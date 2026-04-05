# Team Development And Orchestration Protocol

Status: active project process doc

Purpose: define the project-owned operating protocol for manager-led multi-agent development so `.codex`, VIDA orchestration, and Release-1 execution all use the same bounded delivery-task model.

## Scope

This protocol defines:

1. the project-level team topology for development work,
2. the packet shape used for delegated development lanes,
3. how one backlog item is decomposed into delivery-task packets,
4. how orchestrator, implementer, coach, verifier, and escalation cooperate,
5. closure rules for packet-level work,
6. how packet shapes, prompt-stack precedence, and boot-readiness rules stay explicit.

This protocol does not define:

1. framework bootstrap law,
2. framework-owned worker dispatch law,
3. product-law semantics for Release 1 capability or seam ownership,
4. Codex runtime schema itself.

## Core Rule

Project development runs as:

1. orchestrator-led,
2. delivery-task shaped,
3. delegation-first,
4. coach-separated,
5. verification-backed,
6. skill-aware before bounded work begins,
7. fail-closed on missing packet data or shared-scope ambiguity.
8. explorer/read-only findings feed packet shaping, not root-session write ownership.

## Team Topology

The active project development team is:

1. root orchestrator session
   - owns framing, decomposition, packet routing, synthesis, and closure decisions
2. `junior`
   - default low-cost carrier tier for one bounded write-producing packet with `runtime_role=worker`
3. `middle`
   - carrier tier for specification/planning packets and formative review with `runtime_role=coach`
4. `senior`
   - carrier tier for independent proof and closure-readiness checks with `runtime_role=verifier`
5. `architect`
   - carrier tier for high-cost conflict resolution with `runtime_role=solution_architect`

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

Readiness rule:

1. readiness is template-specific and must follow `docs/process/project-development-packet-template-protocol.md`,
2. `delivery_task_packet` and `execution_block_packet` are invalid if any of `goal`, `scope_in`, `owned_paths` or `read_only_paths`, `definition_of_done`, `verification_command`, `proof_target`, `stop_rules`, or `blocking_question` is missing,
3. `coach_review_packet` is invalid if any of `review_goal`, `owned_paths` or `read_only_paths`, `definition_of_done`, `proof_target`, or `blocking_question` is missing,
4. `verifier_proof_packet` is invalid if any of `proof_goal`, `verification_command`, `proof_target`, `owned_paths` or `read_only_paths`, or `blocking_question` is missing,
5. `escalation_packet` is invalid if any of `decision_needed`, `options`, `constraints`, or `blocking_question` is missing,
6. invalid packets must be reshaped by the orchestrator before delegation.

Template rule:

1. render project packets using `docs/process/project-development-packet-template-protocol.md`,
2. do not treat prose-only delegation as a valid substitute for the canonical packet family.

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
5. one implementer -> coach -> verifier cycle can judge closure without further subdivision.

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
12. treat project-delegated execution as the runtime lane flow through `vida agent-init`; host executor subagent APIs remain optional carrier details and are not the canonical packet-dispatch surface.

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

For write-producing packets, the default sequence is:

1. orchestrator shaping
2. implementer
3. coach
4. verifier
5. orchestrator synthesis

Explorer-to-writer rule:

1. when explorer or other read-only lanes find a bounded writable gap, that result feeds the next packet,
2. the next lawful write-producing sequence still remains implementer -> coach -> verifier -> synthesis unless a recorded exception path says otherwise,
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

1. orchestrator owns shaping, routing, synthesis, and closure decisions,
2. the runtime-selected `worker` carrier owns one bounded write-producing packet,
3. the runtime-selected `coach` carrier owns bounded formative review,
4. the runtime-selected `verifier` carrier owns independent proof and closure readiness,
5. the runtime-selected `solution_architect` carrier is exceptional and activates only when normal packet closure cannot be made coherent.

Agent-init interpretation rule:

1. `vida agent-init` is a lane-activation and packet-consumption surface, not by itself an execution-complete receipt,
2. `vida agent-init --dispatch-packet ...` or `--downstream-packet ...` does not transfer writer ownership back to the root session,
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
2. task lifecycle truth lives in `.vida/state/taskflow-state.db`,
3. lifecycle/task mutation goes through `vida taskflow task`,
4. JSONL is bounded import/export only,
5. delivery-task packets are the only lawful delegated write unit,
6. the default `vida taskflow` surface is expected to resolve to the project-local runtime path for this repository rather than an installed shim rooted elsewhere.
7. the default decomposition leaf is `delivery_task`, with `execution_block` reserved for packets that still violate one-owner bounded closure.
8. delegated agents are the normal path for write-producing work once a lawful packet exists.
9. packet interpretation follows the project prompt-stack protocol rather than ad hoc precedence guesses.
10. no session is write-ready until the project boot-readiness validation protocol passes.

## Routing

1. for project top-level orchestrator routing, read `docs/process/project-orchestrator-operating-protocol.md`,
2. for repeatable orchestrator startup, read `docs/process/project-orchestrator-session-start-protocol.md`,
3. for reusable upper-lane wording, read `docs/process/project-orchestrator-reusable-prompt.md`,
4. for mandatory skill activation, read `docs/process/project-skill-initialization-and-activation-protocol.md`,
5. for canonical packet templates, read `docs/process/project-development-packet-template-protocol.md`,
6. for prompt-stack precedence, read `docs/process/project-agent-prompt-stack-protocol.md`,
7. for bounded boot validation, read `docs/process/project-boot-readiness-validation-protocol.md`,
8. for project Codex configuration, read `docs/process/codex-agent-configuration-guide.md`,
9. for project agent-system posture, read `docs/process/agent-system-guide.md`,
10. for project role/skill/profile/flow registries, read `docs/process/agent-extensions/README.md`,
11. for canonical spec-to-task decomposition law, read `command-instructions/planning.form-task-protocol.md`,
12. for delegated packet invariants, read `instruction-contracts/lane.worker-dispatch-protocol.md`,
13. for Release-1 execution-program ownership, read `docs/product/spec/release-1-plan.md`.

-----
artifact_path: process/team-development-and-orchestration-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/team-development-and-orchestration-protocol.md
created_at: '2026-03-13T17:00:00+02:00'
updated_at: 2026-03-16T08:15:49.128428218Z
changelog_ref: team-development-and-orchestration-protocol.changelog.jsonl
