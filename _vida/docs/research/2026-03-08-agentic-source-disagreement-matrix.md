# Agentic Source Disagreement Matrix

**Purpose:** Capture the important areas where sources do not fully align, or where the alignment is conditional enough that VIDA needs local decision rules.

---

## Disagreement Areas

| Topic | Disagreement shape | Why it matters | Current local handling |
|---|---|---|---|
| optimal agent count | sources agree on adaptivity, but not on exact thresholds | runtime still needs thresholds and caps | keep as threshold hypothesis, not law |
| size of useful ensembles | some guidance favors small councils, some supports larger fan-out on decomposable work | affects cost and latency | use task shape and proving data |
| when to switch from disagreement to arbiter | not all sources specify the same pivot point | affects escalation quality | keep explicit arbiter path and mark threshold open |
| exact proof burden formulas | broad agreement on adaptive proof, less clarity on formulas | impacts health checks and rollout policy | document bands first, numbers later |
| how much persona should affect runtime | everyone supports explicit profiles, but degree of behavioral richness varies | affects profile complexity | keep profiles compact and role-first |
| how aggressively to refresh volatile sources | strong agreement on refresh, weaker agreement on cadence | affects documentation upkeep | use watchlist + freshness buckets |

---

## Use Rule

When a protocol or child task touches one of these areas:

1. cite the disagreement explicitly
2. avoid pretending the research is more settled than it is
3. resolve locally with thresholds, pilots, or experiments rather than rhetoric

