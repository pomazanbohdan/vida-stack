# Verification Lane Runtime Capsule

Purpose: provide a compact runtime-facing projection of the highest-frequency verification-lane law for routine delegated closure and proof gating.

Boundary rule:

1. this file is a compact projection, not the owner of verification-lane law,
2. the canonical owner remains `runtime-instructions/work.verification-lane-protocol`,
3. use the owner file when independence overrides, review-pool merge admissibility, or proving-pack edge cases are not resolved by this capsule.

## Always-Keep-Visible

1. whether coach review is required before verification,
2. whether verifier independence is required,
3. current proof target and verification command,
4. whether a lawful verifier distinct from the author exists,
5. whether closure is blocked by missing coach, verifier, or independence receipt.

## High-Frequency Laws

1. verification is a first-class lane, not an afterthought,
2. coach feedback does not replace independent verification,
3. for code-shaped work, coach review runs before independent verification when an eligible coach exists,
4. same-lane verification is not independent unless an explicit override or `no_eligible_verifier` receipt exists,
5. closure is invalid when proof exists only in chat, required coach review is missing, or verifier independence is still implicit,
6. single-verifier closure still requires a verifier-independence receipt when route law requires independence.

## Escalate To Owner File When

1. coach bypass or verifier override legality is disputed,
2. review-pool merge admissibility is active,
3. proving-pack or reusable proving evidence is required,
4. verifier independence cannot be established cleanly,
5. a framework/protocol mutation may change verification-lane law itself.

-----
artifact_path: config/runtime-instructions/verification-lane-runtime-capsule
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.verification-lane-runtime-capsule.md
created_at: '2026-03-14T00:35:00+02:00'
updated_at: '2026-03-14T00:35:00+02:00'
changelog_ref: work.verification-lane-runtime-capsule.changelog.jsonl
