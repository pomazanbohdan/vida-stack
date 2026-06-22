# LDRK Baseline Deletion Candidates

Status: generated baseline artifact for TaskFlow task `ldr-001`.

## Candidate Classes

1. Duplicate status/verdict classifiers should move behind `CompletionOutcome`.
2. Direct runtime artifact writes should move behind `VidaCommandEnvelope` and `OperationalJournal`.
3. Repeated command-specific flags should become global context flags or operation payload fields.
4. Legacy command aliases should remain adapter-only until the LDRK CLI reduction slice removes them.

## Repeated Flag Candidates

No repeated `Arg::new` option names found by the lexical baseline.

-----
artifact_path: product/spec/ldrk-baseline/deletion-candidates
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-06-22
schema_version: 1
status: generated
source_path: docs/product/spec/ldrk-baseline/deletion-candidates.md
created_at: 2026-06-22T00:00:00Z
updated_at: 2026-06-22T00:00:00Z
changelog_ref: ldrk-baseline/deletion-candidates.changelog.jsonl
