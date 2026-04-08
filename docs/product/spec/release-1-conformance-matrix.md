# Release 1 Conformance Matrix

Status: active Release-1 implementation law (runtime checkpoint refreshed `2026-04-07`)

Purpose: bind Release-1 control specs and adjacent runtime-law specs to code targets, proof targets, and observed local implementation posture so implementation progress can be judged against one explicit conformance table.

## 1. Scope

This matrix maps:

1. control docs,
2. adjacent runtime-law docs that materially change Release-1 requirements,
3. target code domains,
4. operator surfaces,
5. proof surfaces,
6. current conformance posture.

## 2. Matrix

| Control surface | Primary code target | Local code evidence | Operator/proof surface | Current posture | Main next move |
|---|---|---|---|---|---|
| `compiled-autonomous-delivery-runtime-architecture.md` | workspace boundary between `crates/vida/**`, `crates/taskflow-*/**`, and `crates/docflow-*/**` | runtime-family split exists, but launcher still owns large parts of execution, activation, bundle, closure glue, and carrier selection; neutral `carrier_runtime` / `runtime_assignment` contracts exist, yet legacy `codex_*` aliases are still mirrored and consumed on bridge paths | architecture review, `vida status`, `vida doctor`, `vida taskflow consume` | `spec_complete / code_bridge_heavy / proof_partial` | thin the shell, burn down alias reads, and move execution/closure truth into family-owned modules |
| `bootstrap-carriers-and-project-activator-model.md` | `crates/vida/src/project_activator_surface.rs`, `crates/vida/src/init_surfaces.rs`, `docs/framework/templates/vida.config.yaml.template` | multi-system bootstrap law is documented, but scaffold, generated docs, fallback materialization, and readiness checks remain codex-first | `vida project-activator`, bootstrap/init docs, host template materialization | `spec_complete / code_partial / proof_partial` | make host-system selection, docs, and materialization generic across configured carriers |
| `project-activation-and-configurator-model.md` | `crates/vida/src/project_activator_surface.rs`, `crates/vida/src/init_surfaces.rs`, `crates/taskflow-state-surreal/**` | activation truth is persisted and read-back verified, but the captured truth still originates from `vida.config.yaml`, compiled launcher bundle, and `.vida/**` projections rather than one DB-native configurator service | `vida project-activator`, activation projections, status surfaces | `spec_complete / code_partial / proof_partial` | make DB-first activation/configurator flow primary and demote YAML/filesystem to import/export projections |
| `operational-state-and-synchronization-model.md` | `crates/vida/src/state_store.rs`, `crates/taskflow-state-fs/**`, `crates/taskflow-state-surreal/**` | DB store plus snapshot import/export/replace exist, but snapshot bridge still dominates and reactive sync/reconcile/projector ownership is absent | task snapshot bridge, state export/import proofs | `spec_complete / code_partial / proof_partial` | formalize projector/checkpoint contracts, explicit sync/reconcile lifecycle, and keep filesystem/Git as projection and lineage only |
| `compiled-runtime-bundle-contract.md` + `taskflow-protocol-runtime-binding-model.md` | `crates/vida/src/taskflow_runtime_bundle.rs`, `crates/vida/src/taskflow_protocol_binding.rs` | bundle-check and protocol-binding checks exist, and neutral runtime identity already emits `carrier_runtime` plus `runtime_assignment`, but legacy `codex_multi_agent` / `codex_runtime_assignment` aliases are still mirrored and still consumed by some launcher paths | bundle check, protocol-binding sync/check, init surfaces | `spec_complete / code_partial / proof_partial` | remove alias mirroring/reads, then move bundle/protocol-binding truth below the launcher |
| `projection-listener-checkpoint-model.md` + `checkpoint-commit-and-replay-model.md` | `crates/vida/src/state_store.rs`, `crates/vida/src/taskflow_run_graph.rs` | `resumability_capsule`, checkpoint, recovery, and gate summaries are real, but they persist only latest resumability state; no projection-checkpoint artifact, replay lineage, or fork scope exists | `vida taskflow recovery`, run-graph proofs, recovery/checkpoint summaries | `spec_complete / code_partial / proof_partial` | add family-owned checkpoint/replay interfaces, projection-checkpoint artifacts, and append-evidence lineage contracts |
| `release-1-closure-contract.md` | TaskFlow closure admission domain | closure admission exists in `main.rs`, `taskflow_consume.rs`, `taskflow_consume_bundle.rs`, and `taskflow_consume_resume.rs`; `status` and `doctor` expose proof-backed trace, tool-contract, retrieval-trust, approval/delegation, and failure-control posture; persisted-final-snapshot selection is regression-covered and final admission was proven before post-release cleanup | closure receipts, seam proof, persisted-final-snapshot recovery proofs | `spec_complete / code_complete_for_release1 / proof_complete_for_release1` | keep seam receipt requirements fail-closed and treat further shell/owner extraction as post-release work |
| `release-1-workflow-classification-and-risk-matrix.md` | shared runtime contract module plus TaskFlow enforcement | shared Rust `workflow_class` and `risk_tier` enums now exist in `crates/vida/src/release1_contracts.rs`, and root operator surfaces emit explicit `null` placeholders when no bounded classifier is available yet | workflow classification outputs, operator risk surfaces | `spec_complete / code_partial / proof_partial` | replace placeholder `null` emission with lawful runtime classification on the remaining in-scope operator surfaces |
| `release-1-control-metrics-and-gates.md` | evaluation and metrics services | fail-closed gate checks exist across `status`, `doctor`, and consume flows for canonical tool-contract, trace, retrieval-trust, approval/delegation, and failure-control evidence; this is sufficient for Release 1 closure even though richer metric/threshold artifacts remain future work | status/doctor/eval dashboards, control-track smoke proofs | `spec_complete / code_complete_for_release1 / proof_complete_for_release1` | materialize richer threshold/metric artifacts only as post-release extension work |
| `release-1-canonical-artifact-schemas.md` | contracts/state crates | operator JSON and closure payloads exist, but canonical structs for trace/policy/approval/tool/eval/incident/memory artifacts are not implemented; `domain_event` and `projection_checkpoint_record` are not present if event-state is later adopted | receipts, trace exports, evaluation outputs | `spec_complete / code_sparse / proof_sparse` | implement canonical structs and extend the schema set if bounded event-state becomes active law |
| `release-1-decision-tables.md` | policy and gate services | fail-closed gating exists across consume, protocol-binding, status, and doctor paths; `status` now emits a canonical tool-contract summary and `taskflow_consume` enforces approval/delegation evidence gates, but logic is still duplicated and shell-local | policy, approval, tool, and retrieval execution paths | `spec_complete / code_partial / proof_partial` | convert prose decisions into one family-owned admission/policy layer |
| `release-1-state-machine-specs.md` | TaskFlow lifecycle modules | run-graph, approval wait, lane state, and recovery statuses exist, but transition law is still spread across launcher files and latest receipts are upserted by run instead of appended as transition history | lane, approval, and recovery status outputs | `spec_complete / code_partial / proof_partial` | enforce append-evidence transition ownership and emit one lawful transition path per state change |
| `release-1-error-and-exception-taxonomy.md` | contracts/state/runtime modules | shared blocker registry canonicalization is now real in `release1_contracts.rs`, and operator-contract validation rejects non-registry blocker arrays, but some ad hoc shell-side blocker construction still remains outside the fully shared path | operator errors and blocker outputs | `spec_complete / code_partial / proof_partial` | finish burning down the remaining ad hoc blocker construction sites into the shared registry path |
| `release-1-ownership-to-code-map.md` | workspace/crate structure | `crates/vida/src/main.rs`, `crates/vida/src/state_store.rs`, `crates/vida/src/project_activator_surface.rs`, and `crates/vida/src/taskflow_run_graph.rs` still carry too much owner law | architecture review and refactor queue | `spec_complete / code_partial / proof_partial` | move shell-owned law to family-owned modules and keep `vida` as router/render surface |
| `release-1-proof-scenario-catalog.md` | tests/eval fixtures | smoke coverage in `crates/vida/tests/**` is substantial, the full package proof set was green for Release 1 closure, and persisted-final-snapshot recovery proofs cover failure-control evidence; some fixtures still reflect legacy naming, but no longer block Release 1 | proofcheck, full package test suite, targeted recovery/failure-control proofs | `spec_complete / code_complete_for_release1 / proof_complete_for_release1` | burn down remaining fixture legacy only as post-release hygiene |
| `release-1-schema-versioning-and-compatibility-law.md` | contract evolution logic | compatibility classification exists, but mixed-version artifact evolution and reader behavior are still narrow | migration and compatibility proofs | `spec_complete / code_partial / proof_partial` | add compatibility handling to artifact readers and cross-version fixtures |
| `release-1-runtime-enum-and-code-contracts.md` | shared enums/types | `crates/vida/src/release1_contracts.rs` now centralizes `workflow_class`, `risk_tier`, `lane_status`, `approval_status`, `gate_level`, `compatibility_class`, and canonical blocker normalization; the remaining gap is complete emission coverage across all live operator/runtime artifacts | machine-readable outputs | `spec_complete / code_partial / proof_partial` | push the shared enum/value layer through the remaining live emitters and remove residual legacy aliases/readers |
| `release-1-operator-surface-contract.md` | CLI rendering layer | `status`, `doctor`, root `vida consume`, and root `vida recovery` now route as canonical top-level surfaces while preserving TaskFlow family ownership underneath; `lane` and `approval` still lack equivalent family-owned root promotion targets and remain open as post-closure conformance gaps | `status`, `doctor`, `consume`, `lane`, `approval`, `recovery` | `spec_complete / code_partial / proof_partial` | keep the shared envelope canonical across existing surfaces, then close or explicitly defer the remaining `lane`/`approval` root-surface promotion work |
| `release-1-unsupported-surface-contract.md` | routing/gate logic | `taskflow_proxy.rs` already fails closed on unsupported subcommands, but reserved boundaries are not fully surfaced across all entrypoints | operator denial messages | `spec_complete / code_partial / proof_partial` | enforce explicit unsupported boundaries across root and TaskFlow proxy routing |
| `release-1-fixture-and-golden-data-contract.md` | tests/eval datasets | tests assert many JSON payload details, but canonical shared fixtures/goldens are not yet published and current fixtures still pin codex-era schema names | golden fixtures and benchmark assets | `spec_complete / code_sparse / proof_sparse` | add canonical fixture set for carrier-neutral contracts, scenarios, and compatibility proofs |
| `release-1-risk-acceptance-register.md` | release governance surface | the register exists as canon; Release 1 closed without an open blocking risk entry, and the main remaining caveat is operational datastore hygiene during local manual cleanup rather than an unclosed product/runtime risk | risk report / closure bundle | `spec_complete / process_complete_for_release1 / proof_complete_for_release1` | record new risks only when a post-release change opens fresh runtime uncertainty |

## 3. Reading Rule

1. A surface is not conformant until code target, operator surface, and proof surface all exist.
2. `code_bridge_heavy` means behavior exists but still sits mainly in launcher-owned or donor-shaped surfaces.
3. `code_sparse` means only fragments exist or the shared contract has not been materially implemented yet.
4. `proof_partial` means bounded smoke or operator evidence exists, but the surface is not yet closure-proven as an owner-complete slice.

## 4. References

1. `docs/product/spec/release-1-plan.md`
2. `docs/product/spec/release-1-ownership-to-code-map.md`
3. `docs/product/spec/release-1-proof-scenario-catalog.md`
4. `docs/product/spec/project-activation-and-configurator-model.md`
5. `docs/product/spec/operational-state-and-synchronization-model.md`
6. `docs/product/spec/checkpoint-commit-and-replay-model.md`
7. `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`

-----
artifact_path: product/spec/release-1-conformance-matrix
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-07
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-conformance-matrix.md
created_at: 2026-03-16T11:45:00Z
updated_at: 2026-04-08T07:14:07.806055101Z
changelog_ref: release-1-conformance-matrix.changelog.jsonl
