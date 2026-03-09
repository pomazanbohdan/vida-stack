# VIDA Verification Merge Law

Status: draft `v1` bounded future artifact

Revision: `2026-03-09`

Purpose: define the future lawful aggregation surface for parallel or multi-verifier verification without collapsing verifier independence into opaque consensus heuristics.

## 1. Scope

This artifact defines:

1. merge policy vocabulary for verification results,
2. admissibility and aggregation boundaries,
3. independence preservation,
4. fallback to manual reconcile.

It does not define:

1. a vendor ensemble runtime,
2. implicit weighted scoring hidden from receipts,
3. replacement of verification with coach review,
4. automatic task closure from merge output alone.

## 2. Candidate Merge Policies

Future verification aggregation should remain explicit and receipt-backed.

Initial candidate policies:

1. `all_pass`
2. `quorum_pass`
3. `first_strong_fail`
4. `manual_reconcile`
5. `best_evidence_wins` only when explicitly lawful for a bounded domain

## 3. Policy Semantics

### 3.1 all_pass

All required independent verifiers must pass before the aggregate verdict can pass.

### 3.2 quorum_pass

A configured quorum may pass only when:

1. verifier independence is satisfied,
2. proof-category coverage is satisfied,
3. no blocking fail rule has fired.

### 3.3 first_strong_fail

A configured blocking failure may short-circuit the aggregate verdict to failed.

Rule:

1. short-circuiting must be explicit in policy,
2. the blocking failure class must be receipt-visible.

### 3.4 manual_reconcile

When verifier outputs conflict materially or admissibility is uncertain, aggregation must stop at a manual reconcile boundary.

## 4. Admissibility

Merge may occur only when:

1. verifier independence holds,
2. minimum verifier count holds,
3. proof-category coverage holds,
4. result schema compatibility holds.

If any admissibility rule fails, the system must not emit a normal merged pass.

## 5. Mapping To Existing VIDA Surfaces

1. `verification_lifecycle` -> `aggregation_pending`, `passed`, `failed`, `inconclusive`
2. `receipt-proof-taxonomy` -> `verification_partial_receipt`, `verification_aggregate_receipt`
3. `external-pattern-borrow-map` -> Elsa merge modes and future parallel verification direction
4. `canonical-machine-map` -> explicit aggregation policy boundary

## 6. Invariants

1. `coach` remains distinct from `verification`
2. merged verification does not erase individual verifier receipts
3. verification pass still does not close the task by itself
4. policy must be inspectable and receipt-backed
5. fallback to `manual_reconcile` is preferred over silent heuristic merge when evidence conflicts materially

-----
artifact_path: product/spec/verification-merge-law
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/product/spec/verification-merge-law.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-09T20:28:59+02:00
changelog_ref: verification-merge-law.changelog.jsonl
