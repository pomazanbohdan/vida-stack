# Agentic Metric Glossary

**Purpose:** Normalize the metrics used to evaluate routing, verification, and proving quality.

---

## Metrics

| Metric | Meaning |
|---|---|
| `task_success_rate` | percentage of tasks that meet acceptance and closure requirements |
| `cost_per_resolved_task` | orchestration and execution cost divided by fully resolved tasks |
| `rework_rate` | proportion of tasks that require at least one meaningful rework pass |
| `false_green_rate` | tasks that appeared complete locally but failed under later verification or proving |
| `consensus_quality` | how often consensus agreed with later independent verification |
| `verifier_overturn_rate` | how often verifier rejected or narrowed prior positive assessments |
| `route_regret` | cases where a different route would likely have been cheaper or safer |
| `prior_effectiveness` | summary signal of how well similar past routes performed |
| `security_gate_hit_rate` | share of tasks that triggered security review or OWASP-relevant gates |
| `refresh_debt` | volume of source-sensitive tasks that have exceeded freshness expectations |

---

## Interpretation Rule

No single metric should be used alone to declare the architecture healthy. Compare:

1. success
2. cost
3. proof quality
4. security posture
-----
artifact_path: framework/research/agentic-metric-glossary
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-metric-glossary.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-metric-glossary.changelog.jsonl
P26-03-09T21: 44:13Z
