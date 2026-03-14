# Verification Lane Protocol

Purpose: define the canonical runtime law for authorship, coaching, verification, and proving lanes so closure does not depend on ad hoc reviewer habits.

## Core Contract

1. Verification is a first-class lane, not a narrative afterthought.
2. Consensus or coach feedback does not replace independent verification.
3. When route law requires separated authorship and verification, the runtime must preserve that separation explicitly.
4. For code-shaped implementation or patch work, formative coach review must run before independent verification when an eligible coach lane exists.
5. Single-verifier closure is lawful only when verifier independence is explicit and receipt-visible.

## Canonical Lane Roles

1. `author`
   - produces the primary implementation or artifact
2. `coach`
   - provides bounded review or formative critique
3. `verifier`
   - performs independent validation against the stated proof contract
4. `prover`
   - prepares reusable proving evidence or proving-pack execution when required

Rules:

1. `coach` is not a substitute for `verifier`,
2. `verifier` should differ from the `author` lane when an eligible verifier exists,
3. proof burden scales with route law, risk, and task class.
4. when the verifier cannot differ from the author lane, closure requires an explicit override or `no_eligible_verifier` receipt; same-lane review alone is not independence.

## Coach-First Rule For Code Work

For implementation-shaped or patch-shaped work:

1. `coach` is the canonical pre-verification lane for spec conformance, bounded critique, and rework shaping,
2. `verifier` should validate only after coach review has either:
   - produced accepted feedback,
   - produced explicit no-change confirmation,
   - or been explicitly bypassed by blocker or override receipt,
3. the runtime must not treat direct author-to-verifier routing as the default path when coach is lawful and eligible,
4. if coach is unavailable, the absence must be recorded explicitly before verification proceeds.

## Required Verification Inputs

Each verification lane must have:

1. explicit proof target
2. explicit verification command or proving surface
3. evidence references
4. bounded acceptance criteria
5. blocker rules
6. verifier-independence basis:
   - distinct verifier lane identity,
   - or explicit override receipt,
   - or explicit `no_eligible_verifier` blocker/exception receipt
7. `review_pool` / merge-checkpoint metadata when the task participates in sibling verification merge.

## Verifier Independence Receipt

When closure depends on one verifier result, the runtime must preserve a compact verifier-independence receipt.

Minimum fields:

1. `author_lane_id`
2. `verifier_lane_id`
3. `independence_verdict`
4. `basis`
5. `override_receipt` when independence is not naturally satisfied

Rules:

1. `independence_verdict=independent` requires that the verifier lane is distinct from the author lane and can inspect the proof target without relying on the author's hidden reasoning.
2. `independence_verdict=override` is lawful only when the active route or human approval path explicitly allows same-lane or degraded verification.
3. Missing verifier-independence receipt keeps the task out of closure-ready state even if the verification command passed.
4. Review-pool members must carry the same receipt before they are admitted to merge admissibility.

## Closure Rule

No routed work is closure-ready when:

1. required proof is absent,
2. proof exists only in chat,
3. coach feedback exists but independent verification is still required,
4. the verifier cannot inspect the author output contract or proof boundary,
5. code-shaped work required coach review but no accepted coach result, explicit no-change result, or recorded bypass exists.
6. a required single-verifier result lacks a verifier-independence receipt,
7. a review-pool member lacks merge-checkpoint metadata or admissible independence proof.

## Runtime Integration

Runtime surfaces that may carry verification state include:

1. `runtime-instructions/core.run-graph-protocol`
2. `diagnostic-instructions/evaluation.product-proving-pack-scaffold-contract`
3. `docs/process/framework-source-lineage-index.md`

## Route Law Rule

If route metadata requires `independent_verification_required`:

1. verification must be selected before closure,
2. absence of an eligible verifier must be recorded explicitly as a blocker or override receipt,
3. local closure without that proof is protocol-invalid.
4. for `required_results=1`, admissibility still requires the verifier-independence receipt; “single verifier” does not mean “independence optional”.

## References

1. `instruction-contracts/core.orchestration-protocol`
2. `instruction-contracts/core.agent-system-protocol`
3. `diagnostic-instructions/evaluation.product-proving-pack-scaffold-contract`
4. `docs/process/framework-source-lineage-index.md`

-----
artifact_path: config/runtime-instructions/verification-lane.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md
created_at: '2026-03-10T15:05:00+02:00'
updated_at: '2026-03-13T07:14:58+02:00'
changelog_ref: work.verification-lane-protocol.changelog.jsonl
