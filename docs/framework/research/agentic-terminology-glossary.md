# Agentic Terminology Glossary

**Purpose:** Keep core orchestration vocabulary stable across plans, protocols, config, tests, and reviews.

---

## Terms

| Term | Canonical meaning |
|---|---|
| `agent definition` | umbrella system artifact that combines role logic, instruction contract, permissions, output contract, fallback/escalation, rendering surface, and versioning |
| `instruction contract` | canonical behavioral law that specifies allowed behavior, forbidden behavior, fallback, escalation, and output obligations |
| `prompt template configuration` | runtime-specific render/config layer that materializes the instruction contract for a concrete provider or templating system |
| `role` | stable responsibility in the system |
| `profile` | structured behavioral contract for a role |
| `execution mode` | situational mode that changes posture without changing role identity |
| `lane` | concrete execution or review path |
| `task class` | primary work category used for routing |
| `task score` | normalized signal combining complexity, risk, coupling, volatility, and prior effectiveness |
| `agent count mode` | selected scale pattern such as single, dual, triad, quorum, or arbiter |
| `consensus mode` | rule used to combine or judge multiple lane outputs |
| `verification burden` | minimum proof expected before closure |
| `research refresh` | required source-validation step before source-sensitive work |
| `proof obligation` | concrete evidence that must exist because of the task |
| `escalation` | forced route change to stricter decision or safety handling |

---

## Vocabulary Rule

Do not introduce synonyms for these terms in protocols or config unless the glossary is updated first.

## Relation Rule

Use this hierarchy consistently:

1. `agent definition`
2. `instruction contract`
3. `prompt template configuration`

Rules:

1. `agent definition` is the umbrella object
2. `instruction contract` is the canonical logic layer
3. `prompt template configuration` is the rendering/configuration layer
4. `prompt template configuration` must not silently become the source of truth for role behavior
-----
artifact_path: framework/research/agentic-terminology-glossary
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-terminology-glossary.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-terminology-glossary.changelog.jsonl
P26-03-09T21: 44:13Z
