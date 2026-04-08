# Release 1 Capability Matrix

Status: active product law

Purpose: define the canonical Release-1 capability matrix that combines the product requirements of `Release 1` with the two bounded runtime-family foundations, `TaskFlow` and `DocFlow`, so the release program can be tracked as one layered capability ladder without collapsing runtime-family ownership.

## 0. Release-1 Capability Status Matrix

Status markers:

1. `✅` closed enough to act as active owner law or proven capability
2. `🟡` partially implemented, under active modernization, or not yet closure-proven
3. `⚪` target-only at the current stage

| Release slice | Core value | Release-1 owner surface | TaskFlow coverage | DocFlow coverage | Law status | Implementation status | Proof status | Builds on | Must not depend on | Standalone value | Detail section |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Slice 1: Operational Spine | first usable CLI/runtime shell with DB-first operational truth and fail-closed bootstrap | master plan Phases 1-2 | Layers 1-4 primary; Layer 8 readiness admission begins | Layers 1-3 supporting; Layer 7 readiness remains downstream gate | ✅ | ✅ | ✅ | none | Slices 2-5 | one operable shell that can initialize, inspect, and govern runtime state | [§4](#4-slice-1-operational-spine) |
| Slice 2: Project Activation Surface | project-aware runtime identity, activation state, and configurator law | master plan Phases 2-4 | Layers 2-5 primary; Layer 6 pre-execution gating starts to bind | Layers 1-2 inventory/config authority; Layer 4 lawful mutation for activation projections | ✅ | ✅ | ✅ | Slice 1 | Slices 3-5 | one DB-first project activation and sync surface usable before compilation | [§5](#5-slice-2-project-activation-surface) |
| Slice 3: Compiled Runtime Bundles | compiled control bundle and cache-safe control partitions for cheap orchestration | master plan Phases 3-4 | Layers 1-4 and 8-9 primary; protocol binding and runtime consumption preparation | Layers 2 and 7 required as canonical inventory/readiness inputs | ✅ | ✅ | ✅ | Slices 1-2 | Slices 4-5 | one inspectable compiled runtime identity usable before end-to-end artifact loop closure | [§6](#6-slice-3-compiled-runtime-bundles) |
| Slice 4: Planning, Execution, Artifact, And Approval Loop | first end-to-end operator loop from intent to durable artifact and approval evidence | master plan Phases 3-6 | Layers 3-6 primary; status, task graph, execution, and approval rendering | Layers 4-6 primary for artifact mutation, relation, and operator rendering | ✅ | ✅ | ✅ | Slices 1-3 | Slice 5 hardening | one useful operator loop without needing final hardening or Release 2 embedding | [§7](#7-slice-4-planning-execution-artifact-and-approval-loop) |
| Slice 5: Release-1 Closure And Hardening | coherent, recoverable, CLI-first runtime with explicit `taskflow -> docflow` proof seam | master plan Phases 5-7 | Layers 7-9 primary; restore/reconcile and closure trust | Layers 7-8 seam closure; final readiness/proof branch consumable by `TaskFlow` | ✅ | ✅ | ✅ | Slices 1-4 | Release 2 capabilities | one stable Release-1 shell that can support further framework work through itself | [§8](#8-slice-5-release-1-closure-and-hardening) |

Matrix reading rule:

1. read top-to-bottom to see the Release-1 capability ladder,
2. read left-to-right to see how each release slice combines product requirements with `TaskFlow` and `DocFlow` obligations,
3. every slice must close as one coherent capability bundle before the next slice becomes required,
4. no slice may borrow closure authority from a later Release-1 slice or from deferred Release-2 work.

## 0.1 Current Release-1 Alignment Snapshot

| Release slice | Strongest current owner surfaces | Main current gap |
|---|---|---|
| Slice 1: Operational Spine | `release-1-plan.md`; `taskflow-v1-runtime-modernization-plan.md`; current Rust `vida` shell and `taskflow-*` crates | no blocking Release-1 gap; remaining work is post-release shell thinning and owner extraction |
| Slice 2: Project Activation Surface | `project-activation-and-configurator-model.md`; `operational-state-and-synchronization-model.md`; current `.vida/**` direction | no blocking Release-1 gap; remaining work is post-release configurator/service extraction and projection cleanup |
| Slice 3: Compiled Runtime Bundles | `compiled-runtime-bundle-contract.md`; `taskflow-protocol-runtime-binding-model.md`; `runtime-paths-and-derived-cache-model.md` | no blocking Release-1 gap; remaining work is post-release alias burn-down and launcher decoupling |
| Slice 4: Planning, Execution, Artifact, And Approval Loop | `user-facing-runtime-flow-and-operating-loop-model.md`; `execution-preparation-and-developer-handoff-model.md`; `release-1-plan.md` | no blocking Release-1 gap; remaining work is post-release workflow/risk enrichment and additional top-level surface promotion |
| Slice 5: Release-1 Closure And Hardening | `compiled-autonomous-delivery-runtime-architecture.md`; `checkpoint-commit-and-replay-model.md`; `vida1-development-conditions.md` | closure is proven; only operational caveat is that local `.vida` datastore hygiene must be preserved because manual state cleanup can invalidate live probes until state is reinitialized |

Interpretation rule:

1. this matrix is the Release-1 capability projection above the runtime-family matrices,
2. it does not replace the owner law of `TaskFlow`, `DocFlow`, or the unified execution program,
3. it exists so Release-1 viability can be judged from one bounded control surface.

## 0.2 Mandatory Production-Baseline Tracks

The Airtable `Vida` spec backlog refreshed on `2026-03-16` adds these Release-1-critical tracks:

| Track | Priority | Minimum Release-1 requirement | Current posture |
|---|---|---|---|
| Trace, Telemetry, And Evidence Foundation | P0 | root trace, step/tool spans, side-effect evidence linkage, audit export, replay for at least one workflow per major class | ✅ |
| Tool Contract And Side-Effect Control | P0 | normalized tool contract with side-effect class, auth mode, retry/idempotency, policy hook, and approval-sensitive failure boundaries | ✅ |
| Retrieval, Freshness, And Citation Reliability | P0 | source registry, ACL-aware retrieval, freshness posture, citation linkage, delete/ACL propagation | ✅ |
| Identity, Delegation, And Approval Enforcement | P0 | principal model, chain-of-delegation, approval lifecycle, just-in-time elevation, denied/approved audit evidence | ✅ |
| Runtime SLO, Failure Recovery, And Rollback | P0 | workflow SLI/SLO registry, failure taxonomy, rollback/fallback path, incident evidence bundle | ✅ |
| Prompt Lifecycle And Controlled Rollout | P1 | prompt registry, benchmark-backed promotion, canary, rollback target, regression gate | ⚪ |
| Process Evaluation And Feedback Loop | P1 | process metrics, feedback ingestion, defect clustering, online/offline evaluator linkage | 🟡 |
| Memory Governance Operationalization | P1 | memory class/risk handling, consent/TTL, correction/deletion, approval for sensitive writes | 🟡 |
| Safety, Red Teaming, And FinOps Maturity | P1/P2 | adversarial suite, safety release blocks, cost-per-success tracking, routing/caching optimization with evidence | ⚪ |

Interpretation rule:

1. these tracks are part of Release-1 closure law, not optional post-release polish,
2. P0 tracks must close or be explicitly risk-accepted before any risky production workflow can claim Release-1 readiness,
3. P1 tracks must be represented in the Release-1 execution program even when their first implementation wave is narrower than the final platform ambition.

## 0.3 Functional Requirement Delta (`2026-04-03`)

The current spec set and local code audit sharpen Release-1 requirements in these bounded ways:

| Requirement delta | What is now required |
|---|---|
| Shared contract layer | one canonical enum/value layer for `workflow_class`, `risk_tier`, `approval_status`, `gate_level`, `compatibility_class`, and the full blocker registry; shell-local substitutes are not allowed |
| Operator surface completion | stable operator contracts must exist not only for `status` and `doctor`, but also for `consume`, `lane`, `approval`, and `recovery`, with shared top-level fields and fail-closed blocker behavior |
| Carrier/runtime neutrality | host carrier systems and subagent backends must be config-driven and executable through one carrier-neutral runtime contract set; `codex_multi_agent`, `codex_runtime_assignment`, and hardwired `vida agent-init` behavior cannot remain the canonical runtime model |
| Activation closure | DB-first activation/configurator lifecycle must become one authoritative operational path; `SurrealDB` remains the default activation/projection truth and filesystem/Git remain synchronized projections and lineage surfaces |
| Activation truth ownership | a persisted launcher snapshot of `vida.config.yaml` and compiled bundle is evidence only; it must not remain the final activation authority model |
| Event and replay posture | lawful transition, checkpoint, replay, and projection integrity must be explicit; if an event backend is introduced, it must stay adapter-backed, feature-gated, and bounded rather than replacing the whole DB-first canon |
| Seam receipt discipline | `TaskFlow -> DocFlow` closure must consume explicit DocFlow readiness/proof receipts and closure-admission artifacts; protocol-binding receipt alone is insufficient seam proof |
| Replay lineage discipline | checkpoint/recovery must include append-evidence receipts, lineage/fork scope, and projection-checkpoint artifacts rather than only latest resumability summaries |
| Execution ownership | `vida` must converge toward thin shell/router/render while execution, approval, closure, replay, and policy truth move into `TaskFlow` and `DocFlow` family-owned modules |
| Proof-neutral fixtures | smoke and golden proofs must validate carrier-neutral contracts and operator capabilities instead of codex-era bundle names, backend literals, or exact legacy surfaces |
| Production-trust closure | trace/evidence, tool governance, retrieval trust, approval/delegation enforcement, and rollback/incident proof are closure-gated Release-1 requirements rather than optional hardening work |

Interpretation rule:

1. this delta does not replace the owner docs below,
2. it sharpens how the existing slices must now be read and implemented,
3. it also blocks the incorrect reading that Release 1 is only a launcher hardening pass.

## 1. Scope

This matrix defines:

1. the canonical Release-1 capability slices,
2. how those slices map onto `TaskFlow` and `DocFlow`,
3. which owner surface and phase range governs each slice,
4. which slices form the mandatory Release-1 closure ladder,
5. the seam rule between product requirements and runtime-family obligations.

This matrix does not define:

1. detailed `TaskFlow` kernel law,
2. detailed `DocFlow` layer law,
3. implementation-only task lists,
4. Release-2 capability planning.

## 2. Layering Rule

Each Release-1 slice must satisfy all of the following:

1. it must deliver standalone product/runtime value before the next slice exists,
2. it must combine only the `TaskFlow` and `DocFlow` capabilities already owned below it,
3. it must not depend on a later Release-1 slice to legitimize its own owned behavior,
4. it must remain CLI-first and Release-1-bounded,
5. it must expose one bounded operator and proof path for closure,
6. it must not claim closure while mandatory production-baseline tracks for its scope remain absent.

Compact rule:

1. Release 1 is a capability ladder over two sibling runtime families,
2. product slices are valid only when both release scope and runtime-family ownership remain explicit.

## 3. Owner And Seam Rule

Primary owner references:

1. `docs/product/spec/release-1-plan.md`
2. `docs/product/spec/release-1-closure-contract.md`
3. `docs/product/spec/release-1-workflow-classification-and-risk-matrix.md`
4. `docs/product/spec/release-1-control-metrics-and-gates.md`
5. `docs/product/spec/release-1-canonical-artifact-schemas.md`
6. `docs/product/spec/release-1-decision-tables.md`
7. `docs/product/spec/release-1-state-machine-specs.md`
8. `docs/product/spec/release-1-error-and-exception-taxonomy.md`
9. `docs/product/spec/release-1-proof-scenario-catalog.md`
10. `docs/product/spec/release-1-schema-versioning-and-compatibility-law.md`
11. `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`
12. `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`
13. `docs/product/spec/docflow-v1-runtime-modernization-plan.md`
14. `docs/product/spec/canonical-runtime-layer-matrix.md`
15. `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`
16. `docs/product/spec/functional-matrix-protocol.md`

Seam rule:

1. Release-1 slices may combine `TaskFlow` and `DocFlow` obligations,
2. but they must not collapse the two runtime families into one generic owner,
3. `TaskFlow` remains the execution substrate and closure authority,
4. `DocFlow` remains the bounded documentation/inventory/validation/readiness/proof runtime,
5. final hardening closes only when the `TaskFlow -> DocFlow` seam is explicit and fail-closed,
6. production-baseline tracks may extend across both runtime families, but closure authority still remains explicit.

## 4. Slice 1: Operational Spine

### 4.1 Purpose

Provide the first usable Release-1 shell that can initialize, inspect, and govern its own runtime state through a DB-first operational spine.

### 4.2 Owns

1. stable boot/init posture,
2. DB-first state spine,
3. bounded status and doctor roots,
4. protocol-binding import/gating needed for safe runtime operation,
5. minimum operator shell needed to drive later slices.

### 4.3 Must Not Own

1. full project activation closure,
2. compiled bundle closure,
3. end-to-end artifact/approval loop,
4. final hardening seam.

### 4.4 TaskFlow Coverage

1. Layers 1-4 are mandatory,
2. Layer 8 begins as the readiness admission boundary,
3. bridge-backed runtime shell surfaces are allowed as continuity support.

### 4.5 DocFlow Coverage

1. Layers 1-3 must remain lawful,
2. Layer 7 readiness remains a downstream gate but not yet the owning closure path for this slice.

### 4.6 Owner Docs

1. `docs/product/spec/release-1-plan.md`
2. `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`
3. `docs/product/spec/taskflow-protocol-runtime-binding-model.md`
4. `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`

### 4.7 Owner Code Surface

1. `crates/vida/**`
2. `crates/taskflow-*/**`
3. bounded compatibility/runtime-support donors under the active TaskFlow runtime-family bridge surfaces

### 4.8 Operator And Proof Surface

1. `vida` boot/init/status/doctor families
2. protocol-binding status and check surfaces
3. `vida docflow readiness-check`
4. bounded runtime boot and restore proofs

### 4.9 Failure Mode

1. missing or invalid protocol-binding/runtime-state prerequisites block execution,
2. runtime must expose bounded remediation rather than silently continuing.

### 4.10 Current Gap

1. bridge-era shell and protocol-binding donors still carry part of the operational spine while native Rust closure is in progress.

### 4.11 Standalone Value

1. Release 1 already has one usable self-operating shell before project activation or compiled bundles fully close.

## 5. Slice 2: Project Activation Surface

### 5.1 Purpose

Turn the operational shell into a project-aware runtime with DB-first activation state, configurable runtime identity, and fail-closed activation validation.

### 5.2 Owns

1. DB-first configurator,
2. project-owned activation state,
3. sync between DB truth and filesystem projections,
4. activation query/status surfaces,
5. explicit pre-execution preparation gating where required.

### 5.3 Must Not Own

1. compiled bundle ownership,
2. broad project protocol auto-compilation,
3. final runtime hardening,
4. Release-2 embedding concerns.

### 5.4 TaskFlow Coverage

1. Layers 2-5 are primary,
2. Layer 6 starts binding execution-preparation and verification posture to activation state.

### 5.5 DocFlow Coverage

1. Layers 1-2 remain authoritative for inventory and activation-facing canonical docs,
2. Layer 4 mutation law is required for lawful projection updates.

### 5.6 Owner Docs

1. `docs/product/spec/release-1-plan.md`
2. `docs/product/spec/project-activation-and-configurator-model.md`
3. `docs/product/spec/execution-preparation-and-developer-handoff-model.md`
4. `docs/product/spec/operational-state-and-synchronization-model.md`

### 5.7 Owner Code Surface

1. `crates/vida/**`
2. `crates/taskflow-*/**`
3. projected activation surfaces under `.vida/**`

### 5.8 Operator And Proof Surface

1. activation/config status families
2. import/export/sync surfaces
3. fail-closed validation for missing execution-preparation artifacts

### 5.9 Failure Mode

1. invalid activation wiring, missing sync state, or absent preparation artifacts block further runtime progression.

### 5.10 Current Gap

1. DB-first activation closure is still converging from bridge-era overlay and projection surfaces, and the authoritative path still runs through `vida.config.yaml` plus `.vida/project/**` projections more than one family-owned activation service.

### 5.11 Standalone Value

1. the runtime becomes project-aware and operable before compiled bundle closure exists.

## 6. Slice 3: Compiled Runtime Bundles

### 6.1 Purpose

Compile the active framework/project posture into one strict control bundle with cache-safe partitions so orchestration can run cheaply without broad raw-canon rereads.

### 6.2 Owns

1. strict compiled bundle schema,
2. protocol-binding registry inside the control bundle,
3. cache-delivery contract,
4. bundle inspection and query surfaces,
5. initialization from compiled control rather than broad manual traversal.

### 6.3 Must Not Own

1. end-to-end operator artifact loop,
2. Release-2 daemon/reactive behavior,
3. final `TaskFlow -> DocFlow` hardening closure by itself.

### 6.4 TaskFlow Coverage

1. Layers 1-4 provide the compiled control substrate,
2. Layer 8 readiness and Layer 9 consumption preparation are mandatory,
3. protocol binding must remain DB-first rather than file-log-first.

### 6.5 DocFlow Coverage

1. Layer 2 canonical inventory and Layer 7 readiness act as mandatory bundle inputs,
2. `DocFlow` remains evidence/runtime-readiness owner rather than execution owner.

### 6.6 Owner Docs

1. `docs/product/spec/release-1-plan.md`
2. `docs/product/spec/compiled-runtime-bundle-contract.md`
3. `docs/product/spec/taskflow-protocol-runtime-binding-model.md`
4. `docs/product/spec/runtime-paths-and-derived-cache-model.md`

### 6.7 Owner Code Surface

1. `crates/vida/**`
2. `crates/taskflow-*/**`
3. derived cache and compiled control surfaces under `.vida/**`

### 6.8 Operator And Proof Surface

1. bundle build and inspection commands
2. schema validation
3. cache-boundary proofs
4. runtime init from compiled bundle

### 6.9 Failure Mode

1. invalid inputs, schema drift, or cache-boundary ambiguity block bundle compilation and runtime init.

### 6.10 Current Gap

1. strict bundle contract is clear canonically, but runtime-native closure, query surfaces, and protocol-binding ownership are still concentrated in launcher-owned code.

### 6.11 Standalone Value

1. Release 1 gains one inspectable machine-readable runtime identity before the full operator loop closes.

## 7. Slice 4: Planning, Execution, Artifact, And Approval Loop

### 7.1 Purpose

Close the first end-to-end Release-1 operator loop from intent and planning through execution reporting, artifact materialization, and approval evidence.

### 7.2 Owns

1. planning and scope queries,
2. execution status rendering,
3. artifact materialization,
4. approval prompts and durable replies,
5. task-graph formation for bounded execution.

### 7.3 Must Not Own

1. final restore/reconcile hardening,
2. Release-2 UI or embedding,
3. broad autonomous project protocol promotion beyond bounded Release-1 law.

### 7.4 TaskFlow Coverage

1. Layers 3-6 are primary for execution, task graph, reporting, verification, and approval routing.

### 7.5 DocFlow Coverage

1. Layers 4-6 are primary for lawful artifact mutation, relation visibility, and operator rendering.

### 7.6 Owner Docs

1. `docs/product/spec/release-1-plan.md`
2. `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
3. `docs/product/spec/execution-preparation-and-developer-handoff-model.md`

### 7.7 Owner Code Surface

1. `crates/vida/**`
2. `crates/taskflow-*/**`
3. `crates/docflow-*/**`

### 7.8 Operator And Proof Surface

1. planning/scope query families
2. execution status families
3. artifact materialization outputs
4. durable approval evidence

### 7.9 Failure Mode

1. if planning state, artifact materialization, or approval evidence cannot become durable runtime truth, the loop is not closed and runtime must stop short of claiming end-to-end usefulness.

### 7.10 Current Gap

1. the full durable operator loop is only partly real in active Rust surfaces: `taskflow consume`, run-graph, and approval wait exist, but shared workflow/risk contracts and stable `consume`/`lane`/`approval` operator surfaces are not yet closed.

### 7.11 Standalone Value

1. Release 1 becomes operator-useful beyond framework-only shell work even before final hardening.

## 8. Slice 5: Release-1 Closure And Hardening

### 8.1 Purpose

Close Release 1 as one coherent CLI-first runtime with restore/reconcile discipline, explicit readiness/proof seam, and stable basis for Release 2.

### 8.2 Owns

1. restore and reconcile flows,
2. DB/filesystem conflict discipline,
3. checkpoint/replay and projection integrity for supported workflows,
4. explicit `taskflow -> docflow` readiness/proof seam,
5. closure-proof surfaces,
6. install/packaging sufficiency for Release-1 use.

### 8.3 Must Not Own

1. Release-2 reactive synchronization engine,
2. always-on memory daemon,
3. host-project embedding,
4. UI.

### 8.4 TaskFlow Coverage

1. Layers 7-9 are primary,
2. `TaskFlow` remains final execution substrate and closure authority.

### 8.5 DocFlow Coverage

1. Layer 7 readiness and Layer 8 runtime-consumption seam become mandatory,
2. `DocFlow` becomes the bounded final readiness/proof branch consumable by `TaskFlow`.

### 8.6 Owner Docs

1. `docs/product/spec/release-1-plan.md`
2. `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`
3. `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`
4. `docs/product/spec/docflow-v1-runtime-modernization-plan.md`
5. `docs/product/spec/projection-listener-checkpoint-model.md`
6. `docs/product/spec/checkpoint-commit-and-replay-model.md`
7. `docs/process/vida1-development-conditions.md`

### 8.7 Owner Code Surface

1. `crates/taskflow-*/**`
2. `crates/docflow-*/**`
3. `crates/vida/**`
4. release/install surfaces needed for bounded Release-1 use

### 8.8 Operator And Proof Surface

1. restore/reconcile proofs
2. readiness and proofcheck surfaces
3. explicit seam verification between `TaskFlow` and `DocFlow`
4. install/packaging proof for bounded runtime use

### 8.9 Failure Mode

1. if restore/reconcile discipline or the `TaskFlow -> DocFlow` seam is not explicit and fail-closed, Release 1 cannot claim closure and must remain open.

### 8.10 Current Gap

1. production-baseline control tracks and closure-facing runtime proofs are now evidence-backed, but final matrix refresh, DocFlow seam hardening, release-candidate build, and explicit closure admission still remain before Release 1 can close.

### 8.11 Standalone Value

1. Release 1 becomes a coherent self-hosting CLI-first runtime and a lawful basis for Release 2.

## 9. Program Closure Rule

Release 1 is closed only when all are true:

1. all five slices are closed,
2. each slice remains independently useful,
3. `TaskFlow` and `DocFlow` ownership remains explicit,
4. the release matrix does not depend on Release-2 capability to explain Release-1 viability,
5. the final `TaskFlow -> DocFlow` seam is explicit, fail-closed, and proof-backed.

-----
artifact_path: product/spec/release-1-capability-matrix
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-04-07
schema_version: '1'
status: canonical
source_path: docs/product/spec/release-1-capability-matrix.md
created_at: '2026-03-13T13:20:00+02:00'
updated_at: 2026-04-07T20:22:57.796525876Z
changelog_ref: release-1-capability-matrix.changelog.jsonl
