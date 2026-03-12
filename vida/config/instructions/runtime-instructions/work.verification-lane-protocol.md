# Verification Lane Protocol

Purpose: define the canonical runtime law for authorship, coaching, verification, and proving lanes so closure does not depend on ad hoc reviewer habits.

## Core Contract

1. Verification is a first-class lane, not a narrative afterthought.
2. Consensus or coach feedback does not replace independent verification.
3. When route law requires separated authorship and verification, the runtime must preserve that separation explicitly.

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

## Required Verification Inputs

Each verification lane must have:

1. explicit proof target
2. explicit verification command or proving surface
3. evidence references
4. bounded acceptance criteria
5. blocker rules

## Closure Rule

No routed work is closure-ready when:

1. required proof is absent,
2. proof exists only in chat,
3. coach feedback exists but independent verification is still required,
4. the verifier cannot inspect the author output contract or proof boundary.

## Runtime Integration

Runtime surfaces that may carry verification state include:

1. `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md`
2. `vida/config/instructions/diagnostic-instructions/evaluation.product-proving-pack-scaffold-contract.md`
3. `docs/process/framework-source-lineage-index.md`

## Route Law Rule

If route metadata requires `independent_verification_required`:

1. verification must be selected before closure,
2. absence of an eligible verifier must be recorded explicitly as a blocker or override receipt,
3. local closure without that proof is protocol-invalid.

## References

1. `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
2. `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`
3. `vida/config/instructions/diagnostic-instructions/evaluation.product-proving-pack-scaffold-contract.md`
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
updated_at: '2026-03-11T13:04:21+02:00'
changelog_ref: work.verification-lane-protocol.changelog.jsonl
