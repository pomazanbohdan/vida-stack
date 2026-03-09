# Agentic Pattern Chooser Matrix

**Purpose:** Normalize route selection so task shape chooses the orchestration pattern rather than preference or habit.

---

## Matrix

| Task shape | Coupling | Risk | Prior effectiveness | Recommended pattern | Avoid | Consensus mode | Notes |
|---|---|---|---|---|---|---|---|
| research-only, bounded | low | low | neutral+ | single lane with independent review | quorum swarm | single_pass | prioritize speed and clarity |
| docs/protocol authoring | medium | low-medium | neutral+ | single author + independent review | multi-writer overlap | verifier_veto | shared write scope makes fan-out wasteful |
| low-risk bugfix | medium | low | good+ | single author + tester/reviewer | quorum by default | verifier_veto | keep proof light but explicit |
| medium bugfix with uncertainty | medium | medium | neutral | dual lane or triad | large ensemble | verifier_veto | use challenger or coach before verifier |
| tightly coupled refactor | high | medium | neutral | single sequential lane | high fan-out | single_pass + verifier | fan-out often adds merge noise |
| decomposable feature slice | low-medium | medium | neutral | dual or triad | single lane if evidence weak | majority or verifier_veto | safe place for bounded parallelism |
| routing / scoring change | medium-high | high | weak-neutral | single author + independent verifier | broad fan-out without arbiter | verifier_veto | route law changes are safety-relevant |
| conflict-heavy analysis | low-medium | medium | weak | quorum + arbiter | more of the same without tie-break | arbiter_tiebreak | disagreement needs interpretation |
| security-sensitive task | medium-high | high-critical | any | single author + security verifier + approval if needed | majority approval | verifier_veto | no security by popularity |
| pilot / rollout | medium | high | weak-neutral | measured pilot with explicit stop rules | silent expansion | weighted_majority + verifier | treat as experiment, not default adoption |

---

## Pattern Families

- `single author + independent review`
- `single author + verifier`
- `dual lane`
- `triad`
- `quorum + arbiter`
- `measured pilot`

---

## Anti-Selection Rules

Do not choose large fan-out when:

1. writable scope overlaps heavily
2. work is sequential or tightly coupled
3. disagreement is caused by ambiguous task framing rather than missing perspectives
4. proof burden is unresolved
-----
artifact_path: framework/research/agentic-pattern-chooser-matrix
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-pattern-chooser-matrix.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-pattern-chooser-matrix.changelog.jsonl
P26-03-09T21: 44:13Z
