# Release 1 Control Metrics And Gates

Status: active Release-1 control law

Purpose: define the canonical control metrics, gate semantics, and minimum thresholds for `Release 1` so trust, safety, retrieval quality, approval discipline, and rollback readiness are measured through explicit release gates instead of broad intent.

## 1. Scope

This document defines:

1. the mandatory Release-1 control metrics,
2. their minimum gate semantics,
3. which metrics block risky workflow support,
4. the distinction between `block`, `warn`, and `observe`,
5. the minimum evidence window for each metric family.

This document does not define:

1. detailed storage layout for every metric event,
2. vendor-specific observability implementation,
3. Release-2 optimization targets.

## 2. Gate Levels

Use these levels:

1. `block`
   - release or workflow-class support is not allowed below threshold
2. `warn`
   - release may proceed only with explicit risk acceptance
3. `observe`
   - metric must exist, but threshold miss alone does not block Release 1

Compact rule:

1. a metric without a named gate level does not exist for Release-1 closure purposes.

## 3. Mandatory Control Metrics

| Metric | Meaning | Default gate | Minimum threshold | Evidence window |
|---|---|---|---|---|
| `trace_coverage_rate` | percent of supported workflows producing root trace plus required span linkage | `block` | `100%` for `R2+`; `>= 95%` for `R0-R1` | latest bounded release-candidate run set |
| `approval_capture_rate` | percent of approval-required actions with explicit approval artifact | `block` | `100%` | latest bounded release-candidate run set |
| `delegation_chain_coverage` | percent of delegated risky actions with explicit chain-of-delegation evidence | `block` | `100%` for delegated `R2+` workflows | latest bounded release-candidate run set |
| `tool_contract_coverage` | percent of supported tool-using workflows backed by normalized tool contracts | `block` | `100%` | active supported tool catalog |
| `citation_coverage_rate` | percent of retrieval-grounded outputs with usable source/citation linkage | `block` | `>= 98%` | latest retrieval evaluation set |
| `freshness_policy_coverage` | percent of retrieval sources/workflows with explicit freshness posture | `block` | `100%` | active source registry |
| `rollback_readiness_rate` | percent of supported mutating workflows with tested rollback/fallback posture | `block` | `100%` for `R2+` mutating classes | latest bounded recovery drill set |
| `incident_evidence_completeness` | percent of failure/incident samples producing complete incident evidence bundle | `block` | `100%` for recovery-class workflows | latest recovery/incident drill set |
| `tool_input_accuracy` | percent of evaluation samples where tool call inputs match intended action | `warn` | `>= 95%` | latest golden benchmark set |
| `tool_output_utilization` | percent of tool results correctly consumed in final workflow output | `warn` | `>= 90%` | latest golden benchmark set |
| `prompt_regression_rate` | percent of benchmark cases regressing after prompt/policy change | `block` for promoted prompt changes | `<= 2%` with `0` critical regressions | latest benchmark comparison set |
| `safety_defect_rate` | percent of evaluated samples containing critical safety failures | `block` | `0` critical defects in supported classes | latest safety suite |
| `cost_per_successful_task` | median or bounded cost for successful workflow completion | `observe` in Release 1 | explicit budget must exist per class; no unbounded cost posture allowed | latest benchmark and operator sample |
| `operator_recovery_success_rate` | percent of recovery drills that restore trusted runtime posture without manual hidden fix | `warn` | `>= 90%` | latest bounded recovery drill set |

## 4. Gate Binding By Workflow Tier

| Workflow tier | Mandatory block metrics | Mandatory warn metrics | Observe metrics |
|---|---|---|---|
| `R0` | trace coverage where applicable | none | cost per successful task |
| `R1` | trace coverage, citation/freshness when retrieval applies | tool input accuracy, tool output utilization | cost per successful task |
| `R2` | trace coverage, approval capture, delegation coverage when delegated, rollback readiness for mutating flows | tool input accuracy, tool output utilization, operator recovery success | cost per successful task |
| `R3-R4` | all block metrics applicable to the class, plus safety defect gate | all warn metrics applicable to the class | cost per successful task |

## 5. Release-1 Gate Rules

Release 1 may not claim support for a workflow class when:

1. any required `block` metric is below threshold,
2. the metric exists only as prose and not as an evaluable artifact,
3. the evidence window is stale relative to the release candidate,
4. the metric is reported globally but not attributable to the supported workflow class.

Release 1 may claim bounded support with risk acceptance only when:

1. all `block` metrics pass,
2. only `warn` metrics are below threshold,
3. the miss is explicitly recorded in bounded risk acceptance,
4. compensating controls and remediation owner are named.

## 6. Measurement Rule

All Release-1 control metrics must be:

1. attributable to a workflow class or workflow tier,
2. attributable to an evaluation set or operator sample window,
3. reproducible from canonical artifacts and traces,
4. tied to one release candidate or active operator window,
5. explainable without chat transcript reconstruction.

## 7. Minimum Control Dashboards

Release 1 must expose at least these operator-facing grouped views:

1. trust and trace dashboard
   - trace coverage
   - approval capture
   - delegation coverage
2. tool and retrieval dashboard
   - tool contract coverage
   - tool input accuracy
   - tool output utilization
   - citation coverage
   - freshness policy coverage
3. safety and recovery dashboard
   - rollback readiness
   - incident evidence completeness
   - safety defect rate
   - operator recovery success rate
4. cost and rollout dashboard
   - prompt regression rate
   - cost per successful task

## 8. References

1. Airtable `Vida` base, `Spec` table records `810-814`, refreshed `2026-03-16`
2. `docs/product/spec/release-1-plan.md`
3. `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
4. `docs/product/spec/release-1-closure-contract.md`

-----
artifact_path: product/spec/release-1-control-metrics-and-gates
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-control-metrics-and-gates.md
created_at: 2026-03-16T11:16:55.80158543Z
updated_at: 2026-03-16T11:22:17.431258378Z
changelog_ref: release-1-control-metrics-and-gates.changelog.jsonl
