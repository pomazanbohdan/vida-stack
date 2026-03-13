# Project Orchestrator Operating Protocol

Status: active project process doc

Purpose: define the project-owned top-level operating protocol for a cheaper but logical orchestrator so the upper lane can route work mechanically through bounded packets instead of relying on deep ad hoc reasoning.

## Scope

This protocol defines:

1. the minimum top-level operating loop for the project orchestrator,
2. the default task-shaping depth,
3. when to delegate, keep work local, or escalate,
4. the minimum packet-routing data needed for normal Release-1 work.

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

## Minimum Read Set

For active project development orchestration, the minimum project-side read set after bootstrap is:

1. `docs/process/project-orchestrator-operating-protocol.md`
2. `docs/process/team-development-and-orchestration-protocol.md`
3. `docs/product/spec/release-1-program-map.md` when Release-1 work is active
4. `docs/product/spec/release-1-restart-backlog.md` when restart execution is active
5. `docs/product/spec/release-1-seam-map.md` when closure or handoff work is active

The orchestrator should not widen beyond that set unless a blocker or ambiguity requires it.

Preferred restart helpers:

1. `docs/process/project-orchestrator-session-start-protocol.md`
2. `docs/process/project-orchestrator-reusable-prompt.md`
3. `docs/process/project-skill-initialization-and-activation-protocol.md`

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

The orchestrator must not:

1. begin broad repository exploration when the active maps already answer routing,
2. delegate milestone- or epic-shaped work,
3. keep normal write-producing work local by default,
4. widen one packet into adjacent backlog work without reshaping,
5. skip skill initialization when relevant skills are available for the bounded step.

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

Keep work local only when:

1. the work is shaping only,
2. the work is bounded read-only analysis,
3. the work is proof-only and cheaper to verify directly,
4. the work is a very small one-file fix where delegation overhead is larger than the change,
5. a recorded saturation or exception path is active.

Escalate only when:

1. packet boundaries cannot be made coherent,
2. write scopes collide,
3. architecture conflict blocks lawful closure,
4. repeated rework still leaves one unresolved design decision.

## Packet Readiness Rule

Before dispatch, the orchestrator must ensure the packet includes:

1. `goal`
2. `non_goals`
3. `scope_in`
4. `scope_out`
5. `owned_paths` or `read_only_paths`
6. `definition_of_done`
7. `verification_command`
8. `proof_target`
9. `stop_rules`
10. one `blocking_question`

If any of those are missing, the packet is not ready and must be reshaped first.

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

1. for packet semantics and team topology, read `docs/process/team-development-and-orchestration-protocol.md`,
2. for repeated session startup, read `docs/process/project-orchestrator-session-start-protocol.md`,
3. for reusable upper-lane wording, read `docs/process/project-orchestrator-reusable-prompt.md`,
4. for mandatory skill activation, read `docs/process/project-skill-initialization-and-activation-protocol.md`,
5. for Codex role/runtime posture, read `docs/process/codex-agent-configuration-guide.md`,
6. for Release-1 restart ownership, read `docs/product/spec/release-1-restart-backlog.md`,
7. for Release-1 closure bottlenecks, read `docs/product/spec/release-1-seam-map.md`.

-----
artifact_path: process/project-orchestrator-operating-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-orchestrator-operating-protocol.md
created_at: '2026-03-13T18:40:00+02:00'
updated_at: '2026-03-13T19:11:00+02:00'
changelog_ref: project-orchestrator-operating-protocol.changelog.jsonl
