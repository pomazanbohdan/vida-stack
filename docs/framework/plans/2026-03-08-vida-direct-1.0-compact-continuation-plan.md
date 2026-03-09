# VIDA Direct 1.0 Compact Continuation Plan

Purpose: define the canonical compact-safe continuation mechanism for the direct `1.0` program, keep durable `done / now / next / later` state outside chat memory, and make multi-session progress and external-environment continuation deterministic.

Status: canonical continuation and session-slicing plan for the remaining spec spine and the first post-spec implementation waves.

Date: 2026-03-08

---

## 1. Executive Decision

The direct `1.0` program will continue through artifact-driven session slices, not transcript-driven continuation.

The continuation mechanism is:

1. keep durable program state in canonical docs,
2. split the remaining spec spine into bounded session slices,
3. update the bridge docs at each session boundary,
4. keep the canonical product-law artifact set stable,
5. make external-environment continuation depend on the documentation bundle, not on the original chat/runtime.

Compact rule:

`never depend on transcript replay; always leave behind enough durable state for the next agent or environment to resume deterministically`

---

## 2. Why This Plan Exists

The program already has enough architectural direction and frozen kernel law to continue safely, but the remaining work no longer fits well into one large session.

This plan exists because:

1. compact/context compression can happen at any moment,
2. the remaining route/parity work is still semantically dense,
3. the user explicitly wants smaller iterations with lower read/write cost,
4. later implementation may happen in another environment,
5. continuation must survive that environment boundary without relying on chat memory.

---

## 3. Canonical Continuation Stack

The compact-safe continuation stack is:

1. `docs/framework/history/research/2026-03-08-agentic-master-index.md`
   - stable navigation bridge,
   - current-scope map,
   - compact-ready resume order,
2. this file
   - durable `done / now / next / later` ledger,
   - session slicing map,
   - post-spec wave map,
   - bridge-update rules,
3. `docs/framework/history/research/2026-03-08-vida-direct-1.0-next-agent-compact-instruction.md`
   - program-level continuation contract,
   - inherited behavioral law,
   - current active stage for the direct `1.0` program,
4. one active artifact-level next-step instruction
   - exact current session scope,
   - bounded read set,
   - exact deliverable,
   - exact following slice,
5. `docs/framework/history/research/2026-03-08-agentic-cheap-worker-packet-system.md`
   - bounded subagent or external-worker handoff contract when delegated work is used.

Stack rule:

1. the continuation stack organizes progress and handoff,
2. it does not override the command, state, instruction, migration, or later route/parity product-law specs,
3. `AGENTS.md` and the active protocol stack remain the execution-law source of truth.

---

## 4. Current Program State

### 4.1 Completed canonical artifacts

Already complete:

1. direct `0.1 -> 1.0` architectural decision,
2. semantic extraction layer map,
3. direct local-first `1.0` program,
4. cheap-worker packet system,
5. cheap-worker prompt pack,
6. semantic freeze spec,
7. bridge policy,
8. command tree spec,
9. state kernel schema spec,
10. instruction kernel spec,
11. migration kernel spec,
12. route-and-receipt spec `Part A` route-law boundary,
13. route-and-receipt spec `Part B` receipt/proof boundary,
14. parity-and-conformance spec `Part A` fixture/evidence boundary,
15. parity-and-conformance spec `Part B` conformance/cutover boundary.

### 4.2 Active session slice

Current exact session slice:

1. `Binary Foundation`

Canonical target artifact:

1. `docs/framework/history/research/2026-03-08-vida-binary-foundation-next-step-after-compact-instruction.md`

Binary Foundation owns:

1. Rust workspace,
2. crate boundaries,
3. minimal bootable `vida` binary,
4. thin command shell,
5. fast local compile/test loop,
6. temp-state harness.

Repository-local execution constraint:

1. framework-owned repo mutations must stay inside `docs/framework/history/_vida-source/*`,
2. if Binary Foundation requires files outside `docs/framework/history/_vida-source/*`, stage templates under `docs/framework/history/_vida-source/templates/*`, perform the actual implementation in the external development environment, and return durable receipts/prompts/docs here.
3. the canonical staged transfer root for that external environment is `docs/framework/history/_vida-source/staged/direct-1.0-development-root/`.

### 4.3 Queued spec slices

After the active slice, continue in implementation-wave order from this plan.

Session-slice rule:

1. `Part A` and `Part B` are session scopes,
2. they are not new product-law artifact families,
3. the canonical route spec remains one file,
4. the canonical parity spec remains one file.

### 4.4 Post-spec implementation waves

After the spec spine is complete, implementation should continue in this order:

1. `Binary Foundation`
2. `State And Migration Backbone`
3. `Instruction And Operator Surface Integration`
4. `Task Execution On Selected Flows`
5. `Verification, Visibility, And Orchestration Hardening`
6. `Primary Switchover`

---

## 5. Remaining Spec Spine Slicing Strategy

| Session slice | Canonical file | Owns now | Must not absorb now |
|---|---|---|---|
| `Route/Receipt Part A` | `docs/framework/history/plans/2026-03-08-vida-0.3-route-and-receipt-spec.md` | authorization law, route stages, lane boundaries, fail-closed posture, proof-vs-state boundary | detailed receipt families, operator visibility detail, parity thresholds, implementation topology |
| `Route/Receipt Part B` | `docs/framework/history/plans/2026-03-08-vida-0.3-route-and-receipt-spec.md` | receipt families, run-graph attachment, approval/escalation/verification/closure proof surfaces, operator visibility boundaries | command/state/instruction/migration redefinition, implementation topology, memory-kernel design |
| `Parity/Conformance Part A` | `docs/framework/history/plans/2026-03-08-vida-0.3-parity-and-conformance-spec.md` | fixture scope, evidence basis, delta categories, parity-testable semantic surface | final thresholds, cutover gates, implementation planning |
| `Parity/Conformance Part B` | `docs/framework/history/plans/2026-03-08-vida-0.3-parity-and-conformance-spec.md` | conformance matrix, thresholds, cutover proof gates, semantic reproduction verdict rules | new product scope, kernel redefinition, runtime implementation topology |

Slicing rule:

1. each session should close one semantic boundary,
2. each session should defer downstream detail explicitly rather than partially absorbing it,
3. each session must end with an updated active next-step instruction for the following slice.

---

## 6. External-Environment Continuation Bundle

Because later development may happen in another environment, every continuation boundary must leave behind a complete documentation bundle.

Minimum continuation bundle:

1. `AGENTS.md`
2. `docs/framework/ORCHESTRATOR-ENTRY.MD`
3. `docs/framework/thinking-protocol.md`
4. `vida.config.yaml`
5. `docs/framework/history/research/2026-03-08-agentic-master-index.md`
6. this file
7. `docs/framework/history/research/2026-03-08-vida-direct-1.0-next-agent-compact-instruction.md`
8. the current active artifact-level next-step instruction
9. every already-complete canonical spec needed by the active slice
10. `docs/framework/history/research/2026-03-08-agentic-cheap-worker-packet-system.md` when subagent or external-worker handoff is used.
11. `docs/framework/history/research/2026-03-08-vida-external-task-slicing-and-development-start-prompt.md` when an external environment should cut tasks from the plan stack and start the earliest lawful development slice.

External-environment rule:

1. implementation may move to another environment,
2. but durable state must come back as canonical docs, receipts, packets, or spec updates in this repository,
3. no environment should assume access to the original chat transcript,
4. no environment should treat local shell topology or helper commands as product law unless a canonical spec says so.

---

## 7. Session Packet Requirements

Every future session-level continuation packet should state:

1. exact objective,
2. canonical target file,
3. current slice boundary,
4. minimal required read set,
5. subagent questions and scope split,
6. explicit non-goals,
7. exact end-of-session updates required,
8. exact following slice or next artifact.

When delegated work is needed:

1. use the cheap-worker packet system,
2. keep `embedded_context` limited to the compact facts the worker cannot cheaply reconstruct,
3. never substitute packet context with transcript replay.

---

## 8. End-Of-Session Update Protocol

At the end of every bounded session slice, update these artifacts:

1. this plan
   - active slice,
   - completed slice list,
   - queued slices,
   - post-spec wave status when relevant,
2. the active artifact-level next-step instruction
   - switch it to the following slice or artifact,
3. the program-level compact instruction
   - update only when the program-level next slice or completed-state inventory changed materially.

Update the master index only when at least one is true:

1. canonical next order changed,
2. a missing blocker artifact became present,
3. compact-ready resume order changed,
4. the current scope boundary changed materially.

Do not update for routine progress only:

1. the autonomous role/orchestration implementation plan,
2. the cheap-worker packet-system doc,
3. unrelated protocol docs.

---

## 9. Post-Spec Implementation Waves

### 9.1 Wave 1: Binary Foundation

Owns:

1. Rust workspace,
2. crate boundaries,
3. minimal bootable `vida` binary,
4. thin command shell,
5. fast local compile/test loop,
6. temp-state harness.

### 9.2 Wave 2: State And Migration Backbone

Owns:

1. state entities and mutation path,
2. startup version/migration checks,
3. migration runner/preflight,
4. bridge import/export path,
5. migration tests.

### 9.3 Wave 3: Instruction And Operator Surface Integration

Owns:

1. runtime instruction loading,
2. composition wiring,
3. minimal `vida boot|status|doctor|memory` surfaces,
4. fail-closed boot visibility.

### 9.4 Wave 4: Task Execution On Selected Flows

Owns:

1. first narrow `vida task` self-hosting path,
2. route/receipt enforcement on selected flows,
3. local integration and end-to-end proof for those flows.

### 9.5 Wave 5: Verification, Visibility, And Orchestration Hardening

Owns:

1. richer proof surfaces,
2. trace and operator visibility,
3. bounded routing/scoring/adaptive-agent behavior,
4. verification burden and related control-plane hardening.

### 9.6 Wave 6: Primary Switchover

Owns:

1. making `vida` the primary operating surface,
2. demoting `0.1` to oracle/bridge fallback,
3. preserving only thin compatibility shims where still required.

Deferral rule:

1. do not pull the full adaptive orchestration system into Wave 1 or Wave 2,
2. foundation must be shaped for it, but not forced to finish it early.

---

## 10. Invariants

1. Do not rely on transcript memory as durable program state.
2. Keep one integrator/writer lane per shared write scope.
3. Preserve the canonical product-law artifact set; use session slices instead of inventing extra product files.
4. Every session must leave behind explicit `done / now / next / later` state in durable docs.
5. External-environment continuation must be possible from the documentation bundle alone.
6. Behavioral inheritance laws must continue to propagate into future next-step instructions and worker packets.
7. Undefined behavior remains forbidden; missing route, proof, or compatibility law must stay fail-closed.

---

## 11. Non-Goals

1. This plan does not redefine command, state, instruction, migration, route, or parity product law.
2. This plan does not authorize broad architecture reopening.
3. This plan does not start Rust implementation by itself.
4. This plan does not widen scope into `MCP`, `A2A`, `A2UI`, remote identity, gateways, or remote memory.
5. This plan does not require one session to finish an entire dense artifact when bounded slices are safer.

---

## 12. Open Ambiguities

1. The exact future memory-kernel and doctor-runtime contracts remain later artifacts and must not be improvised during route/parity slicing.
2. The exact serialized parity fixture formats remain open until parity work is completed.
3. A rolling active artifact-level next-step file is the current continuation strategy for the remaining route work; if later continuation proves this too ambiguous, it may be split into explicit per-slice next-step files through a future tracked update.
