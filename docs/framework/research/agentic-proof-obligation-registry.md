# Agentic Proof Obligation Registry

**Purpose:** Define what evidence must exist before different classes of work are considered complete.

---

## Proof Families

| Task family | Required proof | Optional proof | Blocking absence |
|---|---|---|---|
| source/registry docs | document existence, cross-links, field completeness | independent review note | missing required fields |
| taxonomy/profile docs | schema completeness, example cards, terminology alignment | peer review | ambiguous role authority |
| routing/scoring policy | protocol text, config exposure, tests, route examples | trace fixtures | missing tests or undefined thresholds |
| consensus/escalation policy | protocol text, merge rules, conflict examples | arbiter simulation | consensus replaces verification |
| task packet / handoff work | template, validator, sample packet | compaction replay proof | packet cannot survive independent execution |
| security / OWASP work | control mapping, ownership mapping, security verification path | review checklist | uncovered critical control family |
| traces / metrics work | schema, sample traces, metric definitions | operator mock output | trace lacks route rationale |
| rollout / pilot work | stop rules, rollback note, metrics, pilot selection | approval artifact | no fail-safe path |

---

## Proof Burden By Risk

| Risk | Minimum burden |
|---|---|
| low | doc proof + review |
| medium | doc proof + tests + reviewer or verifier |
| high | doc proof + tests + independent verifier + rollback path |
| critical | all of the above + approval/security gate as applicable |

---

## Closure Rule

No task is closure-ready if:

1. required proof is absent
2. required proof exists only in chat
3. proof contradicts the declared acceptance
-----
artifact_path: framework/research/agentic-proof-obligation-registry
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-proof-obligation-registry.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-proof-obligation-registry.changelog.jsonl
P26-03-09T21: 44:13Z
