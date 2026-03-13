# Team Development And Orchestration Protocol

Status: active project process doc

Purpose: define the project-owned operating protocol for manager-led multi-agent development so `.codex`, VIDA orchestration, and Release-1 restart execution all use the same bounded delivery-task model.

## Scope

This protocol defines:

1. the project-level team topology for development work,
2. the packet shape used for delegated development lanes,
3. how one backlog item is decomposed into delivery-task packets,
4. how orchestrator, implementer, coach, verifier, and escalation cooperate,
5. closure rules for packet-level work.

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

## Team Topology

The active project development team is:

1. root orchestrator session
   - owns framing, decomposition, packet routing, synthesis, and closure decisions
2. `development_implementer`
   - owns one bounded write-producing packet
3. `development_coach`
   - owns formative review for one bounded packet before independent verification
4. `development_verifier`
   - owns independent proof and closure-readiness checks for one bounded packet
5. `development_escalation`
   - owns high-cost conflict resolution when the normal packet path cannot close lawfully

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

1. if any of `goal`, `owned_paths`, `definition_of_done`, `verification_command`, or `blocking_question` is missing, the packet is invalid,
2. invalid packets must be reshaped by the orchestrator before delegation.

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
2. bind it to the active backlog or restart unit,
3. initialize the available skill catalog and activate the relevant skill set,
4. shape one lawful packet,
5. choose the lane sequence,
6. keep writer ownership singular,
7. synthesize coach and verifier returns,
8. decide closure or rework.

The orchestrator must not:

1. act as the default local writer for normal development work,
2. delegate a packet with ambiguous writable scope,
3. skip coach or verifier when the packet still requires them.

### Implementer

The implementer must:

1. execute one packet,
2. activate the relevant skills before packet work begins,
3. stay inside assigned write scope,
4. return changed files, verification result, blockers, and residual risks.

The implementer must not:

1. widen the packet,
2. self-approve closure,
3. silently absorb neighboring backlog work.

### Coach

The coach must:

1. review the packet result against `definition_of_done`,
2. activate the relevant skills before packet review begins,
3. identify rework signals,
4. return bounded corrective guidance.

The coach must not:

1. replace the verifier,
2. convert review into milestone-wide architecture scope,
3. silently accept missing proof.

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
2. `development_implementer` owns one bounded write-producing packet,
3. `development_coach` owns bounded formative review,
4. `development_verifier` owns independent proof and closure readiness,
5. `development_escalation` is exceptional and activates only when normal packet closure cannot be made coherent.

Local orchestrator-only work is lawful only for:

1. shaping or reshaping packets,
2. bounded read-only analysis,
3. proof-only checks,
4. very small one-file fixes where delegation would cost more than the change itself,
5. recorded saturation or escalation exceptions.

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

For the active Release-1 restart line:

1. `R1-Bxx` backlog items are not yet delegated packets,
2. each `R1-Bxx` must first be split into one or more delivery-task packets,
3. only those packet leaves may be delegated into `.codex` agent lanes.

## Bootstrap Rule

After bootstrap, development agents must know immediately:

1. project-local TaskFlow env lives in `taskflow-v0/.env`,
2. task lifecycle truth lives in `.vida/state/taskflow-state.db`,
3. lifecycle/task mutation goes through `taskflow-v0 task`,
4. JSONL is bounded import/export only,
5. delivery-task packets are the only lawful delegated write unit,
6. the default `taskflow-v0` shell command is expected to resolve to the project-local wrapper/runtime path for this repository rather than an installed shim rooted elsewhere.
7. the default decomposition leaf is `delivery_task`, with `execution_block` reserved for packets that still violate one-owner bounded closure.
8. delegated agents are the normal path for write-producing work once a lawful packet exists.

## Routing

1. for project top-level orchestrator routing, read `docs/process/project-orchestrator-operating-protocol.md`,
2. for repeatable orchestrator startup, read `docs/process/project-orchestrator-session-start-protocol.md`,
3. for reusable upper-lane wording, read `docs/process/project-orchestrator-reusable-prompt.md`,
4. for mandatory skill activation, read `docs/process/project-skill-initialization-and-activation-protocol.md`,
5. for project Codex configuration, read `docs/process/codex-agent-configuration-guide.md`,
6. for project agent-system posture, read `docs/process/agent-system-guide.md`,
7. for project role/skill/profile/flow registries, read `docs/process/agent-extensions/README.md`,
8. for canonical spec-to-task decomposition law, read `vida/config/instructions/command-instructions/planning.form-task-protocol.md`,
9. for delegated packet invariants, read `vida/config/instructions/instruction-contracts/lane.worker-dispatch-protocol.md`,
10. for Release-1 restart backlog ownership, read `docs/product/spec/release-1-restart-backlog.md`.

-----
artifact_path: process/team-development-and-orchestration-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/team-development-and-orchestration-protocol.md
created_at: '2026-03-13T17:00:00+02:00'
updated_at: '2026-03-13T19:11:00+02:00'
changelog_ref: team-development-and-orchestration-protocol.changelog.jsonl
