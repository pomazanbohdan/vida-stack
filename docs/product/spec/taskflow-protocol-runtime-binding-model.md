# TaskFlow Protocol Runtime Binding Model

Status: active product spec

Revision: `2026-03-12`

Purpose: define how `taskflow` becomes a runtime that actually executes against canonical protocol law rather than merely coexisting with protocol documents, using an interim script-era bridge and a dedicated Rust subrelease for compiled protocol binding.

## 1. Problem

The current system already has substantial protocol canon under `vida/config/instructions/**`, but `taskflow-v0` is still written largely as a runtime substrate beside that canon rather than as a runtime that deterministically consumes protocol contracts.

That means:

1. protocol activation is documented, but not yet fully runtime-resolved as executable policy,
2. protocol gates and blockers are documented, but not yet consistently enforced through one compiled binding layer,
3. proof and receipt expectations are documented, but not yet emitted from one explicit protocol-runtime execution path,
4. runtime work can continue without a hard protocol-binding release slice, which risks deeper drift between implementation and canon.

## 1.1 Current Binding Rule

The current bridge wave must treat the DB-backed `taskflow` runtime state as the primary binding authority.

That means:

1. protocol-binding state must anchor to the same authoritative runtime state root used by `taskflow`,
2. standalone file logs are not a valid primary truth surface for protocol binding,
3. JSON snapshots may exist as exported receipts or debug artifacts,
4. but binding truth, status, and queryability must resolve from the DB/taskflow runtime path first.

## 2. Goal

`taskflow` must become protocol-driven in a fail-closed way.

That requires one binding layer that can:

1. resolve which protocols are active for a given runtime situation,
2. load executable protocol contracts rather than relying on prose interpretation alone,
3. execute gates, transitions, and close/handoff checks from those contracts,
4. emit receipts and state artifacts that prove which protocol rules were activated and enforced,
5. keep the current script-era runtime usable while the Rust-native binding layer is being built.

## 3. Release Rule

This work must not remain an implicit background improvement inside general runtime writing.

It requires one explicit subrelease:

1. `TaskFlow Protocol Binding Subrelease`

Rule:

1. runtime modernization work that claims stronger protocol alignment must not bypass this subrelease,
2. features added to `taskflow` after this point should not be treated as protocol-bound by default unless they go through the binding layer defined here,
3. the binding subrelease is the canonical delivery slice that turns protocol canon into executable runtime authority.

## 4. Two-Track Delivery Model

The delivery model has two parallel tracks.

### 4.1 Track A: Script-Era Binding Bridge

Purpose:

1. provide an immediate bounded operational bridge while the Rust-native runtime is not yet ready,
2. let the current repository validate activation, gate selection, and receipt expectations against protocol canon now,
3. surface protocol-binding gaps before the Rust implementation is treated as complete.

Canonical shape:

1. one bounded script-era protocol binding tool,
2. reads protocol-routing and activation surfaces,
3. produces machine-readable binding snapshots and gap reports,
4. remains bridge-only and must not become the long-term owner of protocol-runtime law.

Current implemented bridge surface:

1. `taskflow-v0 protocol-binding build [--json]`
2. `taskflow-v0 protocol-binding sync [--json]`
3. `taskflow-v0 protocol-binding status [--json]`
4. `taskflow-v0 protocol-binding check [--json]`
5. installed and launcher-owned delegation may still expose the same bounded surface through `vida taskflow ...` or installer bootstrap, but `taskflow-v0` is the script-era primary owner for `v0.2.2`

Current state rule:

1. the implemented bridge stores binding rows and receipts in the authoritative TaskFlow state store,
2. the bridge first materializes a deterministic compiled JSON payload under `taskflow-v0/generated/protocol_binding.compiled.json`,
3. installer bootstrap and repo-local sync import that compiled payload into `taskflow-state.db`,
4. runtime execution fails closed when the DB-backed protocol-binding state is missing or invalid,
5. the installed runtime keeps the same authoritative state path and does not fall back to detached file-log truth,
6. this closes the first DB-backed proof slice for the `v0.2.2` bridge wave without yet claiming Rust-native crate closure.

Compiled-control-bundle linkage rule:

1. the current protocol-binding compiled payload is not the final top-level Release-1 control bundle by itself,
2. it is one bounded executable input that must later become the `protocol_binding_registry` section of the strict top-level compiled control bundle,
3. the final schema must therefore preserve the current protocol row semantics while making the protocol-binding payload one section of a wider runtime control contract instead of a standalone forever-format.

Minimum responsibilities:

1. resolve active protocol set for a bounded runtime scenario,
2. classify each active protocol by enforcement type:
   - activation-only
   - pre-execution gate
   - in-flight transition rule
   - closure or handoff gate
   - recovery gate
   - proof or receipt requirement
3. report missing runtime owners for active protocols,
4. emit a deterministic binding snapshot for review and later Rust parity tests.

Current implementation location:

1. bounded `taskflow-v0` tooling surface
2. `taskflow-v0/config/protocol_binding.seed.json` as the script-era metadata owner
3. `taskflow-v0/generated/protocol_binding.compiled.json` as the deterministic compiled bridge artifact
4. `taskflow-v0/helpers/turso_task_store.py` as the DB materialization layer
5. `scripts/precommit-build-json.sh` as the thin JSON-artifact build hook

Recommended output artifacts:

1. DB-backed TaskFlow binding rows or equivalent state-store records under the authoritative runtime state root
2. one deterministic compiled bridge payload under `taskflow-v0/generated/protocol_binding.compiled.json`
3. bounded JSON export snapshots derived from that runtime truth for proof, parity, or operator inspection

Invalid pattern:

1. plain file-log-only protocol binding state
2. append-only JSONL audit files treated as authoritative binding truth

Reason:

1. file logs drift too easily from runtime state,
2. they are not query-safe enough for the current taskflow runtime direction,
3. the bridge must already follow the DB/taskflow-first authority that the Rust-native layer will keep.

### 4.2 Track B: Rust-Native Protocol Runtime Binding

Purpose:

1. move protocol activation and enforcement into compiled runtime code,
2. give `taskflow` one explicit protocol binding engine,
3. stop relying on distributed shell/scripts as the only practical way to honor protocol law.

Canonical shape:

1. one dedicated Rust crate for protocol-runtime binding,
2. optional helper modules inside `crates/vida` and `taskflow-v0` that consume that crate,
3. shared typed contracts for activation, gates, blockers, receipts, and proof requirements.

Recommended crate:

1. `crates/vida_protocol_runtime`

Minimum module set:

1. `contracts`
   - compiled protocol contract structs/enums
2. `activation`
   - activation-class and trigger resolver
3. `registry`
   - protocol-to-runtime owner lookup
4. `gates`
   - gate and blocker executor interfaces
5. `receipts`
   - emitted protocol activation/enforcement receipts
6. `proof`
   - proof expectations and validation hooks

Rule:

1. protocol-binding logic must not remain scattered across unrelated runtime modules once this crate exists,
2. new protocol-bound runtime behavior should be added through this crate first, not as ad hoc direct wiring.

Primary ownership rule:

1. the crate must bind to the authoritative runtime state store used by `taskflow`,
2. it must not introduce a parallel file-log truth path for activation or enforcement state,
3. exported JSON artifacts are secondary receipts, not the primary protocol state authority.

## 5. Required Runtime Binding Contract

For each runtime-bearing protocol, the binding layer must be able to materialize:

1. `protocol_id`
2. `activation_class`
3. `activation_trigger`
4. `owner_layer`
5. `runtime_owner`
6. `enforcement_type`
7. `required_inputs`
8. `blocker_codes`
9. `expected_receipts`
10. `proof_requirements`
11. `state_surfaces`

Compact rule:

1. if one of these is missing, the protocol may still exist as canon,
2. but it is not yet runtime-bound in the strong sense required by this spec.

Primary state rule:

1. `state_surfaces` must identify the DB/taskflow authority first,
2. exported snapshots may be listed only as secondary proof or operator surfaces,
3. file-log-only state surfaces are invalid for this subrelease.
4. for the current script-era implementation, the authoritative DB path is `.vida/state/taskflow-state.db`.

## 6. Binding Sources

The binding layer must consume and stay aligned with at least:

1. `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`
2. `vida/config/instructions/system-maps/framework.protocol-domains-map.md`
3. `vida/config/instructions/system-maps/framework.protocol-layers-map.md`
4. `vida/config/instructions/system-maps/protocol.index.md`
5. runtime-bearing protocol owners under `vida/config/instructions/runtime-instructions/**`
6. closure and diagnostic protocol owners when they materially affect runtime behavior

The binding layer must not treat changelog entries, transition notes, or generated status artifacts as primary protocol law.

Runtime truth rule:

1. the binding layer must read and write through the authoritative taskflow/runtime state path where the bound protocol affects runtime execution,
2. detached file logs may be emitted for debugging only when they are derived from runtime truth and can be regenerated.

## 7. Binding Matrix Requirement

This subrelease requires one explicit matrix that maps:

1. protocol
2. runtime module or tool owner
3. activation trigger
4. enforcement type
5. receipt output
6. proof surface
7. current status:
   - `unbound`
   - `script-bound`
   - `rust-bound`
   - `fully-runtime-bound`
8. primary state authority
9. secondary export or receipt surfaces

Rule:

1. no runtime-bearing protocol should be claimed as fully integrated without this matrix row,
2. the matrix is the canonical planning and audit surface for binding progress.

## 8. Subrelease Scope

The `TaskFlow Protocol Binding Subrelease` is closed only when all are true:

1. the script-era bridge can resolve bounded active protocol sets and emit deterministic binding snapshots,
2. the Rust crate exists and is the canonical owner of protocol-runtime binding logic,
3. `taskflow` can execute at least one bounded end-to-end runtime path through compiled protocol activation and gate enforcement,
4. runtime receipts show which protocols were activated and enforced,
5. protocol-binding parity between the script bridge and the Rust layer is testable,
6. runtime work after this subrelease no longer needs to pretend that prose-only protocol alignment is enough.
7. script-era bridge outputs are queryable from DB/taskflow runtime truth rather than only from detached file logs.

## 9. Minimum First Bound Slice

The first bound slice should cover the smallest runtime-bearing set that proves the architecture:

1. `bridge.instruction-activation-protocol`
2. `work.taskflow-protocol`
3. `runtime.task-state-telemetry-protocol`
4. `work.execution-health-check-protocol`
5. `work.task-state-reconciliation-protocol`

Reason:

1. this set is enough to prove activation, in-flight execution discipline, close/handoff gating, and stale/drift reconciliation,
2. it is smaller and safer than trying to bind the full runtime family in one wave.

Implementation priority rule:

1. use the existing DB-backed taskflow implementation and state-store surfaces as the primary host for this slice,
2. prefer launcher-owned `vida taskflow` query and mutation paths over new detached helper stores,
3. treat file-log outputs as optional exports only after DB-backed truth exists.

## 10. Non-Goals

This spec does not require, yet:

1. full DB-first runtime completion,
2. full direct runtime consumption closure for every protocol family,
3. replacing all script-era helpers immediately,
4. making every framework protocol runtime-bound at once.

## 11. Acceptance Rule

This spec is satisfied only when runtime claims about protocol alignment can be checked mechanically.

That means:

1. a protocol can be shown as active by runtime evidence,
2. a protocol gate can be shown as executed or blocked by runtime evidence,
3. a protocol proof requirement can be shown as satisfied or missing by runtime evidence,
4. the current runtime no longer depends on hidden human interpretation to claim protocol binding.

Database-first acceptance rule:

1. the operator must be able to inspect protocol-binding state through taskflow/runtime query surfaces backed by the authoritative state store,
2. an exported JSON file alone is not sufficient evidence of binding closure.

## 12. References

1. `docs/product/spec/canonical-runtime-layer-matrix.md`
2. `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`
3. `docs/product/spec/compiled-runtime-bundle-contract.md`
4. `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`
5. `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
6. `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`
7. `vida/config/instructions/diagnostic-instructions/analysis.protocol-consistency-audit-protocol.md`

-----
artifact_path: product/spec/taskflow-protocol-runtime-binding-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/taskflow-protocol-runtime-binding-model.md
created_at: '2026-03-12T12:15:00+02:00'
updated_at: '2026-03-12T19:00:00+02:00'
changelog_ref: taskflow-protocol-runtime-binding-model.changelog.jsonl
