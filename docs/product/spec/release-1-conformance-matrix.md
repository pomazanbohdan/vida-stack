# Release 1 Conformance Matrix

Status: active Release-1 implementation law

Purpose: bind Release-1 control specs to code targets, proof targets, and active status so implementation progress can be judged against one explicit conformance table.

## 1. Scope

This matrix maps:

1. control docs,
2. target code domains,
3. operator surfaces,
4. proof surfaces,
5. current conformance posture.

## 2. Matrix

| Control surface | Primary code target | Operator/proof surface | Current posture | Main next move |
|---|---|---|---|---|
| `release-1-closure-contract.md` | TaskFlow closure admission domain | closure receipts, seam proof | `spec_complete / code_partial` | move closure truth below shell |
| `release-1-workflow-classification-and-risk-matrix.md` | shared runtime contract module plus TaskFlow enforcement | workflow classification outputs | `spec_complete / code_partial` | introduce canonical workflow/risk enums |
| `release-1-control-metrics-and-gates.md` | evaluation and metrics services | status/doctor/eval dashboards | `spec_complete / code_partial` | materialize metric artifacts and gate views |
| `release-1-canonical-artifact-schemas.md` | contracts/state crates | receipts, trace exports, evaluation outputs | `spec_complete / code_partial` | implement canonical structs and serialization contracts |
| `release-1-decision-tables.md` | policy and gate services | policy/approval/tool execution paths | `spec_complete / code_partial` | convert prose decisions to runtime admission logic |
| `release-1-state-machine-specs.md` | TaskFlow lifecycle modules | lane/approval/recovery status outputs | `spec_complete / code_partial` | enforce explicit transition law |
| `release-1-error-and-exception-taxonomy.md` | contracts/state/runtime modules | operator errors and blocker outputs | `spec_complete / code_partial` | replace ad hoc error strings with canonical codes |
| `release-1-ownership-to-code-map.md` | workspace/crate structure | architecture review and refactor queue | `spec_complete / code_partial` | move shell-owned law to family-owned modules |
| `release-1-proof-scenario-catalog.md` | tests/eval fixtures | proofcheck, test suites, benchmark runs | `spec_complete / code_partial` | create canonical scenario fixtures |
| `release-1-schema-versioning-and-compatibility-law.md` | contract evolution logic | migration and compatibility proofs | `spec_complete / code_partial` | add compatibility handling to artifact readers |
| `release-1-runtime-enum-and-code-contracts.md` | shared enums/types | machine-readable outputs | `spec_complete / code_partial` | centralize canonical enum set |
| `release-1-operator-surface-contract.md` | CLI rendering layer | `status`, `doctor`, `consume`, `lane`, `approval`, `recovery` | `spec_complete / code_partial` | stabilize output payloads |
| `release-1-unsupported-surface-contract.md` | routing/gate logic | operator denial messages | `spec_complete / code_partial` | enforce explicit unsupported boundaries |
| `release-1-fixture-and-golden-data-contract.md` | tests/eval datasets | golden fixtures and benchmark assets | `spec_complete / code_partial` | add canonical fixture set |
| `release-1-risk-acceptance-register.md` | release governance surface | risk report / closure bundle | `spec_started / process_only` | populate actual open risks |

## 3. Reading Rule

1. A surface is not conformant until code target, operator surface, and proof surface all exist.
2. `spec_complete / code_partial` means implementation work is now clearly bounded.

## 4. References

1. `docs/product/spec/release-1-plan.md`
2. `docs/product/spec/release-1-ownership-to-code-map.md`
3. `docs/product/spec/release-1-proof-scenario-catalog.md`

-----
artifact_path: product/spec/release-1-conformance-matrix
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-conformance-matrix.md
created_at: 2026-03-16T11:45:00Z
updated_at: 2026-03-16T11:34:32.229739088Z
changelog_ref: release-1-conformance-matrix.changelog.jsonl
