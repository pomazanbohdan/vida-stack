# Agentic Escalation Policy Matrix

**Purpose:** Define when work must stop being “normal execution” and reopen a stricter lane.

---

## Matrix

| Trigger | Escalate to | Output artifact |
|---|---|---|
| material external source change | source refresh + task delta | updated delta log and revised task packet |
| non-equivalent spec or product shift | spec reopen | spec delta / decision record |
| unresolved coach/verifier conflict | arbiter or human review | conflict summary |
| security posture widens autonomy or permissions | security review | security decision note |
| proof obligation missing or contradicted | verifier rework | proof blocker |
| pilot metrics regress below stop threshold | rollback / pilot halt | rollback note |
| terminology ambiguity affects routing or proof | terminology normalization | glossary update note |
| parameter family needs a new value | registry mutation review | registry update note |
| route law cannot justify chosen pattern | routing reconsideration | route rationale update |

---

## Escalation Rule

Escalation is required when continuing “as normal” would:

1. hide ambiguity,
2. lower proof burden silently,
3. widen security risk,
4. convert disagreement into false certainty.
-----
artifact_path: framework/research/agentic-escalation-policy-matrix
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-escalation-policy-matrix.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-escalation-policy-matrix.changelog.jsonl
P26-03-09T21: 44:13Z
