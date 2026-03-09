# Agentic Heuristic Library

**Purpose:** Preserve conditional design rules distilled from research when the evidence is strong enough to guide design but not strong enough to become a universal invariant.

---

## Heuristics

| ID | Heuristic | Use when | Avoid when |
|---|---|---|---|
| H01 | Prefer single-lane authoring for shared writable scope. | protocol/config/doc work with overlapping files | decomposable read-heavy analysis |
| H02 | Add a second independent lane when uncertainty or critique value is high. | medium-risk bugfixes, routing changes, research checks | trivial bounded tasks |
| H03 | Use triad form when formative critique and independent verification are both valuable. | medium/high-risk implementation and policy work | low-risk documentation updates |
| H04 | Escalate to arbiter when disagreement persists after bounded independent passes. | conflict-heavy research or policy questions | cases where ambiguity actually requires spec rewrite |
| H05 | Increase proof burden after weak prior effectiveness before increasing agent count aggressively. | routes with recent low-quality outcomes | well-performing low-risk routes |
| H06 | Treat volatile source families as the shortest freshness-SLA bucket. | OWASP, agent scaling guidance, prompt safety guidance | stable local-only conventions |
| H07 | Promote only the highest-value documented parameters into runtime law first. | early architecture stage | when full runtime support is already justified |
| H08 | Use examples and golden samples when ambiguity remains after prose definitions. | profile cards, task packets, route laws | fields already enforced by tests |

---

## Heuristic Status

These are `good default rules`, not permanent law. Promote or retire them based on pilots, evals, and future source refresh.
-----
artifact_path: framework/research/agentic-heuristic-library
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-heuristic-library.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-heuristic-library.changelog.jsonl
P26-03-09T21: 44:13Z
