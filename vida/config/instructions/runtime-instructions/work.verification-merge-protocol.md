# Verification Merge Protocol

Purpose: define the canonical executable admissibility and merge surface for review-pool, single-verifier, and multi-verifier results in the `taskflow` runtime.

## Core Contract

1. Verification merge must remain explicit, inspectable, and receipt-visible.
2. Merge must not collapse `coach` into `verification`.
3. Inadmissible verifier sets must fail to `manual_reconcile`, not silently pass.
4. A single verifier result still requires explicit admissibility; `required_results=1` is not an escape hatch.
5. Review-pool merge must remain sibling-bounded and checkpoint-bound.

## Canonical Runtime Surfaces

1. `taskflow-v0/src/gates/verification_merge.nim`
2. `taskflow-v0 verification admissibility <required_results> <results_json_file|-> [required_categories_csv] [--json]`
3. `taskflow-v0 verification merge <policy> <required_results> <results_json_file|-> [required_categories_csv] [quorum] [--json]`
4. `docs/product/spec/verification-merge-law.md`
5. `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md`
6. `vida/config/instructions/command-instructions/execution.implement-execution-protocol.md`

## Admissibility Rule

Merge is lawful only when:

1. required verifier count is satisfied,
2. verifier independence is explicit,
3. result schema compatibility holds,
4. required proof-category coverage holds,
5. duplicate verifier identities do not invalidate the merge set.

Single-verifier admissibility rule:

1. when `required_results=1`, the result still needs a verifier-independence receipt from `work.verification-lane-protocol.md`,
2. if independence is unavailable, merge must fail to `manual_reconcile` or an explicit override/human gate,
3. a passing command result without independence receipt is not admissible closure evidence.

## Review-Pool Merge Rule

Review-pool merge is lawful only when:

1. every member task belongs to the same declared `review_pool`,
2. every member task points to the same milestone and merge checkpoint,
3. each member task is individually verifier-ready before pool merge,
4. writable scope overlap is either absent or already serialized by prior receipt,
5. each member carries:
   - `done_verdict`
   - `stop_reason`
   - `residual_risks`
   - verifier-independence receipt
6. the pool policy is explicit (`all_pass|quorum_pass|manual_reconcile`).

## Supported Policies

1. `all_pass`
2. `quorum_pass`
3. `first_strong_fail`
4. `manual_reconcile`

## Closure Rule

Layered runtime closure may rely on merged verification only when:

1. admissibility passes,
2. merge policy is explicit,
3. per-verifier results remain inspectable,
4. merged verification does not bypass approval or final closure authority.
5. review-pool closure does not bypass per-task proof or per-task blocker visibility.
6. single-verifier closure uses the same admissibility law, with a one-result set plus independence receipt.

## References

1. `vida/config/instructions/runtime-instructions/work.human-approval-protocol.md`
2. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
3. `docs/product/spec/canonical-runtime-layer-matrix.md`

-----
artifact_path: config/runtime-instructions/verification-merge.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.verification-merge-protocol.md
created_at: '2026-03-10T16:00:00+02:00'
updated_at: '2026-03-13T07:14:58+02:00'
changelog_ref: work.verification-merge-protocol.changelog.jsonl
