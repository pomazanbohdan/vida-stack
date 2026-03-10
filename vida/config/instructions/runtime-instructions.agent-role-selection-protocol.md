# Agent Role Selection And Conversational Mode Protocol

Purpose: define how VIDA selects an active agent role from user intent, how project overlays may enable auto-role routing, and how bounded conversational modes such as scope discussion and PBI discussion remain lawful before tracked execution starts.

## Core Contract

1. role selection happens after request-intent classification and before tracked-flow handoff,
2. role selection must not weaken framework authority boundaries,
3. conversational role modes may remain chat-native only while no tracked mutation or formal artifact handoff is required,
4. once a conversational mode requires canonical artifact creation or tracked task execution, it must hand off to the lawful pack or TaskFlow path.

## Role Selection Modes

Supported overlay-level role-selection modes:

1. `fixed`
   - keep the default framework lane role selected by the current runtime/entry contract.
2. `auto`
   - select one framework or validated project role from request shape and configured conversational modes before deeper routing.

Fallback rule:

1. if `auto` cannot resolve one lawful role, fall back to `orchestrator`,
2. fallback must be explicit and inspectable through overlay/config state,
3. auto-role selection must not silently promote one role into stronger authority than framework role law allows.

## Standard Conversational Modes

Current standard conversational modes:

1. `scope_discussion`
   - default role: `business_analyst`
   - purpose: scope shaping, requirement clarification, acceptance-boundary discovery, assumption surfacing
   - tracked-flow handoff: `spec-pack`
2. `pbi_discussion`
   - default role: `pm`
   - purpose: PBI framing, priority and delivery-cut discussion, launch-shape discussion, task-formation preparation
   - tracked-flow handoff: `work-pool-pack`

Projects may define additional conversational modes, but they must still bind:

1. one bounded role,
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
3. if implementation starts, the route must continue through `taskflow-v0` and the implementation protocol,
4. one-task boundedness must remain visible in the resulting tracked artifact or task pool.

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

Role selection is valid only when:

1. `mode` is one of the supported values,
2. `fallback_role` resolves to a known framework role or validated project role,
3. each enabled conversational mode resolves to a known framework or validated project role,
4. each conversational mode points to one lawful tracked-flow handoff target,
5. conversational modes marked `single_task_only=true` are not reinterpreted as multi-task backlog generation modes.

## Operational Proof

Current bounded proof surfaces:

1. `taskflow-v0 config validate`
2. `taskflow-v0 role-select bundle --json`
3. `taskflow-v0 role-select request "<request>" --json`

## References

1. `vida/config/instructions/agent-definitions.orchestrator-entry.md`
2. `vida/config/instructions/command-instructions.use-case-packs.md`
3. `vida/config/instructions/command-instructions.form-task-protocol.md`
4. `vida/config/instructions/runtime-instructions.taskflow-protocol.md`
5. `vida/config/instructions/runtime-instructions.project-agent-extension-protocol.md`
6. `docs/product/spec/agent-role-selection-and-conversation-mode-model.md`

-----
artifact_path: config/runtime-instructions/agent-role-selection.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions.agent-role-selection-protocol.md
created_at: '2026-03-10T16:40:00+02:00'
updated_at: '2026-03-10T16:53:58+02:00'
changelog_ref: runtime-instructions.agent-role-selection-protocol.changelog.jsonl
