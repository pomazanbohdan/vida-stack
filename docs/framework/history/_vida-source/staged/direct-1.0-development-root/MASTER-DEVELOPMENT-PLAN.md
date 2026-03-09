# VIDA 1.0 Master Development Plan

Purpose: provide one compact-safe master plan that the next agent can use to slice tasks and start lawful implementation in the final development environment without replaying prior chat context.

Status: canonical staged development plan for external execution.

---

## 1. Program Goal

Deliver a working local `VIDA 1.0` binary with the frozen operator surface:

1. `vida boot`
2. `vida task ...`
3. `vida memory ...`
4. `vida status`
5. `vida doctor`

The release target is a direct `0.1 -> 1.0` binary transition.

The goal is not more speculative planning.
The goal is a working binary reached through frozen specs, bounded implementation waves, proofs, and controlled switchover.

---

## 2. Canonical Inputs

This plan depends on these canonical artifacts:

1. `_vida/docs/plans/2026-03-08-vida-0.2-semantic-freeze-spec.md`
2. `_vida/docs/plans/2026-03-08-vida-0.2-bridge-policy.md`
3. `_vida/docs/plans/2026-03-08-vida-0.3-command-tree-spec.md`
4. `_vida/docs/plans/2026-03-08-vida-0.3-state-kernel-schema-spec.md`
5. `_vida/docs/plans/2026-03-08-vida-0.3-instruction-kernel-spec.md`
6. `_vida/docs/plans/2026-03-08-vida-0.3-migration-kernel-spec.md`
7. `_vida/docs/plans/2026-03-08-vida-0.3-route-and-receipt-spec.md`
8. `_vida/docs/plans/2026-03-08-vida-0.3-parity-and-conformance-spec.md`
9. `_vida/docs/plans/2026-03-08-vida-direct-1.0-compact-continuation-plan.md`
10. `_vida/docs/research/2026-03-08-vida-direct-1.0-next-agent-compact-instruction.md`
11. `_vida/docs/plans/2026-03-08-vida-autonomous-role-profiles-and-adaptive-orchestration-plan.md`

If this plan conflicts with those files, those files win.

---

## 3. Done State

Already complete:

1. direct `0.1 -> 1.0` architectural decision
2. semantic extraction layer map
3. direct local-first `1.0` program
4. cheap-worker packet system
5. cheap-worker prompt pack
6. semantic freeze spec
7. bridge policy
8. command tree spec
9. state kernel schema spec
10. instruction kernel spec
11. migration kernel spec
12. route-and-receipt spec
13. parity-and-conformance spec

These are frozen enough to start implementation.

---

## 4. Current Lawful Start Point

Current lawful first implementation wave:

1. `Binary Foundation`

Bounded meaning of `Binary Foundation`:

1. Rust workspace
2. crate boundaries
3. minimal bootable `vida` binary
4. thin command shell
5. fast local compile/test loop
6. temp-state harness

Nothing heavier is authorized until this wave is started and proven.

---

## 5. Release Pipeline

Implementation must progress in this order:

1. `Wave 1 - Binary Foundation`
2. `Wave 2 - State And Migration Backbone`
3. `Wave 3 - Instruction And Operator Surface Integration`
4. `Wave 4 - Task Execution On Selected Flows`
5. `Wave 5 - Verification, Visibility, And Orchestration Hardening`
6. `Wave 6 - Primary Switchover`

No wave may silently absorb the next one.

---

## 6. Wave Contracts

### Wave 1 - Binary Foundation

Owns:

1. workspace layout
2. minimal `vida` binary scaffold
3. root command shell only
4. temp-state harness seam
5. cheap local proof loop

Non-goals:

1. full kernels
2. bridge replacement
3. route enforcement
4. instruction runtime
5. adaptive orchestration

Initial staged asset:

1. `binary-foundation/`

### Wave 2 - State And Migration Backbone

Owns:

1. binary-owned state entities
2. authoritative mutation path
3. startup compatibility and migration checks
4. migration preflight and runner
5. bridge import/export path
6. migration tests

Safe first slice:

1. minimal authoritative state spine
2. fail-closed boot compatibility classification
3. blocked boot outcomes

Non-goals:

1. instruction loading
2. route/receipt enforcement
3. full `vida task ...`
4. adaptive-agent control-plane work

### Wave 3 - Instruction And Operator Surface Integration

Owns:

1. instruction loading and composition
2. precedence activation runtime
3. minimal operator-visible surfaces for `boot|status|doctor|memory`
4. fail-closed operator reporting around instruction/migration state

Non-goals:

1. selected-flow task execution
2. route proof closure
3. rollout/proving hardening

### Wave 4 - Task Execution On Selected Flows

Owns:

1. first real `vida task ...` execution flows
2. route/receipt enforcement on selected flows
3. resumable run-path wiring
4. selected-flow bridge replacement steps

Non-goals:

1. full rollout
2. full orchestration hardening

### Wave 5 - Verification, Visibility, And Orchestration Hardening

Owns:

1. richer proofs and visibility
2. parity and conformance proving
3. trace/eval/operator visibility surfaces
4. adaptive orchestration payload integration where lawfully enabled

Non-goals:

1. final rollout by default

### Wave 6 - Primary Switchover

Owns:

1. measured cutover
2. rollback-safe switchover
3. final promotion proofs

---

## 7. Agent-System Payload Integration

The autonomous role/orchestration work is part of `VIDA 1.0`, not a separate product.

It should be cut into these iterations:

1. Iteration 1: `Tasks 1-4`
   - sources
   - profiles
   - taxonomy
   - role contracts
2. Iteration 2: `Tasks 5-13`
   - routing
   - scoring
   - adaptive count
   - consensus
   - task packets
   - handoffs
   - verification burden
   - OWASP/security spine
3. Iteration 3: `Tasks 14-18`
   - traces
   - evals
   - proving
   - rollout
   - pilot
   - epic/TODO slicing

Execution rule:

1. do not start this payload before the binary host is ready enough,
2. do not treat it as permission to reopen frozen specs,
3. slice it through existing scripts/tests/process in the final environment.

---

## 8. Task-Slicing Contract

Any next agent must cut bounded child tasks from this plan using these rules:

1. each child task belongs to exactly one wave or one agent-system iteration,
2. each child task has one writer/integrator owner,
3. each child task has explicit proof,
4. each child task names fallback and escalation,
5. each child task includes non-goals,
6. no child task relies on chat memory,
7. no child task widens scope silently.

Every child task packet should include:

1. objective
2. exact write scope
3. exact read set
4. blocking question for subagents
5. proof path
6. rollback note
7. next exact action

---

## 9. External-Environment Execution Rule

This staged root exists because the current repository must stay clean outside `_vida/*`.

So the final environment should:

1. import this staged root,
2. use the staged assets as transfer material,
3. run the actual code changes there,
4. return durable receipts and updated docs here.

This repository remains the canonical documentation and continuation state source.

---

## 10. Staged Assets

Current staged assets:

1. `binary-foundation/`

`binary-foundation/` contains:

1. root `Cargo.toml`
2. `crates/vida/Cargo.toml`
3. `crates/vida/src/main.rs`
4. `crates/vida/src/temp_state.rs`
5. `crates/vida/tests/boot_smoke.rs`
6. `Makefile.binary-foundation.template`

These are staging artifacts only.

---

## 11. Blocking Rules

The next agent may start implementation only if all are true:

1. the target wave is currently lawful,
2. the needed spec family already exists,
3. the write scope is bounded,
4. proof is explicit,
5. no unresolved architecture choice remains for that slice,
6. the slice does not deepen `0.1` with future-kernel-grade behavior.

If any gate fails:

1. do not implement,
2. do not stall silently,
3. fall back to child-task slicing, packet drafting, and continuation-state updates.

---

## 12. Exact Next Action

The next agent in the final environment should:

1. read `NEXT-AGENT-START-PROMPT.md`,
2. verify whether `Binary Foundation` is still the active lawful wave,
3. cut bounded child tasks for the first Binary Foundation slice,
4. apply the staged `binary-foundation/` scaffold only if no newer blocker state exists,
5. start implementation with proof.

If `Binary Foundation` is already complete in that environment, the next agent should stop, update state, and move to the first bounded slice of `Wave 2 - State And Migration Backbone`.

