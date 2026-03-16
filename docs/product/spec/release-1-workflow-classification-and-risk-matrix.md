# Release 1 Workflow Classification And Risk Matrix

Status: active Release-1 control law

Purpose: define the canonical workflow classes, risk tiers, approval posture, and lifecycle variants used by `Release 1` so safety, proof, and closure rules apply to concrete workflow categories rather than broad architecture prose.

## 1. Scope

This matrix defines:

1. the workflow classes that Release 1 may support,
2. the risk-tier model used by closure gates,
3. which controls are mandatory per class,
4. which classes are in-scope for Release 1 support,
5. the lifecycle variants required for approval, rollback, and incident handling.

This matrix does not define:

1. detailed metric thresholds,
2. detailed artifact field schemas,
3. Release-2 workflow expansion.

## 2. Risk Tiers

Use these tiers:

1. `R0`
   - read-only, no external side effect, no sensitive data mutation
2. `R1`
   - bounded internal mutation or low-impact operator-visible side effect
3. `R2`
   - sensitive read, approval-relevant state change, or broad delegated action
4. `R3`
   - external write, irreversible or user-affecting side effect, or privileged action
5. `R4`
   - critical or multi-system side effect with safety, compliance, or material business impact

Tier rule:

1. controls only increase as risk rises,
2. Release 1 may define `R4` architecture law without claiming broad `R4` workflow support.

## 3. Workflow Class Matrix

| Workflow class | Description | Default risk tier | Release-1 support posture | Mandatory controls | Approval posture | Rollback posture |
|---|---|---|---|---|---|---|
| `informational_answer` | answer from canon or internal state with no tool call and no side effect | `R0` | supported | trace root, response provenance, policy check | none by default | not applicable |
| `retrieval_grounded_answer` | answer uses retrieval, freshness, or citation-sensitive evidence | `R1` | supported | source registry, freshness posture, citation linkage, ACL propagation, trace spans | optional unless data sensitivity elevates | not applicable |
| `documentation_mutation` | changes docs/specs/process artifacts inside project boundary | `R1` | supported | trace, proof artifact, canonical mutation law, rollback via git/runtime-safe path | optional by default, required for protected canon surfaces | required |
| `internal_state_mutation` | mutates runtime state, activation state, registry state, or receipts | `R2` | supported in bounded operator paths | trace, actor identity, policy decision, receipt evidence, restore path | required when affecting shared/project runtime truth | required |
| `delegated_development_packet` | launches or advances implementer/coach/verifier packets | `R2` | supported | chain-of-delegation, lane receipts, exception-path gate, proof linkage | required for sensitive or write-capable packets | required for supersession/takeover |
| `tool_assisted_read` | uses external or internal tool in read-only posture | `R1` | supported | normalized tool contract, auth mode, trace spans, retry posture | optional unless data sensitivity elevates | not applicable |
| `tool_assisted_write` | uses any tool that causes project or external mutation | `R3` | architecture-supported, bounded Release-1 support only | tool contract, approval, idempotency class, rollback/fallback, incident evidence bundle | required | required |
| `memory_write` | writes durable memory/knowledge state | `R2` | bounded support only | memory class, consent/policy posture, TTL/deletion contract, trace | required for sensitive memory classes | required |
| `identity_or_policy_change` | changes auth, approval, policy, permissions, or delegation state | `R3` | architecture-supported, tightly bounded in Release 1 | actor identity, approval, audit chain, rollback, incident proof | required | required |
| `incident_response_or_recovery` | restore/reconcile, rollback, recovery, or trust re-establishment action | `R3` | supported for Release-1 closure | incident evidence bundle, root cause record, rollback execution trace, closure proof | required | intrinsic |

## 4. Supported Release-1 Surface

Release 1 explicitly supports these classes as normal operating surface:

1. `informational_answer`
2. `retrieval_grounded_answer`
3. `documentation_mutation`
4. `internal_state_mutation`
5. `delegated_development_packet`
6. `tool_assisted_read`
7. `incident_response_or_recovery`

Release 1 supports these only in tightly bounded, explicit operator/admin surfaces:

1. `tool_assisted_write`
2. `memory_write`
3. `identity_or_policy_change`

Support rule:

1. no workflow class is supported implicitly,
2. any class not listed here is out of Release-1 scope.

## 5. Control Requirements By Tier

| Control | R0 | R1 | R2 | R3 | R4 |
|---|---|---|---|---|---|
| Root trace and span linkage | required | required | required | required | required |
| Policy decision artifact | optional | required when data-sensitive | required | required | required |
| Citation/freshness contract | optional | required for retrieval | required if retrieval-backed | required if retrieval-backed | required if retrieval-backed |
| Approval record | none | optional | required for sensitive actions | required | required |
| Chain-of-delegation evidence | none | optional | required when delegated | required | required |
| Rollback/fallback plan | none | required for mutation | required | required | required |
| Incident evidence bundle | none | optional | required on failure | required | required |
| Human review gate | none | optional | conditional | required | required |

## 6. Lifecycle Variants

### 6.1 Base Lifecycle

All supported workflows bind to:

1. `RECEIVED`
2. `AUTHORIZED`
3. `CLASSIFIED`
4. `ROUTED`
5. `PLANNED`
6. `EXECUTION_PREPARATION`
7. `EXECUTING`
8. `VALIDATING`
9. `COMPLETED | FAILED | CANCELLED | ESCALATED`

### 6.2 Approval-Sensitive Lifecycle

`R2+` workflows with approval needs also require:

1. `WAITING_FOR_APPROVAL`
2. `APPROVED | DENIED`
3. `RESUMED`

### 6.3 Tool/Side-Effect Lifecycle

Tool-using workflows also require:

1. `WAITING_FOR_TOOL`
2. `TOOL_RESULT_CAPTURED`
3. `COMPENSATING_ACTION_REQUIRED` when failure posture says so

### 6.4 Incident/Recovery Lifecycle

Recovery workflows also require:

1. `INCIDENT_DECLARED`
2. `ROLLBACK_OR_RESTORE_RUNNING`
3. `TRUST_REEVALUATION`
4. `RECOVERED | ESCALATED`

## 7. Closure Implications

Release 1 may not claim closure unless:

1. each supported workflow class has an explicit control posture,
2. each supported `R2+` class has approval and rollback law,
3. each supported retrieval-grounded class has citation/freshness law,
4. each supported tool-using class has a normalized tool-contract posture,
5. current-state evidence does not claim support broader than this matrix.

## 8. References

1. Airtable `Vida` base, `Spec` table, refreshed `2026-03-16`
2. `docs/product/spec/release-1-plan.md`
3. `docs/product/spec/release-1-closure-contract.md`
4. `docs/product/spec/release-1-control-metrics-and-gates.md`
5. `docs/product/spec/release-1-canonical-artifact-schemas.md`

-----
artifact_path: product/spec/release-1-workflow-classification-and-risk-matrix
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-workflow-classification-and-risk-matrix.md
created_at: 2026-03-16T11:16:55.790713272Z
updated_at: 2026-03-16T11:22:13.625174455Z
changelog_ref: release-1-workflow-classification-and-risk-matrix.changelog.jsonl
