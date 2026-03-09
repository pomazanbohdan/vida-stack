# Agentic Anti-Pattern Catalog

**Purpose:** Keep common orchestration and documentation failures visible so future work can detect them early.

---

## Catalog

| ID | Anti-pattern | Symptom | Why harmful | Mitigation |
|---|---|---|---|---|
| A01 | Hard-minimum agent dogma | every task gets 2+ lanes by default | wastes cost and degrades sequential work | use task score and task shape |
| A02 | Fake consensus | multiple lanes agree but all inherited the same bias | false confidence | keep independence and verifier separation |
| A03 | Role collapse | writer, verifier, reviewer, and approver blur together | proof becomes self-approval | enforce compatibility matrix |
| A04 | Stale-source execution | work uses old research without refresh | decisions drift from reality | use refresh gate and watchlist |
| A05 | Verification bypass | consensus or coach stands in for proof | unsafe closure | require proof obligations and verifier |
| A06 | Security flattening | security review merged into generic review | OWASP surfaces disappear | keep threat/control matrix and security owners |
| A07 | Context drift | child task depends on chat memory | non-repeatable execution | use task packets and handoff artifacts |
| A08 | Metric blindness | system ships without measurable outcomes | cannot compare routes | use metric glossary and eval plan |
| A09 | Silent terminology drift | same word means different things across docs | route and proof ambiguity | use terminology glossary |
| A10 | Over-documenting without promotion | docs collect values that never become testable | paper architecture only | promote high-value items into protocols/tests |

---

## Review Use

When reviewing a new protocol, task packet, or config change:

1. scan for all 10 anti-patterns
2. record any hits in verification notes
3. if a hit is severe, route to escalation instead of papering over it
-----
artifact_path: framework/research/agentic-anti-pattern-catalog
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-anti-pattern-catalog.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-anti-pattern-catalog.changelog.jsonl
P26-03-09T21: 44:13Z
