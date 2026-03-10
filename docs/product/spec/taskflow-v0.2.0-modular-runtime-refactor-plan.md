# TaskFlow v0.2.0 Modular Runtime Refactor Plan

Status: active product refactor plan

Purpose: define the canonical refactor target for `taskflow-v0` so the current transitional runtime becomes modular, adaptive, feature-driven, and free from project-specific hardcoded behavior in the framework runtime core.

## 1. Core Direction

The final `taskflow-v0` architecture must follow this rule:

1. framework code hardcodes law,
2. features provide behavior,
3. project-specific routing, provider, model, role, skill, profile, and flow choices must not be hardcoded in the base runtime,
4. runtime mechanics must be shared,
5. domain behavior must be extension-driven.

Compact rule:

1. `taskflow-v0` should evolve from a command-and-helper collection into a modular runtime kernel plus feature packs.

## 2. Target Runtime Shape

Target top-level source tree:

```text
taskflow-v0/src/
├── cli/
├── framework/
├── runtime/
├── features/
├── flow/
├── state/
├── gates/
├── providers/
├── config/
└── vida.nim
```

Target ownership:

1. `cli/`
   - thin command adapter and command registry only
2. `framework/`
   - runtime law, contracts, kernel registration, and invariant surfaces only
3. `runtime/`
   - shared execution mechanics such as dispatch, rounds, synthesis, verification, receipts, and recovery
4. `features/`
   - feature-owned policy packs and semantics, including `Party Chat`
5. `flow/`
   - one explicit owner for `next/ready/blocked/escalate/resume`
6. `state/`
   - persistence only
7. `gates/`
   - gate logic only
8. `providers/`
   - provider/model registry and backend adapters only
9. `config/`
   - loader, accessors, schema, and validation modules only

## 3. Architectural Rules

### 3.1 Framework Rule

Framework runtime code may hardcode only:

1. primitive execution types,
2. artifact base contracts,
3. gate result classes,
4. transition classes,
5. fail-closed validation rules,
6. extension registration rules,
7. ownership boundaries.

Framework runtime code must not hardcode:

1. project-specific route profiles,
2. provider names,
3. model names,
4. project role IDs,
5. project skill IDs,
6. project profile IDs,
7. project flow IDs,
8. feature-specific board compositions.

### 3.2 Feature Rule

Anything that is not base runtime law must be implemented as a feature layer over the framework/runtime kernel.

This includes:

1. `Party Chat`,
2. feature-specific route policies,
3. provider/model binding preferences,
4. project council semantics,
5. project-specific packets,
6. project role, skill, profile, and flow sets.

### 3.3 Runtime Rule

Shared runtime must own mechanics, not feature semantics.

Shared runtime mechanics include:

1. dispatch planning,
2. session planning,
3. round execution,
4. synthesis/aggregation,
5. verification stage,
6. execution artifacts,
7. resumability/recovery.

Feature semantics include:

1. entry triggers,
2. board presets,
3. role meaning,
4. packet semantics,
5. escalation policy,
6. DEI policy.

## 4. Current Hotspots Requiring Refactor

### 4.1 CLI Bottleneck

Current hotspot:

1. `taskflow-v0/src/vida.nim`

Problem:

1. command dispatch is centralized in one large `case`,
2. every new runtime surface increases central coupling,
3. the CLI entrypoint acts as a command registry, dependency aggregator, and policy choke point.

Required refactor:

1. move command dispatch into `cli/dispatch.nim`,
2. introduce command registration or per-command dispatch modules,
3. reduce `vida.nim` to a thin bootstrap adapter.

### 4.2 Config Monolith

Current hotspot:

1. `taskflow-v0/src/core/config.nim`

Problem:

1. YAML loading, accessors, and broad multi-domain validation are colocated,
2. `agent_system`, `agent_extensions`, `autonomous_execution`, and `party_chat` validation all live in one file,
3. this will continue to grow as more features are promoted.

Required refactor:

1. split into:
   - loader,
   - accessors,
   - validation aggregator,
   - per-domain validators.

### 4.3 Party Chat Mini-Engine

Current hotspot:

1. `taskflow-v0/src/state/problem_party.nim`

Problem:

1. one file now owns:
   - manifest rendering,
   - role/profile resolution,
   - dispatch planning,
   - session planning,
   - seat prompts,
   - synthesis packets,
   - execute path,
   - receipt writing,
   - partial runtime gating,
2. this is feature behavior mixed with shared runtime mechanics.

Required refactor:

1. move shared orchestration mechanics into `runtime/`,
2. move Party Chat policy and packets into `features/party_chat/`,
3. leave a thin adapter surface for `problem_party`.

### 4.4 Route / Flow Ownership Blur

Current hotspots:

1. `taskflow-v0/src/agents/route.nim`
2. `taskflow-v0/src/state/run_graph.nim`
3. `taskflow-v0/src/gates/**`
4. feature-level runtime files

Problem:

1. route selection, next-step decisions, and readiness/blocking semantics are still partially distributed,
2. there is no single transition owner for all taskflow state movement.

Required refactor:

1. introduce `flow/transition_engine.nim`,
2. centralize `next/ready/blocked/escalate/resume` decisions there.

## 5. Final Target Module Shape

### 5.1 CLI

```text
cli/
├── dispatch.nim
├── registry.nim
└── commands/
    ├── config_cmd.nim
    ├── route_cmd.nim
    ├── task_cmd.nim
    ├── problem_party_cmd.nim
    ├── prepare_execution_cmd.nim
    └── ...
```

### 5.2 Framework

```text
framework/
├── primitives/
│   ├── execution.nim
│   ├── artifacts.nim
│   ├── transitions.nim
│   ├── sessions.nim
│   ├── verification.nim
│   └── recovery.nim
├── contracts/
│   ├── artifact_contracts.nim
│   ├── gate_contracts.nim
│   ├── transition_contracts.nim
│   └── feature_contracts.nim
└── kernel/
    ├── orchestration_kernel.nim
    ├── feature_registry.nim
    ├── runtime_bundle.nim
    └── dispatch_kernel.nim
```

### 5.3 Shared Runtime

```text
runtime/
├── execute.nim
├── dispatch.nim
├── rounds.nim
├── synthesis.nim
├── verifier.nim
├── receipts.nim
├── plans.nim
└── observability.nim
```

### 5.4 Features

```text
features/
├── party_chat/
│   ├── feature.nim
│   ├── policy.nim
│   ├── packets.nim
│   ├── prompts.nim
│   ├── routing.nim
│   ├── validation.nim
│   └── registries/
├── prepare_execution/
├── spec_intake/
├── spec_delta/
└── ...
```

### 5.5 Flow

```text
flow/
├── transition_engine.nim
├── route_entry.nim
├── node_rules.nim
├── escalation.nim
└── taskflow_bridge.nim
```

### 5.6 Config

```text
config/
├── loader.nim
├── accessors.nim
├── schema.nim
├── bundle_builder.nim
└── validation/
    ├── framework.nim
    ├── providers.nim
    ├── features.nim
    ├── routing.nim
    └── aggregate.nim
```

### 5.7 Providers

```text
providers/
├── registry.nim
├── contracts.nim
├── resolver.nim
├── backends/
│   ├── qwen.nim
│   ├── kilo.nim
│   ├── opencode.nim
│   └── ...
└── models/
```

## 6. Unified Artifact Family

The final runtime should converge on one canonical artifact family:

1. `manifest`
2. `dispatch_plan`
3. `session_plan`
4. `execution_artifact`
5. `verification_artifact`
6. `receipt`
7. `recovery_capsule`

Required shared fields:

1. `runtime_domain`
2. `feature_id`
3. `status`
4. `reason`
5. `inputs`
6. `outputs`
7. `next_step`
8. `blocking_conditions`
9. `written_at`

## 7. Full Refactor Program

Cross-wave execution rules:

1. every wave must end with full bounded testing for the changed scope rather than ad hoc spot checks,
2. every wave must end with independent architecture/code review by `qwen` and `kilo`,
3. `qwen` and `kilo` reviews are mandatory closure gates for each wave, not optional postscript validation,
4. the next wave must not begin until:
   - changed-scope tests pass,
   - `qwen` review is complete,
   - `kilo` review is complete,
   - material findings are either fixed or recorded as explicit blocking decisions,
5. if `qwen` and `kilo` disagree, the disagreement must be resolved explicitly before the wave is considered closed.

### Phase A — Structural Decoupling

Deliverables:

1. CLI dispatch extracted from `vida.nim`
2. config loader/access/validation split
3. command registry introduced

Required testing:

1. config validation regression tests
2. command dispatch regression tests
3. CLI compatibility tests for existing top-level commands
4. fail-closed tests for invalid overlay/config/provider/feature settings

Required independent review:

1. `qwen` review of modularization and compatibility risk
2. `kilo` review of modularization and compatibility risk

### Phase B — Shared Runtime Kernel

Deliverables:

1. shared dispatch/session/execute primitives
2. shared synthesis/verification primitives
3. flow transition ownership extracted

Required testing:

1. unit tests for dispatch/session/synthesis/receipt primitives
2. regression tests for existing runtime commands now using shared primitives
3. artifact-schema tests for manifest/dispatch/session/execution/receipt outputs
4. fail-closed tests for invalid manifests and invalid shared-runtime inputs

Required independent review:

1. `qwen` review of shared-runtime ownership boundaries
2. `kilo` review of shared-runtime ownership boundaries

### Phase C — Party Chat Promotion

Deliverables:

1. Party Chat becomes a feature pack over shared runtime
2. `problem_party` becomes a thin compatibility/feature adapter
3. no Party Chat-specific runtime mechanics remain in shared framework code

Required testing:

1. Party Chat manifest/render/dispatch/session/synthesis/receipt regression tests
2. feature-pack resolution tests through registries and overlay config
3. no-hardcode regression tests for Party Chat provider/model/role/profile resolution
4. compatibility tests for `problem-party` command behavior

Required independent review:

1. `qwen` review of Party Chat extraction and feature-boundary correctness
2. `kilo` review of Party Chat extraction and feature-boundary correctness

### Phase D — Provider And Feature Registry Hardening

Deliverables:

1. provider/model resolution becomes registry-driven
2. role/skill/profile/flow behavior becomes fully feature/config-driven
3. no project-specific provider/model/role constants remain in base runtime

Required testing:

1. provider registry tests
2. resolver fallback/fail-closed tests
3. feature registration and activation tests
4. overlay-driven role/skill/profile/flow resolution tests
5. negative tests proving unknown providers/models/features are rejected cleanly

Required independent review:

1. `qwen` review of registry design and de-hardcode completeness
2. `kilo` review of registry design and de-hardcode completeness

### Phase E — Full Runtime Integration

Deliverables:

1. lawful TaskFlow integration
2. explicit verifier and resume behavior
3. observability and recovery completion
4. final docs/release normalization

Required testing:

1. end-to-end TaskFlow integration tests
2. verifier/approval/resume regression tests
3. checkpoint/recovery/replay tests
4. observability/readiness/proving tests for the changed runtime paths
5. cross-feature execution and transition tests

Required independent review:

1. `qwen` review of runtime integration, gate preservation, and recovery behavior
2. `kilo` review of runtime integration, gate preservation, and recovery behavior

### Phase F — Final Runtime-Consumption And Closure Proof

Deliverables:

1. direct runtime-consumption path remains explicit and intact after refactor
2. explicit `taskflow -> codex` final-layer activation remains testable
3. final closure proof is refreshed for the refactored runtime

Required testing:

1. direct runtime-consumption tests
2. explicit `taskflow -> codex` activation tests
3. readiness/proof snapshot tests
4. final closure regression tests covering the canonical runtime-consumption loop

Required independent review:

1. `qwen` review of final runtime-consumption integrity
2. `kilo` review of final runtime-consumption integrity

## 8. Missing Or Incomplete Code Today

The following items are still absent or only partially implemented in code and must be added before the architecture can be considered fully complete.

### 8.1 Shared Kernel / Modularity Gaps

Missing:

1. command registry / modular CLI dispatch
2. shared orchestration kernel
3. explicit transition engine
4. domain-separated runtime mechanics modules
5. plugin-style feature registration layer

### 8.2 Party Chat Runtime Gaps

Missing or partial:

1. live multi-agent seat spawning
2. actual provider-backed round execution
3. verifier execution stage as a real runtime phase
4. DEI runtime policy
5. fully generic council runtime mechanics separated from Party Chat policy

### 8.3 Config / Registry Gaps

Missing or partial:

1. fully split config architecture
2. provider registry as a first-class runtime module
3. feature registration schema beyond current ad hoc feature blocks
4. removal of remaining Party Chat-specific constants from base code

### 8.4 Flow / Runtime Integration Gaps

Missing or partial:

1. fully centralized next-step ownership
2. automatic feature entry from general runtime transitions
3. shared runtime ownership of execution lifecycle across all domains

### 8.5 Observability / Recovery Gaps

Missing or partial:

1. unified artifact family across all runtime domains
2. shared per-round observability for live council execution
3. generalized recovery model for feature executions beyond current partial artifacts

### 8.6 Testing / Quality Gates Gaps

Missing or partial:

1. one complete wave-by-wave regression test program for the refactor
2. explicit artifact-schema tests across shared runtime outputs
3. direct runtime-consumption regression tests after modular extraction
4. mandatory independent `qwen` and `kilo` review gates after each wave
5. explicit disagreement-resolution rule when independent reviewers diverge

## 9. Hard Rule For Completion

This refactor is not complete until all of the following are true:

1. base runtime no longer hardcodes project-specific routes, providers, models, roles, skills, profiles, or flows,
2. features register behavior through canonical extension interfaces,
3. shared runtime owns execution mechanics,
4. flow layer owns task progression,
5. config/validation is modular and fail-closed,
6. Party Chat is a feature pack over the shared runtime rather than a bespoke mini-engine,
7. every refactor wave has passed its bounded changed-scope tests,
8. every refactor wave has received independent `qwen` and `kilo` review,
9. no unresolved material review disagreement remains open at final closure.

## 10. Completion Evidence

This refactor program should be considered closed only when:

1. code structure matches the final ownership model,
2. old monolithic hotspots have been reduced or retired,
3. external validation confirms no new critical/high architecture defects,
4. docs, runtime, and config law all describe the same architecture,
5. the final refactored runtime still passes direct runtime-consumption and closure proof,
6. wave-by-wave `qwen` and `kilo` review evidence exists for the full refactor sequence.

-----
artifact_path: product/spec/taskflow-v0.2.0-modular-runtime-refactor-plan
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: docs/product/spec/taskflow-v0.2.0-modular-runtime-refactor-plan.md
created_at: '2026-03-10T23:59:00+02:00'
updated_at: '2026-03-10T21:03:36+02:00'
changelog_ref: taskflow-v0.2.0-modular-runtime-refactor-plan.changelog.jsonl
