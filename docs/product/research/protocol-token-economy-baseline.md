# Protocol Token Economy Baseline

Status: current research baseline
Revision: 2026-07-03

Purpose: record the first measured token baseline for always-loaded or high-frequency protocol documents after adopting `docs/product/spec/protocol-authoring-and-token-economy-law.md`.

## Scope

This baseline covers three protocol documents that were identified as the first token-economy targets:

1. `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
2. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
3. `vida/config/instructions/instruction-contracts/core.orchestration-runtime-capsule.md`

The baseline does not replace those runtime instruction documents. It records measured size, algorithm route, and safe reduction target for the next bounded rewrite.

## Measurement Method

1. Tokenizer: `tiktoken-cli --model gpt-4o`
2. Date: 2026-07-03
3. Command:
   - `tiktoken-cli --model gpt-4o vida/config/instructions/instruction-contracts/core.orchestration-protocol.md vida/config/instructions/runtime-instructions/work.taskflow-protocol.md vida/config/instructions/instruction-contracts/core.orchestration-runtime-capsule.md`

## Baseline Table

| Document | Current tokens | Gate route | Safe loaded target | Candidate delta |
| --- | ---: | --- | ---: | ---: |
| `core.orchestration-protocol.md` | 8726 | high `quality_risk`, high `size_pressure` | 3800 | -4926 |
| `work.taskflow-protocol.md` | 6338 | high `quality_risk`, high `size_pressure` | 3000 | -3338 |
| `core.orchestration-runtime-capsule.md` | 689 | high `quality_risk`, low `size_pressure` | 650 | -39 |
| Total loaded set | 15753 | mixed | 7450 | -8303 |

## First Compression Result

Measured after compressing `core.orchestration-protocol.md` on 2026-07-03:

| Document | Before tokens | After tokens | Actual delta |
| --- | ---: | ---: | ---: |
| `core.orchestration-protocol.md` | 8726 | 4268 | -4458 |
| Loaded set with other two unchanged | 15753 | 11295 | -4458 |

The final after-count is higher than the 3800-token target because the compact owner preserves exact legacy section anchors through a crosswalk. This is an intentional quality-preservation exception under the protocol-authoring law.

## Quality-Preserving Rewrite Route

### `core.orchestration-protocol.md`

1. Apply semantic atom coverage before compression.
2. Preserve exact atoms for active bounded unit, continuation, dispatch readiness, wait boundary, saturation recovery, partial worker return, exception receipt, lane identity, and final-report law.
3. Move explanation-heavy rationale and repeated invariant restatements into lazy owner sections.
4. Keep root-lane hard law and stop gates in the first and last loaded thirds.
5. Candidate after-state: compact loaded kernel around 3800 tokens with full owner detail kept lazy-loadable.

### `work.taskflow-protocol.md`

1. Apply semantic atom coverage and conservative LLMLingua.
2. Preserve exact atoms for Q-Gate output, sequential/parallel decision matrix, anti-loop contract, worker parallel mode, step definition, operational commands, gates, and anti-patterns.
3. Convert command lists and repeated workflow prose into tables.
4. Move examples and explanatory material to appendix or task-selected reference.
5. Candidate after-state: compact loaded kernel around 3000 tokens with command detail lazy-loadable.

### `core.orchestration-runtime-capsule.md`

1. Keep preserve-exact validation as the primary route.
2. Do not aggressively compress unless the capsule grows past 700 tokens.
3. Use RFC 2119 cleanup only when wording becomes vague.
4. Candidate after-state: keep around 650 tokens.

## Acceptance For Future Rewrite

The next rewrite task MUST pass:

1. protected-atom diff for commands, paths, ids, field names, and stop gates,
2. semantic atom coverage for all hard runtime laws,
3. `tiktoken-cli --model gpt-4o` before/after table,
4. `vida docflow check-file` on changed protocol docs when they are project-visible,
5. focused runtime/bootstrap smoke that proves the compact protocol still activates the same owner laws.

## Current Decision

1. The source DocFlow gate is implemented for new or opt-in protocol/instruction docs.
2. `core.orchestration-protocol.md` has been compressed into a compact loaded owner kernel.
3. The safe next step is a bounded rewrite of `work.taskflow-protocol.md` toward a compact loaded kernel plus lazy command detail.

-----
artifact_path: product/research/protocol-token-economy-baseline
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: 2026-07-03
schema_version: '1'
status: canonical
source_path: docs/product/research/protocol-token-economy-baseline.md
created_at: 2026-07-03T00:00:00+03:00
updated_at: 2026-07-03T07:22:56.7588641Z
changelog_ref: protocol-token-economy-baseline.changelog.jsonl
