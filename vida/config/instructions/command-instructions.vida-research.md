# /vida-research — External Business Research

Purpose: conduct external, business-level research that precedes technical work and feeds planning artifacts.

## Protocol Layers

This command maps layers as:

1. `CL1 Intake` -> topic resolution, scope limits, and continuation mode selection.
2. `CL2 Reality And Inputs` -> external evidence collection plus WVP-backed factual validation when needed.
3. `CL3 Contract And Decisions` -> actionable candidate filtering and approval boundary for what may mutate checklist/decisions.
4. `CL4 Materialization` -> research document updates plus approved feature/decision sync.
5. `CL5 Gates And Handoff` -> handoff inputs for `/vida-spec`, not implementation or task-pool mutation.

Canonical source: `command-layer-protocol.md`

Handoff boundary:

1. `/vida-research` hands off evidence and approved business-level deltas only.
2. Technical contract formation starts in `/vida-spec`.
3. Research must not jump directly from evidence gathering to practical validation or implementation-shaped work.
4. The mandatory phase order is:
   - bounded research pass,
   - research artifact update,
   - requirement formation,
   - specification/intake formation,
   - only then practical validation, technical spec work, or implementation-facing continuation.

## Role Boundary

1. This command is BA-first: market/domain/problem/opportunity research.
2. This command does not execute implementation tasks.
3. Technical specification and technical research belong to `/vida-spec`.
4. Feature extraction/integration is handled directly in this command.

## Mandatory Validation Contract

1. Use `web-validation-protocol.md` as canonical internet validation standard.
2. For external factual claims, keep source evidence and reconciliation status.
3. For API/server assumptions discovered during research, require live validation evidence before escalation to `/vida-spec`.
4. Apply `spec-contract-protocol.md` gates for this non-development flow.

## Research Completeness Checklist

Every non-trivial research pass must end with an explicit completeness checklist.

Required checklist questions:

1. Was a full bounded research pass completed for the active question, not just a first-hit summary?
2. Which evidence classes were checked:
   - existing research docs,
   - current product/spec canon,
   - relevant local code/config/runtime surfaces,
   - external web sources,
   - adjacent framework/project protocols?
3. What still remains unknown, weakly supported, or conflicting?
4. Does any unresolved gap materially affect the recommendation or handoff?
5. If yes, what must be researched next before closure?

Completion rule:

1. If the checklist reveals unresolved material gaps, the research pass is not complete.
2. Continue research before finalizing recommendations, approvals, or handoff.
3. Do not present a bounded partial scan as if it were comprehensive.
4. Research may close only when no unresolved material research questions remain for the active decision.
5. The required target is `100% decision-ready confidence`, meaning:
   - no known material unknowns remain,
   - no known unresolved source conflicts remain,
   - no missing evidence class is still needed for the decision,
   - the current evidence is strong enough to support the decision without speculative fill-in.
6. If the operator cannot truthfully claim `100% decision-ready confidence`, the research pass remains open and must continue or explicitly downgrade the scope until that condition becomes true.

Autonomous continuation rule:

1. Once research is active, the next required research pass must be executed automatically when the current checklist still shows material gaps.
2. Do not stop after one pass just to ask whether the remaining required research should continue, unless:
   - the next step would widen scope materially,
   - the next step would spend money or use privileged systems,
   - the user explicitly asked to pause after the current pass.
3. The default behavior is `research -> gap check -> next required research -> repeat` until the checklist closes or a lawful blocker is reached.

Task-completion rule:

1. Research must not stop at the first acceptable-looking intermediate result when lawful task-owned work still remains.
2. The operator must continue until the active bounded research task is actually complete, including:
   - remaining required evidence collection,
   - artifact updates,
   - requirement formation,
   - thematic consolidation,
   - spec/intake handoff preparation.
3. A partial report is not a completion reason by itself.
4. Completion is reached only when the active research task has no remaining lawful next step inside its current scope.

Auto-continuation after reports:

1. Intermediate research reports do not close the flow by default.
2. If a report is emitted at the end of a bounded pass and the checklist still implies a next lawful step, the default behavior is `report -> continue`.
3. Stop after a report only when:
   - the next step would materially widen scope,
   - the next step needs paid, privileged, or user-owned systems,
   - the user explicitly asked to pause at the current report boundary,
   - the user explicitly asked to discuss the current report before continuation.

Research progression rule:

1. After each bounded research pass, update the living research artifact before treating the pass as complete.
2. After research artifacts are updated, form explicit requirements from the validated findings.
3. After requirements are formed, produce or update the bounded spec/intake artifact that will govern downstream practical work.
4. Practical research, technical validation, prototyping, or implementation-facing continuation is forbidden until steps 1-3 are complete for the current bounded question.
5. If new evidence reopens a closed assumption, repeat the sequence:
   - update research,
   - refresh requirements,
   - refresh spec/intake,
   - then continue downstream.

Thematic consolidation rule:

1. Research closure must not leave materially related findings scattered only across unrelated or weakly-linked artifacts.
2. When the topic accumulates meaningfully related findings across multiple passes, create or update a thematic research artifact that consolidates the relevant evidence, open questions, and implications.
3. Prefer one thematic living artifact per bounded topic over many fragmented notes.
4. If an existing artifact is too broad, split or add a topic-focused companion artifact rather than forcing unrelated material into one oversized document.
5. The required result is not only `updated artifacts` but `coherent topic-level consolidation`.

Coverage rule:

1. Research should be comprehensive across the relevant evidence stack:
   - research artifacts,
   - spec artifacts,
   - code/runtime evidence,
   - web validation,
   - competing alternatives when selection is involved.
2. Prefer explicit notes such as `covered`, `not-needed`, `not-found`, or `still-open` for each evidence class.

## Mandatory Artifacts

1. Topic research file: `docs/research/<topic>.md` (single living document per topic).
2. Feature list: `docs/feature-checklist.md`.
3. Decisions list: `docs/decisions.md` (only approved business-level decisions).
4. When the topic spans multiple related questions, a thematic consolidated artifact must exist or be created in the lawful research home.

## Pre-Search + Continuation (Mandatory)

Before collecting new data:

1. Search existing research files for the topic.
2. If matching topic exists: continue same file, do not duplicate.
3. If adjacent topics exist: reuse references and state deltas.
4. If nothing exists: create `docs/research/<topic>.md`.

Continuation rule:

1. Add a new iteration block (date + delta), never overwrite prior findings silently.
2. Preserve prior conclusions and explicitly mark what changed.

## Extraction Model

### Actionable Types (approval required)

1. `FEATURE`
2. `PROBLEM`
3. `REC`
4. `OPPORTUNITY`
5. `DECISION`

### Informational Types (no direct planning mutation)

1. `INSIGHT`
2. `RISK`
3. `COMPETITOR`

Priority for actionable items:

1. `🔴` critical
2. `🟡` important
3. `🟢` nice-to-have

## Feature List Contract (Do Not Change Format)

When approved actionable items are added to feature checklist, preserve existing file format exactly:

1. Status marks: `[ ]`, `[/]`, `[x]`.
2. Tier marks: `🆓`, `💎`, `🔮`.
3. Priority marks: `🔴`, `🟡`, `🟢`.
4. Entry shape: `- [status] {tier} {priority?} {feature name}`.

## Algorithm

1. Resolve topic and business goal.
2. Run pre-search and select mode: `continue | merge | new`.
3. Collect external evidence (market, competitor, workflow, domain practices).
4. Extract items by category and priority.
5. Deduplicate against existing checklist/decisions/spec index.
6. Present approval set (actionable only).
7. Apply approved changes:
   - Update `feature-checklist.md` (format preserved).
   - Update `decisions.md` for approved decisions.
8. Update research document iteration with:
   - New evidence,
   - Extracted items,
   - Approved/rejected actions,
   - Open questions,
   - Research completeness checklist,
   - Remaining gaps / next research actions.
9. Derive or refresh explicit requirement statements from the updated research where downstream decisions depend on them.
10. Create or refresh the topic-level thematic consolidation artifact when the findings are materially related but distributed across multiple subquestions or passes.
11. Record handoff-ready scope/contract inputs for downstream spec/intake formation.
12. Record evidence path in execution log artifacts.

## Output Template (Research File)

Each `docs/research/<topic>.md` iteration should contain:

1. `Iteration` (date, scope, objective).
2. `Sources`.
3. `Findings`.
4. `Actionable Candidates`.
5. `Informational Notes`.
6. `Decision Options (business-level)`.
7. `Approved to Feature List`.
8. `Approved to Decisions`.
9. `Handoff Inputs for /vida-spec`.
10. `Requirements Derived From This Iteration`.
11. `Spec / Intake Delta Needed Before Practical Continuation`.
12. `Related Topic Consolidation`.

## Lawful Report Stages

Reports may appear during `/vida-research` at these stages:

1. `CL1 Intake`
   - scope framing report,
   - topic normalization report.
2. `CL2 Reality And Inputs`
   - evidence progress report,
   - source coverage / conflict report,
   - completeness-check report.
3. `CL3 Contract And Decisions`
   - requirement summary report,
   - handoff-readiness report,
   - unresolved-decision report.
4. `CL4 Materialization`
   - research artifact update summary,
   - thematic consolidation summary.
5. `CL5 Gates And Handoff`
   - closure-ready report,
   - next-step report,
   - blocker report.

Rule:

1. Reports at `CL1`-`CL4` are normally intermediate and should auto-continue into the next lawful step when no blocker exists.
2. `CL5` may close the research flow only when the completeness rules are actually satisfied.
3. Any explicit user request to discuss the current report suspends auto-continuation for that report boundary.
4. If the report leaves any still-required work inside the same bounded research task, the operator must continue and finish that work rather than treating the report as closure.

## Command Variants

1. `/vida-research <topic>` — pre-search + continue/new.
2. `/vida-research refresh <topic>` — add new iteration to existing topic.
3. `/vida-research integrate <topic>` — approval and artifact updates.

## Constraints

1. No legacy `state/views/*` usage.
2. No separate sync-command dependency.
3. No old transition auto-read chains.
4. No implementation/task execution inside this command.

## Related

1. `docs/feature-checklist.md`
2. `docs/decisions.md`
3. `/vida-spec`
4. `use-case-packs.md`

-----
artifact_path: config/command-instructions/vida.research
artifact_type: command_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/command-instructions.vida-research.md
created_at: 2026-03-06T22:42:30+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: command-instructions.vida-research.changelog.jsonl
