# VIDA Canonical Runtime Layer Matrix

Status: active product law

Purpose: define the layered runtime capability model for VIDA from the minimum lawful runtime kernel up to maximum direct runtime consumption, so each layer is independently useful, internally coherent, and does not depend on a future unfinished layer.

## 0. Runtime Layer Status Matrix

Status markers:

1. `✅` completed and already available in the current transitional implementation or promoted law
2. `🟡` partially implemented, draft-promoted, or still missing one or more closing requirements
3. `⚪` target architecture only, not yet closed as active runtime law

| Category | Layer 1 | Layer 2 | Layer 3 | Layer 4 | Layer 5 | Layer 6 | Layer 7 | Layer 8 | Layer 9 |
|---|---|---|---|---|---|---|---|---|---|
| Layer name | Runtime Kernel Boundary | State, Route, And Receipt Kernel | Tracked Execution Substrate | Lane Routing And Assignment | Handoff And Context Governance | Review, Verification, And Approval Gates | Resumability, Checkpoint, And Gateway Recovery | Observability And Runtime Readiness | Direct Runtime Consumption |
| Status | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Three-level protocol spine | `core` | `core` | `runtime-family execution` | `orchestration shell` | `orchestration shell` | `orchestration shell` + `runtime-family execution` gate seam | `runtime-family execution` | `runtime-family execution` | `runtime-family execution` + `DocFlow` activation seam |
| Core value | one canonical split between config, state, receipts, proofs, projections, and runtime-owned artifacts | one lawful state/route/progression model with explicit stage and gate semantics | one bounded execution substrate that records how work is executed | one lawful way to select lanes, agents, and route receipts | one bounded packet-driven delegation model with governed context | one explicit quality and governance gate chain that keeps `coach`, `verifier`, and `approval` distinct | one lawful recovery model with explicit resumability and future gateway targeting | one explicit health/readiness gate before runtime trust or closure | runtime consumes canonical law directly and treats bridge files only as export or migration surfaces |
| Required implementation | kernel scope, ownership rules, machine set boundaries | machine map, route law, receipt/proof taxonomy, stage and guard law | TaskFlow block lifecycle, execution plans, run-graph initialization, boot packet | route resolution, capability registry, assignment engine, route receipts, lane selection | worker packet contract, handoff law, context-governance ledger, bounded context shaping | coach/verifier/approval law, proof boundary, merge/admissibility rules, human approval | run-graph ledger, checkpoint/replay law, gateway handle law, replay-safe retry semantics | observability map, trace/proving surfaces, reconciliation, readiness verdicts and blockers | DB-first/runtime-owned truth, canonical bundle consumption, boot/migration/load from canonical law |
| Builds on | none | Layer 1 | Layers 1-2 | Layers 1-3 | Layers 1-4 | Layers 1-5 | Layers 1-6 | Layers 1-7 | Layers 1-8 |
| Must not depend on | TaskFlow, assignment, checkpoints, runtime DB | direct runtime execution, gateway runtime adapters, DB substrate | dynamic role expansion, gateway resume handles, DB-first migration completion | future hierarchical supervisor runtime, DB-first storage completion | future remote federation or transcript inheritance | future consensus heuristics replacing proof, runtime-consumption DB completion | final DB-first runtime implementation | direct runtime consumption | none; final layer |
| Standalone value | one stable vocabulary and ownership split for all runtime work | one inspectable route/state/proof model | one traceable way to execute and resume work inside a task | one deterministic way to pick who does which slice | one safe delegation and context model | one lawful closure gate chain | one interruption-safe runtime recovery model | one trust/readiness and health gate | one full live runtime path |
| Detail section | [§4](#4-layer-1-runtime-kernel-boundary) | [§5](#5-layer-2-state-route-and-receipt-kernel) | [§6](#6-layer-3-tracked-execution-substrate) | [§7](#7-layer-4-lane-routing-and-assignment) | [§8](#8-layer-5-handoff-and-context-governance) | [§9](#9-layer-6-review-verification-and-approval-gates) | [§10](#10-layer-7-resumability-checkpoint-and-gateway-recovery) | [§11](#11-layer-8-observability-and-runtime-readiness) | [§12](#12-layer-9-direct-runtime-consumption) |

Matrix reading rule:

1. read left-to-right to see the capability progression from minimum lawful runtime to full runtime consumption,
2. each layer must already deliver standalone value before the next layer is added,
3. later layers may strengthen earlier layers but must not redefine their ownership boundary,
4. any layer marked `🟡` is allowed to guide current work, but it is not yet closed enough to absorb the next layer completely,
5. a runtime layer is lawful only when it forms one coherent capability bundle with its own operator surface, proof surface, and fail-closed boundary.

## 0.1 Current Runtime Alignment Snapshot

| Layer | Alignment | Strongest evidence | Main current gap |
|---|---|---|---|
| Layer 1 | ✅ | `partial-development-kernel-model.md`, `canonical-machine-map.md`, `runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`, `taskflow-v0/src/core/runtime_bundle.nim` | no blocking gap for the current runtime kernel boundary; taskflow now composes and validates one executable kernel bundle over canonical law |
| Layer 2 | ✅ | `vida-0.3-route-and-receipt-spec.md`, `canonical-machine-map.md`, `receipt-and-proof-law.md`, `runtime-instructions/runtime.runtime-kernel-bundle-protocol.md` | no blocking gap for the current state/route/receipt kernel; machine, route, and receipt law now close through canonical specs plus executable bundle composition |
| Layer 3 | ✅ | `runtime-instructions/work.taskflow-protocol.md`, `runtime-instructions/model.boot-packet-protocol.md`, `taskflow-v0/**` | no blocking gap for the current transitional execution substrate |
| Layer 4 | ✅ | `core.agent-system-protocol.md`, `runtime-instructions/core.capability-registry-protocol.md`, `runtime-instructions/work.project-agent-extension-protocol.md`, `runtime-instructions/work.agent-lane-selection-protocol.md`, `taskflow-v0/src/agents/route.nim`, `taskflow-v0/src/core/assignment_engine.nim`, `taskflow-v0/src/core/role_selection.nim` | no blocking gap for the current transitional routing/assignment layer; dynamic role extensions and auto-lane selection now have runtime-owned executable surfaces |
| Layer 5 | ✅ | `lane.worker-dispatch-protocol.md`, `runtime-instructions/lane.agent-handoff-context-protocol.md`, `runtime-instructions/core.context-governance-protocol.md` | no blocking gap for bounded handoff/context law in the current transitional runtime |
| Layer 6 | ✅ | `runtime-instructions/work.verification-lane-protocol.md`, `runtime-instructions/work.verification-merge-protocol.md`, `verification-merge-law.md`, `taskflow-v0/src/gates/verification_merge.nim` | no blocking gap for verification merge/admissibility; taskflow now exposes executable verifier admissibility and merge policies |
| Layer 7 | ✅ | `runtime-instructions/core.run-graph-protocol.md`, `runtime-instructions/recovery.checkpoint-replay-recovery-protocol.md`, `runtime-instructions/runtime.direct-runtime-consumption-protocol.md`, `taskflow-v0/src/state/recovery.nim` | no blocking gap for checkpoint lineage and gateway recovery in the current transitional runtime; recovery, gateway resolution, and resumable resume are executable surfaces |
| Layer 8 | ✅ | `system-maps/observability.map.md`, `runtime-instructions.observability.trace-grading-protocol.md`, `runtime-instructions/work.task-state-reconciliation-protocol.md`, `canonical-runtime-readiness-law.md`, `codex-v0` readiness/proof checks | no blocking gap for readiness as a pre-consumption gate in the current canon |
| Layer 9 | ✅ | `runtime-instructions/runtime.direct-runtime-consumption-protocol.md`, `taskflow-v0/src/core/direct_consumption.nim`, `taskflow-v0 consume final`, `vida-0.3-db-first-runtime-spec.md` | no blocking gap for the current transitional direct runtime-consumption layer; taskflow now consumes the compiled bundle directly and activates codex explicitly for final evidence before closure trust |

Current codex-backed evidence:

1. `python3 codex-v0/codex.py overview --profile active-canon` -> `177` active files, `0` missing footers, `0` missing changelogs
2. `python3 codex-v0/codex.py readiness-check --profile active-canon` -> `OK`
3. `python3 codex-v0/codex.py proofcheck --profile active-canon` -> `OK`

Interpretation rule:

1. these proofs show that the current canon is structurally consistent enough to support a runtime-layer matrix,
2. they do not by themselves promote a `🟡` runtime layer to `✅` when the law is still intentionally draft or future-bounded.

## 0.2 Three-Level Kernel Protocol Spine

This matrix is anchored to the current three-level framework kernel:

1. `core`
   - authoritative owner for foundational runtime law, admissibility vocabulary, run-graph/state continuity, and protocol-binding placement
2. `orchestration shell`
   - authoritative owner for lane entry, routing, dispatch, handoff shaping, and verification posture around runtime execution
3. `runtime-family execution`
   - authoritative owner for concrete `TaskFlow` execution, recovery, observability, readiness, and direct runtime consumption

Runtime matrix rule:

1. each runtime layer must name which part of the three-level kernel gives it authority,
2. no layer may pull ownership upward from `runtime-family execution` into `core`,
3. no layer may hide a shell concern inside execution-state law,
4. cross-runtime closure at `TaskFlow -> DocFlow` must remain explicit rather than implied by shared vocabulary.

Primary owner references for the three-level spine:

1. `docs/process/framework-three-layer-refactoring-audit.md`
2. `vida/config/instructions/instruction-contracts/meta.core-protocol-standard-protocol.md`
3. `docs/product/spec/framework-project-documentation-layer-model.md`
4. `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`
5. `docs/process/framework-source-lineage-index.md`
6. `vida/config/instructions/system-maps/framework.protocol-layers-map.md`
7. `docs/product/spec/taskflow-protocol-runtime-binding-model.md`
8. `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`

Upper-layer derivation rule:

1. `meta.core-protocol-standard-protocol.md` defines what foundational kernel protocols may own,
2. `framework-project-documentation-layer-model.md` defines where those protocols and adjacent project docs must live,
3. `compiled-autonomous-delivery-runtime-architecture.md` defines how the layered kernel is compiled into the target runtime,
4. this matrix is the runtime capability projection of that higher-level packet rather than an isolated standalone diagram.

## 0.3 Runtime Operational Control Matrix

This control matrix strengthens the layer status view with owner, proof, and migration surfaces required by `functional-matrix-protocol.md`.

| Layer | Three-level owner | Owner docs | Owner code surface | Operator and proof surface | Migration posture | Fail-closed failure mode | Main current gap |
|---|---|---|---|---|---|---|---|
| Layer 1 | `core` | `partial-development-kernel-model.md`; `canonical-machine-map.md`; `taskflow-protocol-runtime-binding-model.md` | `taskflow-v0/src/core/runtime_bundle.nim`; runtime bundle composition surfaces | runtime bundle validation; `codex.py proofcheck --profile active-canon` | `bridge_backed` | runtime bundle boundary becomes ambiguous and later layers lose lawful owner splits | Rust-native kernel bundle ownership is not yet the sole execution path |
| Layer 2 | `core` | `vida-0.3-route-and-receipt-spec.md`; `receipt-and-proof-law.md`; `canonical-machine-map.md` | `taskflow-v0` route/state/receipt surfaces; DB-backed taskflow state authority | route/receipt proofs; runtime bundle composition checks | `bridge_backed` | route stages, receipt classes, or proof boundaries become non-queryable and runtime must stop before trustworthy progression | DB-first compiled Rust authority is still converging with script-era route/state law |
| Layer 3 | `runtime-family execution` | `work.taskflow-protocol.md`; `model.boot-packet-protocol.md`; `taskflow-v1-runtime-modernization-plan.md` | `taskflow-v0/**`; active Rust `taskflow-*` crates for native closure | tracked execution receipts; run-graph proofs; bounded task execution inspection | `bridge_backed` | tracked execution cannot prove block lifecycle or resumable progression and execution must not claim lawful continuity | Rust-native TaskFlow execution substrate is under active implementation beside bridge runtime |
| Layer 4 | `orchestration shell` | `core.agent-system-protocol.md`; `core.capability-registry-protocol.md`; `work.agent-lane-selection-protocol.md` | `taskflow-v0/src/agents/route.nim`; `taskflow-v0/src/core/assignment_engine.nim`; `crates/vida/src/main.rs` routing shell | route receipts; capability registry checks; lane-selection proofs | `bridge_backed` | lane assignment becomes implicit or role drift bypasses lawful route selection | `vida` shell still concentrates routing integration while Rust TaskFlow boundaries are being carved out |
| Layer 5 | `orchestration shell` | `lane.worker-dispatch-protocol.md`; `lane.agent-handoff-context-protocol.md`; `core.context-governance-protocol.md` | packet/handoff surfaces in `taskflow-v0`; launcher-owned delegation bridges in `crates/vida` | handoff packets; bounded context receipts; verifier packet proofs | `bridge_backed` | delegation inherits broad context or loses provenance and replay-safe execution must stop | Native handoff/context governance still depends on bridge-era packet surfaces |
| Layer 6 | `orchestration shell` + `runtime-family execution` gate seam | `work.verification-lane-protocol.md`; `work.verification-merge-protocol.md`; `verification-merge-law.md` | `taskflow-v0/src/gates/verification_merge.nim`; Rust review/verification closure remains in active modernization track | admissibility checks; merge proofs; approval/verification receipts | `bridge_backed` | verification, approval, and proof collapse into one informal close path and runtime must fail closed | Native closure gates are not yet the sole owner for end-to-end TaskFlow verification |
| Layer 7 | `runtime-family execution` | `core.run-graph-protocol.md`; `recovery.checkpoint-replay-recovery-protocol.md`; `taskflow-v1-runtime-modernization-plan.md` | `taskflow-v0/src/state/recovery.nim`; active Rust recovery/state crates | checkpoint lineage; replay-safe recovery checks; resumability proofs | `bridge_backed` | interruption-safe recovery is no longer provable and runtime may not advertise resumability | Rust-native checkpoint and replay ownership is still being completed |
| Layer 8 | `runtime-family execution` | `observability.map.md`; `canonical-runtime-readiness-law.md`; `operational-state-and-synchronization-model.md` | `codex-v0`; `taskflow-v0` observability/readiness surfaces; future Rust readiness shell in `vida`/`taskflow-*` | `codex.py readiness-check`; `proofcheck`; readiness reports and reconciliation views | `bridge_backed` | readiness cannot prove trust and direct runtime use must remain blocked | readiness is green canonically, but native TaskFlow observability/readiness closure is still converging |
| Layer 9 | `runtime-family execution` + `DocFlow` activation seam | `runtime.direct-runtime-consumption-protocol.md`; `compiled-autonomous-delivery-runtime-architecture.md`; `taskflow-v1-runtime-modernization-plan.md`; `docflow-v1-runtime-modernization-plan.md` | `taskflow-v0/src/core/direct_consumption.nim`; `crates/vida/src/main.rs`; active Rust `taskflow-*` and `docflow-*` integration surfaces | `taskflow-v0 consume final`; `codex.py proofcheck`; explicit `DocFlow` activation for final evidence | `bridge_backed` | runtime consumes incomplete canon or bypasses `DocFlow` evidence and closure trust must fail closed | `TaskFlow -> DocFlow` direct native seam is still converging while both Rust families are under active development |

## 1. Scope

This spec defines:

1. the canonical runtime capability layers,
2. the required standalone value of each layer,
3. the lower-layer dependency rule,
4. the current completion status of each layer,
5. the strongest current evidence and remaining blockers.

This spec does not define:

1. one specific vendor workflow engine,
2. one specific provider/backend transport,
3. the final implementation details of `VIDA 1.0`,
4. project-specific host commands or runtime adapters outside the canonical runtime law.

## 2. Runtime Layering Rule

Each runtime layer must satisfy all of the following:

1. it must deliver standalone runtime value before later layers exist,
2. it must depend only on already-closed or already-adopted lower layers,
3. it must not borrow authority from a later unfinished layer,
4. it must expose one coherent bundle of capability rather than a scattered feature pile,
5. it must have a bounded proof or bounded evidence surface,
6. it must be inspectable through its own bounded operator surface,
7. it must fail closed when its owned prerequisites or proofs are missing.

Compact rule:

1. each next runtime layer deepens the system,
2. no next runtime layer may smuggle in unfinished future-runtime behavior as if it were already closed law.

## 3. Runtime Research Closure Notes

The following external research questions are treated as closed enough for the current runtime-layer model:

1. bounded supervisor-to-worker handoff and context filtering:
   - OpenAI Agents SDK handoffs
   - LangGraph supervisor/handoff model
2. durable execution, replay safety, and resumability:
   - Temporal durable execution
   - LangGraph checkpointers and pending writes
   - Eventuous checkpoint guidance
3. bookmark/correlation/gateway resume targeting:
   - Elsa blocking activities, bookmarks, triggers, and event correlation
4. human-in-the-loop and fan-in/fan-out gating:
   - Elsa workflow patterns for approvals, fan-in, and idempotent external signals

Closed implications:

1. explicit handoffs are stronger than hidden transcript inheritance,
2. checkpoint/replay/idempotency are runtime-law concerns, not only implementation details,
3. correlation-based resume targeting is stronger than broad-scan resume,
4. `coach`, `verification`, and `approval` must remain separate gate semantics even when all are resumable or parallelizable.

Primary external links:

1. OpenAI Agents SDK overview:
   - https://developers.openai.com/api/docs/guides/agents-sdk
2. OpenAI Agents SDK handoffs:
   - https://openai.github.io/openai-agents-js/guides/handoffs/
3. LangGraph supervisor:
   - https://langchain-ai.github.io/langgraphjs/reference/modules/langgraph-supervisor.html
4. LangGraph checkpointing:
   - https://langchain-ai.github.io/langgraphjs/reference/modules/langgraph-checkpoint.html
5. Temporal docs:
   - https://docs.temporal.io/
6. Eventuous checkpoints:
   - https://eventuous.dev/docs/subscriptions/checkpoint/
7. Eventuous diagnostics:
   - https://eventuous.dev/docs/subscriptions/subs-diagnostics/
8. Elsa blocking activities and triggers:
   - https://docs.elsaworkflows.io/activities/blocking-and-triggers
9. Elsa workflow patterns:
   - https://docs.elsaworkflows.io/guides/patterns

## 4. Layer 1: Runtime Kernel Boundary

### 4.1 Purpose

Freeze the runtime ownership split between canonical config, canonical state, receipts, proofs, projections, checkpoints, and runtime adapters.

### 4.2 Must Define

1. what is product/runtime law,
2. what is state,
3. what is receipt/proof,
4. what is projection,
5. what is runtime-owned but non-canonical.

### 4.3 Strongest Current Evidence

1. `docs/product/spec/partial-development-kernel-model.md`
2. `docs/product/spec/canonical-machine-map.md`
3. `docs/process/framework-source-lineage-index.md`

### 4.4 Standalone Value

1. this layer now provides one closed runtime kernel boundary through canonical kernel specs plus executable bundle composition and readiness validation.

## 5. Layer 2: State, Route, And Receipt Kernel

### 5.1 Purpose

Define the lawful runtime state machine, route-stage law, gate semantics, and proof boundary without collapsing them into one blob.

### 5.2 Must Define

1. canonical task and route stages,
2. route authorization and gate law,
3. receipt/proof taxonomy,
4. machine ownership boundaries.

### 5.3 Strongest Current Evidence

1. `docs/process/framework-source-lineage-index.md`
2. `docs/product/spec/canonical-machine-map.md`
3. `docs/product/spec/receipt-and-proof-law.md`

### 5.4 Standalone Value

1. this layer now provides one inspectable state/route/receipt kernel that taskflow can consume through the runtime bundle without waiting for later layers.

## 6. Layer 3: Tracked Execution Substrate

### 6.1 Purpose

Provide the bounded runtime that records how one task is executed, resumed, and inspected without replacing task truth.

### 6.2 Must Define

1. TaskFlow block lifecycle,
2. execution-plan semantics,
3. pack/block registration,
4. boot packet and compact resumability helpers.

### 6.3 Strongest Current Evidence

1. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
2. `vida/config/instructions/runtime-instructions/model.boot-packet-protocol.md`
3. `taskflow-v0/**`

### 6.4 Standalone Value

1. this layer already provides a lawful way to execute and inspect work even without dynamic role expansion or direct DB-first runtime completion.

## 7. Layer 4: Lane Routing And Assignment

### 7.1 Purpose

Select the lawful lane, agent backend, and route receipt for each task class or execution slice.

### 7.2 Must Define

1. route snapshot/receipt semantics,
2. capability compatibility rules,
3. assignment engine behavior,
4. lane independence and route-task-class binding.

### 7.3 Strongest Current Evidence

1. `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`
2. `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md`
3. `taskflow-v0/src/agents/route.nim`
4. `taskflow-v0/src/core/assignment_engine.nim`
5. `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`
6. `vida/config/instructions/runtime-instructions/work.agent-lane-selection-protocol.md`
7. `taskflow-v0/src/core/role_selection.nim`

### 7.4 Standalone Value

1. this layer now provides one executable runtime-owned path for lane assignment, project extension bundle consumption, and bounded auto-lane selection before tracked handoff.

## 8. Layer 5: Handoff And Context Governance

### 8.1 Purpose

Make delegation lawful, bounded, and replay-safe by using explicit packets and governed context instead of broad inherited chat context.

### 8.2 Must Define

1. worker packet contract,
2. handoff shape and context filtering,
3. context provenance and freshness classes,
4. scope-in/scope-out and verification boundary.

### 8.3 Strongest Current Evidence

1. `vida/config/instructions/instruction-contracts/lane.worker-dispatch-protocol.md`
2. `vida/config/instructions/runtime-instructions/lane.agent-handoff-context-protocol.md`
3. `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`

### 8.4 Standalone Value

1. this layer already gives VIDA a bounded multi-lane delegation model without requiring final runtime consumption or remote federation.

## 9. Layer 6: Review, Verification, And Approval Gates

### 9.1 Purpose

Keep formative critique, independent verification, proof obligations, and approval semantics explicit and non-interchangeable.

### 9.2 Must Define

1. `coach` lane semantics,
2. `verifier` lane semantics,
3. approval/post-governance semantics,
4. proof and closure boundary,
5. aggregation/merge admissibility when multiple verifiers participate.

### 9.3 Strongest Current Evidence

1. `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md`
2. `vida/config/instructions/runtime-instructions/work.verification-merge-protocol.md`
3. `vida/config/instructions/runtime-instructions/work.human-approval-protocol.md`
4. `docs/product/spec/verification-merge-law.md`
5. `taskflow-v0/src/gates/verification_merge.nim`

### 9.4 Standalone Value

1. this layer now provides executable verifier admissibility and merge behavior while keeping coach, verification, approval, and closure authority distinct.

## 10. Layer 7: Resumability, Checkpoint, And Gateway Recovery

### 10.1 Purpose

Let routed execution survive interruption, restart, and long-running waits through explicit runtime artifacts rather than operator memory.

### 10.2 Must Define

1. run-graph resumability,
2. checkpoint/replay boundaries,
3. idempotent retry expectations,
4. gateway resume targeting and consumption policy.

### 10.3 Strongest Current Evidence

1. `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md`
2. `vida/config/instructions/runtime-instructions/recovery.checkpoint-replay-recovery-protocol.md`
3. `docs/product/spec/projection-listener-checkpoint-model.md`
4. `docs/product/spec/gateway-resume-handle-and-trigger-index.md`
5. `taskflow-v0/src/state/recovery.nim`

### 10.4 Standalone Value

1. this layer now provides executable checkpoint commits, gateway-handle resolution, resumable status, and fail-closed resume payloads for the current transitional runtime.

## 11. Layer 8: Observability And Runtime Readiness

### 11.1 Purpose

Expose the runtime health, proving, trace, reconciliation, and readiness gate needed before trusting runtime use or closure.

### 11.2 Must Define

1. observability discovery,
2. trace/proving surfaces,
3. reconciliation and drift diagnosis,
4. readiness verdicts and blockers.

### 11.3 Strongest Current Evidence

1. `vida/config/instructions/system-maps/observability.map.md`
2. `vida/config/instructions/runtime-instructions/observability.trace-grading-protocol.md`
3. `vida/config/instructions/runtime-instructions/work.task-state-reconciliation-protocol.md`
4. `docs/product/spec/canonical-runtime-readiness-law.md`
5. `python3 codex-v0/codex.py readiness-check --profile active-canon`

### 11.4 Standalone Value

1. this layer already gives VIDA a fail-closed runtime trust gate before direct runtime consumption exists.

## 12. Layer 9: Direct Runtime Consumption

### 12.1 Purpose

Allow the runtime to consume canonical law, canonical bundles, and canonical state directly, with bridge files reduced to export or migration-only posture.

### 12.2 Must Define

1. DB-first runtime truth,
2. canonical load/ingest from bundles and projections,
3. boot/migration fail-closed consumption path,
4. export-only treatment of bridge files,
5. explicit activation of the bounded `DocFlow` runtime-family surface as the canonical documentation/inventory/readiness branch consumed by the final `taskflow` layer.

### 12.3 Strongest Current Evidence

1. `docs/process/framework-source-lineage-index.md`
2. `docs/product/spec/root-map-and-runtime-surface-model.md`
3. `vida/config/instructions/bundles/default_runtime.yaml`
4. `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
5. `taskflow-v0/src/core/direct_consumption.nim`
6. `taskflow-v0 consume final`

### 12.4 Standalone Value

1. this layer now provides one executable direct-consumption path where taskflow consumes the compiled runtime bundle and activates the bounded DocFlow branch explicitly before final trust/closure.

### 12.5 Cross-Runtime Activation Rule

1. when `taskflow` enters Layer 9 direct runtime consumption or evaluates final runtime-trust closure for that layer, it must activate the bounded `DocFlow` runtime-family surface,
2. this activation exists so `taskflow` consumes the canonical documentation/inventory/runtime-readiness surfaces through one explicit downstream branch instead of inferring them from local files or ad hoc helper behavior,
3. `DocFlow` remains the bounded documentation/inventory/readiness resolution surface, but `taskflow` remains the primary runtime-consumption owner and closure authority,
4. the direct runtime-consumption layer is not considered fully wired unless the `taskflow` -> `docflow` final-layer activation path is explicit and testable.

## 13. Runtime Gap Summary

The highest-value remaining runtime gaps are:

1. keep Layer 1 and Layer 2 kernel bundle consumption aligned with canonical specs rather than letting runtime drift back into overlay-only shortcuts,
2. keep Layer 4 executable role/flow selection aligned with direct runtime consumption rather than letting it drift back into overlay-only behavior,
3. keep Layer 6 verifier admissibility and merge law explicit if richer ensemble policies are added later,
4. keep Layer 7 recovery surfaces fail-closed if delayed checkpoint progression or richer gateway policies are added later,
5. keep Layer 9 direct runtime consumption aligned with the future DB-first substrate without weakening the current explicit `taskflow` -> `docflow` bridge.

## 14. Source Absorption

This spec absorbs and concentrates runtime-layer law previously scattered across:

1. `docs/process/framework-source-lineage-index.md`
2. `docs/process/framework-source-lineage-index.md`
3. `docs/product/spec/partial-development-kernel-model.md`
4. `docs/product/spec/canonical-machine-map.md`
5. `docs/product/spec/projection-listener-checkpoint-model.md`
6. `docs/product/spec/gateway-resume-handle-and-trigger-index.md`
7. `docs/product/spec/verification-merge-law.md`
8. `docs/product/spec/canonical-runtime-readiness-law.md`
9. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
10. `vida/config/instructions/runtime-instructions/lane.agent-handoff-context-protocol.md`
11. `vida/config/instructions/runtime-instructions/recovery.checkpoint-replay-recovery-protocol.md`
12. `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md`
13. `vida/config/instructions/system-maps/observability.map.md`
14. `docs/process/framework-source-lineage-index.md`

-----
artifact_path: product/spec/canonical-runtime-layer-matrix
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/canonical-runtime-layer-matrix.md
created_at: '2026-03-10T15:01:10+02:00'
updated_at: '2026-03-13T09:09:25+02:00'
changelog_ref: canonical-runtime-layer-matrix.changelog.jsonl
