# Release 1 Closure Contract

Status: active Release-1 control law

Purpose: define the non-negotiable closure contract for `Release 1` so implementation, proof, risk acceptance, and release admission do not drift between architecture intent and runtime reality.

## 1. Scope

This contract defines:

1. the exact Release-1 closure gate,
2. the minimum evidence required for `implemented`, `proven`, and `risk-accepted` states,
3. the classes of gap that are never silently tolerated,
4. who may accept bounded release risk and under what form,
5. how the closure contract binds the Release-1 capability matrix, seam map, and current-state checkpoint.

This contract does not define:

1. detailed runtime-family implementation law,
2. detailed workflow risk classes,
3. detailed metric formulas,
4. Release-2 readiness.

## 2. Release-1 Closure Identity

Release 1 closes only when all of the following are true:

1. the active owner surface remains `release-1-plan.md`,
2. the `TaskFlow -> DocFlow -> Release 1 closure` seam is explicit and green enough,
3. the supported workflow classes are explicitly named and bounded,
4. mandatory P0 production-baseline tracks are green enough for each supported risky workflow class,
5. any still-open P1 control-maturity track is explicitly bounded and explicitly risk-accepted or explicitly out of the supported workflow surface,
6. no required proof depends on hidden transcript state or operator memory,
7. all closure evidence exists as canonical artifacts or receipts.

Compact rule:

1. Release 1 is closed only by explicit owner law, explicit proof, and explicit admission.

## 3. Release-1 Status States

Use these states exactly:

1. `implemented`
   - the bounded code/document/runtime behavior exists and is wired into the active owner surface
2. `proven`
   - the bounded behavior has required evidence, checks, and operator-proof coverage
3. `risk_accepted`
   - a bounded gap remains open, but a named owner has accepted it explicitly for a bounded scope and bounded time
4. `blocked`
   - a required closure condition is absent or contradicted
5. `closed`
   - all mandatory gates for the bounded supported workflow surface are satisfied

Status rule:

1. `implemented` is not enough for release admission,
2. `proven` is not enough if required P0 tracks are still absent for supported risky workflows,
3. `risk_accepted` is allowed only as an explicit exception path,
4. `closed` is the only state that permits Release-1 admission.

## 4. Definition Of Done

### 4.1 Implemented

A bounded Release-1 slice is `implemented` only when all are true:

1. owner docs are updated,
2. code/runtime surfaces exist in the bounded owner files,
3. required operator surfaces are present,
4. required receipts/artifacts can be materialized,
5. no hidden manual step is required to make the slice appear complete.

### 4.2 Proven

A bounded Release-1 slice is `proven` only when all are true:

1. required `DocFlow` checks pass,
2. required runtime/operator proofs exist,
3. seam-facing behavior is exercised where the slice claims seam relevance,
4. trace/evidence artifacts exist for risky or side-effecting paths in scope,
5. current-state and capability/seam surfaces can explain the actual posture without contradiction.

### 4.3 Risk Accepted

A bounded slice may be `risk_accepted` only when all are true:

1. the remaining gap is explicitly named,
2. the affected workflow classes are explicitly bounded,
3. the acceptance owner is explicitly named,
4. the compensating controls are explicitly recorded,
5. an expiry/revisit condition is explicitly recorded,
6. the gap is not one of the non-waivable blockers listed below.

### 4.4 Closed

Release 1 is `closed` only when:

1. all supported workflow classes have a lawful closure posture,
2. all non-waivable blockers are absent,
3. all required P0 tracks are proven for supported risky workflows,
4. all open P1 tracks are either proven or explicitly risk-accepted,
5. final closure admission is explicit and receipt-backed.

## 5. Non-Waivable Blockers

The following may not be silently waived for Release-1 closure:

1. no explicit supported workflow-class list,
2. missing `TaskFlow -> DocFlow` seam proof,
3. missing trace/evidence linkage for supported side-effecting workflows,
4. missing approval or delegation audit chain for supported sensitive workflows,
5. missing rollback/fallback posture for supported write-capable workflows,
6. missing citation/freshness contract for supported retrieval-grounded workflows,
7. missing tool contract normalization for supported tool-using workflows,
8. hidden exception paths that bypass receipt-governed ownership,
9. owner-doc contradiction between plan, matrix, seam map, and current-state checkpoint.

## 6. Risk Acceptance Law

Risk acceptance is lawful only when recorded as a bounded artifact with:

1. `risk_id`
2. `release_scope`
3. `affected_workflow_classes`
4. `affected_tracks`
5. `reason`
6. `compensating_controls`
7. `acceptance_owner`
8. `accepted_at`
9. `review_by`
10. `closure_target`

Risk acceptance rule:

1. P0 production-baseline gaps for a supported risky workflow class require explicit named acceptance,
2. no acceptance may widen the supported workflow surface implicitly,
3. acceptance expires when the bounded review condition is reached,
4. acceptance is invalid if the compensating controls are not themselves proven.

## 7. Closure Evidence Bundle

The Release-1 closure bundle must contain or reference all of:

1. active owner plan
2. current capability matrix
3. current seam map
4. current implementation checkpoint
5. supported workflow-class and risk matrix
6. control metrics and gate verdicts
7. canonical artifact-schema contract
8. runtime readiness and proof receipts
9. trace/evidence exports for supported risky workflow classes
10. explicit closure admission verdict

Evidence rule:

1. the closure bundle must be reconstructible from canonical artifacts alone,
2. chat history is never required evidence.

## 8. Forbidden Closure Shortcuts

Release 1 must not close by:

1. “the architecture is obvious” without proof artifacts,
2. “the runtime mostly works” without supported workflow classes,
3. “the metrics are implied” without explicit gate definitions,
4. “risk is acceptable” without explicit bounded acceptance,
5. “the orchestrator can compensate manually” for missing runtime governance,
6. “the docs say it should” when current-state evidence says otherwise.

## 9. Owner Links

This contract binds directly to:

1. `docs/product/spec/release-1-plan.md`
2. `docs/product/spec/release-1-capability-matrix.md`
3. `docs/product/spec/release-1-seam-map.md`
4. `docs/product/spec/release-1-current-state.md`
5. `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
6. `docs/product/spec/release-1-control-metrics-and-gates.md`
7. `docs/product/spec/release-1-canonical-artifact-schemas.md`
8. `docs/product/spec/release-1-decision-tables.md`
9. `docs/product/spec/release-1-state-machine-specs.md`
10. `docs/product/spec/release-1-error-and-exception-taxonomy.md`
11. `docs/product/spec/release-1-proof-scenario-catalog.md`
12. `docs/product/spec/release-1-schema-versioning-and-compatibility-law.md`

## 10. References

1. Airtable `Vida` base, `Spec` table, refreshed `2026-03-16`
2. `docs/product/spec/release-1-plan.md`
3. `docs/product/spec/release-1-capability-matrix.md`
4. `docs/product/spec/release-1-seam-map.md`
5. `docs/product/spec/release-1-current-state.md`

-----
artifact_path: product/spec/release-1-closure-contract
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-closure-contract.md
created_at: 2026-03-16T11:16:55.780492225Z
updated_at: 2026-03-16T11:28:28.647878266Z
changelog_ref: release-1-closure-contract.changelog.jsonl
