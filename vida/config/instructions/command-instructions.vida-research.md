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

## Mandatory Artifacts

1. Topic research file: `docs/research/<topic>.md` (single living document per topic).
2. Feature list: `docs/feature-checklist.md`.
3. Decisions list: `docs/decisions.md` (only approved business-level decisions).

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
   - Open questions.
9. Record evidence path in execution log artifacts.

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
