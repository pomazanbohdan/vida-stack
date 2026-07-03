# /vida-research — External Business Research

Purpose: conduct external, business-level research that precedes technical work and feeds planning artifacts.

## Protocol Layers

This command maps layers as:

| Layer | Research role |
| --- | --- |
| `CL1 Intake` | topic resolution, scope limits, continuation mode |
| `CL2 Reality And Inputs` | external evidence + WVP-backed validation |
| `CL3 Contract And Decisions` | actionable filtering + approval boundary for checklist/decisions mutation |
| `CL4 Materialization` | research document updates + approved feature/decision sync |
| `CL5 Gates And Handoff` | handoff inputs for `/vida-spec`, not implementation or task-pool mutation |

Canonical source: `command-layer-protocol.md`

Handoff boundary: `/vida-research` hands off evidence and approved business-level deltas only; technical contract formation starts in `/vida-spec`; research must not jump from evidence gathering directly to practical validation or implementation-shaped work. Mandatory order: bounded research pass -> research artifact update -> requirement formation -> specification/intake formation -> practical validation, technical spec work, or implementation-facing continuation.

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

Required checklist questions: full bounded research pass vs first-hit summary; checked evidence classes (`existing research docs`, `current product/spec canon`, `relevant local code/config/runtime surfaces`, `external web sources`, `adjacent framework/project protocols`); remaining unknown/weak/conflicting evidence; material effect on recommendation/handoff; next research required before closure.

Completion rule: unresolved material gaps mean the research pass is not complete; continue research before recommendations, approvals, or handoff; do not present bounded partial scan as comprehensive; close only when no material research questions remain for the active decision. Target: `100% decision-ready confidence` = no known material unknowns, no unresolved source conflicts, no missing evidence class needed, and evidence strong enough without speculative fill-in. If this cannot be claimed truthfully, continue or explicitly downgrade scope until true.

Autonomous continuation rule: once research is active, execute the next required research pass automatically when the checklist shows material gaps. Do not stop after one pass to ask whether to continue unless the next step widens scope materially, spends money/uses privileged systems, or the user explicitly asked to pause. Default loop: `research -> gap check -> next required research -> repeat` until closed or blocked.

Task-completion rule: do not stop at the first acceptable-looking intermediate result while lawful task-owned work remains. Continue through required evidence collection, artifact updates, requirement formation, thematic consolidation, and spec/intake handoff preparation. A partial report is not completion. Completion requires no remaining lawful next step inside current scope.

Auto-continuation after reports: intermediate reports do not close the flow by default. If a bounded-pass report still implies a lawful next step, default to `report -> continue`. Stop only when the next step widens scope, needs paid/privileged/user-owned systems, or the user explicitly asked to pause/discuss at the report boundary.

Research progression rule: after each bounded pass, update the living research artifact, form explicit requirements from validated findings, then produce/update the bounded spec/intake artifact for downstream practical work. Practical research, technical validation, prototyping, or implementation-facing continuation is forbidden until those steps are complete for the current bounded question. If new evidence reopens a closed assumption, repeat: update research -> refresh requirements -> refresh spec/intake -> continue downstream.

Thematic consolidation rule: research closure must not leave related findings scattered across unrelated or weakly-linked artifacts. When related findings accumulate across passes, create/update a thematic research artifact with evidence, open questions, and implications. Prefer one living artifact per bounded topic; if an artifact is too broad, split or add a topic-focused companion. Required result: `coherent topic-level consolidation`, not only `updated artifacts`.

Coverage rule: research should cover the relevant evidence stack: research artifacts, spec artifacts, code/runtime evidence, web validation, and competing alternatives when selection is involved. Prefer `covered`, `not-needed`, `not-found`, or `still-open` for each evidence class.

## Mandatory Artifacts

1. Topic research file: `docs/product/research/<topic>-survey.md` (single living document per topic).
2. Feature list: `docs/feature-checklist.md`.
3. Decisions list: `docs/decisions.md` (only approved business-level decisions).
4. When the topic spans multiple related questions, a thematic consolidated artifact must exist or be created in the lawful research home.

## Pre-Search + Continuation (Mandatory)

Before collecting new data:

1. Search existing research files for the topic.
2. If matching topic exists: continue same file, do not duplicate.
3. If adjacent topics exist: reuse references and state deltas.
4. If nothing exists: create `docs/product/research/<topic>-survey.md`.

Continuation rule:

1. Add a new iteration block (date + delta), never overwrite prior findings silently.
2. Preserve prior conclusions and explicitly mark what changed.

## Extraction Model

### Actionable Types (approval required)

`FEATURE`, `PROBLEM`, `REC`, `OPPORTUNITY`, `DECISION`.

### Informational Types (no direct planning mutation)

`INSIGHT`, `RISK`, `COMPETITOR`.

Priority for actionable items: `🔴` critical, `🟡` important, `🟢` nice-to-have.

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

Each `docs/product/research/<topic>-survey.md` iteration should contain: `Iteration` (date, scope, objective), `Sources`, `Findings`, `Actionable Candidates`, `Informational Notes`, `Decision Options (business-level)`, `Approved to Feature List`, `Approved to Decisions`, `Handoff Inputs for /vida-spec`, `Requirements Derived From This Iteration`, `Spec / Intake Delta Needed Before Practical Continuation`, `Related Topic Consolidation`.

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
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/command-instructions/operator.vida-research-guide.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: 2026-07-03T14:05:00+03:00
changelog_ref: operator.vida-research-guide.changelog.jsonl
protocol_authoring_gate: enforced
protocol_compression_status: audit_passed
protocol_compression_algorithm: guide-prose-compaction+list-normalization+protected-command-validation
protocol_compression_baseline_ref: 4aee9451c:vida/config/instructions/command-instructions/operator.vida-research-guide.md
protocol_compression_audit_at: 2026-07-03T14:05:00+03:00
protocol_compression_before_tokens: 2606
protocol_compression_after_tokens: 2350
protocol_compression_content_sha256: 9f40ef878833b7ab0197457ff8240f3dd0aed4c0e3f292c79b81ee3c77570420
