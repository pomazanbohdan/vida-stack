# Protocol Authoring And Token Economy Law

Status: active canonical law
Revision: 2026-07-03

Purpose: define the project law for writing, reducing, validating, and registering protocol, instruction, bootstrap, and process documents so token use decreases without losing operational quality.

## Purpose

1. Protocol and instruction documents MUST be written as compact operational contracts before they are expanded with explanation.
2. Compression MUST preserve behavior, authority, proof gates, protected atoms, and bootstrap discoverability.
3. Shorter text is accepted only when the compressed document keeps the same executable meaning for an agent or operator.

## Trigger

Apply this law when:

1. creating a new protocol, instruction, bootstrap, process, product-law, or runtime-facing documentation artifact,
2. revising an existing protocol or instruction for token reduction,
3. promoting research into a reusable protocol or instruction,
4. adding a bootstrap-visible project documentation pointer,
5. reviewing an agent-facing document whose full body is likely to be loaded during runtime initialization, dispatch, or handoff.

## Scope

1. In scope: `docs/product/spec/**`, `docs/process/**`, `AGENTS.sidecar.md`, bootstrap-visible maps, and project-facing instruction/protocol documents.
2. In scope: compact summaries, runtime capsules, protocol owners, authoring templates, and map/catalog entries that affect document discovery.
3. Out of scope for this first law: executable DocFlow/runtime code changes, model-provider-specific prompt rewriting, and automatic mutation of existing documents without a bounded task.

## Authority

1. This document is the product-spec owner for protocol authoring and token-economy law.
2. `docs/product/spec/project-documentation-law.md` remains the owner for canonical metadata, sidecar changelogs, and markdown documentation state.
3. `docs/process/documentation-tooling-map.md` remains the owner for DocFlow operator commands.
4. `AGENTS.sidecar.md` and `docs/project-root-map.md` are bootstrap routing surfaces; they point here but do not duplicate this law.
5. Source DocFlow validation enforces this law for the canonical law itself, opt-in artifacts with `protocol_authoring_gate: enforced`, and protocol/instruction/bootstrap docs created or revised on or after 2026-07-03.
6. Installed runtime enforcement requires the current source build to be released or installed through the normal VIDA release path.

## Inputs

Each authoring or compression run MUST identify:

1. target artifact path,
2. artifact type,
3. intended audience: root orchestrator, worker lane, operator, developer, verifier, or reader,
4. loading posture: always-loaded bootstrap, frequently-loaded runtime protocol, task-selected reference, or lazy appendix,
5. source document or research references,
6. protected atoms,
7. target token budget,
8. required proof commands.

## Outputs

A completed protocol-authoring run MUST produce:

1. canonical markdown with required metadata footer,
2. sibling `*.changelog.jsonl`,
3. owning map/catalog registration,
4. bootstrap registration when the document is bootstrap-visible,
5. token count before/after when compressing an existing artifact,
6. validation evidence from DocFlow and token counting,
7. compression route evidence when quality and size conflict,
8. recorded budget exception when exact anchors or mandatory atoms force a higher token count.

## Required Protocol Shape

A protocol or instruction document SHOULD use this block order unless a stricter owner template applies:

1. Purpose,
2. Trigger,
3. Scope,
4. Authority,
5. Inputs,
6. Outputs,
7. Rules,
8. Forbidden,
9. Escalation,
10. Validation,
11. Token Budget,
12. Metadata.

Blocks MAY be omitted only when they are not meaningful for the artifact class. If a required block is omitted, the reason SHOULD be recorded in the changelog or task evidence.

## Algorithmic Pipeline

Every authoring or compression run MUST follow this pipeline:

1. `classify`: determine artifact type, document family, audience, loading posture, and validation profile.
2. `split blocks`: split the document into semantic blocks and mark each block as kernel, slice, appendix, or metadata.
3. `choose algorithm`: select the block algorithm from explicit mappings or the unknown-block quality/size router.
4. `compress`: apply the selected algorithm without changing protected atoms.
5. `recover protected atoms`: restore exact identifiers, commands, paths, code spans, field names, dates, and versions.
6. `recover anchors`: for existing runtime/protocol owners, preserve legacy section headings or add an exact crosswalk.
7. `validate`: check semantic atoms, protected atoms, references, headings, commands, proof gates, and budget exceptions.
8. `count tokens`: measure with `tiktoken-cli --model gpt-4o` or the active tokenizer declared by the task.
9. `register`: update owning maps, catalogs, sidecar, and changelog.

## Algorithm Library

### Document Type Classification

Intent: choose the correct document schema and proof profile before rewriting.

Procedure:

1. Read `artifact_type` from the footer when present.
2. If the footer is absent, infer from path, filename suffix, and top-level headings.
3. Assign `doc_family` as `normative`, `process`, `reference`, `decision`, `research`, `map`, or `template`.
4. Select the expected block schema and validation profile for that family.
5. Fail closed when the path and footer disagree about authority.
6. Do not apply protocol-authoring gates to `artifact_type: product_research_doc` unless the artifact explicitly opts in with `protocol_authoring_gate: enforced`.

Acceptance: the run names one artifact type, one document family, one loading posture, and one validation profile before compression.

### Block Extraction

Intent: make compression decisions at the smallest meaningful unit.

Procedure:

1. Preserve title and metadata footer exactly unless the task explicitly updates metadata.
2. Split the body by `##` and `###` headings.
3. Classify each block as `purpose`, `scope`, `trigger`, `authority`, `rules`, `workflow`, `inputs`, `outputs`, `gates`, `exceptions`, `examples`, `references`, or `metadata`.
4. Mark each block as:
   - `kernel` when it changes behavior or authority,
   - `slice` when it supports a bounded workflow,
   - `appendix` when it explains or illustrates,
   - `metadata` when it enables discovery or validation.

Acceptance: every non-empty block has exactly one block type and one retention class.

### LLMLingua Coarse-To-Fine Compression

Intent: reduce low-value text while preserving high-information tokens.

Procedure:

1. Set a token budget for the whole artifact and for each block.
2. Score coarse units by relevance, authority, protected-atom density, and duplication.
3. Remove or move low-value units before token-level compression.
4. Split retained units into smaller segments.
5. Score tokens or short phrases for importance.
6. Retain high-importance tokens and remove redundant connectors, restatements, and examples.
7. Re-run protected-atom recovery after compression.

Acceptance: the compressed block preserves mandatory semantic atoms and records the token delta.

### LongLLMLingua Question-Aware Compression

Intent: compress for the active task instead of for a generic summary.

Procedure:

1. Treat the current task or operator question as the compression query.
2. Rank blocks by query relevance.
3. Allocate more budget to high-ranked blocks and less budget to low-ranked blocks.
4. Place high-relevance blocks before secondary context.
5. Move low-relevance explanation to appendix or lazy-load references.

Acceptance: the compressed output answers the active task without forcing the reader to load the full source document.

### Lost-In-The-Middle Reordering

Intent: preserve recall of critical rules in long prompts or loaded protocol stacks.

Procedure:

1. Rank retained blocks by operational importance.
2. Put identity, authority, hard requirements, and primary workflow near the beginning.
3. Put terminal stop, safety, validation, and escalation gates near the end.
4. Put explanation, examples, rejected alternatives, and source notes in the middle or appendix.

Acceptance: a reader can identify the owner, trigger, main rule, and stop condition from the beginning and end of the artifact.

### Subsequence Recovery

Intent: restore exact source spans after generative or lossy compression.

Procedure:

1. Compare compressed text with the original block.
2. Detect protected atoms and exact spans that were paraphrased.
3. Replace approximations with the longest exact original subsequence that preserves the compressed sentence.
4. Repeat until protected atoms match the original or the task explicitly changed them.

Protected atoms: artifact ids, protocol ids, task ids, commands, paths, environment variables, JSON fields, error strings, dates, version tuples, code symbols, and URLs.

Acceptance: no protected atom is altered, normalized, translated, reordered, or dropped without an explicit change record.

### Exact Legacy Anchor Crosswalk

Intent: allow aggressive token reduction of existing protocol owners without breaking references, bootstrap recall, or operator memory.

Procedure:

1. Extract exact legacy section headings before compression.
2. Keep every still-authoritative heading in place when feasible.
3. When the compact structure changes, add a `Compressed Legacy Section Crosswalk` or equivalent block.
4. Preserve exact legacy heading text inside the crosswalk.
5. Mark removed headings only when their rule is obsolete, superseded, or moved to a named owner.
6. Count crosswalk tokens as a quality-preservation cost, not as avoidable duplication.

Acceptance: every required legacy anchor is present either as a heading or as an exact crosswalk entry, and the token report records any target-budget exception caused by anchor preservation.

### Preserve-Exact Validation

Intent: prevent compression from corrupting executable or discoverable content.

Procedure:

1. Extract headings, code blocks, inline code, URLs, paths, command atoms, and footer keys before compression.
2. Extract the same atom classes after compression.
3. Compare required exact atoms.
4. Fail when a required exact atom is missing or changed.
5. Warn when non-required headings or examples were moved to appendix.

Acceptance: exact-atom diff is empty for required atoms.

### Semantic Atom Coverage

Intent: prove that meaning survived even when wording changed.

Procedure:

1. Before compression, list mandatory semantic atoms: hard requirements, prohibitions, triggers, authority owner, escalation owner, inputs, outputs, and proof gates.
2. After compression, verify each atom by exact wording or an explicitly accepted equivalent.
3. Probe the compressed artifact with expected scenarios: when to apply it, what is forbidden, what proof is required, and who owns the decision.
4. Fail when behavior, grounding, authority, or proof detail is lost.

Acceptance: all mandatory semantic atoms are present and each scenario answer is unchanged.

### Quality-Preserving Refactor Contract

Intent: define "refactor without content loss" for protocol and instruction compression.

Procedure:

1. Before rewriting, list the protected atom classes and mandatory semantic atoms.
2. Replace repeated prose with one binding rule, table row, or crosswalk entry.
3. Preserve operational meaning for triggers, authority, commands, gates, stop conditions, forbidden actions, and proof requirements.
4. Prefer shorter wording only after the atom list still passes.
5. Treat a larger-than-target result as acceptable when the excess tokens preserve exact anchors or mandatory atoms.

Acceptance: the new text changes shape and token count, but not executable behavior, authority, proof gates, or discoverability.

### RFC 2119 Normative Rewrite

Intent: turn vague prose into compact binding requirements.

Procedure:

1. Use `MUST`, `MUST NOT`, `SHOULD`, `SHOULD NOT`, and `MAY` only for binding requirements.
2. Replace soft phrases such as "try to", "usually", or "it is good to" with one normative keyword or remove them.
3. Deduplicate repeated rules by keeping the strongest binding statement.
4. Put prohibitions in `Forbidden` or another kernel block.

Acceptance: each binding rule is necessary, direct, testable, and not repeated elsewhere in the same artifact.

### Diataxis Split

Intent: keep protocols operational while moving learning material out of always-loaded paths.

Procedure:

1. Classify each block as tutorial, how-to, reference, or explanation.
2. Keep protocol bodies as reference plus minimal how-to.
3. Move tutorials, examples, rationale, and broad explanation to appendix, research, or linked lazy-load docs.
4. Keep only the examples needed to disambiguate a rule.

Acceptance: the main body remains executable without reading explanation-heavy material.

### ADR/MADR Decision Capture

Intent: separate durable decisions from protocol law.

Procedure:

1. Use ADR/MADR shape only for decisions that need historical context.
2. Required blocks are Status, Context, Decision, and Consequences.
3. Add rejected alternatives when they prevent repeated debate.
4. Supersede old ADRs instead of rewriting decision history.
5. Link the ADR from the law or protocol it supports.

Acceptance: the protocol states current law, while the ADR records why that law was chosen.

### IEEE 29148 Requirement Quality

Intent: make requirements compact and verifiable.

Procedure:

1. Each requirement MUST be necessary, unambiguous, feasible, verifiable, and traceable.
2. Convert vague paragraphs into requirement rows or numbered rules.
3. Attach proof or acceptance evidence to each requirement class.
4. Remove unverifiable requirements or rewrite them into observable behavior.

Acceptance: every requirement has an owner, observable pass/fail condition, and trace target.

### C4 Architecture Mapping

Intent: use the lowest useful architecture view instead of broad prose.

Procedure:

1. Choose the lowest useful zoom: Context, Container, Component, or Code.
2. Do not include all C4 levels by default.
3. Prefer maps, tables, and owner matrices over long narrative.
4. Link lower-level details only when implementation requires them.

Acceptance: the reader can identify system boundary, owner, dependency, and changed surface without loading unrelated architecture text.

### Prompt Cache Prefix Layout

Intent: reduce runtime cost for repeated prompt stacks.

Procedure:

1. Put stable, repeated content before dynamic content.
2. Put runtime state, task-specific evidence, and user request deltas near the end.
3. Keep cache-prefix text byte-stable across runs when the meaning has not changed.
4. Avoid unnecessary reordering of tools, instruction blocks, and bootstrap sections.
5. Use `prompt_cache_key` or equivalent runtime cache routing when the active platform supports it.

Acceptance: stable bootstrap/protocol prefixes do not change during unrelated task updates.

### Source-Versus-Installed Enforcement Split

Intent: keep validation honest when source DocFlow rules exist but the installed runtime has not been released or is defective.

Procedure:

1. Prefer normal `vida docflow` validation when the runtime is usable.
2. When the runtime is defective for the active documentation block, use source-built validation binaries or script-only checks.
3. Do not create synthetic TaskFlow, DocFlow, lane, or receipt evidence.
4. Record source-only validation separately from installed-runtime enforcement.
5. Treat installed-runtime coverage as deferred until the normal release/install path consumes the source change.

Acceptance: the report states whether enforcement was source-only, installed-runtime-backed, or blocked by a runtime defect.

### Token Budget Gate

Intent: make token reduction measurable.

Procedure:

1. Count the original artifact with `tiktoken-cli --model gpt-4o` or the active tokenizer.
2. Set a target budget before rewriting.
3. Compress and count again.
4. Fail when the compressed artifact exceeds budget without a recorded exception.
5. Record before/after counts for compression work.

Default budgets:

1. Always-loaded bootstrap pointer: 50 to 150 tokens.
2. Runtime capsule: 250 to 700 tokens.
3. Protocol kernel: 400 to 1200 tokens.
4. Full owner law: 30 to 50 percent smaller than source when compressing, or a recorded cap when newly authored.
5. Research appendix: lazy-load by default.

Acceptance: token count and exception status are visible in task or changelog evidence.

## Auto Algorithm Selection

Known blocks MUST use explicit mappings:

| Block type | Default algorithm |
| --- | --- |
| `purpose`, `trigger`, `scope` | RFC 2119 cleanup plus semantic atom coverage |
| `authority`, `forbidden`, `validation`, `metadata` | preserve-exact validation plus subsequence recovery |
| `rules`, `gates`, `escalation` | semantic atom coverage plus conservative LLMLingua |
| `workflow`, `inputs`, `outputs` | block extraction plus table normalization plus semantic atom coverage |
| `examples`, `rationale`, `explanation` | Diataxis split plus aggressive LLMLingua |
| `legacy-anchor`, `legacy-crosswalk` | exact legacy anchor crosswalk plus preserve-exact validation |
| `decision` | ADR/MADR capture |
| `requirement` | IEEE 29148 requirement quality |
| `architecture-map` | C4 architecture mapping |
| `cache-layout` | prompt cache prefix layout |
| `reference` | LongLLMLingua question-aware retention |

Unknown blocks MUST compute `quality_risk` and `size_pressure`.

`quality_risk` inputs:

1. protected atom density,
2. normative keyword density,
3. command, path, code, or JSON-field density,
4. authority level,
5. proof-gate count,
6. ambiguity score.

`size_pressure` inputs:

1. token count,
2. duplication ratio,
3. example and reference volume,
4. relevance to the active task,
5. cache-prefix stability.

Default scoring:

1. `quality_risk=high` when any kernel block has protected atom density above 8 percent, contains binding normative keywords, names commands or paths, owns authority, or defines proof/stop gates.
2. `quality_risk=low` only when the block is explanatory, non-authoritative, and has no required protected atoms.
3. `size_pressure=high` when the block is above 500 tokens, repeats another block, spends more than 30 percent of its tokens on examples/references, or is low relevance for the active task.
4. `size_pressure=low` when the block is below 500 tokens, non-duplicative, and directly needed for the active task or stable cache prefix.

Routing:

1. High quality risk plus low size pressure: use preserve-exact validation and RFC 2119 cleanup only.
2. High quality risk plus high size pressure: use semantic atom coverage, subsequence recovery, and conservative LLMLingua.
3. Low quality risk plus high size pressure: use aggressive LLMLingua, LongLLMLingua, and Diataxis appendix split.
4. Low quality risk plus low size pressure: use light rewrite and table normalization.

Unknown block acceptance gate:

1. protected-atom validation passes,
2. semantic atom coverage passes,
3. legacy-anchor coverage passes when compressing an existing owner artifact,
4. token delta is recorded,
5. selected algorithm and route are recorded in task or changelog evidence.

## Rules

1. Kernel law MUST be shorter than explanatory material.
2. Authority, trigger, stop, and validation rules MUST be discoverable without reading examples.
3. Maps and bootstrap carriers MUST point to the canonical law; they MUST NOT duplicate the law.
4. Compression MUST NOT remove fail-closed behavior, owner boundaries, proof commands, or escalation rules.
5. Token savings MUST NOT be counted as success unless validation confirms behavior preservation.
6. When token and quality goals conflict, quality wins and the exception MUST be recorded.
7. Compression of existing runtime/protocol owners MUST be treated as refactoring: behavioral atoms are preserved, explanatory form may change.
8. Research baselines MAY measure and recommend protocol changes without becoming gated protocol artifacts unless they explicitly opt in.

## Forbidden

1. Do not compress commands, paths, ids, JSON fields, or code symbols by paraphrase.
2. Do not move hard requirements into appendices.
3. Do not hide bootstrap-visible law only in a detailed catalog.
4. Do not replace protocol law with a research summary.
5. Do not claim runtime enforcement until an executable runtime surface consumes the law.
6. Do not optimize MCP/tool output when the task explicitly targets assistant instruction, reasoning, or response token cost.
7. Do not claim "content preserved" from token reduction alone.
8. Do not fake TaskFlow, DocFlow, lane, or receipt evidence when the runtime is under a declared defect bypass.

## Escalation

1. If exact atoms conflict with readability, preserve exact atoms and record the readability limitation.
2. If the target budget would remove mandatory semantic atoms, keep the atoms and record a budget exception.
3. If a document becomes bootstrap-visible, update `AGENTS.sidecar.md`, `docs/project-root-map.md`, and the owning product/spec maps in the same bounded change.
4. If DocFlow rejects a valid registration, classify the result as a DocFlow/runtime defect and keep the documentation step open.
5. If the runtime is declared defective for the active documentation block, use bounded static/source validation and record missing runtime proof as deferred evidence, not as a clean installed-runtime pass.

## Validation

Minimum validation for this law and future protocol-authoring changes:

1. `vida docflow check-file --path <changed-doc>`
2. `vida docflow activation-check --root docs`
3. `vida docflow protocol-coverage-check --profile active-canon`
4. `vida docflow closeout --changed --compact`
5. `tiktoken-cli --model gpt-4o <changed-doc>`

When runtime validation is bypassed because the runtime is defective, the minimum substitute proof is:

1. source-built DocFlow `check-file` for each changed protocol or instruction doc,
2. protected-atom coverage report against the pre-rewrite source when compressing,
3. semantic atom checklist for hard requirements, prohibitions, triggers, authority, and gates,
4. `tiktoken-cli --model gpt-4o <changed-doc>`,
5. explicit note that installed-runtime enforcement is deferred.

Bootstrap visibility validation:

1. `AGENTS.sidecar.md` MUST list this law when it is bootstrap-visible.
2. `docs/project-root-map.md` MUST route protocol-authoring or token-economy questions to this law.
3. `docs/product/spec/current-spec-map.md` and `docs/product/spec/current-spec-catalog.md` MUST register this law.
4. `vida orchestrator-init --full --json` SHOULD keep bootstrap map references discoverable through the project docs read path.

## Token Budget

1. Always-loaded bootstrap docs SHOULD carry only pointers and hard bootstrap invariants.
2. Protocol owner docs SHOULD carry law, algorithm selection, and validation rules.
3. Research notes SHOULD be lazy-load references unless promoted into law.
4. A protocol rewrite is incomplete until token count is measured or a missing-tokenizer blocker is recorded.
5. The default tokenizer is `gpt-4o` through `tiktoken-cli` unless the task declares another model.

## Source References

1. LLMLingua: `https://arxiv.org/html/2310.05736v2`
2. LongLLMLingua: `https://arxiv.org/html/2310.06839v2`
3. Prompt information preservation evaluation: `https://arxiv.org/html/2503.19114v2`
4. RFC 2119 normative keywords: `https://datatracker.ietf.org/doc/html/rfc2119`
5. Diataxis documentation framework: `https://diataxis.fr/start-here/`
6. ADR templates: `https://adr.github.io/adr-templates/`
7. C4 model: `https://c4model.com/`
8. OpenAI prompt caching guidance: `https://developers.openai.com/api/docs/guides/prompt-caching`
9. IEEE 29148 requirements engineering: `https://standards.ieee.org/standard/29148-2011.html`

## Metadata

1. Artifact path: `product/spec/protocol-authoring-and-token-economy-law`
2. Artifact type: `product_spec`
3. Bootstrap visibility: project-visible through `AGENTS.sidecar.md`
4. Runtime enforcement: deferred until a runtime/DocFlow consumer is implemented
5. Initial task: `protocol-authoring-token-economy-law-doc-20260703`
6. Session hardening: source-only enforcement split, legacy-anchor crosswalk, quality-preserving refactor gate, research exemption

-----
artifact_path: product/spec/protocol-authoring-and-token-economy-law
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-07-03
schema_version: '1'
status: canonical
source_path: docs/product/spec/protocol-authoring-and-token-economy-law.md
created_at: 2026-07-03T00:00:00+03:00
updated_at: 2026-07-03T10:44:28.4021126+03:00
changelog_ref: protocol-authoring-and-token-economy-law.changelog.jsonl
