# Agent Role Selection And Conversational Mode Model

Status: active product law

Purpose: define the canonical product/runtime model for auto-role selection, bounded conversational role modes, and the handoff from chat-native scope/PBI discussion into lawful tracked runtime flows.

## 1. Why This Layer Exists

VIDA already distinguishes:

1. request intent,
2. role,
3. skill,
4. profile,
5. flow set,
6. tracked runtime execution.

This model adds the missing bridge:

1. how one role is chosen before tracked flow,
2. how conversational roles may stay useful without silently becoming execution lanes,
3. how scope and PBI discussion remain bounded and lawful.

## 2. Role Selection Modes

Supported runtime posture:

1. `fixed`
   - the runtime uses the default framework lane role and does not auto-switch from user wording alone.
2. `auto`
   - the runtime may select one framework or validated project role from the request shape before deeper routing.

`auto` must remain fail-closed:

1. unresolved selection falls back to `orchestrator`,
2. fallback must not invent stronger authority,
3. role selection never overrides request-intent law, pack law, or TaskFlow gate law.

## 3. Standard Conversational Modes

Current standard conversational modes:

1. `scope_discussion`
   - default role: `business_analyst`
   - target outcome: bounded scope, clarified constraints, acceptance direction
   - lawful tracked handoff: `spec-pack`
2. `pbi_discussion`
   - default role: `pm`
   - target outcome: one bounded task/PBI candidate, delivery cut, ordering, launch readiness for task formation
   - lawful tracked handoff: `work-pool-pack`

## 4. Single-Task Rule

`scope_discussion` and `pbi_discussion` are conversationally freeform, but they are not backlog explosion modes.

Rules:

1. one active bounded scope or one active candidate task/PBI at a time,
2. if additional task candidates appear, they must be parked or forked explicitly,
3. no silent conversion of one discussion into many tracked tasks,
4. the resulting tracked flow must still show one bounded task-formation target unless the user explicitly broadens scope and approves a larger formation flow.

## 5. Relation To Existing Pack Law

This layer does not replace existing pack routing.

It sits before it:

1. request intent still determines `answer_only | artifact_flow | execution_flow | mixed`,
2. role selection determines which bounded role posture should lead the conversational stage,
3. once tracked flow is required, runtime must hand off to the existing lawful pack:
   - `scope_discussion` -> `spec-pack`
   - `pbi_discussion` -> `work-pool-pack`

## 6. Overlay Activation

Project activation lives in:

1. `vida.config.yaml`
2. `agent_extensions.role_selection`

The project may:

1. enable `auto`,
2. choose a fallback role,
3. enable or disable standard conversational modes,
4. add project-specific conversational modes,
5. keep them bounded to lawful tracked handoff targets.

## 7. Runtime Validation

Minimum validation:

1. role-selection mode resolves,
2. fallback role resolves,
3. conversation mode role resolves,
4. tracked handoff target resolves to a lawful standard runtime pack/entrypoint,
5. bounded single-task requirement remains explicit where required.

## 8. Layered Development Order

1. add framework roles such as `business_analyst` and `pm`,
2. add overlay-configured role-selection mode,
3. add standard conversational modes,
4. validate tracked handoff targets,
5. consume the selected role/mode inside `taskflow-v0` runtime routing,
6. only then add project-specific custom conversational modes.

## 9. Completion Proof

This model is wired when:

1. one runtime protocol owns role selection and conversational mode law,
2. root `vida.config.yaml` can enable `auto` and the standard conversational modes,
3. runtime validation fails closed on invalid role-selection wiring,
4. runtime exposes executable role-selection proof surfaces such as `taskflow-v0 role-select bundle` and `taskflow-v0 role-select request`,
5. pack/taskflow routing remains the only lawful handoff path for tracked work.

-----
artifact_path: product/spec/agent-role-selection-and-conversation-mode-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/agent-role-selection-and-conversation-mode-model.md
created_at: '2026-03-10T16:40:00+02:00'
updated_at: '2026-03-10T16:53:58+02:00'
changelog_ref: agent-role-selection-and-conversation-mode-model.changelog.jsonl
