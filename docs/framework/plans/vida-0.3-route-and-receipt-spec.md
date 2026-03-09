# VIDA 0.3 Route And Receipt Spec

Purpose: define the authoritative route and receipt model for direct `1.0`, freeze route-owned semantic law, and separate route/receipt proof from state facts, instruction behavior, migration compatibility, and discardable `0.1` topology.

Status: canonical `0.3` route-and-receipt spec artifact for the direct `1.0` program, completed through `Part A` route law and `Part B` receipt/proof boundary on 2026-03-08.

Date: 2026-03-08

---

## 1. Executive Decision

The direct `1.0` route-and-receipt kernel owns:

1. route stages,
2. authorization and progression gates,
3. lane-boundary law across analysis, writer, coach, verification, approval, and synthesis,
4. the semantic split between state-owned facts and route/receipt-owned proof,
5. fail-closed behavior when route law is incomplete.

This `Part A` step freezes the route-law boundary first.

It does **not** yet freeze:

1. detailed receipt families,
2. exact receipt payload schemas,
3. run-graph attachment detail,
4. operator visibility detail,
5. final closure-ready proof surfaces.

Compact rule:

`freeze route law, keep facts in state, keep proof in route/receipt, fail closed on missing authorization law, defer receipt detail explicitly`

---

## 2. Why This Spec Comes Next

The command-tree spec already froze the future operator families, the state-kernel spec already froze authoritative workflow/runtime facts, the instruction-kernel spec already froze behavior ownership, and the migration-kernel spec already froze compatibility and boot fail-closed law.

The next blocker is the missing route law that answers:

1. how work is authorized to move across execution lanes,
2. which gates must be satisfied before mutation, verification, approval, or synthesis,
3. what route owns as proof rather than state,
4. where route ownership ends so later receipt detail does not swallow command, state, instruction, or migration law.

Without this artifact:

1. receipts could become a shadow state model,
2. route-stage authorization could remain implicit,
3. later proof surfaces could be defined without a stable route law,
4. parity would not know which route semantics are canonical.

---

## 3. Source Basis

Primary local source basis:

1. `AGENTS.md`
2. `vida/config/instructions/agent-definitions.orchestrator-entry.md`
3. `vida/config/instructions/instruction-contracts.thinking-protocol.md`
4. `vida.config.yaml`
5. `docs/framework/history/research/2026-03-08-agentic-master-index.md`
6. `docs/framework/history/research/2026-03-08-vida-direct-1.0-next-agent-compact-instruction.md`
7. `docs/framework/history/research/2026-03-08-vida-route-and-receipt-next-step-after-compact-instruction.md`
8. `docs/framework/history/plans/2026-03-08-vida-direct-1.0-compact-continuation-plan.md`
9. `docs/framework/history/plans/2026-03-08-vida-0.3-command-tree-spec.md`
10. `docs/framework/history/plans/2026-03-08-vida-0.3-state-kernel-schema-spec.md`
11. `docs/framework/history/plans/2026-03-08-vida-0.3-instruction-kernel-spec.md`
12. `docs/framework/history/plans/2026-03-08-vida-0.3-migration-kernel-spec.md`
13. `docs/framework/history/plans/2026-03-08-vida-0.1-to-1.0-direct-binary-transition-plan.md`
14. `docs/framework/history/plans/2026-03-08-vida-direct-1.0-local-spec-program.md`
15. `docs/framework/history/research/2026-03-08-agentic-agent-definition-system.md`
16. `vida/config/instructions/agent-definitions.protocol.md`
17. `vida/config/instructions/instruction-contracts.agent-system-protocol.md`

Bounded explorer lanes used for synthesis:

1. route stages, authorization gates, lane boundaries, and fail-closed route law,
2. state-owned facts versus route/receipt-owned proof boundary,
3. safe deferral boundary from `Part A` into `Route/Receipt Part B`.

Source synthesis rule:

1. command law defines the operator homes route must serve,
2. state law defines the facts route must not absorb,
3. instruction law defines behavior/fallback/escalation/proof semantics route must not steal,
4. migration law defines compatibility and boot law route must not redefine,
5. the agent system protocol supplies the minimum route-stage and fail-closed authorization semantics.

---

## 4. Purpose And Current Completion Boundary

This artifact answers one question:

1. what route and receipt semantics must the direct `1.0` binary own so execution is authorized, inspectable, and fail-closed without turning proof into state or topology into product law?

Current completion boundary:

1. `Part A` is authoritative for route law,
2. `Part B` is authoritative for receipt/proof categories, run-graph attachment semantics, and operator visibility boundaries,
3. parity thresholds and conformance verdict policy remain downstream.

This `Part A` step defines:

1. route-owned semantics,
2. route stages and authorization gates,
3. lane responsibilities,
4. fail-closed route law,
5. the state/proof ownership boundary,
6. kernel boundaries against command/state/instruction/migration,
7. invariants, non-goals, deferred ambiguities, and next contracts.

This `Part A` step does not define:

1. exact receipt payloads,
2. exact receipt family inventory,
3. final closure-ready proof taxonomy,
4. exact operator visibility output shapes,
5. exact Rust/runtime topology.

---

## 5. Route-Owned Semantics

The route-and-receipt kernel owns the semantic law for:

1. whether a route stage is authorized to start,
2. whether a route stage may progress to the next stage,
3. whether a stage is blocked on missing analysis, verification, approval, or other required gates,
4. which lane owns authorship versus coaching, verification, approval, and synthesis,
5. which proof category is required to justify progression.

The route-and-receipt kernel does **not** own:

1. root command-family law,
2. authoritative task or run facts,
3. instruction-owned fallback or escalation logic,
4. migration compatibility law or boot fail-closed posture,
5. provider, shell, path, or storage topology.

Route-owned law rule:

1. route semantics authorize or block progression,
2. route semantics do not replace the fact model owned by state,
3. route semantics do not replace behavior law owned by instruction,
4. route semantics do not replace compatibility law owned by migration.

---

## 6. Authoritative Route Stages And Gates

### 6.1 Canonical Route Stages

The minimum route stages frozen by `Part A` are:

1. `analysis`
2. `writer`
3. `coach`
4. `verification`
5. `approval`
6. `synthesis`

Stage rule:

1. `coach` is conditional and exists only when the route requires it,
2. `approval` is conditional and exists only when governance/review policy requires it,
3. omitted conditional stages must still resolve to explicit `not_required` route posture rather than silent omission,
4. `synthesis` is the final integration/reporting stage and stays under orchestrator ownership.

### 6.2 Stage Purpose

| Stage | Purpose | Must not do |
|---|---|---|
| `analysis` | form bounded evidence, prepare route understanding, answer the blocking question when analysis is required | mutate shared product state as the authoring lane |
| `writer` | perform the single mutation-owning or artifact-authoring pass | bypass analysis, verification, or approval gates |
| `coach` | run post-write formative review when required and either approve forward or return for rework | replace final independent verification |
| `verification` | perform independent validation when required | silently collapse into the writer lane when an eligible verifier exists |
| `approval` | satisfy policy/senior/human approval when review law requires it | replace technical verification |
| `synthesis` | integrate route evidence into the user-facing or closure-facing verdict | invent missing route receipts or bypass blockers |

### 6.3 Authorization Gates

The minimum authorization gates frozen by `Part A` are:

1. writer is not authorized before required analysis is complete,
2. writer is not authorized when issue-driven work lacks a `writer_ready` issue contract,
3. mutation is not authorized until writer ownership and local execution authorization are explicit,
4. synthesis is not authorized before required verification is complete,
5. closure-ready posture is not authorized while required approval is still pending,
6. any stage transition that depends on unresolved law-bearing route fields is invalid,
7. if no lawful fallback exists, or required escalation cannot proceed, the route fails closed.

### 6.4 Fail-Closed Rule

Route law must fail closed when any law-bearing route field used for authorization or gating is unresolved.

Minimum law-bearing categories for `Part A`:

1. whether analysis is required,
2. whether writer authorization exists,
3. whether coach review is required,
4. whether independent verification is required,
5. whether approval is required,
6. whether local mutation execution is authorized,
7. whether a lawful fallback or escalation path still exists.

Fail-closed rule:

1. unresolved route law is blocking state,
2. blocking state is not soft warning,
3. synthesis must not convert unresolved route law into advisory prose and continue anyway.

---

## 7. Lane Responsibilities

### 7.1 Analysis Lane

Analysis is evidence-forming, bounded, and may use fanout or read-only worker lanes when authorized.

Minimum responsibilities:

1. answer the blocking question for the route,
2. produce the analysis basis required for writer authorization,
3. remain non-authoring unless the route explicitly grants mutation ownership.

### 7.2 Writer Lane

Writer is the single mutation-owning lane.

Minimum responsibilities:

1. create or update the shared artifact or authorized runtime target,
2. stay inside explicit write scope,
3. respect the route and proof gates instead of improvising progress.

### 7.3 Coach Lane

Coach is the post-write formative gate when required.

Minimum responsibilities:

1. review against the original contract/spec,
2. either approve forward or return the work for rework,
3. stay distinct from final verification.

### 7.4 Verification Lane

Verification is the independent validation lane when required.

Minimum responsibilities:

1. validate the authored result against the route, contract, and proof expectations,
2. stay independent from authorship when an eligible verifier exists,
3. block progression when the required proof is missing or invalid.

### 7.5 Approval Lane

Approval is separate from technical verification.

Minimum responsibilities:

1. satisfy policy, senior, or human review obligations when present,
2. block closure-ready posture until approval is satisfied or explicitly not required.

### 7.6 Synthesis Lane

Synthesis stays with the orchestrator.

Minimum responsibilities:

1. integrate evidence and receipts into the final verdict,
2. report route blockers and unresolved ambiguities honestly,
3. keep raw worker output as evidence, not as the default final deliverable.

---

## 8. Boundary Between State Facts And Route/Receipt Proof

### 8.1 State-Owned Facts

The following remain canonical state-owned facts:

1. task/workflow identity and class,
2. lifecycle state,
3. dependency edges,
4. blocker posture,
5. current review-state requirement,
6. execution-step status and `next_step`,
7. run and run-node state,
8. resumability hints and capsules,
9. reconciliation/readiness summaries,
10. migration-state classification and boot/doctor go-no-go posture.

### 8.2 Route/Receipt-Owned Proof

The following may be route/receipt-owned proof surfaces:

1. authorization proof that a stage transition was lawful,
2. progression proof that the route moved lawfully across stages,
3. coach, verification, approval, escalation, or closure-related evidence,
4. proof references attached to blockers or readiness decisions.

### 8.3 Ownership Rules

The ownership split frozen by `Part A` is:

1. state owns current facts and statuses,
2. route/receipt owns evidence that authorized, blocked, or justified progression,
3. route receipts must prove authorization and progression without replacing state facts,
4. proof may explain or justify a state fact, but it must not become the new canonical fact store,
5. run-graph structure stays state-owned in `Part A`; receipt/run-node attachment detail is deferred to `Part B`.

Shadow-state prohibition:

1. route receipts must not encode the authoritative current workflow or governance state as a parallel shadow model.

---

## 9. Boundary Against Other Kernels

### 9.1 Command Boundary

Route law serves the frozen operator families:

1. `vida task ...` is the normal mutation-owning command family,
2. `vida status`, `vida boot`, and `vida doctor` may observe or summarize route posture,
3. route law must not redefine the command tree or root-family semantics.

### 9.2 State Boundary

State owns:

1. authoritative facts,
2. lifecycle, dependency, blocker, execution-plan, run, and governance posture.

Route owns:

1. progression authorization and proof categories around those facts.

### 9.3 Instruction Boundary

Instruction owns:

1. behavioral logic,
2. fallback ladders,
3. escalation rules,
4. output contract,
5. proof requirements inside role behavior.

Route owns:

1. whether the route stage may advance or must block based on those requirements.

Instruction/route rule:

1. route may gate on instruction-owned obligations,
2. route must not redefine those obligations.

### 9.4 Migration Boundary

Migration owns:

1. compatibility law,
2. startup fail-closed boot posture,
3. migration proof families,
4. rollback notes and cutover preconditions.

Route owns:

1. stage authorization during normal routed execution after those migration gates are satisfied.

Migration/route rule:

1. route must not absorb migration compatibility or boot law.

---

## 10. Part A Invariants

1. Route law must fail closed when law-bearing authorization fields are missing.
2. Writer ownership remains singular.
3. Authorship, coaching, verification, approval, and synthesis remain semantically distinct lanes.
4. State facts stay canonical even when route proof exists.
5. Route receipts prove progression; they do not replace state.
6. Raw worker output is evidence, not the default final deliverable.
7. Conditional stages may be skipped only through explicit `not_required` resolution, not silent omission.

---

## 11. Non-Goals

1. This `Part A` step does not finalize detailed receipt families.
2. This `Part A` step does not finalize run-graph attachment detail.
3. This `Part A` step does not finalize operator visibility surfaces for `boot|task|status|doctor`.
4. This `Part A` step does not define final closure-ready proof payloads.
5. This `Part A` step does not redefine command/state/instruction/migration law.
6. This `Part A` step does not authorize implementation topology, storage layout, or provider transport decisions.

---

## 12. Explicit Deferral To Route/Receipt Part B

The following are intentionally deferred to `Route/Receipt Part B`:

1. detailed receipt families and family inventory,
2. exact relationship between route receipts and run-graph nodes,
3. approval proof surfaces in detailed form,
4. escalation proof surfaces in detailed form,
5. verification proof surfaces in detailed form,
6. closure-ready proof surfaces in detailed form,
7. operator visibility boundaries and route/receipt visibility detail.

Deferral rule:

1. these topics are downstream of the route law frozen here,
2. they must extend this route law rather than redefine it.

---

## 13. Downstream Contracts Unlocked By Part A

Completing `Part A` unlocks:

1. `Route/Receipt Part B` receipt-family and visibility work on a stable route-law base,
2. parity fixture scoping for route-stage evidence,
3. conformance work that can test route authorization semantics separately from receipt payload detail,
4. later implementation planning that can assume route-stage and ownership law are not moving.

---

## 14. Open Ambiguities At The Part A Boundary

1. The exact future route-stage naming may still be normalized later, but the semantic stages frozen here must remain.
2. The exact mapping between route progression proof and run-graph node attachment is deferred to `Part B`.
3. The exact detailed relationship between `closure_ready` posture and approval/verification proof categories is deferred to `Part B`.

---

## 15. Part B Completion Update

`Part B` completes the receipt/proof boundary on top of the already-frozen `Part A` route law.

This step freezes:

1. the minimal semantic receipt/artifact families,
2. the semantic relationship between route receipts and run-graph state,
3. operator visibility boundaries across `boot|status|doctor|task`,
4. approval, escalation, verification, and closure-ready proof categories,
5. the remaining invariants and non-goals before parity.

This step does **not** freeze:

1. exact receipt payload schemas,
2. exact operator output rendering,
3. parity thresholds or verdict rules,
4. migration receipt payload specifics,
5. implementation topology.

---

## 16. Semantic Receipt And Artifact Families

The minimum semantic receipt/artifact families frozen by `Part B` are:

1. `analysis receipt`
2. `route progression receipt`
3. `escalation receipt`
4. `coach outcome artifact`
5. `verification artifact family`
6. `approval receipt family`
7. `closure-ready proof artifact`
8. `run-node attachment references`

### 16.1 Family Meanings

| Family | Semantic purpose | Must not become |
|---|---|---|
| `analysis receipt` | prove that required analysis completed and writer authorization may proceed | authoritative run or task state |
| `route progression receipt` | prove that a stage transition was lawfully authorized and executed | canonical current stage state |
| `escalation receipt` | prove that lawful escalation or fail-closed posture was followed | instruction-owned escalation logic |
| `coach outcome artifact` | prove `coach_approved` or structured return-for-rework when coach is required | final independent verification |
| `verification artifact family` | prove that independent validation ran when required and what verdict it produced | authoritative readiness state |
| `approval receipt family` | prove policy/senior/human approval when required | governance state itself |
| `closure-ready proof artifact` | prove that all route-required gates for closure/synthesis are satisfied | canonical closure state |
| `run-node attachment references` | attach proof artifacts to routed run/node state semantically | run-graph ownership |

### 16.2 Ownership Boundary

Ownership is frozen as:

1. route/receipt owns proof artifacts and semantic proof categories,
2. state owns current facts, statuses, review posture, run identities, run-node identities, and resumability posture,
3. route/receipt may attach proof to state-owned run entities, but must not become the owner of run state,
4. migration keeps ownership of migration-specific receipts and cutover proof,
5. instruction keeps ownership of the logic that determines fallback, escalation, output, and proof obligations.

Shadow-state prohibition:

1. no receipt family may become the authoritative source for workflow, governance, or migration state.

---

## 17. Relationship Between Receipts, Run-Graph State, And Operator Visibility

### 17.1 Run-Graph Relationship

The semantic relationship is:

1. run-graph structure remains state-owned,
2. route receipts are proof-bearing companions to routed run and run-node state,
3. receipts explain why progression, blockage, verification, approval, or closure-ready posture was lawful or blocked,
4. receipts do not define the authoritative current node or current task posture.

### 17.2 Attachment Rule

Attachment is frozen semantically as:

1. route proof artifacts may reference routed runs or specific run nodes,
2. the attachment proves why a node transition or gate posture exists,
3. the attachment must remain referential rather than ownership-transferring,
4. exact reference fields and serialization remain open.

### 17.3 Operator Visibility Boundaries

`vida boot` may expose:

1. compact startup classification,
2. resumable next-step guidance,
3. boot-time blockers and the proof category causing the block.

`vida status` may expose:

1. read-only task and run summary,
2. readiness/blocker posture,
3. summarized approval/verification counts or categories,
4. receipt-derived summary signals grounded in authoritative state.

`vida doctor` may expose:

1. detailed blocker diagnosis,
2. evidence-driven explanation for route, approval, verification, or migration blockage,
3. proof references relevant to why normal progression is blocked.

`vida task ...` may expose:

1. execution-stage inspection,
2. mutation-adjacent route posture,
3. route/receipt visibility closest to the active workflow lane.

Visibility rule:

1. `boot|status|doctor` may observe, summarize, or diagnose route posture,
2. they must not silently mutate workflow state or bypass route law,
3. exact output layout remains outside this spec.

---

## 18. Proof Surface Categories

The following proof surface categories are frozen by `Part B`:

1. `approval proof`
2. `escalation proof`
3. `verification proof`
4. `closure-ready proof`
5. `proof references on blockers/readiness decisions`

### 18.1 Semantic Meaning

`approval proof` proves:

1. the required policy/senior/human approval was satisfied,
2. or that approval was explicitly `not_required`.

`escalation proof` proves:

1. the route exhausted lawful progression,
2. triggered the correct escalation path,
3. or failed closed because no lawful escalation remained.

`verification proof` proves:

1. independent validation ran when required,
2. and produced a route-valid pass/block verdict.

`closure-ready proof` proves:

1. all route-required gates for closure or final synthesis are satisfied,
2. and no unresolved blocker remains in approval, escalation, or verification.

`proof references on blockers/readiness decisions` prove:

1. why a state-facing blocker or readiness status exists,
2. while leaving the authoritative current status in state.

### 18.2 Cross-Kernel Boundary Rule

Instruction still owns:

1. the canonical `fallback_ladder`,
2. `escalation_rules`,
3. `output_contract`,
4. `proof_requirements`,
5. the `Agent Definition` / `Instruction Contract` / `Prompt Template Configuration` hierarchy.

Migration still owns:

1. startup compatibility,
2. boot fail-closed posture,
3. migration receipt families,
4. migration proof obligations,
5. rollback notes and cutover preconditions.

Route/receipt may record that those upstream requirements were satisfied, but must not redefine their logic or ownership.

---

## 19. Part B Invariants

1. Receipt families stay proof-bearing, not fact-owning.
2. Route receipts may justify run-node posture but do not own run-node state.
3. Approval proof stays distinct from verification proof.
4. Escalation proof stays distinct from instruction-owned escalation logic.
5. Closure-ready proof depends on upstream gate satisfaction and does not create a second closure-state engine.
6. Operator visibility remains downstream of state plus receipt summaries rather than becoming a second truth source.

---

## 20. Part B Non-Goals

1. `Part B` does not define exact payload schemas or field lists.
2. `Part B` does not define final operator output formatting.
3. `Part B` does not define parity thresholds or conformance verdict policy.
4. `Part B` does not redefine command/state/instruction/migration ownership.
5. `Part B` does not authorize implementation topology or serialization layout.

---

## 21. Explicit Deferral To Parity/Conformance Part A

The following are intentionally deferred to `Parity/Conformance Part A`:

1. fixture scope across command/state/instruction/migration/route outputs,
2. parity evidence basis,
3. exact-versus-intentional delta categories,
4. which route/receipt artifact families must appear in parity fixtures.

The following remain deferred beyond `Parity/Conformance Part A`:

1. final conformance thresholds,
2. cutover pass/fail gates,
3. semantic reproduction verdict rules.

---

## 22. Downstream Contracts Unlocked By Part B

Completing `Part B` unlocks:

1. `Parity/Conformance Part A` against a complete route/receipt semantic surface,
2. fixture extraction and parity evidence mapping for route-stage proof categories,
3. later implementation planning that can assume route and receipt ownership boundaries are no longer moving.

---

## 23. Remaining Open Ambiguities After Part B

1. Exact receipt payload schemas and serialization formats remain open.
2. Exact operator output rendering for `boot|status|doctor|task` remains open.
3. Final parity thresholds and cutover gates remain downstream.
-----
artifact_path: framework/plans/vida-0.3-route-and-receipt-spec
artifact_type: plan
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/plans/vida-0.3-route-and-receipt-spec.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-0.3-route-and-receipt-spec.changelog.jsonl
P26-03-09T21: 44:13Z
