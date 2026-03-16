# Release 1 Current State

Status: active product execution report (runtime checkpoint refreshed `2026-03-15`)

Purpose: record the bounded implementation-reality checkpoint for `Release 1` across `TaskFlow`, `DocFlow`, and the current `vida` launcher shell so execution planning can be based on actual code and proven runtime surfaces rather than architectural intent alone.

## 1. Method

This reality pass is based on:

1. active Release-1 matrices and seam map,
2. current Rust workspace structure,
3. current launcher and crate boundaries,
4. proven conditions already recorded in `docs/process/vida1-development-conditions.md`,
5. code concentration and ownership analysis across `crates/taskflow-*`, `crates/docflow-*`, and `crates/vida/**`.

Interpretation rule:

1. percentages below are bounded readiness estimates,
2. they describe Release-1 usefulness and current execution posture,
3. they do not claim mathematical certainty or one global product percentage.

## 1.1 Runtime Delta Checkpoint (`2026-03-15`)

Since the initial pass, the runtime reality changed in these bounded ways:

1. `status`/`doctor` fresh-state reliability defect was closed by adding the missing `run_graph_dispatch_receipt` table to canonical bootstrap schema and by hardening `open_existing` with schema reconciliation.
2. state-store lock handling was centralized in `StateStore::open_existing`, reducing command-surface drift where some paths retried locks and others failed immediately.
3. installed-binary smoke was rerun after the fix (`41/50 pass`), and remaining failures were classified as expected precondition, expected fail-closed checks, or expected validation behavior.
4. no new evidence changed the main architectural finding: Release-1 risk remains concentrated in DB-first activation/entity closure and in launcher concentration.

## 1.2 Airtable Delta Checkpoint (`2026-03-16`)

The Airtable `Vida` `Spec` table was refreshed and now sharpens Release-1 scope in these ways:

1. P0 production-baseline tracks are now explicit:
   - trace, telemetry, and evidence foundation
   - tool contract and side-effect control
   - retrieval, freshness, and citation reliability
   - identity, delegation, and approval enforcement
   - runtime SLO, failure recovery, and rollback
2. P1 control-maturity tracks are now also explicit:
   - prompt lifecycle and controlled rollout
   - process evaluation and feedback loop
   - memory governance operationalization
   - safety, red teaming, and FinOps maturity
3. starter workflows, escalation flows, feedback triage, prompt rollout, and memory approval are now described as canonical operating patterns rather than future optional examples.
4. benchmark/control metrics are now explicit product surfaces:
   - tool input accuracy
   - tool output utilization
   - safety defect rate
   - cost per successful task
   - prompt regression rate

Interpretation:

1. Release 1 is no longer accurately described as runtime-refactor-only work.
2. Release 1 must now be judged against production trust/control readiness as well as architectural cleanup.

## 2. Workspace Reality Snapshot

Current Rust workspace members:

1. `DocFlow` family:
   - `docflow-core`
   - `docflow-contracts`
   - `docflow-config`
   - `docflow-format-jsonl`
   - `docflow-format-toon`
   - `docflow-markdown`
   - `docflow-inventory`
   - `docflow-relations`
   - `docflow-validation`
   - `docflow-readiness`
   - `docflow-operator`
   - `docflow-cli`
2. `TaskFlow` family:
   - `taskflow-core`
   - `taskflow-contracts`
   - `taskflow-config`
   - `taskflow-format-jsonl`
   - `taskflow-format-toon`
   - `taskflow-state`
   - `taskflow-state-fs`
   - `taskflow-state-surreal`
3. launcher shell:
   - `vida`

Concentration findings:

1. `crates/vida/src/main.rs` is `15079` lines
2. `crates/vida/src/state_store.rs` is `8869` lines
3. the `DocFlow` family is already decomposed into many bounded crates,
4. the `TaskFlow` family has strong kernel/state foundations, but much runtime behavior still lives in the launcher shell.

## 3. Readiness Scale

Use the following interpretation:

1. `0-20%`
   - target only
2. `21-40%`
   - thin implementation foothold
3. `41-60%`
   - meaningful partial implementation, not closure-ready
4. `61-80%`
   - strong bounded implementation with visible remaining closure work
5. `81-100%`
   - near-closure or closure-ready for the bounded scope

Rewrite-pressure scale:

1. `low`
   - keep and extend
2. `medium`
   - bounded refactor/carve-out needed
3. `high`
   - current owner shape is wrong for the target and substantial replacement is required

## 4. Codebase Action Summary

| Area | Estimated readiness | Keep as is | Extend | Refactor / carve out | Replace / rewrite | Rewrite pressure | Main reason |
|---|---:|---:|---:|---:|---:|---|---|
| `DocFlow` crate family | 78% | 55% | 25% | 15% | 5% | low | crate lattice is already close to target architecture and many operator/readiness surfaces are proven |
| `TaskFlow` crate family | 58% | 35% | 25% | 30% | 10% | medium | strong kernel/state footing exists, but many execution/orchestration surfaces are not yet fully native in the family crates |
| `vida` launcher shell | 52% | 20% | 15% | 45% | 20% | high | launcher is functionally rich but too concentrated; much runtime ownership still sits in monolithic shell code |
| bridge donors (`taskflow-v0`, `codex-v0`) | 65% for continuity value | 25% | 35% | 20% | 20% | medium | valuable as donor/proof/continuity surfaces, but wrong as long-term owners |

Overall execution interpretation:

1. the codebase is not early-stage,
2. it is substantial enough to progress through control and carving, not through wholesale rewrite,
3. the main risk is ownership concentration, not lack of implementation.

## 4.1 Airtable Delta Gap Ledger

| Airtable track | Current local posture | Gap severity | Notes |
|---|---|---|---|
| Trace, telemetry, and evidence foundation | partial observability/readiness surfaces exist | high | local canon is strong on receipts and state, but not yet explicit enough on trace model, audit export, and side-effect evidence linkage |
| Tool contract and side-effect control | contract direction exists, runtime enforcement incomplete | high | must become a first-class Release-1 closure gate |
| Retrieval freshness and citation reliability | conceptually present in broader architecture, not yet closure-owned in Release-1 surfaces | high | needs source registry, freshness posture, and citation contract |
| Identity, delegation, and approval enforcement | approval/policy law exists, enterprise-grade principal/delegation enforcement remains incomplete | high | must move from guidance into explicit runtime contract and proof |
| Runtime SLO, failure recovery, and rollback | recovery/checkpoint posture is meaningful, operational SLO and incident model remain weak | high | restore/reconcile alone is not enough for production closure |
| Prompt lifecycle and controlled rollout | largely absent as Release-1 owner surface | medium-high | should be added now as explicit P1 control track |
| Process evaluation and feedback loop | evaluation exists architecturally, process-quality loop is under-specified | medium-high | feedback ingestion and remediation linkage need owner status |
| Memory governance operationalization | policy intent exists, operational memory controls remain under-specified | medium-high | consent, TTL, deletion, and approval posture need explicit bounded closure |
| Safety, red teaming, and FinOps maturity | some governance direction exists, explicit release-gating metrics and attack-class testing remain absent | medium-high | needs to be tracked as P1/P2 maturity rather than implied later work |

## 5. TaskFlow Layer Reality

| TaskFlow layer | Estimated readiness | Strongest current code reality | Main gap | Recommended action |
|---|---:|---|---|---|
| Layer 1: Runtime Kernel Boundary | 72% | `taskflow-core`, `taskflow-contracts`, protocol-binding and kernel concepts also reflected in launcher/store | canonical boundary exists but is not yet the sole native owner of all kernel-facing behavior | keep core, extend contracts, carve launcher leakage out |
| Layer 2: State, Route, And Receipt Kernel | 76% | `taskflow-state`, `taskflow-state-fs`, `taskflow-state-surreal`, strong persisted runtime state in launcher | route/receipt semantics still partly shell-owned and not fully isolated in native family crates | keep stores, extend contracts, bounded refactor |
| Layer 3: Tracked Execution Substrate | 60% | real runtime commands and launcher-owned execution surfaces exist | execution substrate is still spread between native crates and monolithic launcher logic | extend and carve out |
| Layer 4: Lane Routing And Assignment | 52% | launcher already exposes taskflow query/help/routing behavior | routing remains concentrated in `vida` rather than a clean TaskFlow ownership package | refactor/carve out |
| Layer 5: Handoff And Context Governance | 48% | some routed-run and gate semantics exist in run-graph and launcher flow logic | no clearly bounded native TaskFlow module cluster yet for handoff/context governance | refactor first, then extend |
| Layer 6: Review, Verification, And Approval Gates | 56% | verification/doctor/closure guardrails are partly real and partly shell-owned | gate logic is not yet cleanly separated from launcher orchestration | bounded refactor |
| Layer 7: Resumability, Checkpoint, And Recovery | 74% | strong run-graph, recovery, checkpoint, gate inspection surfaces already proven in `vida` and state store | native ownership exists behaviorally, but still shell-heavy structurally | keep behavior, carve into native modules |
| Layer 8: Observability And Runtime Readiness | 78% | `vida status`, `vida doctor`, readiness-related proofs and summaries are already real | observability/readiness is strong operationally but still too concentrated in launcher/store | keep and refactor structurally |
| Layer 9: Direct Runtime Consumption | 57% | `vida taskflow consume final|continue|advance` exist and bounded `DocFlow` activation is already proven | direct-consumption seam is real, persisted continuation is now present, but ownership is still bridge-heavy and not yet cleanly native in family-owned code | extend seam, refactor ownership |

TaskFlow summary:

1. functional readiness is ahead of architectural cleanliness,
2. the biggest TaskFlow problem is not missing code, but launcher concentration,
3. execution should prioritize carve-out and seam-hardening, not broad rewrite of already-proven stores and flow projections.

## 6. DocFlow Layer Reality

| DocFlow layer | Estimated readiness | Strongest current code reality | Main gap | Recommended action |
|---|---:|---|---|---|
| Layer 1: Canonical Schema | 86% | `docflow-core`, `docflow-contracts`, config/schema loading crates | mostly mature for Release 1 | keep and extend only where needed |
| Layer 2: Canonical Inventory | 82% | `docflow-inventory`, registry scans, canonical write surfaces proven through `vida docflow` | some bridge/parity cleanup remains | keep and extend |
| Layer 3: Canonical Validation | 81% | `docflow-validation`, `check`, `fastcheck`, `doctor`, `activation-check`, `protocol-coverage-check` already proven in-process | final cleanup is mainly parity and packaging discipline | keep and extend |
| Layer 4: Canonical Mutation | 74% | `finalize-edit`, `touch`, `rename-artifact`, `init`, `move`, `migrate-links` already in-process | mutation shell is large and still concentrated in `docflow-cli` | keep semantics, refactor shell structure |
| Layer 5: Canonical Relations | 73% | `docflow-relations`, `links`, `deps-map`, `artifact-impact`, `task-impact` are real | relation internals can still be deepened and simplified | extend |
| Layer 6: Canonical Operator | 79% | `docflow-operator`, `docflow-cli`, `vida docflow` in-process operator surfaces | operator shell is strong, but command surface remains dense | keep and moderate refactor |
| Layer 7: Canonical Runtime Readiness | 77% | `docflow-readiness`, `readiness-check`, `readiness-write`, `proofcheck` and profile-backed proofs | seam-facing readiness ownership is good, but final native closure into TaskFlow is not complete | keep and extend toward seam |
| Layer 8: Canonical Runtime Consumption | 38% | bounded seam readiness exists conceptually and partly operationally through `consume final` | `DocFlow` is Layer-8-ready but not Layer-8-closed as an independent native seam package | do not rewrite whole family; build seam-specific slice |

DocFlow summary:

1. `DocFlow` is the strongest native Rust modernization line right now,
2. most of the family should be preserved and continued,
3. the main missing area is not Layer 1-7 functionality, but Layer 8 seam closure.

## 7. Release-1 Slice Reality

| Release-1 slice | Estimated readiness | Main current reality | Main current gap | Execution priority |
|---|---:|---|---|---|
| Slice 1: Operational Spine | 68% | real shell, state, doctor/status, protocol-binding and many taskflow/docflow commands already exist | native ownership is still split across launcher and donors | highest |
| Slice 2: Project Activation Surface | 54% | law is clear and some shell/config path exists | DB-first activation/configurator closure is not yet stable enough as the sole path | high |
| Slice 3: Compiled Runtime Bundles | 49% | bundle law and binding law are clear; some runtime surfaces exist | strict compiled bundle runtime closure still trails the law | high |
| Slice 4: Planning / Execution / Artifact / Approval | 36% | components exist separately | end-to-end durable loop is not yet closed as one slice | medium |
| Slice 5: Closure And Hardening | 31% | seam and closure rules are now explicit; some runtime proof already exists | restore/reconcile + final seam hardening remains open | highest |

Release-1 summary:

1. Release 1 is materially underway, not conceptual,
2. it is not ready to close because the critical path still runs through activation, compiled control, and seam hardening,
3. execution should focus on Slice 1, Slice 2, Slice 3, and Slice 5 before broadening Slice 4 work,
4. slice execution must now carry the Airtable P0 production-baseline tracks explicitly rather than assuming they are downstream hardening.

## 8. Seam Reality

| Seam segment | Estimated readiness | Current reality | Main gap | Recommended action |
|---|---:|---|---|---|
| Segment 1: TaskFlow -> DocFlow activation | 58% | explicit activation exists in `consume final` path | seam is real but still shell-heavy and bridge-shaped | extend and carve out |
| Segment 2: DocFlow proof return -> TaskFlow closure path | 52% | readiness/proof outputs already exist in `DocFlow` | final native contract for TaskFlow consumption still needs hardening | extend with seam-specific contract tests |
| Segment 3: Release-1 closure admission | 34% | closure rules are explicit and some proofs exist | final hardening, restore/reconcile, and final admission proof remain open | keep narrow, do not widen scope |

Seam summary:

1. the seam is no longer undefined,
2. it is partially implemented,
3. it is the main closure bottleneck for Release 1.

## 9. Rewrite Versus Add Versus Keep

Recommended product-wide posture:

1. `keep`
   - most `DocFlow` family crates
   - `TaskFlow` core/contracts/state/store foundations
   - already-proven operator and proof surfaces
2. `extend`
   - seam-specific `TaskFlow -> DocFlow` contracts
   - compiled bundle/runtime control path
   - DB-first activation and sync path
3. `refactor / carve out`
   - launcher-owned TaskFlow routing/execution/gate logic in `crates/vida/src/main.rs`
   - heavy state/orchestration concentration in `crates/vida/src/state_store.rs`
   - dense command-shell logic in `docflow-cli`
4. `replace / rewrite`
   - only bridge-only or wrongly owned slices that block native family ownership
   - not the already-proven family foundations

Approximate execution-wide action split:

1. keep: `45-50%`
2. extend: `25-30%`
3. refactor/carve out: `20-25%`
4. replace/rewrite: `5-10%`

## 10. Execution Implication

The right execution posture is:

1. not a fresh rewrite,
2. not indefinite continuation of the current launcher concentration,
3. a controlled carve-out and gap-closure program over real existing assets.

Best immediate next moves:

1. build the first reality-backed execution queue from this report,
2. isolate launcher concentration tasks for `TaskFlow`,
3. isolate seam-hardening tasks for Slice 5,
4. keep `DocFlow` progressing mostly through extension rather than redesign.

## 11. Confidence Note

Confidence by area:

1. `DocFlow`: high
2. `TaskFlow`: medium-high
3. `Release 1 closure readiness`: medium
4. exact rewrite percentages: medium

Reason:

1. crate topology and proven-condition evidence are strong,
2. but some shell-owned behavior is still concentrated enough that exact carve-out size will sharpen only after the next execution-pass refresh.

-----
artifact_path: product/spec/release-1-current-state
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-15'
schema_version: '1'
status: canonical
source_path: docs/product/spec/release-1-current-state.md
created_at: '2026-03-13T14:12:00+02:00'
updated_at: 2026-03-16T11:12:01.233256745Z
changelog_ref: release-1-current-state.changelog.jsonl
