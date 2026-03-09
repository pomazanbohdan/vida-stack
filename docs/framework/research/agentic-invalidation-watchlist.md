# Agentic Invalidation Watchlist

**Purpose:** Identify external changes that should automatically trigger re-review of the current research architecture.

---

## Watchlist

| Source family | Trigger | Re-review these docs first |
|---|---|---|
| OpenAI | major prompting, eval, or agent-safety guidance change | decision ledger, proof registry, source registry |
| Anthropic | major context engineering, worker, or auditing guidance change | task packet docs, anti-patterns, archetype library |
| Google | new scaling science or pattern-selection guidance | task score, pattern chooser, adaptive routing docs |
| Microsoft | persona, governance, or approval-pattern changes | role profile docs, escalation matrix, rollout docs |
| OWASP | new GenAI, ASVS, MASVS, MASTG, or SAMM releases | threat/control matrix, proof registry, OWASP mapping docs |

---

## Invalidation Rule

If a trigger changes:

1. source registry
2. source delta log
3. any affected decision ledger entries

before protocol or config updates continue.
-----
artifact_path: framework/research/agentic-invalidation-watchlist
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-invalidation-watchlist.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-invalidation-watchlist.changelog.jsonl
P26-03-09T21: 44:13Z
