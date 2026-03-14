# Core Agent Prompt Stack Protocol

Purpose: define the framework-owned prompt-stack model so bootstrap, lane identity, dynamic packet data, skill overlays, and runtime state compose in one explicit precedence order.

## Scope

This protocol defines:

1. the canonical prompt-stack order,
2. the precedence rule between stack layers,
3. the minimum active-layer visibility rule,
4. the fail-closed rule for missing stack layers.

This protocol does not define:

1. one project's specific role prompts,
2. one specific packet template,
3. one specific skill body,
4. one runtime-family implementation.

## Core Rule

Agent behavior is produced by a stack, not by one free-form prompt string.

Framework rule:

1. higher-precedence layers own safety and routing,
2. lower-precedence layers may narrow behavior but must not weaken higher-precedence law,
3. the active packet is governed by the full stack, not by chat wording alone.

## Canonical Stack Order

1. framework bootstrap and lane entry,
2. host-project docs map and project posture,
3. role-specific static prompt,
4. dynamic bounded packet,
5. active relevant skill overlay,
6. current bounded runtime/task state.

## Precedence Rule

When stack layers conflict, use this order:

1. framework bootstrap and lane entry,
2. host-project process posture,
3. role-specific static prompt,
4. dynamic bounded packet,
5. skill overlay,
6. chat phrasing or recollection.

Interpretation rule:

1. lower layers may narrow,
2. lower layers must not override higher-precedence safety, routing, or fail-closed rules.

## Active-Layer Visibility Rule

Before bounded work begins, the active lane should be able to state:

1. which role layer is active,
2. which packet layer is active,
3. which skills are active or that `no_applicable_skill` applies,
4. which runtime state confirms the bounded unit.

If those cannot be named, the lane is not ready.

## Fail-Closed Rule

Do not begin bounded work when:

1. no bounded packet layer exists,
2. a required skill overlay is missing,
3. the runtime/task state contradicts the claimed bounded unit,
4. stack-layer precedence is ambiguous.

## Related

1. `instruction-contracts/core.orchestration-protocol`
2. `instruction-contracts/core.skill-activation-protocol`
3. `instruction-contracts/lane.worker-dispatch-protocol`
4. `system-maps/protocol.index`

-----
artifact_path: config/instructions/instruction-contracts/core.agent-prompt-stack-protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/core.agent-prompt-stack-protocol.md
created_at: '2026-03-13T22:00:00+02:00'
updated_at: '2026-03-13T22:00:00+02:00'
changelog_ref: core.agent-prompt-stack-protocol.changelog.jsonl
