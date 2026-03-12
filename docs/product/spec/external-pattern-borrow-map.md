# VIDA External Pattern Borrow Map

Status: draft `v1` design input

Revision: `2026-03-09`

Purpose: record which external state-machine and agent-runtime patterns are intentionally borrowed into the `vida` kernel, which are deferred, and which are explicitly rejected.

## 1. Source Families

Primary external references used in this pass:

1. `python-statemachine` documentation and release notes
2. LangChain OSS and LangGraph runtime documentation
3. Elsa Workflows ADRs
4. Eventuous runtime documentation

Rule:

1. These sources are semantic references only.
2. They do not replace root `vida/config` law.
3. They do not authorize importing provider-specific workflow behavior into product law.

## 2. Borrowed From python-statemachine

### 2.1 Adopted Now

The current thin kernel adopts these semantics now:

1. `event_id` separate from human-readable transition names
2. `event_aliases` support for alternate event identifiers
3. global transitions through `from_any`
4. ordered transition matching under one event/command
5. explicit distinction between transition trigger and transition display name

Why:

1. they strengthen machine/event semantics without changing root ownership boundaries
2. they preserve explicit trigger law while letting runtime accept alternate command names

VIDA mapping:

1. `transition_engine.nim`
2. root machine transition specs

### 2.2 Adopted As Kernel Direction

These patterns are accepted as future kernel direction but not fully implemented in this pass:

1. internal/eventless automatic transitions
2. compound state semantics
3. parallel state semantics
4. history/resume semantics
5. listener-based side-effect separation
6. strict machine-definition validation for unreachable or conflicting transition graphs

Why:

1. `execution_plan`, `verification_lifecycle`, and `boot_migration_gate` naturally benefit from compound/history semantics
2. `verification_lifecycle` may require parallel aggregation semantics
3. projections should evolve toward listeners instead of transition-side ad hoc effects

VIDA mapping:

1. future execution-plan resume logic
2. future verification aggregation
3. projection/listener kernel
4. future `vida config lint` for alias conflicts, unreachable states, and illegal overlaps

### 2.3 Rejected As-Is

These are not adopted as product-law mechanisms:

1. direct dependency on `python-statemachine` as current runtime core
2. Python callback injection as canonical behavior law
3. library-specific DSL/API as root config law

Reason:

1. `vida` already owns a config-driven kernel contract
2. product law must stay YAML plus explicit receipts/proofs
3. runtime implementation must remain swappable between `taskflow-v0` and future `vida-v1`

## 3. Borrowed From LangChain OSS / LangGraph

### 3.1 Pattern Decisions

| Pattern | Decision | Why | VIDA Mapping |
|---|---|---|---|
| Explicit runtime context object | adopt now | Fits current guard and instruction composition boundaries without moving policy into agents | transition context, assignment context, instruction composition |
| Middleware/listener layer above core transitions | adopt now | Gives a lawful place for projection and subscription derivation without mutating canonical state in callbacks | derived projection/listener kernel |
| Structured output contracts with validation | adopt now | Aligns with explicit role output contracts already present in instruction catalog | instruction catalog, role output contracts, proof requirements |
| Human interrupt/resume semantics | adopt now | LangGraph confirms pause/resume is a runtime primitive, not just UI wording; VIDA should keep it receipt-backed and fail-closed | manual intervention, approval waits, resumability checkpoints, governance interrupts |
| Dynamic runtime capability exposure | adopt now | Already consistent with overlay-derived runtime inventory and bounded capability matching | `agent_inventory.nim`, `assignment_engine.nim` |
| Stream/update subscriptions for runtime observers | adopt now | Useful as rebuildable operator projection channels, not as canonical state | projection topics, listener topics, status/readiness surfaces |
| Time-travel and fork-from-checkpoint debugging | adopt as future direction | Valuable for proof repro and doctor/debug replay, but must not redefine canonical truth | replay from checkpoint, proof reproduction, debug forks |
| Pending checkpoint writes after partial failure | adopt as future direction | Useful for resumable retry without re-running already successful work | future checkpoint write ledger and retry law |
| Shared-state graph execution with explicit nodes and edges | adopt now | Matches VIDA direction better than transcript-only execution and keeps control-flow explicit | route graph, run graph, compiled control bundle |
| Reducer-style merge semantics for parallel state updates | adopt as future direction | Needed once specialist fanout and parallel branches become stronger runtime reality | verification aggregation, future fanout merge law |
| Subgraphs with namespace isolation | adopt now | Strong fit for bounded specialist composition under one orchestrator | specialist subflows, future multi-agent graph runtime |
| Durable execution with stable execution identity | adopt now | Confirms the need for resumable execution lineage owned by VIDA rather than by vendor threads | run graph, checkpoint lineage, project-local DB truth |
| Memory as structured runtime state rather than transcript inheritance | adopt now | Fits VIDA's DB-first and governed-context direction | context governance, memory/query surfaces, runtime cache |
| Replay-safe side-effect discipline | adopt as future direction | Durable execution requires explicit treatment of non-repeatable actions | future execution-boundary policy, replay guards |
| Vendor auth or request middleware as kernel gateway | reject as-is | Middleware stacks are adapter behavior, not root product law | framework adapter layer only |
| Provider-owned threads/checkpointers as kernel identity | reject as-is | `vida` must own checkpoint identity and semantics even if adapters bridge later | checkpoint kernel over `vida` state and receipts |

## 4. Borrowed From Elsa Workflows ADRs

### 4.1 Pattern Decisions

| Pattern | Decision | Why | VIDA Mapping |
|---|---|---|---|
| Execution snapshots captured at execution time | adopt now | Strong fit for checkpoint/proof correctness and replay/debug value | execution checkpoint snapshot, proof attachments, operator inspection |
| Durable suspension markers persisted with execution context | adopt as future direction | VIDA has no bookmark entity today, but the durability rule is valid | route blocked/manual intervention state, approval wait handles |
| Correlation-based bookmark hashes for resume targeting | adopt as future direction | Strong fit for lawful resume targeting without broad scan | future gateway handle, resume token, correlation targeting |
| Trigger indexing for start/resume matching | adopt as future direction | Valuable for bounded resume lookup and gateway routing | future trigger index over gateway handles |
| Burn-on-success bookmark consumption and duplicate-resume protection | adopt as future direction | Strong fit for one-time gateway handles and explicit repeated-resume failure | future gateway handle consumption policy |
| Distributed locking around resume of the same suspended instance | adopt as future direction | Useful for clustered safety without changing kernel ownership | future resume lock boundary around gateway handles |
| Token-centric branch execution state | reject as-is | VIDA route kernel is stage-based, not free-form flowchart token routing | possible future branch module only |
| Explicit merge modes for joins | adopt as future direction | Valuable only when VIDA introduces lawful parallel branches | verification aggregation, future execution-plan joins |
| Fault/cancellation propagation across child branches | adopt as future direction | Useful once parallel route nodes exist, but current kernel is mostly linear stage law | future branch cancellation and escalation |
| Explicit wait-all / merge-mode restoration after deadlock regressions | adopt as future direction | Useful signal that merge semantics must stay explicit and testable | future verification merge law and branch joins |

### 4.2 Elsa-Specific Rejections

These Elsa patterns are not adopted as product-law architecture:

1. workflow bookmark API as a first-class root entity in this pass
2. flowchart token list as the primary route state model
3. activity-level merge mode properties as current root route law

Reason:

1. current `vida` kernel is stage/machine driven, not graph-activity driven
2. introducing bookmark/token entities now would widen scope beyond the lawful thin kernel
3. borrowed value is semantic durability and explicit merge policy, not Elsa's workflow object model

## 5. Borrowed From Eventuous

### 5.1 Pattern Decisions

| Pattern | Decision | Why | VIDA Mapping |
|---|---|---|---|
| Checkpointed subscriptions with ordered position progression | adopt now | Strong fit for deterministic operator projections and resumable listeners | projection checkpoint cursor, listener checkpoint hint |
| Persistent subscriptions separate from projection handlers | adopt now | Preserves the boundary between event delivery, projection rebuild, and canonical state | listener kernel separate from projection surfaces |
| One subscription serving multiple handlers that move together | adopt now | Useful for keeping grouped operator projections consistent | grouped projections under one listener/subscription boundary |
| All-or-nothing handler group progression | adopt as future direction | Valuable for consistency across related projections, but needs explicit failure law | grouped projection consistency and resubscribe semantics |
| Gap handling / partition sequencing before advancing checkpoint | adopt as future direction | Useful for stronger projection correctness, but current kernel is single-process and bounded | future projection rebuild guarantees |
| Idempotent handlers required when checkpoint commit lags processing | adopt as future direction | Necessary once checkpoint writes can lag successful handler execution | future projection retry and duplicate-delivery law |
| Replay/rebuild as a first-class projection concern | adopt as future direction | Useful for rebuilding derived views without rewriting canonical state | checkpoint replay lineage and projection rebuild |
| Transactional event-store coupling as required kernel law | reject as-is | VIDA must stay storage-agnostic at product-law level | framework adapter or future storage module only |
| Gateway/command handler separation from projections | adopt now | Matches VIDA distinction between route mutation and derived views | route/assignment commands vs projections/readiness surfaces |

## 6. Current Kernel Consequences

These borrow decisions already affect the current implementation:

1. `assignment_engine.nim` uses dynamic overlay-derived runtime inventory instead of product-law hardcoding
2. `instruction_engine.nim` provides an explicit instruction composition surface
3. `transition_engine.nim` supports event semantics beyond raw `command`
4. route and assignment law stay config-owned, while concrete runtime inventory stays overlay/runtime-owned
5. projection/listener/checkpoint behavior is justified only as a derived runtime surface, never as a replacement for state/receipt/proof ownership
6. checkpoint semantics are now explicitly split between `checkpoint hint`, future `checkpoint commit`, and future `checkpoint replay/fork` concerns

## 7. Deferred Work

The next external-pattern borrow candidates are:

1. automatic/eventless transitions
2. deeper listener/projection hooks with explicit subscription channels
3. richer interrupt/resume semantics for human approval and manual intervention
4. structured outcome schemas for `writer`, `coach`, `verifier`, and `approver`
5. history semantics for resumable execution plan and route progression
6. execution-time checkpoint snapshots attached to bounded proof categories
7. explicit merge strategies if and only if lawful parallel branches are introduced
8. stronger ordered listener checkpoint progression for multi-subscriber runtimes
9. correlation-based resume targeting and trigger indexing for gateway waits
10. future config lint for strict machine-definition validation
11. one-time gateway handle consumption and duplicate resume protection
12. checkpoint commit and replay lineage
13. explicit verification merge law for parallel verification
14. subgraph namespace and state-sharing law for specialist compositions
15. replay-safe side-effect law for durable execution

## 8. Non-Negotiable VIDA Constraints

Borrowed patterns must not violate these invariants:

1. root `vida/config` remains product law
2. state, receipt, proof, and projection stay distinct
3. agents never mutate canonical state directly
4. `coach` remains distinct from `verification`
5. `approval` remains distinct from technical validation
6. route/assignment decisions remain inspectable and receipt-backed
7. vendor runtime checkpoints, threads, bookmarks, tokens, or middleware stacks must never become implicit root law

-----
artifact_path: product/spec/external-pattern-borrow-map
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/external-pattern-borrow-map.md
created_at: '2026-03-09T12:00:46+02:00'
updated_at: '2026-03-12T20:45:00+02:00'
changelog_ref: external-pattern-borrow-map.changelog.jsonl
