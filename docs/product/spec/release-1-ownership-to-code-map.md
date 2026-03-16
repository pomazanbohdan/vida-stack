# Release 1 Ownership To Code Map

Status: active Release-1 implementation law

Purpose: bind the Release-1 owner documents and control contracts to target crate/module ownership so implementation work does not drift back into shell concentration or ambiguous ownership.

## 1. Scope

This map defines:

1. which docs own which code domains,
2. which code domains must move out of `crates/vida`,
3. where control contracts should live in code,
4. what must remain shell-only.

## 2. Ownership Matrix

| Owner doc | Primary code domain | Must live in shell | Must move below shell |
|---|---|---|---|
| `release-1-plan.md` | overall orchestration routing and migration sequencing | CLI composition only | lane law, closure law, state transitions |
| `release-1-closure-contract.md` | closure admission and risk acceptance contracts | none | TaskFlow-owned closure and admission enforcement |
| `release-1-workflow-classification-and-risk-matrix.md` | workflow classification and risk application | none | shared runtime contract module plus TaskFlow enforcement |
| `release-1-control-metrics-and-gates.md` | evaluation and release-gate computation | summary rendering only | evaluation/runtime metric services |
| `release-1-canonical-artifact-schemas.md` | canonical artifact types | format/render helpers only | contracts/state crates |
| `release-1-decision-tables.md` | policy/approval/tool/retrieval/memory/rollback decisions | none | policy and execution gate services |
| `release-1-state-machine-specs.md` | lane/approval/tool/incident/prompt FSMs | none | TaskFlow lifecycle modules and shared contracts |
| `release-1-error-and-exception-taxonomy.md` | canonical blocker/failure codes | user-facing rendering only | contracts/state/runtime modules |

## 3. Target Code Placement

1. `crates/vida/**`
   - shell-only:
   - argument parsing
   - subcommand routing
   - text/json rendering
2. `taskflow-*`
   - lane lifecycle
   - closure admission
   - run graph
   - delegated-cycle enforcement
   - execution receipts
3. `docflow-*`
   - readiness proof
   - validation outputs
   - documentation mutation/validation law
4. shared contract crates or bounded modules
   - canonical artifact schemas
   - error/blocker enums
   - decision-table application inputs/outputs

## 4. Anti-Drift Rule

The following must not remain long-term in `crates/vida`:

1. lane lifecycle truth
2. closure admission truth
3. approval truth
4. tool policy truth
5. schema truth
6. workflow risk truth

They may be rendered or routed by the shell, but not owned by it.

## 5. References

1. `docs/product/spec/release-1-plan.md`
2. `docs/product/spec/release-1-canonical-artifact-schemas.md`
3. `docs/product/spec/release-1-state-machine-specs.md`

-----
artifact_path: product/spec/release-1-ownership-to-code-map
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-03-16
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-ownership-to-code-map.md
created_at: 2026-03-16T11:35:00Z
updated_at: 2026-03-16T11:28:19.79540352Z
changelog_ref: release-1-ownership-to-code-map.changelog.jsonl
