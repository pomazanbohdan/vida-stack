# TaskFlow v1 Runtime Modernization Plan

Status: active product architecture plan and Rust implementation program

Purpose: retain the modular `TaskFlow` architecture originally shaped through the earlier `v0.2.0` refactor work as the canonical runtime law, while promoting the active implementation program to an explicit `TaskFlow v1` standalone Rust runtime substrate with bounded bridge/donor inputs from the current transitional Nim implementation and the current monolithic Rust `vida` binary line, preserving runtime-layer obligations, coordinating with the now-active sibling `DocFlow v1` modernization track, and keeping storage law adapter-bounded without letting backend details rewrite kernel law.

## 1. Mission

The target is not another long-lived refactor of the current Nim monolith and not indefinite expansion of the current monolithic Rust `vida` line.

The target is:

1. a standalone Rust `TaskFlow` kernel,
2. built as componentized crates from the start,
3. with framework law separated from feature behavior,
4. with compatibility bridges retained only where needed to preserve continuity,
5. with later `VIDA` integration treated as an adapter problem, not as the ownership source of the kernel,
6. with crate and runtime-family seams chosen so the active sibling Rust rewrite of `DocFlow` can follow the same pattern without architectural drift,
7. with storage and persistence isolated behind explicit store contracts so backend evolution does not rewrite kernel law.

Architectural continuity rule:

1. this plan keeps the modular runtime architecture as canon,
2. it deprecates only the idea that the canonical architecture must be implemented by continued `Nim -> Nim` refactor,
3. therefore the module boundaries and layer coverage remain the basis for `TaskFlow`,
4. while the implementation target is now `taskflow-rs`.

Compact rule:

1. build the target architecture directly,
2. keep the current Nim runtime only as a bridge, proof source, and behavioral reference,
3. keep the current monolithic Rust `vida` binary only as a donor, proof source, and transition bridge,
4. do not invent new permanent architecture inside either bridge layer,
5. treat `taskflow-rs` as the first member of a wider Rust runtime-family, not as a one-off port.

## 2. Mandatory Matrix Alignment

This plan must align with the already-promoted matrices and the active sibling runtime plan:

1. `docs/product/spec/canonical-runtime-layer-matrix.md`
2. `docs/product/spec/canonical-documentation-and-inventory-layer-matrix.md`
3. `docs/product/spec/docflow-v1-runtime-modernization-plan.md`

Alignment rule:

1. the Rust rewrite must preserve `TaskFlow` runtime obligations through Layer 9 of the runtime matrix,
2. the implementation and proofs for the rewrite must remain compatible with `DocFlow` documentation/readiness obligations through Layer 7 of the documentation matrix,
3. the rewrite must not regress the explicit Layer 9 runtime rule that `taskflow` remains the execution substrate and `DocFlow` remains the bounded final readiness/proof surface,
4. the rewrite must leave a clean downstream seam for the active `docflow-rs` line so `codex-v0` can later be reduced to a bounded donor bridge without redefining the runtime-family boundary.

### 2.1 Runtime-Layer Obligations That Must Survive

The Rust plan must preserve these runtime-layer capabilities as first-class constraints:

1. Layer 1: runtime kernel boundary
2. Layer 2: state, route, and receipt kernel
3. Layer 3: tracked execution substrate
4. Layer 4: lane routing and assignment
5. Layer 5: handoff and context governance
6. Layer 6: review, verification, and approval gates
7. Layer 7: resumability, checkpoint, and replay-safe recovery
8. Layer 8: observability and runtime readiness
9. Layer 9: direct runtime consumption with explicit `taskflow -> docflow` final activation

### 2.2 DocFlow-Layer Obligations That Must Survive

The Rust plan must also preserve `DocFlow`-relevant documentation/operator law:

1. canonical schema remains explicit,
2. inventory and validation remain fail-closed,
3. mutation remains lawful through canonical docs,
4. relation and operator surfaces remain inspectable,
5. readiness remains the gate before runtime trust,
6. `DocFlow` remains the canonical validation/proof surface for documentation and readiness artifacts until a later `docflow-rs` line is ready.

Implementation rule:

1. Rust runtime changes may deepen runtime behavior,
2. but they must not bypass the canonical documentation/readiness/proof chain.

## 3. Non-Negotiable Architectural Rules

### 3.1 Kernel Rule

Kernel code may hardcode only:

1. execution primitives,
2. state and receipt contracts,
3. transition and gate classes,
4. fail-closed validation rules,
5. feature registration law,
6. recovery and observability invariants,
7. ownership boundaries between crates.

Kernel code must not hardcode:

1. project-specific routes,
2. provider names,
3. model names,
4. project roles,
5. project skills,
6. project profiles,
7. project flow IDs,
8. Party Chat board compositions,
9. `VIDA`-specific product semantics,
10. database-engine-specific behavior.

### 3.2 Feature Rule

Anything that is not kernel law must live outside the kernel.

That includes:

1. provider/model preferences,
2. project feature packs,
3. council semantics such as Party Chat,
4. project-owned packets,
5. project role/profile/skill/flow resolution,
6. downstream runtime adapters,
7. database-engine adapters,
8. runtime-family-specific operator shells.

### 3.3 Bridge Rule

The current Nim runtime remains:

1. a compatibility bridge,
2. a regression oracle,
3. a source of current receipts/artifact behavior,
4. a temporary migration path.

The Nim bridge must not remain the place where new permanent architecture is designed.

### 3.4 State-Machine Rule

The Rust kernel owns its own transition engine.

The kernel state machine must be built explicitly from:

1. states,
2. events,
3. transitions,
4. guards,
5. actions,
6. context,
7. invariants,
8. recovery points.

Decision:

1. do not adopt a third-party state-machine framework as the foundation of the kernel,
2. implement the transition engine directly so taskflow-specific gate law and recovery semantics remain explicit.

### 3.5 Runtime-Family Continuity Rule

This plan is intentionally one active track inside a two-runtime modernization program.

Implications:

1. `TaskFlow` is rewritten as the execution and closure-authority track,
2. `DocFlow` is rewritten in parallel as the documentation/readiness/proof track,
3. both tracks must mirror the same discipline:
   - kernel-first,
   - crate-bounded,
   - fail-closed,
   - format-aware,
   - explicit operator/readiness/proof surfaces,
4. decisions taken now for formats, contracts, receipts, runtime-family seams, and storage seams must not block or distort the active `docflow-rs` line.

Runtime-family rule:

1. `taskflow-rs` and future `docflow-rs` are sibling runtimes in one family,
2. they may share patterns and support crates later,
3. but `TaskFlow` must not absorb `DocFlow` responsibilities and `DocFlow` must not absorb execution authority.

### 3.6 Cross-Runtime Dependency Rule

The modernization program runs as two coordinated tracks with one explicit seam.

Rules:

1. `TaskFlow` owns execution, task truth, runtime state, and closure authority,
2. `DocFlow` owns documentation/inventory/validation/readiness/proof runtime behavior,
3. `TaskFlow` waves before final consumption may proceed independently when their contracts do not depend on `DocFlow`,
4. final runtime-consumption closure may not close until the required `docflow-rs` capabilities are green,
5. cross-runtime integration must occur only through explicit contracts, not through hidden shared assumptions.

### 3.7 Storage Rule

`TaskFlow` must keep storage law behind store contracts, while aligning product direction with the already-closed `VIDA 1` storage decision.

Storage rule:

1. kernel crates define store contracts only,
2. concrete storage engines are adapters,
3. the first fast bridge/test implementation may use filesystem/JSONL state for speed and inspectability,
4. the product storage line for direct `VIDA 1` is embedded `SurrealDB` on `kv-surrealkv`,
5. future additional adapters are allowed only if admitted by a higher-precedence product/runtime spec,
6. no kernel invariant may depend on one specific storage backend.

### 3.8 Direct-Rewrite Efficiency Rule

The implementation program must optimize for direct target build rather than compatibility churn.

Rules:

1. write the target crate directly when ownership is clear,
2. use donor runtimes only to extract behavior, proofs, artifact shapes, and migration inputs,
3. do not widen a bounded bridge helper into a permanent runtime subsystem,
4. keep one owner per crate write scope,
5. close one vertical kernel slice at a time with tests and review before broadening.

## 4. Build Strategy

### 4.1 Fast-But-Effective Delivery Rule

The fastest safe path is:

1. library-first,
2. contracts-first,
3. bridge-compatible,
4. vertical slice by vertical slice,
5. proof at the end of every wave.

This means:

1. build reusable crates before feature shells,
2. prefer a small executable slice over broad scaffolding,
3. defer project features until the substrate is real,
4. keep all later features attachable through explicit contracts,
5. keep storage engines behind adapters from day one,
6. keep runtime-family seams clean enough that a later `docflow-rs` line can reuse the same discipline.

### 4.2 Donor And Oracle Inputs

The rewrite has two bounded implementation donors and one architectural canon.

Canonical architecture source:

1. this modular runtime plan itself,
2. the runtime layer matrix,
3. the documentation/readiness layer matrix,
4. the `taskflow` and `docflow` runtime-family maps.
5. `docs/product/spec/docflow-v1-runtime-modernization-plan.md`.

Behavior/proof donors:

1. `taskflow-v0/**`
   - transitional `TaskFlow` runtime behavior,
   - route/gate/recovery/handoff semantics,
   - current artifact and command behavior,
   - current parity tests and bridge receipts.
2. `crates/vida/**`
   - direct `VIDA 1` monolithic Rust line,
   - already-proven boot/status/doctor/memory surfaces,
   - embedded `SurrealDB` state spine and instruction ingest/composition work,
   - migration and operator-surface proofs.

Usage rule:

1. architectural decisions come from canonical specs, not donor code,
2. donor code supplies executable evidence and reusable bounded implementation slices,
3. where donors conflict, higher-precedence canon wins and the conflict must be resolved explicitly.

### 4.4 Joint Modernization Milestones

The two runtime tracks must meet explicit joint milestones.

1. `T-C0`
   - `taskflow` and `docflow` plans are promoted and mutually aligned
2. `T-C1`
   - core/contracts/config foundations are stable enough in both runtimes to keep ownership boundaries explicit
3. `T-C2`
   - state/inventory/readiness surfaces are materially present and parity-fixture work can start
4. `T-C3`
   - direct `taskflow -> docflow` consumption seam is green in bounded integration tests
5. `T-C4`
   - bridge retirement posture is explicit and the final modernization cutover can be evaluated

### 4.3 Scope Rule

The first target is `TaskFlow` itself, not the full `VIDA` product runtime.

In scope for the first line:

1. core task state and transitions,
2. receipts and execution artifacts,
3. gates and recovery,
4. config and CLI,
5. runtime observability,
6. JSONL and TOON artifact surfaces,
7. explicit final `taskflow -> docflow` branch,
8. storage contracts and at least one concrete storage adapter.

Out of scope until the kernel is stable:

1. Party Chat,
2. project role/profile/skill packs,
3. project-specific provider routing,
4. `VIDA` integration behavior beyond compatibility adapters,
5. ad hoc user-product features not needed by the substrate,
6. speculative shared crates that neither `taskflow-rs` nor `docflow-rs` has proven necessary yet.

## 5. Target Cargo Workspace

The Rust rewrite should be a Cargo workspace with one ownership boundary per concern.

```text
taskflow-rs/
├── Cargo.toml
├── crates/
│   ├── taskflow-core/
│   ├── taskflow-contracts/
│   ├── taskflow-config/
│   ├── taskflow-format-jsonl/
│   ├── taskflow-format-toon/
│   ├── taskflow-state/
│   ├── taskflow-state-fs/
│   ├── taskflow-state-surreal/
│   ├── taskflow-flow/
│   ├── taskflow-gates/
│   ├── taskflow-runtime/
│   ├── taskflow-providers/
│   ├── taskflow-observability/
│   ├── taskflow-cli/
│   ├── taskflow-testkit/
│   ├── taskflow-bridge-nim/
│   └── taskflow-bridge-vida/
└── tests/
```

Future-parity note:

1. this workspace shape should be treated as the reference pattern for a later `docflow-rs` rewrite,
2. names and concerns may differ,
3. but the ownership style should remain analogous so both runtimes can coexist without architectural drift.

### 5.1 Crate Ownership

1. `taskflow-core`
   - primitives only
   - task refs, node IDs, statuses, artifact IDs, invariant vocabulary
2. `taskflow-contracts`
   - graph snapshots, gate outcomes, receipt shapes, feature descriptors, store traits
3. `taskflow-config`
   - typed config, merge rules, validation, compiled bundle
4. `taskflow-format-jsonl`
   - canonical JSONL read/write surfaces for runtime artifacts
5. `taskflow-format-toon`
   - canonical TOON encoding/rendering surface
6. `taskflow-state`
   - storage-agnostic state and checkpoint APIs
7. `taskflow-state-fs`
   - filesystem/JSONL state engine for initial velocity
8. `taskflow-state-surreal`
   - embedded `SurrealDB` adapter for direct `VIDA 1` product storage
9. `taskflow-flow`
   - transition engine, route progression, resume/block/escalate law
10. `taskflow-gates`
    - coach/verifier/approval admissibility and outcomes
11. `taskflow-runtime`
    - execution planning, dispatch, rounds, synthesis, replay-safe execution
12. `taskflow-providers`
    - provider registry, contracts, adapters, transport integration
13. `taskflow-observability`
    - tracing, readiness events, proof surfaces, reconciliation helpers
14. `taskflow-cli`
    - binary entrypoints and operator commands only
15. `taskflow-testkit`
   - fixtures, golden artifacts, mocks, property-test helpers
16. `taskflow-bridge-nim`
   - compatibility adapters to current Nim artifacts/paths/commands only
17. `taskflow-bridge-vida`
   - bounded extraction adapters to the current monolithic Rust `vida` line only

### 5.2 Dependency Law Between Crates

1. `taskflow-core` depends on nothing project-specific
2. `taskflow-contracts` depends on `taskflow-core`
3. `taskflow-config` may depend on `taskflow-core` and `taskflow-contracts`
4. `taskflow-format-jsonl` may depend on `taskflow-core` and `taskflow-contracts`
5. `taskflow-format-toon` may depend on `taskflow-core` and `taskflow-contracts`
6. `taskflow-state` may depend on `taskflow-core`, `taskflow-contracts`, and format crates
7. `taskflow-state-fs` and `taskflow-state-surreal` may depend on `taskflow-state`, format crates, and config
8. `taskflow-flow` may depend on `taskflow-core`, `taskflow-contracts`, and `taskflow-state`
9. `taskflow-gates` may depend on `taskflow-core` and `taskflow-contracts`
10. `taskflow-runtime` may depend on `taskflow-core`, `taskflow-contracts`, `taskflow-flow`, `taskflow-gates`, `taskflow-state`, and format crates
11. `taskflow-providers` may depend on `taskflow-core`, `taskflow-contracts`, and `taskflow-runtime` transport contracts
12. `taskflow-observability` may depend on every kernel/runtime crate, but must not become task truth
13. `taskflow-cli` may depend on all workspace crates
14. `taskflow-bridge-nim` and `taskflow-bridge-vida` may depend on all workspace crates, but no kernel crate may depend back on either bridge

Shared-family implication:

1. any crate that proves runtime-family-generic later may be promoted into a shared support crate for both `taskflow-rs` and `docflow-rs`,
2. but no speculative shared crate should be extracted before both runtimes demonstrate the same need.

## 6. Selected Library Policy

This section fixes the initial dependency policy for fast, stable implementation.

### 6.1 Approved Foundation Dependencies

1. `serde`
   - typed serialization/deserialization
2. `serde_json`
   - canonical JSON surface
3. `serde-jsonlines`
   - primary JSONL crate for artifact streams
4. `toon-format`
   - primary TOON crate
5. `clap`
   - CLI parsing and command ergonomics
6. `tokio`
   - async runtime and process/network primitives
7. `tracing`
   - structured spans/events
8. `tracing-subscriber`
   - log/JSON/layer wiring
9. `thiserror`
   - typed library errors
10. `uuid`
    - stable artifact/checkpoint/request identifiers
11. `time`
    - canonical timestamp/time handling
12. `config`
    - config layering/merge support
13. `tempfile`
    - safe temp artifact handling
14. `fs-err`
    - ergonomic filesystem I/O errors

### 6.2 Approved With Limits

1. `anyhow`
   - allowed in binaries and glue code only
   - forbidden as the public error model of kernel crates
2. `indexmap`
   - allowed where deterministic key iteration is part of artifact/config output
   - not a blanket replacement for all maps
3. `tracing-attributes`
   - allowed in runtime/provider/CLI layers
   - avoid as default instrumentation in core law crates
4. `reqwest`
   - allowed in provider adapters only
   - not in kernel crates
5. `futures`
   - allowed later in runtime/provider crates when fan-out, stream composition, or multiplexing actually needs it
   - not required in the initial kernel bootstrap
6. `typify`
   - allowed later only if schema-first contract generation becomes necessary
   - not required for the first kernel line

### 6.3 Rejected Or Deferred For Foundation Use

1. third-party state-machine frameworks such as `rs-statemachine`
2. low-level YAML parser foundations such as `yaml-rust`
3. low-level async signaling crates such as `event-listener`
4. binary hostile-input helpers such as `untrusted`
5. memory-layout/binary optimization crates such as `zerocopy`
6. parser frameworks such as `pest`
   - allowed only later if a real parser/DSL is approved

Reason rule:

1. these crates are not rejected universally,
2. they are rejected as foundation choices for the first `TaskFlow` kernel line because they either solve the wrong layer or would blur the ownership boundary too early.

## 7. Canonical Format Policy

### 7.1 JSONL

Decision:

1. JSONL is mandatory for append-oriented runtime artifacts and ledgers,
2. the primary implementation crate is `serde-jsonlines`,
3. `taskflow-format-jsonl` wraps it with taskflow-specific record types and deterministic write/read policy.

Use cases:

1. event ledgers,
2. receipt streams,
3. recovery logs,
4. replay input streams,
5. provider execution transcripts when they must remain machine-readable.

### 7.2 TOON

Decision:

1. TOON is mandatory for compact human-readable operator and proof surfaces where JSON is too noisy,
2. the primary implementation crate is `toon-format`,
3. `taskflow-format-toon` defines the taskflow-specific rendering profile and canonical field ordering.

Use cases:

1. operator summaries,
2. compact run-graph views,
3. proof summaries,
4. readiness and closure reporting,
5. human review packets.

### 7.3 Format Rule

1. JSONL is the append/log/projection format,
2. TOON is the compact human-facing structured format,
3. neither format is allowed to replace task truth in core state,
4. state remains typed and internal,
5. format crates are view/transport surfaces, not semantic owners,
6. the same JSONL and TOON policy must remain usable by a future `docflow-rs` line so runtime-family operator surfaces stay coherent.

## 8. Kernel State-Machine Model

The kernel state machine must define:

1. states
   - pending, ready, running, blocked, failed, completed, skipped
2. nodes
   - analysis, writer, coach, verifier, approval, synthesis, plus future feature nodes
3. events
   - command-driven, gate-driven, provider-driven, recovery-driven, operator-driven
4. guards
   - authorization, verification required, approval present, recovery available, feature enabled
5. actions
   - write receipt, update graph, write checkpoint, emit proof event, schedule resume
6. context
   - task ref, route snapshot, gate outcomes, receipts, recovery capsule
7. invariants
   - fail-closed transitions, explicit verifier separation, replay-safe writes, deterministic receipt semantics

Hard rule:

1. `taskflow-flow` is the single owner of `next`, `ready`, `blocked`, `escalate`, and `resume`,
2. `taskflow-gates` may return outcomes,
3. but only `taskflow-flow` is allowed to turn those outcomes into task progression.

## 9. Storage And Database Strategy

### 9.1 Initial Storage Line

The first concrete storage line should optimize for speed and inspectability:

1. filesystem-backed state,
2. JSON/JSONL artifacts,
3. checkpoint files,
4. deterministic local replay.

This line exists to unblock early kernel and test work quickly.

### 9.2 Product Storage Line

For direct `VIDA 1`, the product storage line is already decided and must be reflected here.

Primary adapter:

1. embedded `SurrealDB`
   - canonical product database
2. backend `kv-surrealkv`
   - canonical embedded backend
3. adapter crate `taskflow-state-surreal`
   - owns the concrete `SurrealDB` binding for the runtime family
4. bridge/test storage may remain filesystem-backed for early waves
   - but this does not replace the product storage decision

### 9.3 Storage Engine Rule

1. store traits live in `taskflow-contracts`,
2. storage-agnostic APIs live in `taskflow-state`,
3. filesystem and `SurrealDB` engines live in dedicated adapter crates,
4. replay, recovery, and gate law must remain backend-neutral,
5. no storage adapter may redefine task truth semantics,
6. future additional database adapters are optional extension work, not current product law.

## 10. Testing And Proof Strategy

Testing must be layered the same way as the runtime.

### 10.1 Required Test Classes

1. unit tests
   - per crate, per contract, per transition/gate rule
2. integration tests
   - cross-crate flows through real boundaries
3. golden tests
   - JSONL and TOON output stability
4. property tests
   - transition invariants, replay/idempotency, merge admissibility
5. compatibility tests
   - Nim bridge parity where the bridge remains active
6. storage-adapter tests
   - identical behavior across filesystem and `SurrealDB` adapters for admitted shared contracts
7. end-to-end tests
   - kernel execution through explicit closure path

### 10.2 Selected Test Libraries

1. `proptest`
   - property tests for transitions, gates, replay, receipts
2. `insta`
   - snapshot tests for TOON/JSON/JSONL artifacts
3. `assert_fs`
   - filesystem-bound testing
4. `tokio-test`
   - async runtime testing where needed
5. `wiremock`
   - provider/http contract testing

### 10.3 Proof Rule

Every wave must prove all of the following before closure:

1. changed-scope tests pass,
2. no fail-closed invariant is weakened,
3. no canonical format surface regresses silently,
4. no runtime-layer obligation is broken,
5. if the wave touches canonical docs/specs/protocols, lawful `codex-v0` validation/check/proof surfaces pass,
6. if the wave touches the `taskflow -> docflow` seam, sibling `docflow-rs` capability parity for the touched seam stays green,
7. if the wave touches final closure behavior, the `taskflow -> docflow` branch still proves correctly.

## 11. Independent Validation Protocol

Every wave must end with independent review by:

1. `qwen`
2. `kilo`

### 11.1 Review Scope

Each reviewer must evaluate:

1. architecture correctness,
2. contract boundary correctness,
3. regression risk,
4. missing tests,
5. fail-closed behavior,
6. layer-obligation preservation,
7. storage-adapter neutrality when storage code changes.

### 11.2 Closure Rule

A wave is closed only when all are true:

1. changed-scope tests pass,
2. `qwen` review is complete,
3. `kilo` review is complete,
4. critical and high findings are fixed or explicitly recorded as accepted blockers,
5. if docs/specs/protocols changed, `codex-v0` validation/proof checks complete successfully,
6. if the wave changes the cross-runtime seam, the matching `DocFlow` track evidence is attached,
7. if `qwen` and `kilo` disagree, the disagreement is explicitly resolved before closure.

### 11.3 Review Artifact Rule

Each review must produce:

1. findings list,
2. evidence references,
3. severity classification,
4. recommended fixes or explicit no-finding verdict,
5. residual risks.

## 12. Wave Plan

The implementation sequence is optimized for speed with bounded risk.

### 12.0 Wave Mapping Rule

Each wave must identify all of the following explicitly before implementation begins:

1. canonical ownership target crate(s),
2. `taskflow-v0` donor modules, if any,
3. monolithic `crates/vida` donor modules, if any,
4. proof/test surfaces to preserve,
5. bridge code that may be retired after the wave closes.

### 12.1 Joint Cross-Runtime Closure Rule

Any wave that changes the final `taskflow -> docflow` branch must also identify:

1. the required `DocFlow` capability or command-family parity,
2. the exact generated artifact or operator surface consumed across the seam,
3. the bridge surfaces that remain temporarily active,
4. the joint milestone advanced by the wave.

### Wave 0 — Workspace Bootstrap

Deliverables:

1. Cargo workspace created
2. crate skeletons created with ownership boundaries
3. base CI, formatting, lint, and test wiring
4. initial JSONL and TOON format crates created

Exit criteria:

1. workspace builds,
2. empty crate integration tests pass,
3. format smoke tests pass,
4. `qwen` and `kilo` sign off on workspace boundaries.

#### Wave 0 Freeze Packet

The Wave 0 planning gate is closed only when the donor map, command surface, artifact shapes, and dependency policy are explicit.

Approved target crate ownership for the first implementation line:

1. `taskflow-core`
2. `taskflow-contracts`
3. `taskflow-config`
4. `taskflow-format-jsonl`
5. `taskflow-format-toon`
6. `taskflow-state`
7. `taskflow-state-fs`
8. `taskflow-state-surreal`
9. `taskflow-flow`
10. `taskflow-gates`
11. `taskflow-runtime`
12. `taskflow-cli`

Primary donor/runtime sources inspected for parity:

1. `taskflow-v0/src/vida.nim`
2. `taskflow-v0/src/cli/dispatch.nim`
3. `taskflow-v0/src/cli/registry.nim`
4. `taskflow-v0/src/core/config.nim`
5. `taskflow-v0/src/core/direct_consumption.nim`
6. `taskflow-v0/src/state/problem_party.nim`
7. `taskflow-v0/src/state/run_graph.nim`
8. `crates/vida/src/main.rs`
9. `crates/vida/src/state_store.rs`
10. `crates/vida/tests/boot_smoke.rs`
11. `crates/vida/tests/task_smoke.rs`

Current donor command and operator surfaces inspected:

1. `taskflow-v0/src/vida task ready --json`
2. `taskflow-v0/src/vida task show <id> --json`
3. `taskflow-v0/src/vida task export-jsonl .beads/issues.jsonl --json`
4. `vida taskflow help`
5. `vida taskflow <delegated-args>`

Current artifact and state-shape expectations preserved or intentionally bridged:

1. task/epic records remain inspectable as JSON rows
2. dependency edges remain explicit and typed rather than implicit
3. direct-consumption seam remains explicit as `taskflow -> docflow`
4. run/state outputs must remain replay-safe and fail-closed
5. CLI/operator output must stay machine-consumable where JSON is already the donor surface

Approved dependency policy for Wave 0 and Wave 1:

1. kernel and contracts stay provider-neutral, model-neutral, role-neutral, and project-neutral
2. foundation crates may use `serde`, `serde_json`, `serde-jsonlines`, `toon-format`, `thiserror`, `tracing`, and `uuid`
3. CLI shell may additionally use `clap` and `anyhow`
4. storage adapters may use `SurrealDB` only behind adapter crates rather than in kernel crates
5. no third-party state-machine framework is admitted into the kernel
6. no shared crate may be extracted with `docflow-rs` until both sides prove the same need independently

Wave 0 proof rule:

1. the crate map must match the workspace actually present in `crates/**`
2. the donor map must name the currently inspected files and commands rather than aspirational future donors
3. any Wave 1 implementation that changes these boundaries must update this packet first or in the same bounded cycle

### Wave 1 — Core And Contracts

Deliverables:

1. `taskflow-core` primitives
2. `taskflow-contracts` snapshots, gate outcomes, receipt envelopes, feature descriptors, and store traits
3. typed error model for kernel crates

Exit criteria:

1. primitives are stable and minimal,
2. contract tests and property tests pass,
3. no feature semantics leak into contracts.

### Wave 2 — Config And CLI

Deliverables:

1. `taskflow-config` typed loading, merge, validation, compiled bundle
2. `taskflow-cli` operator shell and command layout
3. compatibility bridge for current top-level command expectations where required

Exit criteria:

1. invalid config fails closed,
2. CLI smoke and regression tests pass,
3. config stays provider-neutral, feature-neutral, and storage-neutral.

### Wave 3 — State And Formats

Deliverables:

1. `taskflow-state` storage-agnostic state APIs
2. `taskflow-state-fs` initial filesystem adapter
3. `taskflow-state-surreal` product storage adapter bootstrap
4. JSONL artifact families stabilized
5. TOON rendering profile stabilized

Exit criteria:

1. JSONL/TOON golden tests pass,
2. filesystem and `SurrealDB` state write/read/recovery suites pass for admitted scope,
3. artifact schemas remain deterministic.

### Wave 4 — Flow And Gates

Deliverables:

1. `taskflow-flow` transition engine
2. `taskflow-gates` coach/verifier/approval semantics
3. explicit merge/admissibility policy

Exit criteria:

1. transition property tests pass,
2. verification separation is explicit,
3. replay-safe and fail-closed invariants hold.

### Wave 5 — Runtime And Providers

Deliverables:

1. `taskflow-runtime` execution planning, dispatch, synthesis, replay-safe runtime
2. `taskflow-providers` registry and adapters
3. async execution path where actually needed

Exit criteria:

1. provider adapters do not leak into kernel crates,
2. dispatch/runtime integration tests pass,
3. fan-out/fan-in behavior is deterministic enough for receipts and replay.

### Wave 6 — Observability And Readiness

Deliverables:

1. `taskflow-observability` tracing, proof events, reconciliation helpers
2. readiness and closure surfaces aligned with runtime-layer obligations
3. explicit closure diagnostics for operator use

Exit criteria:

1. observability is not task truth,
2. readiness checks cover closure-critical paths,
3. proof outputs are stable and reviewable.

### Wave 7 — SurrealDB Product Store

Deliverables:

1. `taskflow-state-surreal` is promoted from bootstrap-ready to product-ready
2. `SurrealDB` query/schema/index strategy is hardened for runtime workloads
3. cross-adapter parity tests between filesystem and `SurrealDB` are completed
4. optional future non-product adapters, if ever admitted, are documented behind separate higher-precedence scope decisions

Exit criteria:

1. store contracts remain unchanged,
2. filesystem and `SurrealDB` adapters pass the same behavioral suite where the contract overlaps,
3. no DB-specific semantics leak into kernel law.

### Wave 8 — Nim Bridge And Behavioral Parity

Deliverables:

1. `taskflow-bridge-nim` compatibility adapters
2. `taskflow-bridge-vida` bounded extraction adapters for the current monolithic Rust `vida` line
3. parity tests against retained Nim behavior and already-proven monolithic Rust behavior where still relevant
4. migration notes for bridge retirement

Exit criteria:

1. bridge is one-way and bounded,
2. no Rust kernel crate depends on bridge code,
3. remaining parity gaps are explicit,
4. donor code still in use is enumerated explicitly rather than implied.

### Wave 9 — Direct Runtime Consumption And DocFlow Closure

Deliverables:

1. explicit final `taskflow -> docflow` closure branch
2. direct runtime-consumption path proven with active `docflow-rs` parity for overview/readiness/proof and canonical generated artifacts
3. retirement plan for `codex-v0` bridge surfaces that are no longer needed
4. joint `taskflow-rs` and `docflow-rs` cutover posture documented explicitly

Exit criteria:

1. Layer 9 behavior remains explicit,
2. `DocFlow` proof/readiness branch remains intact through the active `docflow-rs` track,
3. the `docflow-rs` replacement seam is explicit rather than implicit,
4. closure regression tests pass,
5. final `qwen` and `kilo` reviews close with no unresolved high-severity disagreement,
6. the matching `DocFlow` modernization wave evidence is attached and green.

## 13. Delivery Heuristics

To maximize speed without losing rigor:

1. prefer one crate completed enough to run tests over many empty crates,
2. prefer one vertical slice from config to receipt over broad horizontal scaffolding,
3. keep APIs explicit even when macros could shorten code,
4. use async only where a real I/O boundary exists,
5. keep parser work deferred unless a parser is required now,
6. avoid premature genericity for features not yet admitted into the kernel line,
7. get filesystem and bounded `SurrealDB` storage slices working early, then deepen product readiness over the same contracts,
8. treat every good `taskflow-rs` boundary as a likely template for sibling `docflow-rs`, but do not force premature shared-crate extraction.

## 14. Completion Criteria

This program is complete only when:

1. the Rust workspace owns the kernel law directly,
2. the Nim line is reduced to a bounded bridge or retired,
3. no project-specific provider/model/role/profile/flow behavior remains in kernel crates,
4. flow owns progression, gates own gate outcomes, state owns persistence, runtime owns mechanics, formats own rendering, providers own transport, and storage adapters own backend integration,
5. runtime-layer obligations through Layer 9 are preserved,
6. documentation/readiness obligations needed by `DocFlow` are preserved,
7. all waves have passed tests and independent `qwen/kilo` review,
8. `taskflow -> docflow` final runtime-consumption activation remains explicit and proven,
9. the crate/family boundaries remain suitable for the active sibling `docflow-rs` line and its eventual cutover.

## 15. Immediate Next Step

The next lawful implementation step after this spec is:

1. create the Rust workspace skeleton,
2. implement `taskflow-core`, `taskflow-contracts`, `taskflow-format-jsonl`, and `taskflow-format-toon` first,
3. add `taskflow-state`, `taskflow-state-fs`, and the minimal `taskflow-state-surreal` bootstrap before provider work,
4. keep the current Nim runtime and monolithic Rust `vida` line unchanged except for bounded compatibility/proof needs until Wave 1 through Wave 4 are executable,
5. treat the resulting crate boundaries as the reference template for the active sibling `docflow-rs` line rather than inventing a separate architecture later.

-----
artifact_path: product/spec/taskflow-v1-runtime-modernization-plan
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/taskflow-v1-runtime-modernization-plan.md
created_at: '2026-03-10T20:59:00+02:00'
updated_at: '2026-03-12T07:48:27+02:00'
changelog_ref: taskflow-v1-runtime-modernization-plan.changelog.jsonl
