# Protocol Token Economy Baseline

Status: current research baseline
Revision: 2026-07-03

Purpose: record measured token baselines and quality-preserving compression evidence for always-loaded or high-frequency protocol documents after adopting `docs/product/spec/protocol-authoring-and-token-economy-law.md`.

## Scope

This baseline covers three protocol documents identified as the first token-economy candidates:

1. `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
2. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
3. `vida/config/instructions/instruction-contracts/core.orchestration-runtime-capsule.md`

The baseline does not replace those runtime instruction documents. It records measured size, algorithm route, validation posture, and accepted compression results. It does not set fixed token targets.

## Measurement Method

1. Tokenizer: `tiktoken-cli --model gpt-4o`
2. Date: 2026-07-03
3. Command:
   - `tiktoken-cli --model gpt-4o vida/config/instructions/instruction-contracts/core.orchestration-protocol.md vida/config/instructions/runtime-instructions/work.taskflow-protocol.md vida/config/instructions/instruction-contracts/core.orchestration-runtime-capsule.md`

## Initial Baseline

| Document | Original tokens | Gate route | Compression posture |
| --- | ---: | --- | --- |
| `core.orchestration-protocol.md` | 8726 | high `quality_risk`, high `size_pressure` | no fixed target; preserve owner law |
| `work.taskflow-protocol.md` | 6338 | high `quality_risk`, high `size_pressure` | no fixed target; preserve command/gate atoms |
| `core.orchestration-runtime-capsule.md` | 689 | high `quality_risk`, low `size_pressure` | preserve-exact unless it grows |
| Total loaded set | 15753 | mixed | quality-first compression |

## Compression Result

Measured after quality-preserving compression, audit metadata registration, and source DocFlow validation on 2026-07-03:

| Document | Before tokens | After tokens | Actual delta | Acceptance |
| --- | ---: | ---: | ---: | --- |
| `core.orchestration-protocol.md` | 8726 | 4438 | -4288 | audit marker valid |
| `work.taskflow-protocol.md` | 6338 | 4633 | -1705 | audit marker valid |
| `core.orchestration-runtime-capsule.md` | 689 | 689 | 0 | unchanged preserve-exact |
| Total loaded set | 15753 | 9760 | -5993 | no mandatory content loss detected |

The result is accepted without a fixed token target because the active rule is algorithm-appropriate compression without content/context loss. Size reduction is measurement evidence, not the primary acceptance criterion.

## Quality-Preserving Rewrite Route

### `core.orchestration-protocol.md`

1. Applied semantic atom coverage before compression.
2. Preserved exact atoms for active bounded unit, continuation, dispatch readiness, wait boundary, saturation recovery, partial worker return, exception receipt, lane identity, and final-report law.
3. Converted repeated invariant prose into compact owner law plus exact legacy section crosswalk.
4. Kept root-lane hard law and stop gates in the loaded owner.
5. Pre-commit audit result: no missing headings, inline atoms, or mandatory semantic phrases.

### `work.taskflow-protocol.md`

1. Applied semantic atom coverage and conservative LLMLingua-style compression.
2. Preserved exact atoms for Q-Gate output, sequential/parallel decision matrix, anti-loop contract, worker parallel mode, step definition, operational commands, gates, anti-patterns, blocked/unblocked flow, execution mode, boot profile, and transparency boundary.
3. Converted repeated workflow prose into compact binding rules while keeping legacy headings in place.
4. Kept exact operational command atoms in the loaded protocol after operator direction clarified that no fixed target should drive content movement.
5. Pre-change audit result: no missing headings, inline atoms, command lines, or mandatory semantic phrases.

### `core.orchestration-runtime-capsule.md`

1. Preserve-exact validation remains the primary route.
2. Do not aggressively compress unless the capsule grows materially.
3. Use RFC 2119 cleanup only when wording becomes vague.

## Acceptance For Future Rewrite

Future protocol compression MUST pass:

1. pre-change or pre-commit baseline audit,
2. protected-atom diff for commands, paths, ids, field names, and stop gates,
3. semantic atom coverage for hard runtime laws,
4. `tiktoken-cli --model gpt-4o` before/after table,
5. source DocFlow `check-file` when runtime DocFlow is unavailable,
6. explicit note when installed-runtime enforcement is deferred.

## Current Decision

1. The source DocFlow gate is implemented for new or opt-in protocol/instruction docs.
2. `core.orchestration-protocol.md` is compressed into a compact loaded owner kernel.
3. `work.taskflow-protocol.md` is compressed without fixed token target and with full command content retained.
4. Further compression must be driven by block-appropriate algorithms and clean baseline audit, not by arbitrary size targets.

-----
artifact_path: product/research/protocol-token-economy-baseline
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: 2026-07-03
schema_version: '1'
status: canonical
source_path: docs/product/research/protocol-token-economy-baseline.md
created_at: 2026-07-03T00:00:00+03:00
updated_at: 2026-07-03T11:45:00+03:00
changelog_ref: protocol-token-economy-baseline.changelog.jsonl
