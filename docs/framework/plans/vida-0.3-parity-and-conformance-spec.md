# VIDA 0.3 Parity And Conformance Spec

Purpose: define the authoritative parity and conformance model for direct `1.0`, freeze the semantic evidence surface that `1.0` must reproduce from frozen `0.1` truth, and separate semantic parity from topology, thresholds, and final cutover verdict logic.

Status: canonical `0.3` parity-and-conformance spec artifact for the direct `1.0` program, completed through `Part A` fixture/evidence boundary and `Part B` conformance/cutover boundary on 2026-03-08.

Date: 2026-03-08

---

## 1. Executive Decision

The direct `1.0` parity-and-conformance kernel owns:

1. fixture scope across the frozen semantic families,
2. canonical parity input families,
3. evidence basis hierarchy,
4. exact-versus-intentional delta categories,
5. the list of semantic outputs that must be parity-testable before thresholds are decided.

This `Part A` step freezes the parity evidence surface first.

It does **not** yet freeze:

1. final conformance matrix,
2. final thresholds and tolerances,
3. cutover proof gates,
4. semantic reproduction verdict rules,
5. release or switchover decisions.

Compact rule:

`freeze semantic evidence surface first; judge semantics, not topology; defer thresholds and cutover verdicts explicitly`

---

## 2. Why This Spec Comes Next

The semantic freeze, bridge policy, command/state/instruction/migration specs, and the completed route-and-receipt spec already define what `1.0` must preserve and how semantic ownership is divided.

The next blocker is the missing parity law that answers:

1. which semantic families must be testable,
2. which fixtures and canonical inputs count as parity evidence,
3. how exact versus intentional divergence is classified,
4. what the conformance layer will later judge.

Without this artifact:

1. conformance thresholds would be set against an underspecified evidence base,
2. topology-heavy carriers could be mistaken for parity truth,
3. cutover discussion could begin before fixture scope is frozen,
4. implementation waves would lack a stable semantic test target.

---

## 3. Source Basis

Primary local source basis:

1. `AGENTS.md`
2. `vida/config/instructions/agent-definitions.orchestrator-entry.md`
3. `vida/config/instructions/instruction-contracts.thinking-protocol.md`
4. `vida.config.yaml`
5. `docs/framework/research/agentic-master-index.md`
6. `docs/framework/research/vida-direct-1.0-next-agent-compact-instruction.md`
7. `docs/framework/research/vida-parity-and-conformance-next-step-after-compact-instruction.md`
8. `docs/framework/plans/vida-direct-1.0-compact-continuation-plan.md`
9. `docs/framework/plans/vida-0.2-semantic-freeze-spec.md`
10. `docs/framework/plans/vida-0.2-bridge-policy.md`
11. `docs/framework/plans/vida-0.3-command-tree-spec.md`
12. `docs/framework/plans/vida-0.3-state-kernel-schema-spec.md`
13. `docs/framework/plans/vida-0.3-instruction-kernel-spec.md`
14. `docs/framework/plans/vida-0.3-migration-kernel-spec.md`
15. `docs/framework/plans/vida-0.3-route-and-receipt-spec.md`
16. `docs/framework/plans/vida-0.1-to-1.0-direct-binary-transition-plan.md`
17. `docs/framework/plans/vida-direct-1.0-local-spec-program.md`

Bounded explorer lanes used for synthesis:

1. semantic families that must be parity-testable and the fixture groups they imply,
2. canonical parity inputs and evidence-basis hierarchy,
3. the safe deferral boundary from `Part A` into `Parity/Conformance Part B`.

Source synthesis rule:

1. the semantic freeze defines what must be preserved semantically,
2. the bridge policy defines what bridge outputs are required as parity inputs,
3. the kernel specs define the canonical semantic families and ownership boundaries,
4. the route-and-receipt spec completes the proof/receipt boundary parity must now include,
5. topology-heavy carriers remain non-authoritative unless normalized into canonical semantic artifacts.

---

## 4. Purpose And Current Completion Boundary

This artifact answers one question:

1. what semantic evidence surface must the direct `1.0` binary reproduce so later conformance judgment can be made on stable, topology-independent evidence?

Current completion boundary:

1. `Part A` is authoritative for fixture scope, canonical parity inputs, evidence basis, delta categories, and parity-testable semantic outputs,
2. `Part B` is authoritative for the conformance matrix, thresholds, cutover proof gates, and semantic reproduction verdict rules,
3. implementation waves remain downstream and must earn these gates with real binary artifacts and tests.

This `Part A` step defines:

1. semantic families in parity scope,
2. fixture groups implied by those families,
3. canonical parity inputs,
4. evidence hierarchy,
5. exact-versus-intentional delta categories,
6. parity-level invariants and non-goals,
7. deferred ambiguities and downstream contracts.

This `Part A` step does not define:

1. final pass/fail thresholds,
2. exact cutover readiness rules,
3. release verdict logic,
4. output rendering or payload serialization details,
5. implementation topology.

---

## 5. Fixture Scope Across Frozen Semantic Families

The minimum semantic families that must be parity-testable are:

1. command semantics,
2. state semantics,
3. instruction semantics,
4. migration semantics,
5. route/receipt semantics.

### 5.1 Command Semantics Fixture Scope

Parity must cover:

1. the five `vida` root families,
2. command-home boundaries,
3. gate semantics,
4. read-only versus mutation ownership.

Implied fixture groups:

1. command/output fixtures,
2. command-family mapping examples for `boot|task|memory|status|doctor`.

### 5.2 State Semantics Fixture Scope

Parity must cover:

1. lifecycle vocabulary,
2. blocker and dependency posture,
3. execution telemetry,
4. governance/review state,
5. reconciliation/readiness summaries,
6. run-graph and resumability semantics.

Implied fixture groups:

1. state-transition fixtures,
2. blocker/dependency examples,
3. execution-step/result fixtures,
4. run-node and resumability/context-capsule fixtures.

### 5.3 Instruction Semantics Fixture Scope

Parity must cover:

1. `Agent Definition`,
2. `Instruction Contract`,
3. `Prompt Template Configuration`,
4. precedence,
5. activation,
6. effective composition,
7. explicit fallback/escalation/output/proof obligations.

Implied fixture groups:

1. versioned instruction entity fixtures,
2. validated overlay and activation cases,
3. effective-instruction bundle examples.

### 5.4 Migration Semantics Fixture Scope

Parity must cover:

1. compatibility input families,
2. fail-closed boot posture,
3. migration states,
4. allowed bridge inputs,
5. migration proof families.

Implied fixture groups:

1. normalized bridge input fixtures,
2. compatibility tuple/outcome examples,
3. migration-state examples,
4. boot/doctor blocker examples grounded in migration law.

### 5.5 Route/Receipt Semantics Fixture Scope

Parity must cover:

1. route stage and gate law,
2. proof-versus-state boundary,
3. receipt/artifact families,
4. run-node attachment semantics,
5. proof categories,
6. operator visibility boundaries.

Implied fixture groups:

1. route-stage and progression fixtures,
2. analysis/escalation/verification/approval/closure-ready proof examples,
3. proof attachment examples grounded in state-owned run surfaces.

Fixture-scope rule:

1. fixture groups must follow semantic ownership boundaries,
2. fixtures must not be expanded into payload/rendering detail during `Part A`.

---

## 6. Canonical Parity Inputs And Evidence Basis

### 6.1 Canonical Parity Input Families

The canonical parity inputs are:

1. frozen semantic law from the semantic freeze spec,
2. canonical bridge outputs from `0.1`,
3. migration-normalized inputs,
4. route/receipt semantic artifacts grounded in the canonical artifact families.

### 6.2 Input Meaning

`frozen semantic law` includes:

1. command semantics,
2. state/review/approval vocabulary,
3. route/authorization law,
4. run-graph and resumability semantics,
5. instruction runtime law,
6. proof-before-close behavior.

`canonical bridge outputs` include:

1. frozen semantic vocabularies,
2. golden command/output fixtures,
3. golden route/approval/verification fixtures,
4. canonical receipt family examples,
5. state-transition examples,
6. run-graph examples,
7. compact/context-capsule examples,
8. parity reference scenarios,
9. exact-versus-intentional delta register when divergence is deliberate.

`migration-normalized inputs` include:

1. normalized bridge exports,
2. compatibility-bearing command/state/instruction inputs,
3. migration proof artifacts showing topology was not promoted into product law.

`route/receipt semantic artifacts` include:

1. route-stage and gate law,
2. proof-vs-state boundary,
3. receipt-family meaning,
4. route-proof examples tied to canonical artifact families rather than helper paths.

### 6.3 Evidence Basis Hierarchy

The evidence basis hierarchy for parity decisions is:

1. frozen semantic specs and the canonical artifact families they define,
2. bridge-era golden fixtures and canonical receipt/proof examples normalized semantically,
3. state and route/receipt artifacts that show current facts versus proof categories,
4. non-authoritative carriers such as shell-era paths, helper names, raw logs, or transcript recollection.

Parity-evidence rule:

1. parity must judge semantics, not shell-era topology,
2. a topology-heavy carrier counts only after it has been normalized into a canonical semantic input family.

---

## 7. Exact-Versus-Intentional Delta Categories

The minimum delta categories frozen by `Part A` are:

1. `exact_required`
2. `intentional_delta`
3. `topology_only_delta`
4. `unresolved_drift`

### 7.1 Category Meaning

`exact_required` means:

1. the frozen semantic law says behavior must be preserved exactly,
2. golden fixtures and canonical examples are the expected proof surface.

`intentional_delta` means:

1. divergence is recorded in a canonical delta register or equivalent bridge/migration artifact,
2. the divergence is explicit rather than accidental.

`topology_only_delta` means:

1. helper paths, file names, CLI glue, provider transport, or other carriers differ,
2. but semantic behavior and proof obligations still match.

`unresolved_drift` means:

1. a semantic difference exists,
2. but no canonical delta record authorizes it,
3. so parity must treat it as unresolved rather than allowed modernization.

### 7.2 Delta Rule

1. topology-only differences are not parity failures when semantic behavior matches,
2. semantic differences without canonical delta records are parity blockers,
3. `Part B` will later define how these categories map into conformance verdicts and thresholds.

---

## 8. Parity-Testable Semantic Outputs

Before thresholds are decided, the following outputs must be parity-testable:

1. command-family semantic behavior and gate posture,
2. authoritative state transitions and state summaries,
3. effective instruction bundles and instruction-owned obligations,
4. migration compatibility and fail-closed boot outcomes,
5. route-stage progression and route-proof categories,
6. receipt-family meaning and proof-versus-state boundaries,
7. bridge-era golden scenarios that normalize `0.1` truth into canonical parity artifacts.

Output rule:

1. parity-testable outputs are semantic outputs, not raw storage or transport artifacts,
2. payload or rendering details remain outside `Part A`.

---

## 9. Part A Invariants

1. Parity judges semantic law, not topology.
2. Fixture scope must cover all frozen semantic families needed for later conformance judgment.
3. Canonical parity inputs must come from frozen specs and normalized bridge outputs rather than ad hoc raw carriers.
4. Topology-only differences must not be misclassified as semantic drift.
5. Intentional semantic divergence must be explicitly recorded to count as lawful.
6. `Part A` must define the evidence surface before `Part B` defines thresholds and verdict rules.

---

## 10. Part A Non-Goals

1. `Part A` does not define final thresholds or tolerances.
2. `Part A` does not define cutover proof gates.
3. `Part A` does not define semantic reproduction verdict rules.
4. `Part A` does not define payload schemas or output rendering formats.
5. `Part A` does not redefine command/state/instruction/migration/route ownership.
6. `Part A` does not authorize implementation topology or shell-era carriers as parity truth.

---

## 11. Explicit Deferral To Parity/Conformance Part B

The following are intentionally deferred to `Part B`:

1. final conformance matrix across the parity-tested surfaces,
2. final thresholds and pass/fail tolerances,
3. cutover proof gates and binary-promotion criteria,
4. semantic reproduction verdict rules,
5. final judgment logic that turns evidence into release or switchover decisions.

Deferral rule:

1. those topics are downstream of the evidence surface frozen here,
2. they must consume the fixture scope, canonical inputs, and delta categories from `Part A`,
3. they must not redefine them.

---

## 12. Downstream Contracts Unlocked By Part A

Completing `Part A` unlocks:

1. `Parity/Conformance Part B` threshold and verdict work on a stable evidence surface,
2. bounded fixture extraction and parity-test planning for the first implementation waves,
3. later implementation planning that can assume the semantic parity surface is no longer moving.

---

## 13. Open Ambiguities At The Part A Boundary

1. Exact fixture payload shapes and rendering remain open.
2. Exact threshold values and tolerance policy remain downstream.
3. Exact cutover gating rules remain downstream.

---

## 14. Part B Completion Update

`Part B` completes the conformance and cutover boundary on top of the frozen `Part A` evidence surface.

This step freezes:

1. the final conformance target matrix,
2. threshold and tolerance categories,
3. semantic reproduction verdict categories,
4. cutover proof gates and binary-promotion criteria,
5. final parity-level invariants and non-goals.

This step does **not** satisfy those gates by itself.

It defines the criteria that later implementation waves must prove.

---

## 15. Final Conformance Target Matrix

The final conformance target matrix categories are:

1. `command semantics conformance`
2. `state semantics conformance`
3. `instruction semantics conformance`
4. `migration semantics conformance`
5. `route/receipt semantics conformance`
6. `cutover-readiness conformance`

### 15.1 Category Meaning

`command semantics conformance` judges:

1. preservation of `vida boot|task|memory|status|doctor` command-home boundaries,
2. gate semantics,
3. mutation-versus-read-only ownership.

`state semantics conformance` judges:

1. lifecycle, blocker/dependency, execution, governance, run-graph, resumability, and readiness semantics,
2. preservation of authoritative facts without turning proof into state.

`instruction semantics conformance` judges:

1. the `Agent Definition -> Instruction Contract -> Prompt Template Configuration` hierarchy,
2. precedence,
3. activation,
4. effective composition,
5. explicit proof/fallback/escalation obligations.

`migration semantics conformance` judges:

1. compatibility classification,
2. fail-closed boot posture,
3. allowed bridge inputs,
4. migration proof families,
5. rollback and cutover preconditions.

`route/receipt semantics conformance` judges:

1. route stages and gates,
2. proof-versus-state boundary,
3. receipt-family meaning,
4. run-node attachment semantics,
5. proof categories,
6. visibility boundaries.

`cutover-readiness conformance` judges:

1. whether all required proof gates are satisfied strongly enough for binary promotion,
2. without redefining the earlier evidence surface.

### 15.2 Matrix-Level Fail Rules

The matrix must fail when any are true:

1. an `unresolved_drift` exists on a semantic surface without canonical authorization,
2. required parity evidence is missing, non-canonical, or topology-dependent,
3. ownership boundaries between command/state/instruction/migration/route are violated,
4. proof artifacts become shadow state,
5. required cutover proof gates are incomplete.

Topology-only rule:

1. topology-only differences must not fail conformance when semantic behavior still matches.

---

## 16. Threshold And Tolerance Categories

The threshold/tolerance categories frozen by `Part B` are:

1. `exact_required`
2. `intentional_delta`
3. `topology_only_delta`
4. `unresolved_drift`
5. `cutover-proof gate`

### 16.1 Category Meaning

`exact_required` means:

1. a frozen semantic surface must reproduce exactly against canonical fixtures and proofs.

`intentional_delta` means:

1. semantic divergence is allowed only when canonically recorded in the delta register or equivalent bridge/migration artifact.

`topology_only_delta` means:

1. carrier, path, helper, rendering, or transport differences are non-failing when semantic behavior still matches.

`unresolved_drift` means:

1. semantic difference exists without canonical authorization,
2. or the evidence basis is insufficiently normalized to judge it safely.

`cutover-proof gate` means:

1. parity matching alone is insufficient,
2. promotion also requires the downstream proof gates frozen in this spec.

### 16.2 Tolerance Rule

1. no tolerance exists for unauthorized semantic drift,
2. tolerance exists only for topology-only differences and canonically recorded intentional deltas,
3. exact numerical tolerances remain implementation-run policy and must be instantiated under this category model rather than replacing it.

---

## 17. Semantic Reproduction Verdict Rules

The final semantic reproduction verdict categories are:

1. `semantically_reproduced`
2. `semantically_reproduced_with_intentional_deltas`
3. `semantically_inconclusive`
4. `semantically_not_reproduced`
5. `not_cutover_ready`

### 17.1 Verdict Meaning

`semantically_reproduced` means:

1. frozen semantics match,
2. with no blocking drift.

`semantically_reproduced_with_intentional_deltas` means:

1. preserved behavior remains acceptable,
2. because the differences are canonically recorded and bounded.

`semantically_inconclusive` means:

1. the evidence surface is incomplete, underspecified, or insufficiently normalized.

`semantically_not_reproduced` means:

1. a blocking semantic mismatch exists.

`not_cutover_ready` means:

1. parity may be acceptable,
2. but required conformance or cutover proofs are still missing.

### 17.2 Mapping Rule

1. `exact_required` plus matching evidence yields `semantically_reproduced`,
2. `intentional_delta` plus canonical authorization may yield `semantically_reproduced_with_intentional_deltas`,
3. `topology_only_delta` never fails by itself,
4. `unresolved_drift` yields `semantically_not_reproduced` or `semantically_inconclusive`,
5. any otherwise-positive semantic verdict remains `not_cutover_ready` until the required proof gates below are satisfied.

Verdict separation rule:

1. semantic verdict and cutover verdict must remain distinct,
2. do not collapse them into one bucket.

---

## 18. Cutover Proof Gates And Binary-Promotion Criteria

The cutover proof gates frozen by `Part B` are:

1. `spec-complete gate`
2. `parity-evidence gate`
3. `conformance-proof gate`
4. `migration-safety gate`
5. `operator-viability gate`
6. `local-proof-path gate`

### 18.1 Gate Meaning

`spec-complete gate` proves:

1. semantic freeze, bridge policy, command, state, instruction, migration, route/receipt, and parity/conformance specs all exist.

`parity-evidence gate` proves:

1. required parity fixtures exist,
2. parity-tested semantic surfaces are covered,
3. exact-versus-intentional delta rules are fixed.

`conformance-proof gate` proves:

1. required conformance proofs exist against the frozen semantic surfaces.

`migration-safety gate` proves:

1. startup checks and fail-closed migration behavior exist as promotion conditions.

`operator-viability gate` proves:

1. `vida boot|task|memory|status|doctor` are usable locally.

`local-proof-path gate` proves:

1. the binary has viable local proof paths and a usable local test harness for daily work.

### 18.2 Promotion Rule

1. `Part B` freezes these gates as promotion criteria,
2. later implementation waves must satisfy them with real binary artifacts and tests,
3. a spec-defined gate is not the same thing as a satisfied gate.

---

## 19. Part B Invariants

1. Conformance judgment must consume the `Part A` evidence surface rather than redefining it.
2. Semantic verdict and cutover verdict remain separate.
3. Unauthorized semantic drift is blocking.
4. Topology-only difference is never a conformance failure by itself.
5. Promotion criteria remain fail-closed until proven by implementation artifacts.

---

## 20. Part B Non-Goals

1. `Part B` does not claim that cutover gates are already satisfied.
2. `Part B` does not define numerical values for every tolerance instance.
3. `Part B` does not replace later binary tests, harnesses, or operator usability proof.
4. `Part B` does not redefine the frozen semantic scope from `Part A`.
5. `Part B` does not authorize implementation topology changes as a substitute for semantic proof.

---

## 21. Downstream Contracts Unlocked By Completing Parity

Completing parity/conformance unlocks:

1. lawful start of post-spec implementation waves,
2. `Binary Foundation` as the first implementation step,
3. packetized coding work against a closed spec spine,
4. later proof and promotion work against frozen cutover criteria instead of moving goals.

---

## 22. Remaining Open Ambiguities After Part B

1. Exact instantiated tolerance values remain for later implementation/test policy.
2. Exact fixture payload schemas and rendering remain implementation detail.
3. Real binary promotion remains contingent on satisfying the frozen cutover gates with actual artifacts and tests.
-----
artifact_path: framework/plans/vida-0.3-parity-and-conformance-spec
artifact_type: plan
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/plans/vida-0.3-parity-and-conformance-spec.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-0.3-parity-and-conformance-spec.changelog.jsonl
P26-03-09T21: 44:13Z
