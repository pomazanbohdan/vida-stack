# Release 1 Seam Map

Status: active product law

Purpose: define the canonical seam map for `Release 1` so the final closure path between `TaskFlow`, `DocFlow`, and Release-1 product completion is explicit, fail-closed, and proof-backed.

## 1. Scope

This seam map defines:

1. the one critical `Release 1` closure seam,
2. which runtime family owns each side of that seam,
3. what inputs cross the seam,
4. what must be proven before Release 1 can close,
5. which blocker classes keep the seam fail-closed.

This seam map does not define:

1. all `TaskFlow` runtime law,
2. all `DocFlow` layer law,
3. the full Release-1 capability ladder,
4. Release-2 embedding or reactive runtime behavior.

## 2. Seam Identity

Canonical seam:

1. `TaskFlow execution and closure authority`
2. `->`
3. `DocFlow documentation/readiness/proof authority`
4. `->`
5. `Release 1 product closure`

Compact rule:

1. `TaskFlow` remains the execution substrate,
2. `DocFlow` remains the bounded readiness/proof branch,
3. `Release 1` closes only when the seam between them is explicit and green.

## 3. Seam Status Matrix

Status markers:

1. `✅` owner law is explicit and closure-ready
2. `🟡` partially implemented or not yet proof-closed
3. `⚪` target-only

| Seam segment | Upstream owner | Downstream owner | Trigger | Required inputs | Required outputs | Law status | Implementation status | Proof status | Current blocker |
|---|---|---|---|---|---|---|---|---|---|
| Segment 1: Runtime trust handoff | `TaskFlow` Layer 9 | `DocFlow` Layers 7-8 | `TaskFlow` enters direct runtime consumption or final closure path | runtime state, compiled control/bundle state, active canonical inventory, explicit readiness branch activation | readiness verdict, blocking reasons, proof-ready documentation branch | ✅ | 🟡 | 🟡 | native Rust seam still converges while both runtime families are under active modernization |
| Segment 2: Readiness/proof return | `DocFlow` Layers 7-8 | `TaskFlow` Layer 9 closure path | `DocFlow` finishes bounded readiness/proof evaluation for the requested closure scope | canonical inventory, validation state, relation/readiness artifacts, projection parity where declared | explicit pass/block verdict consumable by `TaskFlow`; no hidden shared state | ✅ | 🟡 | 🟡 | final `docflow-rs` Layer-8-ready seam is not yet closure-proven end-to-end |
| Segment 3: Product closure admission | `TaskFlow` final closure authority | `Release 1` closure proof | `TaskFlow` receives green downstream readiness/proof and bounded restore/reconcile state | executable runtime state, downstream proof receipts, restore/reconcile discipline, operator closure evidence | Release-1 closure admission or fail-closed blocker | ✅ | ⚪ | ⚪ | Wave-5 hardening and closure-proof surfaces remain open |

Matrix reading rule:

1. read top-to-bottom as one closure chain,
2. if any segment is not green enough, Release 1 remains open,
3. no segment may be bypassed by chat-level confidence or by bridge-only assumptions.

## 4. Owner Rule

Primary owner references:

1. `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`
2. `docs/product/spec/release-1-capability-matrix.md`
3. `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`
4. `docs/product/spec/docflow-v1-runtime-modernization-plan.md`
5. `docs/product/spec/canonical-runtime-layer-matrix.md`
6. `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`
7. `docs/product/spec/release-1-wave-plan.md`

Owner rule:

1. `TaskFlow` owns execution, runtime state, and final closure authority,
2. `DocFlow` owns documentation, inventory, validation, readiness, and proof behavior,
3. `Release 1` uses the seam as a closure gate and must not invent a third hidden owner.

## 5. Trigger Rule

The seam activates only when one of the following is true:

1. `TaskFlow` enters Layer 9 direct runtime consumption,
2. `TaskFlow` evaluates final runtime-trust closure for the bounded scope,
3. Release-1 Wave 5 closure or hardening proof is being evaluated,
4. a restore/reconcile flow requires renewed readiness/proof admission before trust is restored.

Trigger rule:

1. `DocFlow` is not a hidden always-on closure owner for every runtime step,
2. the seam activates explicitly at the trust/closure boundary,
3. before that boundary, earlier `TaskFlow` waves may proceed independently when they do not require the seam.

## 6. Seam Input Contract

When the seam activates, `TaskFlow` must hand off all of:

1. bounded closure scope,
2. active runtime identity and state needed for the request,
3. compiled control/bundle identity where relevant,
4. canonical inventory and readiness scope selectors,
5. explicit request for readiness/proof evaluation,
6. any restore/reconcile context that changes trust posture.

Input rule:

1. the seam must consume explicit inputs,
2. it must not rely on hidden transcript inheritance,
3. it must not infer closure scope from operator intent alone.

## 7. Seam Output Contract

`DocFlow` must return all of:

1. explicit readiness/proof verdict,
2. explicit blocker classes when not green,
3. bounded proof artifacts or references,
4. enough machine-readable outcome for `TaskFlow` to continue or fail closed,
5. no transfer of execution ownership.

Output rule:

1. `DocFlow` may resolve readiness and proof,
2. but it must not claim top-level closure authority,
3. `TaskFlow` must consume the result explicitly rather than treating documentation state as ambient truth.

## 8. Forbidden Ownership Transfer

The seam must not permit:

1. `TaskFlow` handing execution authority to `DocFlow`,
2. `DocFlow` claiming Release-1 closure by itself,
3. Release-1 closure being inferred from readiness-only evidence without `TaskFlow` closure admission,
4. hidden shared-state shortcuts that bypass the explicit handoff/result contract.

## 9. Failure Mode And Blocker Classes

The seam fails closed when any of the following is true:

1. `TaskFlow` cannot activate the bounded `DocFlow` branch explicitly,
2. `DocFlow` cannot produce an explicit readiness/proof verdict,
3. required inventory/readiness/proof inputs are missing or mismatched,
4. restore/reconcile state is not trustworthy enough for closure,
5. operator-facing closure evidence is incomplete.

Minimum blocker families:

1. `missing_docflow_activation`
2. `missing_readiness_verdict`
3. `missing_inventory_or_projection_evidence`
4. `restore_reconcile_not_green`
5. `missing_closure_proof`

## 10. Proof Surface

The seam is considered closure-ready only when bounded proof exists across all of:

1. `TaskFlow` Layer-9 direct-consumption and closure-path proofs,
2. `DocFlow` Layer-7 readiness proof,
3. `DocFlow` Layer-8-ready seam proof sufficient for `TaskFlow` consumption,
4. explicit seam verification that `TaskFlow` consumes `DocFlow` outputs rather than bypassing them,
5. Release-1 Wave-5 closure proof.

Current proof anchors:

1. `docs/product/spec/release-1-capability-matrix.md`
2. `docs/product/spec/canonical-runtime-layer-matrix.md`
3. `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`
4. `docs/process/vida1-development-conditions.md`
5. bounded `proofcheck`, `readiness-check`, and runtime/operator proof surfaces named by the owner specs

## 11. Closure Rule

Release 1 is not closure-ready unless all are true:

1. the seam is explicit,
2. ownership remains split correctly,
3. `TaskFlow` can activate `DocFlow` at the trust/closure boundary,
4. `DocFlow` can return explicit readiness/proof outputs,
5. `TaskFlow` remains the final closure authority,
6. Wave-5 hardening proofs are green enough to trust the full chain.

-----
artifact_path: product/spec/release-1-seam-map
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/product/spec/release-1-seam-map.md
created_at: '2026-03-13T13:42:00+02:00'
updated_at: '2026-03-13T09:20:47+02:00'
changelog_ref: release-1-seam-map.changelog.jsonl
