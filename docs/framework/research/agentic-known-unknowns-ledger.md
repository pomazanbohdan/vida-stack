# Agentic Known Unknowns Ledger

**Purpose:** Keep unresolved research and design questions explicit so they are not silently converted into undocumented defaults.

---

## Known Unknowns

| ID | Open question | Why still open | Best next resolution path |
|---|---|---|---|
| U01 | exact score thresholds for single vs dual vs triad vs quorum | current research supports adaptivity more than precise numeric cutoffs | pilots + ablation evals |
| U02 | exact cap per task class for agent count | best cap depends on cost, coupling, and proof burden | route experiments |
| U03 | exact formulas for prior effectiveness influence | direction is clear, weighting is not | metrics + retrospective analysis |
| U04 | precise security-gate threshold for routing-law changes | risk is obvious, line placement is still local-policy work | OWASP mapping + security review |
| U05 | best trigger for switching from disagreement to arbiter mode | sources support the pattern more than the exact threshold | conflict logging + proving wave |
| U06 | which documented parameter families should be promoted first into config | many values exist, not all deserve runtime law yet | promotion backlog review |
| U07 | best route-regret threshold for rollback or policy retune | metric exists conceptually, threshold is not mature | eval package + pilot data |
| U08 | exact freshness cadence per source family | volatility is known, cadence remains local-policy tuning | watchlist + refresh history |

---

## Rule

If any of these unknowns materially affects a live implementation task:

1. do not guess silently
2. document the local temporary choice
3. route follow-up through experiment, pilot, or policy review
-----
artifact_path: framework/research/agentic-known-unknowns-ledger
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-known-unknowns-ledger.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-known-unknowns-ledger.changelog.jsonl
P26-03-09T21: 44:13Z
