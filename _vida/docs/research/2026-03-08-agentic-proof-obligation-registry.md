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

