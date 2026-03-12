# Verification Merge Protocol

Purpose: define the canonical executable admissibility and merge surface for parallel or multi-verifier results in the `taskflow` runtime.

## Core Contract

1. Verification merge must remain explicit, inspectable, and receipt-visible.
2. Merge must not collapse `coach` into `verification`.
3. Inadmissible verifier sets must fail to `manual_reconcile`, not silently pass.

## Canonical Runtime Surfaces

1. `taskflow-v0/src/gates/verification_merge.nim`
2. `taskflow-v0 verification admissibility <required_results> <results_json_file|-> [required_categories_csv] [--json]`
3. `taskflow-v0 verification merge <policy> <required_results> <results_json_file|-> [required_categories_csv] [quorum] [--json]`
4. `docs/product/spec/verification-merge-law.md`
5. `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md`

## Admissibility Rule

Merge is lawful only when:

1. required verifier count is satisfied,
2. verifier independence is explicit,
3. result schema compatibility holds,
4. required proof-category coverage holds,
5. duplicate verifier identities do not invalidate the merge set.

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
updated_at: '2026-03-11T13:04:25+02:00'
changelog_ref: work.verification-merge-protocol.changelog.jsonl
