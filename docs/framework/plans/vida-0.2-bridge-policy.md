# VIDA 0.2 Bridge Policy

Purpose: define how `0.1` and direct `1.0` work coexist during the binary transition without reopening the forbidden middle path.

Status: canonical transition policy between semantic freeze and kernel-spec implementation.

Date: 2026-03-08

---

## 1. Policy Goal

The bridge era exists to:

1. keep current delivery velocity high on `0.1`,
2. preserve `0.1` as the behavioral oracle,
3. export parity fixtures and canonical artifacts,
4. close the rewrite input bundle before broad binary coding,
5. prevent new long-horizon platform logic from being buried into the old engine,
6. let `1.0` be built directly from specs and proofs.

Compact formula:

`0.1 = current delivery runtime + semantic oracle + export bridge`

`1.0 = direct target product built from frozen semantics`

---

## 2. Source Basis

Local source basis:

1. `docs/framework/plans/vida-0.2-semantic-freeze-spec.md`
2. `docs/framework/plans/vida-0.1-to-1.0-direct-binary-transition-plan.md`
3. `docs/framework/plans/vida-direct-1.0-local-spec-program.md`
4. `docs/framework/plans/vida-semantic-extraction-layer-map.md`
5. `vida/config/instructions/runtime-instructions.beads-protocol.md`
6. `vida/config/instructions/instruction-contracts.agent-system-protocol.md`
7. `vida/config/instructions/agent-definitions.protocol.md`
8. `docs/framework/research/agentic-cheap-worker-packet-system.md`
9. `docs/framework/research/agentic-proof-obligation-registry.md`

Web source basis:

1. `vida-stack` README:
   - https://raw.githubusercontent.com/pomazanbohdan/vida-stack/refs/heads/main/README.md
2. `vida-stack` VERSION-PLAN:
   - https://raw.githubusercontent.com/pomazanbohdan/vida-stack/main/VERSION-PLAN.md

Current source takeaways:

1. `0.1` remains the behavioral source of truth until the binary reproduces it.
2. `1.0` is a self-hosted local binary, not a daemon release.
3. `0.2` freezes semantics before binary migration.
4. `0.3` defines migration and kernel specs.
5. `0.9` makes the binary the primary operating surface while the script runtime remains reference and migration support.

---

## 3. Allowed Mission Of 0.1 During Bridge Era

`0.1` work is allowed only when it serves at least one of these goals:

1. current product/framework bugfix delivery,
2. current runtime speed and reliability,
3. semantic freeze clarification,
4. parity fixture export,
5. canonical receipt export,
6. bridge helper implementation for migration safety,
7. narrow compatibility work required to keep the transition viable,
8. task-state, route, approval, verification, memory, compact, or diagnosis correctness that affects current real work.

Concrete allowed categories:

1. fix `br`/queue/runtime friction that blocks current engineering work,
2. improve current status/doctor/boot visibility if it reduces ambiguity for freeze or parity,
3. add exporters, normalizers, and fixture helpers,
4. harden verification/proof surfaces that `1.0` must later reproduce,
5. harden semantic law clarity in framework-owned protocols,
6. keep current bugfix throughput high in the mobile/product repo.

Rewrite-input completion work is also allowed:

1. source registry,
2. source delta log,
3. role-profile/source eval-plan seed,
4. normalized direct-`1.0` reference bundle.

---

## 4. Forbidden 0.1 Work During Bridge Era

The following are forbidden in `0.1` unless the user explicitly changes product direction:

1. new persistent runtime kernels,
2. deep new shell orchestration layers intended to survive into `1.0`,
3. new long-horizon instruction storage logic,
4. a new replacement state backend hidden inside the script runtime,
5. direct implementation of future binary command surfaces inside shell wrappers as permanent architecture,
6. inventing new behavioral platform features in docs and then implementing them deeply in `0.1`,
7. using `0.1` as the primary home for `1.0` product logic.

Forbidden middle path:

1. define future behavior,
2. deeply implement it in shell/Python/`br`,
3. port it again into the binary later.

This path is disallowed.

---

## 5. Bridge Outputs Required From 0.1

`0.1` must continue producing or supporting these bridge-era outputs:

1. frozen semantic vocabularies,
2. frozen receipt vocabularies,
3. golden command/output fixtures,
4. golden route/approval/verification fixtures,
5. canonical receipt family examples,
6. state-transition examples,
7. compact/context-capsule examples,
8. run-graph examples,
9. framework-memory examples,
10. web-validation evidence examples where external reality matters,
11. parity reference scenarios for later binary tests,
12. an exact-vs-intentional delta register when `1.0` behavior diverges by design.

Bridge rule:

1. if a future `1.0` behavior cannot be proven against a frozen `0.1` artifact family, the bridge output set is still incomplete.

---

## 6. Cheap-Agent Boundaries During Bridge Era

Cheap workers may help early, but only on bounded, non-architectural slices.

Allowed cheap-worker work:

1. fixture extraction,
2. schema inventory,
3. enum/vocabulary catalogs,
4. receipt normalization,
5. test-matrix scaffolding,
6. packet/template drafting,
7. bounded documentation updates inside a declared write scope.

Forbidden cheap-worker work during bridge era:

1. architecture boundary changes,
2. deciding what `0.1` semantics mean,
3. deciding what gets preserved or discarded,
4. cross-kernel tradeoff decisions,
5. shared kernel coding before the required specs exist.

Readiness rule:

1. no cheap worker should code shared `1.0` runtime behavior until:
   - semantic freeze exists,
   - bridge policy exists,
   - the next relevant kernel spec exists,
   - the task packet is bounded and proof-bearing.

---

## 7. Cut Line Between 0.1 And 1.0

Use this deterministic split:

### Keep In 0.1

1. current delivery work,
2. bridge/export helpers,
3. parity fixture generation,
4. narrow reliability improvements,
5. semantic ambiguity reduction,
6. bugfixes that improve current operator speed or correctness.

### Move Directly To 1.0

1. command tree,
2. state kernel,
3. instruction kernel,
4. migration kernel,
5. route/receipt kernel law,
6. memory-kernel productization,
7. binary doctor/status/boot product surfaces,
8. any durable runtime surface meant to survive the script era.

---

## 8. Cutover Preconditions

The binary may become the primary operating surface only when all are true:

1. semantic freeze is complete,
2. bridge policy is complete,
3. command tree spec exists,
4. state kernel schema spec exists,
5. instruction kernel spec exists,
6. migration kernel spec exists,
7. route and receipt spec exists,
8. parity and conformance spec exists,
9. required parity fixtures exist,
10. exact-vs-intentional delta rules exist,
11. required conformance proofs exist,
12. local binary test harness is viable for daily work,
13. fast local proof paths exist for unit, integration, e2e, snapshot, property, and parity tests,
14. startup checks and fail-closed migration behavior exist,
15. operator-critical surfaces `vida boot|task|memory|status|doctor` are usable locally.

Until then:

1. `0.1` remains the primary operating surface,
2. `1.0` remains under spec-first productization.

---

## 9. Bridge-Era Speed Policy

Speed comes from:

1. keeping current product work on `0.1`,
2. preventing speculative deep changes to the old engine,
3. freezing semantics once,
4. generating reusable parity fixtures,
5. building `1.0` from specs and tests,
6. using cheap workers only on bounded packets,
7. keeping one senior integrator for cross-kernel synthesis.

Do not trade speed for hidden rework:

1. a quick `0.1` patch that later forces a second implementation in `1.0` is not considered fast.

---

## 10. Bridge-Era Proof Obligations

Before bridge-era work is considered complete, at least one of these proof families must exist:

1. document proof for transition law,
2. fixture proof for preserved behavior,
3. route/receipt example proof,
4. compatibility or migration proof,
5. parity-target proof for a downstream binary test,
6. rollback note when the bridge change can affect future cutover.

No bridge task is closure-ready if:

1. the result exists only in chat,
2. the semantic/non-semantic split is still ambiguous,
3. parity-relevant behavior was changed without updating the bridge artifact set.

---

## 11. Next Work Unlocked By This Policy

This bridge policy unlocks:

1. `docs/framework/plans/vida-0.3-command-tree-spec.md`
2. `docs/framework/plans/vida-0.3-state-kernel-schema-spec.md`
3. `docs/framework/plans/vida-0.3-instruction-kernel-spec.md`
4. `docs/framework/plans/vida-0.3-migration-kernel-spec.md`
5. `docs/framework/plans/vida-0.3-route-and-receipt-spec.md`
6. `docs/framework/plans/vida-0.3-parity-and-conformance-spec.md`

Immediate next artifact:

1. `docs/framework/plans/vida-0.3-command-tree-spec.md`

Immediate sequencing rule:

1. `command tree spec` goes first because it freezes the future operator surface and boot boundary,
2. `state kernel schema spec` follows because command semantics need a stable state model,
3. `instruction kernel spec` follows because command/runtime behavior must bind to explicit instruction law,
4. later specs may refine internals, but should not invalidate the command boundary already frozen.

---

## 12. Final Rule

During the bridge era:

1. `0.1` is optimized for correctness, speed, and exportability,
2. `1.0` is optimized for direct productization,
3. anything that would be implemented twice must be stopped and rerouted into the binary spec path.
-----
artifact_path: framework/plans/vida-0.2-bridge-policy
artifact_type: plan
artifact_version: 1
artifact_revision: 2026-03-10
schema_version: 1
status: canonical
source_path: docs/framework/plans/vida-0.2-bridge-policy.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T01:12:03+02:00
changelog_ref: vida-0.2-bridge-policy.changelog.jsonl
P26-03-09T21: 44:13Z
