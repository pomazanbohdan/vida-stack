# Agent Lane Selection And Conversational Mode Protocol

Purpose: define how VIDA selects an active agent lane class from user intent, how project overlays may enable auto-lane routing, and how bounded conversational modes such as scope discussion and PBI discussion remain lawful before tracked execution starts.

## Core Contract

1. lane-class selection happens after request-intent classification and before tracked-flow handoff,
2. lane-class selection must not weaken framework authority boundaries,
3. conversational lane modes may remain chat-native only while no tracked mutation or formal artifact handoff is required,
4. once a conversational mode requires canonical artifact creation or tracked task execution, it must hand off to the lawful pack or tracked-execution path.
5. this protocol owns conversational/pre-tracked lane-class selection only; execution-agent selection for tracked work remains in `core.agent-system-protocol.md`.

## Lane Selection Modes

Supported overlay-level lane-selection modes:

1. `fixed`
   - keep the default framework lane class selected by the current runtime/entry contract.
2. `auto`
   - select one framework or validated project lane class from request shape and configured conversational modes before deeper routing.

Fallback rule:

1. if `auto` cannot resolve one lawful lane class, fall back to `orchestrator`,
2. fallback must be explicit and inspectable through overlay/config state,
3. auto-lane selection must not silently promote one lane class into stronger authority than framework lane law allows.

## Standard Conversational Modes

Current standard conversational modes:

1. `scope_discussion`
   - default lane class: `business_analyst`
   - purpose: scope shaping, requirement clarification, acceptance-boundary discovery, assumption surfacing
   - tracked-flow handoff: `spec-pack`
2. `pbi_discussion`
   - default lane class: `pm`
   - purpose: PBI framing, priority and delivery-cut discussion, launch-shape discussion, task-formation preparation
   - tracked-flow handoff: `work-pool-pack`

Projects may define additional conversational modes, but they must still bind:

1. one bounded lane class,
2. one bounded conversational purpose,
3. one lawful tracked-flow handoff target when chat mode is no longer sufficient.

## Single-Task Conversational Rule

Conversational modes for scope/PBI work must stay single-task bounded.

Rules:

1. one active bounded scope or one active candidate task/PBI per conversational mode session,
2. do not silently branch one scope discussion into multiple unrelated task lines,
3. if a second task/PBI candidate emerges, either:
   - park the first one explicitly,
   - or fork the second into a separate lawful task-formation path,
4. `scope_discussion` and `pbi_discussion` may remain exploratory, but they must not become silent backlog expansion.

## Tracked-Flow Handoff Rule

When conversational work becomes artifact-bearing or execution-bearing:

1. `scope_discussion` hands off to `spec-pack` / `vida-spec`,
2. `pbi_discussion` hands off to `work-pool-pack` / `vida-form-task`,
3. if implementation starts, the route must continue through the canonical tracked-execution owner and the implementation protocol,
4. one-task boundedness must remain visible in the resulting tracked artifact or task pool.

Mixed feature-delivery rule:

1. if one request asks for research, specifications, planning, and implementation/code together, route it through `scope_discussion` first even when it is phrased as a direct build request,
2. that route must hand off to `spec-pack` before any development-team execution posture is activated,
3. the lawful order is:
   - bounded todo/design checklist,
   - one feature epic and one spec-pack task in `vida taskflow`,
   - bounded design/spec document through `vida docflow`,
   - close the spec-pack task only after the bounded design/spec document is finalized and validated,
   - tracked task/task-pool shaping through the canonical TaskFlow/form-task path,
   - only then delegated implementation/review/proof lanes.
4. when the runtime exposes `vida taskflow consume final <request> --json`, use that direct-consumption surface to materialize the spec-first tracked-flow bootstrap and required next commands before broader orchestration.
5. do not treat the presence of implementation language inside the request as permission to skip the conversational/spec stage.

## Overlay Activation Surface

The active overlay section is:

1. `vida.config.yaml`
2. `agent_extensions.role_selection`

Supported keys:

1. `mode`
2. `fallback_role`
3. `conversation_modes`

Supported `conversation_modes.<mode_id>` keys:

1. `enabled`
2. `role`
3. `single_task_only`
4. `tracked_flow_entry`
5. `allow_freeform_chat`

## Validation Rule

Lane-class selection is valid only when:

1. `mode` is one of the supported values,
2. `fallback_role` resolves to a known framework lane class or validated project lane class,
3. each enabled conversational mode resolves to a known framework or validated project lane class,
4. each conversational mode points to one lawful tracked-flow handoff target,
5. conversational modes marked `single_task_only=true` are not reinterpreted as multi-task backlog generation modes.

## Operational Proof

Current bounded proof surfaces:

1. the active overlay/config validation surface,
2. the active runtime-family lane-selection bundle proof surface,
3. the active runtime-family lane-selection request proof surface.

## References

1. `agent-definitions/entry.orchestrator-entry`
2. `command-instructions/routing.use-case-packs-protocol`
3. `command-instructions/planning.form-task-protocol`
4. `runtime-instructions/work.taskflow-protocol`
5. `runtime-instructions/work.project-agent-extension-protocol`
6. `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`
7. `system-maps/runtime-family.taskflow-map`

-----
artifact_path: config/runtime-instructions/agent-lane-selection.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.agent-lane-selection-protocol.md
created_at: '2026-03-10T16:40:00+02:00'
updated_at: 2026-03-14T12:05:10.555382992Z
changelog_ref: work.agent-lane-selection-protocol.changelog.jsonl
