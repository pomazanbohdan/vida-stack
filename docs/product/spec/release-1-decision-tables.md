# Release 1 Decision Tables

Status: active Release-1 control law

Purpose: define canonical if-then decision tables for approval, delegation, tool use, retrieval, memory, and rollback so `Release 1` implementation can enforce bounded rules without relying on prose interpretation.

## 1. Scope

This document defines decision tables for:

1. approval requirements,
2. delegation requirements,
3. tool execution gates,
4. retrieval trust gates,
5. memory write gates,
6. rollback and incident gates.

This document does not define:

1. code-level API shapes,
2. vendor-specific policy engines,
3. release-metric thresholds.

## 2. Reading Rule

1. Each row is normative.
2. The first matching row applies unless a later column explicitly says `escalate`.
3. When two rows would conflict, the stricter control wins.
4. If no row matches, fail closed.

## 3. Approval Decision Table

| Condition | Approval required | Required artifacts | Outcome if absent |
|---|---|---|---|
| `workflow_class` is `informational_answer` and `risk_tier = R0` | no | trace root only | continue |
| `workflow_class` is `retrieval_grounded_answer` and no sensitive source is involved | no by default | trace, citation linkage, freshness posture | continue |
| `workflow_class` is `documentation_mutation` on non-protected docs | optional | trace, proof artifact, rollback posture | warn only |
| `workflow_class` is `documentation_mutation` on protected canon surfaces | yes | approval record, trace, rollback posture | block |
| `risk_tier >= R2` and the workflow mutates shared/runtime truth | yes | approval record, policy decision, trace | block |
| `workflow_class` is `tool_assisted_write` | yes | approval record, tool contract, rollback posture, incident posture | block |
| `workflow_class` is `identity_or_policy_change` | yes | approval record, policy decision, audit chain | block |
| `workflow_class` is `memory_write` for sensitive memory classes | yes | approval record, consent basis, TTL/deletion posture | block |

## 4. Delegation Decision Table

| Condition | Delegation allowed | Required artifacts | Outcome if absent |
|---|---|---|---|
| read-only bounded analysis packet | yes | packet id, lane role, trace root | block if packet missing |
| write-producing bounded packet | yes | developer handoff packet, lane receipt, bounded file set | block |
| open delegated lane already exists for same packet | no local takeover | open lane receipt, wait/resume/supersede path | block |
| takeover requested with explicit exception path | yes | exception-path receipt, supersession linkage | block |
| worker timeout with no exception receipt | no | open lane receipt remains authoritative | block |
| sensitive delegated workflow `risk_tier >= R2` | yes with approval | delegation chain evidence, approval record | block |

## 5. Tool Execution Decision Table

| Condition | Tool call allowed | Required artifacts | Outcome if absent |
|---|---|---|---|
| tool is read-only and normalized contract exists | yes | tool contract, trace span | block if contract missing |
| tool is mutating and normalized contract exists | yes with approval and rollback posture | tool contract, approval record, rollback plan | block |
| tool contract missing | no | none | block |
| tool contract present but idempotency or retry posture missing for retryable tool | no | complete tool contract | block |
| tool returns failure and rollback posture exists | yes, enter compensation flow | incident evidence bundle, rollback execution trace | escalate if rollback fails |
| tool returns failure and rollback posture missing for mutating workflow | no | incident evidence bundle | block and escalate |

## 6. Retrieval Trust Decision Table

| Condition | Retrieval answer allowed | Required artifacts | Outcome if absent |
|---|---|---|---|
| answer is retrieval-grounded and source registry entry exists | yes | source registry ref, citation linkage, freshness posture | block if any missing |
| source requires ACL propagation | yes if ACL context present | source registry ref, ACL context, trace | block |
| source freshness posture is stale beyond allowed policy | no | explicit stale override receipt if allowed | block |
| answer cites unsupported or unregistered source | no | none | block |
| answer is non-retrieval and no external citation claim is made | yes | trace root | continue |

## 7. Memory Write Decision Table

| Condition | Memory write allowed | Required artifacts | Outcome if absent |
|---|---|---|---|
| memory class is low-risk and non-sensitive | yes in bounded scope | memory record, trace root, TTL policy | block if memory schema missing |
| memory class is sensitive | yes with approval | approval record, consent basis, TTL/deletion posture | block |
| correction or deletion requested | yes | correction/deletion ref, trace, audit chain | block |
| memory source trace is absent | no | none | block |

## 8. Rollback And Incident Decision Table

| Condition | Rollback required | Required artifacts | Outcome if absent |
|---|---|---|---|
| workflow is mutating and fails after side effect | yes | rollback posture, incident evidence bundle | block closure if missing |
| workflow is mutating and completes successfully | rollback plan must still exist | rollback/fallback plan | warn or block by tier |
| workflow is read-only and fails | no rollback | trace and failure taxonomy | continue or retry |
| recovery workflow executes | yes as core behavior | incident evidence bundle, restore trace, trust reevaluation verdict | block if incomplete |

## 9. References

1. `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
2. `docs/product/spec/release-1-closure-contract.md`
3. `docs/product/spec/release-1-control-metrics-and-gates.md`
4. `docs/product/spec/release-1-canonical-artifact-schemas.md`

-----
artifact_path: product/spec/release-1-decision-tables
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-decision-tables.md
created_at: 2026-03-16T11:35:00Z
updated_at: 2026-03-16T11:28:19.767293185Z
changelog_ref: release-1-decision-tables.changelog.jsonl
