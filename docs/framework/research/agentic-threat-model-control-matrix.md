# Agentic Threat Model And Control Matrix

**Purpose:** Capture the main risks to the autonomous orchestration architecture and the controls expected to contain them.

---

## Matrix

| Asset / surface | Threat | Control | Owner | Proof |
|---|---|---|---|---|
| source refresh flow | stale external guidance | blocking refresh gate, invalidation watchlist | orchestrator / researcher | updated registry + delta log |
| role profile layer | hidden authority escalation | role profile schema, compatibility matrix | role author + verifier | profile examples + review |
| routing law | unsafe fan-out or silent autonomy widening | adaptive policy, escalation matrix, verifier | routing owner | tests + route examples |
| consensus layer | false agreement or verifier bypass | consensus rules + verifier veto | routing owner + verifier | merge tests |
| task packets | hidden transcript dependency | packet schema + sample packet | packet owner | validator + sample packet |
| context/handoff | stale or incomplete summaries | handoff protocol + summary requirements | orchestrator | handoff example |
| security posture | prompt injection, excessive agency, data leakage | OWASP mapping + security review | security owner | control mapping |
| proving wave | unsafe rollout | measured pilot + stop rules + rollback | rollout owner | pilot plan |
| docs registry layer | drift or synonym sprawl | terminology glossary + mutation rules | documentation owner | glossary consistency review |

---

## OWASP Connection

Map these threats to OWASP surfaces:

- prompt injection
- unsafe tool use
- excessive agency
- insecure memory handling
- sensitive data exposure
- backend API verification
- mobile auth/storage/network
- SDLC maturity
-----
artifact_path: framework/research/agentic-threat-model-control-matrix
artifact_type: framework_research_doc
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/research/agentic-threat-model-control-matrix.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: agentic-threat-model-control-matrix.changelog.jsonl
P26-03-09T21: 44:13Z
