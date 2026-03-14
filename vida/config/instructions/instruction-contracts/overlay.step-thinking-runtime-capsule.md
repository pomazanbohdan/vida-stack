# Step Thinking Runtime Capsule

Purpose: provide a compact runtime-facing projection of `overlay.step-thinking-protocol.md` for orchestrator bootstrap and routine execution.

Boundary rule:

1. this file is a compact projection, not the owner of step-thinking law,
2. the canonical owner remains `instruction-contracts/overlay.step-thinking-protocol`,
3. when ambiguity, escalation, or uncommon flow composition appears, consult the owner sections directly.

## Core Use

1. choose the smallest lawful thinking mode for the current step,
2. keep reasoning step-scoped rather than replaying large narrative context,
3. preserve impact analysis and fail-closed routing,
4. do not expose chain-of-thought in user-facing output.

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
2. route to bug/error reasoning when regression or repeated technical failure dominates,
3. route to `5-SOL` when the step is explicitly about choosing between bounded alternatives.

## Minimal Runtime Rules

1. run the selector before expanding into heavier reasoning,
2. load only the selected algorithm section(s) from the owner file when needed,
3. keep web/internet validation tied to `work.web-validation-protocol.md`,
4. use reasoning modules only when the selected algorithm or step explicitly benefits,
5. expand from compact mode only when conflict, uncertainty, preservation risk, or admissibility pressure requires it.

## Owner Sections

When deeper semantics are required, read:

1. `overlay.step-thinking-protocol.md#section-algorithm-selector`
2. `overlay.step-thinking-protocol.md#section-stc`
3. `overlay.step-thinking-protocol.md#section-pr-cot`
4. `overlay.step-thinking-protocol.md#section-mar`
5. `overlay.step-thinking-protocol.md#section-5-solutions`
6. `overlay.step-thinking-protocol.md#section-meta-analysis`
7. `overlay.step-thinking-protocol.md#section-bug-reasoning`
8. `overlay.step-thinking-protocol.md#section-web-search`
9. `overlay.step-thinking-protocol.md#section-reasoning-modules`

-----
artifact_path: config/instructions/instruction-contracts/overlay.step-thinking-runtime-capsule
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/overlay.step-thinking-runtime-capsule.md
created_at: '2026-03-13T21:50:00+02:00'
updated_at: '2026-03-13T21:50:00+02:00'
changelog_ref: overlay.step-thinking-runtime-capsule.changelog.jsonl
