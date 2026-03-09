# VIDA Direct 1.0 Local Spec Program

Purpose: define how VIDA should build `1.0` directly as a local-first binary product through layered specs and bounded epics, using cheap workers for implementation from strong packets rather than iteratively reshaping the old engine.

Status: canonical program plan for direct local-first `1.0` implementation.

Date: 2026-03-08

---

## 1. Program Goal

Build `1.0` directly as:

1. one local binary,
2. one fast local development harness,
3. one deterministic spec-to-code pipeline,
4. one test-heavy self-hosting path,
5. one bridge from `0.1` semantics into runtime-owned kernels.

This program assumes:

1. local-only development first,
2. fast compiled Rust feedback loops,
3. tests run against the binary itself,
4. unit, integration, and e2e tests can all be local and cheap,
5. cheap workers can execute most code tasks if packets are bounded enough.

---

## 2. Local-First Development Assumptions

### 2.1 Runtime Assumptions

1. `1.0` is a local binary, not a daemon.
2. There is no required remote control plane in this phase.
3. Local embedded storage is available and canonical.
4. The canonical storage engine is embedded `SurrealDB` on `kv-surrealkv`.
5. Fast startup and fast test cycles matter more than remote interoperability.

### 2.2 Build Assumptions

1. fast Rust build profile exists or will exist,
2. local test harness targets the compiled binary,
3. pre-commit formatting, fixes, and linting are already available,
4. local full-project testing can remain inside one machine boundary.

### 2.3 Execution Model Assumptions

1. orchestrator writes specs and integration decisions,
2. cheap workers receive bounded packets,
3. one integrator lane keeps shared contracts coherent,
4. no task may depend on chat memory.

---

## 3. Program Operating Model

### 3.1 Orchestrator Responsibilities

1. own architecture and kernel boundaries,
2. own spec quality,
3. own packet quality,
4. own final synthesis and integration,
5. own parity and cutover decisions.

### 3.2 Cheap Agent Responsibilities

Cheap agents should do:

1. schema implementation
2. bounded module implementation
3. unit test writing
4. fixture generation
5. integration-test scaffolding
6. doc updates inside bounded write scope

Cheap agents should not own:

1. architecture boundary changes
2. cross-kernel tradeoff decisions
3. final integration of overlapping modules
4. semantic freeze decisions
5. approval of parity or cutover

### 3.3 Integration Rule

One writer per write scope.

If multiple agents work in parallel:

1. scopes must not overlap,
2. shared contracts must already be specified,
3. final merge happens through one integrator lane.

---

## 4. Epic Order

### Epic 1. Research Freeze And Rewrite Inputs

Goal:

1. close missing research artifacts,
2. normalize rewrite inputs,
3. lock the direct-1.0 input bundle before semantic freeze.

Core outputs:

1. source registry
2. source delta log
3. role-profile/source eval-plan seed
4. normalized direct-1.0 reference bundle

Cheap agents can do:

1. fill dated research docs
2. normalize source tables
3. produce bundle coverage ledgers

Senior integrator owns:

1. materiality rules
2. promotion decisions from research into runtime law

Gate:

1. no semantic freeze should proceed on an incomplete rewrite input bundle.

### Epic 2. Semantic Freeze And Fixture Export

Goal:

1. freeze `0.1` semantics,
2. export golden fixtures and receipts,
3. stop behavioral drift before major binary work.

Core outputs:

1. semantic vocabulary
2. receipt vocabulary
3. golden command/output fixtures
4. golden route/approval/verification fixtures
5. `0.1 bridge policy`

Cheap agents can do:

1. fixture extraction helpers
2. schema transcription
3. golden-case normalization

Senior integrator owns:

1. final freeze decisions
2. conflict resolution
3. parity baseline approval

Gate:

1. no later epic may finalize contracts against moving behavior.

### Epic 3. State, Route, And Receipt Specs

Goal:

1. define binary-owned state kernel and route/receipt law surfaces.

Core outputs:

1. state kernel schema spec
2. route-and-receipt spec
3. run-graph mapping
4. mutation rules

Cheap agents can do:

1. schema tables
2. receipt inventories
3. fixture-ready enum catalogs
4. draft test matrices

Senior integrator owns:

1. state model boundaries
2. receipt boundaries
3. resumability model

Gate:

1. no binary foundation should harden around moving state/receipt contracts.

### Epic 4. Instruction And Command Specs

Goal:

1. define runtime-owned instructions and the binary command tree.

Core outputs:

1. instruction kernel spec
2. command tree spec
3. overlay/validation rules
4. agent-definition integration

Cheap agents can do:

1. command mapping tables
2. schema drafts
3. prompt/packet template derivation

Senior integrator owns:

1. precedence law
2. instruction composition
3. operator UX contract

Gate:

1. runtime behavior must be explicit, versioned, and fail-closed.

### Epic 5. Parity Harness And Local Test Matrix

Goal:

1. make direct-1.0 behavior testable before large-scale Rust implementation.

Core outputs:

1. parity/conformance spec
2. golden fixtures
3. local unit/integration/e2e matrix
4. exact-vs-intentional-delta rules

Cheap agents can do:

1. fixture catalogs
2. test case inventories
3. golden file normalization

Senior integrator owns:

1. parity thresholds
2. exact behavior boundary
3. final test architecture

Gate:

1. no large kernel coding should begin without executable parity targets.

### Epic 6. Rust Binary Foundation

Goal:

1. create Rust workspace and fast local harness,
2. make binary bootable,
3. make tests cheap enough to drive the whole program.

Core outputs:

1. workspace layout
2. fast build profile
3. binary entry shell
4. local temp-state test harness
5. test command conventions

Cheap agents can do:

1. crate scaffolding
2. test harness plumbing
3. CI/local script generation

Senior integrator owns:

1. workspace architecture
2. crate boundary decisions

Gate:

1. fast local compile-and-test loop must exist before heavy kernel implementation.

### Epic 7. Kernel Implementation And Switchover

Goal:

1. replace long-term dependence on `br`/`.beads`,
2. implement authoritative state storage and migration path.

Core outputs:

1. state entities
2. mutation rules
3. migration runner
4. import/export bridge
5. startup compatibility checks

Cheap agents can do:

1. schema structs
2. storage adapters
3. migration tests
4. fixture importers/exporters

Senior integrator owns:

1. state model boundaries
2. migration safety law

Gate:

1. state kernel must preserve frozen semantics, not current storage topology.

Goal:

1. implement kernels and move selected flows onto `vida`.

Core outputs:

1. state kernel
2. instruction kernel
3. memory kernel
4. core commands
5. local integration/e2e tests
6. switchover plan

Cheap agents can do:

1. bounded module implementation
2. unit and integration tests
3. compatibility shims
4. golden-fixture assertions

Senior integrator owns:

1. cross-kernel integration
2. self-hosting cutover
3. fallback policy

Gate:

1. `vida` must be viable for daily local framework work before `0.1` is demoted.

---

## 5. Spec Families

The program should generate and maintain these spec families:

1. semantic freeze specs
2. command specs
3. state kernel specs
4. instruction kernel specs
5. migration specs
6. route and receipt specs
7. parity and conformance specs
8. cheap worker packet specs

Rule:

1. cheap agents should write code only from these spec families, never from a loose chat request.

---

## 6. Local Tooling And Speed Strategy

### 6.1 Fast Feedback

Prefer:

1. fast Rust profile
2. per-crate test runs
3. `cargo nextest`-style fast test execution
4. snapshot testing for compact outputs
5. property testing for state-machine invariants
6. fixture-driven integration tests

### 6.2 Cheap Test Hierarchy

1. unit tests for structs, parsers, rules, migrations
2. integration tests for kernel boundaries
3. binary e2e for command contracts and golden scenarios
4. parity tests against frozen `0.1` fixtures

### 6.3 Cheap Agent Acceleration

1. generate schema-first tasks
2. generate fixture-first tests
3. keep write scopes narrow
4. batch independent tasks in parallel
5. let one integrator merge contract-bearing work

---

## 7. Packet-Driven Execution Rule

Every coding task sent to a cheap agent must contain:

1. exact scope
2. exact files
3. exact schemas to satisfy
4. exact tests to make pass
5. exact forbidden moves
6. exact output artifact expectations

If a task cannot be described that way:

1. it is not cheap-agent-ready,
2. it belongs first to spec work or senior integration work.

---

## 8. Anti-Patterns

Avoid:

1. building binary kernels before fixtures and semantics are frozen,
2. giving cheap agents architecture work hidden inside coding tasks,
3. parallelizing overlapping write scopes,
4. letting tests depend on unstable old-engine behavior not captured in fixtures,
5. continuing to deepen the old engine while the new program is already specified.

---

## 9. Immediate Next Outputs

The next concrete outputs after this program doc should be:

1. `0.1 bridge policy`
2. `cheap worker packet system`
3. `command tree spec`
4. `state kernel schema spec`
5. `instruction kernel spec`
6. `migration kernel spec`
7. `route and receipt spec`
8. `parity and conformance spec`

Execution rule after compact:

1. finish `semantic freeze spec` and `0.1 bridge policy` before any broad binary coding,
2. finish `command tree`, `state kernel`, `instruction kernel`, `migration kernel`, and `route/receipt` specs before dispatching cheap workers for shared kernel code,
3. allow cheap workers early only for bounded preparatory work:
   - fixture extraction
   - schema inventory
   - enum catalogs
   - test-matrix scaffolding
   - prompt/packet template drafting
4. keep one senior integrator lane for all cross-kernel decisions and final synthesis.

Current state:

1. `semantic freeze spec` now exists and is part of the canonical read-set,
2. the next required artifact is `0.1 bridge policy`.

---

## 10. Final Rule

Speed comes from:

1. freezing semantics once,
2. specifying target kernels clearly,
3. letting cheap agents implement bounded slices from packets,
4. integrating through one strong verifier/integrator lane,
5. testing the binary locally and cheaply at every layer.
