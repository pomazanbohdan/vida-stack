# Step Thinking Runtime Capsule

Purpose: provide a compact runtime-facing projection of `overlay.step-thinking-protocol.md` for orchestrator bootstrap and routine execution.

Boundary rule:

1. this file is a compact projection, not the owner of step-thinking law,
2. the canonical owner remains `instruction-contracts/overlay.step-thinking-protocol`,
3. change-producing candidate minimality remains owned by `instruction-contracts/overlay.solution-minimality-protocol`,
4. when ambiguity, escalation, or uncommon flow composition appears, consult the owner sections directly.

## Core Use

1. choose the smallest lawful thinking mode for the current step,
2. keep reasoning step-scoped rather than replaying large narrative context,
3. preserve impact analysis and fail-closed routing,
4. run solution minimality after trace/constraint gates and before implementation selection for every change-producing decision,
5. do not expose chain-of-thought in user-facing output.

## Runtime Selection

Default progression:

1. `STC`
   - routine bounded reasoning,
   - low-risk next-step selection,
   - simple critique/check.
2. `PR-CoT`
   - multi-perspective validation,
   - moderate uncertainty or trade-offs.
3. `MAR`
   - architecture or multi-round conflict resolution,
   - competing design forces or deeper coordination questions.
4. `5-SOL`
   - explicit option generation/selection,
   - "choose among candidates" problems.
5. `META`
   - high-risk framework/protocol/policy work,
   - composed block flow for hard cases.

Mandatory overrides:

1. route to `META` for framework-owned behavior change, protocol conflict, execution-gate mismatch, or fail-closed law risk,
2. route to `TRACE` when a bug, incident, regression, repeated technical failure, or multi-error pool dominates,
3. route to `5-SOL` when the step is explicitly about choosing between bounded alternatives.

## Minimal Runtime Rules

1. run the selector before expanding into heavier reasoning,
2. load only the selected algorithm section(s) from the owner file when needed,
3. load `instruction-contracts/overlay.solution-minimality-protocol` when a code, configuration, documentation, or process change is being selected,
4. apply its compact reflex budget: `reuse current evidence -> one batched lookup round -> first admissible rung -> stop`,
5. expand that lookup budget only for material ownership, correctness, safety, compatibility, or proof uncertainty; it is not a hard one-tool-call limit,
6. apply its safe-default formula: `one reversible admissible option + preserved scope/safety/authority + known rollback -> proceed; otherwise ask only for a material or irreversible choice`,
7. resolve equal candidates by `deletion -> reuse -> fewer files -> fewer dependencies -> fewer calls -> lower cognitive load`, then scan the current candidate/diff only for single-implementation interfaces, one-product factories, unused configuration, delegating wrappers, and speculative scaffold,
8. keep web/internet validation tied to `work.web-validation-protocol.md`,
9. use reasoning modules only when the selected algorithm or step explicitly benefits,
10. expand from compact mode only when conflict, uncertainty, preservation risk, or admissibility pressure requires it.

## Owner Sections

When deeper semantics are required, read:

1. `overlay.step-thinking-protocol.md#section-algorithm-selector`
2. `overlay.step-thinking-protocol.md#section-stc`
3. `overlay.step-thinking-protocol.md#section-pr-cot`
4. `overlay.step-thinking-protocol.md#section-mar`
5. `overlay.step-thinking-protocol.md#section-5-solutions`
6. `overlay.step-thinking-protocol.md#section-meta-analysis`
7. `overlay.step-thinking-protocol.md#section-bug-reasoning` (`TRACE`)
8. `overlay.step-thinking-protocol.md#section-web-search`
9. `overlay.step-thinking-protocol.md#section-reasoning-modules`
10. `instruction-contracts/overlay.solution-minimality-protocol`

-----
artifact_path: config/instructions/instruction-contracts/overlay.step-thinking-runtime-capsule
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/overlay.step-thinking-runtime-capsule.md
created_at: '2026-03-13T21:50:00+02:00'
updated_at: 2026-08-15T16:59:19.2673012Z
changelog_ref: overlay.step-thinking-runtime-capsule.changelog.jsonl
