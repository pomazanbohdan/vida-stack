# LDRK Baseline Deletion Candidates

Status: generated baseline artifact for TaskFlow task `ldr-001`.

## Candidate Classes

1. Duplicate status/verdict classifiers should move behind `CompletionOutcome`.
2. Direct runtime artifact writes should move behind `VidaCommandEnvelope` and `OperationalJournal`.
3. Repeated command-specific flags should become global context flags or operation payload fields.
4. Legacy command aliases should remain adapter-only until the LDRK CLI reduction slice removes them.

## Repeated Flag Candidates

| Option | Count | Disposition |
| --- | --- | --- |
| acceptance-target | 3 | globalize or move to command payload |
| acceptance-target-literals | 2 | globalize or move to command payload |
| all | 3 | globalize or move to command payload |
| artifact-refs | 3 | globalize or move to command payload |
| attempt-id | 5 | globalize or move to command payload |
| backend | 2 | globalize or move to command payload |
| basis | 3 | globalize or move to command payload |
| blocker | 2 | globalize or move to command payload |
| child-id | 2 | globalize or move to command payload |
| conflict-domain | 3 | globalize or move to command payload |
| consolidation-receipt | 5 | globalize or move to command payload |
| created-by | 3 | globalize or move to command payload |
| depends-on-id | 2 | globalize or move to command payload |
| description | 3 | globalize or move to command payload |
| dry-run | 8 | globalize or move to command payload |
| edge-type | 2 | globalize or move to command payload |
| epics | 2 | globalize or move to command payload |
| evidence | 4 | globalize or move to command payload |
| execution-mode | 3 | globalize or move to command payload |
| expect | 2 | globalize or move to command payload |
| fields | 6 | globalize or move to command payload |
| file | 3 | globalize or move to command payload |
| from-parent-id | 2 | globalize or move to command payload |
| full | 3 | globalize or move to command payload |
| include-edge-proxy | 2 | globalize or move to command payload |
| install-root | 2 | globalize or move to command payload |
| isolation | 2 | globalize or move to command payload |
| issue-type | 2 | globalize or move to command payload |
| json | 79 | globalize or move to command payload |
| labels | 3 | globalize or move to command payload |
| limit | 4 | globalize or move to command payload |
| model-profile | 2 | globalize or move to command payload |
| next-action | 2 | globalize or move to command payload |
| notes | 3 | globalize or move to command payload |
| notes-file | 2 | globalize or move to command payload |
| order-bucket | 3 | globalize or move to command payload |
| owned-path | 3 | globalize or move to command payload |
| owned-path-literal | 2 | globalize or move to command payload |
| parallel-group | 3 | globalize or move to command payload |
| parent-display-id | 2 | globalize or move to command payload |
| parent-id | 5 | globalize or move to command payload |
| path | 3 | globalize or move to command payload |
| priority | 4 | globalize or move to command payload |
| proof | 2 | globalize or move to command payload |
| proof-target | 4 | globalize or move to command payload |
| proof-target-literals | 2 | globalize or move to command payload |
| provider | 2 | globalize or move to command payload |
| query | 2 | globalize or move to command payload |
| reason | 4 | globalize or move to command payload |
| release-proof-template | 2 | globalize or move to command payload |
| render | 55 | globalize or move to command payload |
| request | 2 | globalize or move to command payload |
| result | 2 | globalize or move to command payload |
| route | 2 | globalize or move to command payload |
| scope | 6 | globalize or move to command payload |
| session-id | 2 | globalize or move to command payload |
| source-binary | 2 | globalize or move to command payload |
| stage-id | 8 | globalize or move to command payload |
| state-dir | 66 | globalize or move to command payload |
| status | 10 | globalize or move to command payload |
| summary | 5 | globalize or move to command payload |
| task-class | 2 | globalize or move to command payload |
| task-id | 33 | globalize or move to command payload |
| title | 3 | globalize or move to command payload |
| to-parent-id | 2 | globalize or move to command payload |
| type | 3 | globalize or move to command payload |
| view | 7 | globalize or move to command payload |

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
