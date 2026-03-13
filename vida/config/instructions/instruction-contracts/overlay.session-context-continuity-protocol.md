# Session Context Continuity Protocol

Purpose: define the canonical session-layer protocol that preserves cross-step context, invariants, and bounded continuity during multi-step orchestrator work.

## Scope

This protocol governs continuity between reasoning steps.

It owns:

1. session state capture and refresh,
2. invariant extraction and normalization,
3. bounded cross-step smell detection,
4. scope preservation across turns,
5. post-step reconciliation and state update,
6. the interface between session continuity and step-scoped thinking algorithms.

It does not replace the step reasoning canon.

Step-local reasoning remains owned by:

1. `vida/config/instructions/instruction-contracts/overlay.step-thinking-protocol.md`

## Core Separation Rule

1. Session continuity answers: "what state must remain true across steps?"
2. Step thinking answers: "how should the current step be reasoned through?"
3. Session continuity must not absorb the named step algorithms.
4. Step thinking must not silently take ownership of long-lived session state.

## Activation Class

Always-on in orchestrator lane after lane entry.

Rules:

1. keep this protocol active throughout orchestrator sessions as the continuity layer,
2. use compact-triggered behavior so always-on activation does not imply full expansion on every step,
3. worker lanes do not inherit this protocol unless a higher-precedence packet or protocol explicitly activates it.

## Session State Packet

Maintain one compact state packet for the active session or bounded task.

Required fields:

1. `task_goal`
2. `current_step_goal`
3. `must_do`
4. `must_not`
5. `fixed_facts`
6. `open_unknowns`
7. `allowed_scope`
8. `protected_scope`

Optional fields:

1. `validated_receipts`
2. `rejected_paths`
3. `regression_watch`
4. `next_step_hints`

Rules:

1. store only durable, decision-relevant continuity state,
2. deduplicate semantically equivalent constraints,
3. prefer explicit user or protocol constraints over inferred preferences,
4. when constraints conflict, apply the active precedence law and record the resolution basis.

Deterministic precedence order:

1. explicit current-user constraint
2. canonical protocol constraint
3. validated live evidence tied to the active decision
4. confirmed prior decision or fix receipt
5. local inference or convenience preference

Clarification rule:

1. validated live evidence may refine factual assumptions and feasibility judgments,
2. it must not silently weaken an explicit current-user constraint unless that constraint conflicts with safety, canonical protocol law, or validated impossibility,
3. any such override must be recorded explicitly with its resolution basis.

Compact-triggered rule:

1. keep the packet compact by default,
2. emit only state deltas when invariants remain unchanged,
3. expand into full packet restatement only on activation, conflict, major state change, or closure proof.

## Session Pipeline

### Phase 0. Capture

1. capture the current request and active bounded task,
2. identify the intended deliverable for the current step,
3. classify whether the new turn is `same_task_continuation`, `branch_of_active_task`, or `separate_task`,
4. hydrate the latest lawful session packet only when that classification permits reuse.

Thread-divergence rule:

1. `same_task_continuation` may reuse the active session packet normally.
2. `branch_of_active_task` may reuse only the minimum packet slice needed for the branch and should prefer a bounded branch packet or subagent context over full-session inheritance.
3. `separate_task` must not inherit the prior task packet by default; create a fresh bounded packet or new task context and carry forward only explicit durable invariants that still apply.

### Phase 1. Invariant Extraction

Extract continuity-critical information from:

1. active user instructions,
2. canonical protocol constraints,
3. confirmed prior decisions or fixes,
4. bounded task receipts,
5. validated live evidence when available.

Normalize into:

1. `must_do`
2. `must_not`
3. `fixed_facts`
4. `allowed_scope`
5. `protected_scope`

Populate when available from bounded evidence:

1. `validated_receipts`
2. `rejected_paths`
3. `regression_watch`
4. `next_step_hints`

### Phase 2. Historical Compliance Gate

Before a new step is reasoned through, check for:

1. `Must-Do Omission` risk,
2. `Must-Not Violation` risk,
3. `Cross-Turn Inconsistency` risk,
4. `Signature/Contract Drift` risk,
5. `Code Rollback` risk,
6. `Repetitive Response / no-progress` risk.

If a material risk is detected:

1. narrow the step,
2. clarify the conflict,
3. revise the plan,
4. or escalate to a stronger step algorithm.

Do not proceed as if the risk were acceptable by default.

Smell-to-gate mapping:

1. `Must-Do Omission` -> preservation/admissibility failure
2. `Must-Not Violation` -> hard block
3. `Signature/Contract Drift` -> contract conflict gate
4. `Code Rollback` -> regression guard failure
5. `Repetitive Response` -> no-progress gate
6. `Cross-Turn Inconsistency` -> fact/conflict reconciliation gate

### Phase 3. Step Packet Handoff

Pass the normalized session packet into the step-thinking protocol as structured context.

Minimum handoff surface:

1. `task_goal`
2. `current_step_goal`
3. `must_do`
4. `must_not`
5. `fixed_facts`
6. `allowed_scope`
7. `protected_scope`
8. `open_unknowns`
9. `rejected_paths` when available
10. `regression_watch` when available
11. `validated_receipts` when available

Handoff compactness rule:

1. send only the fields needed by the selected step algorithm and current task class,
2. omit unchanged optional fields by reference when a prior packet already established them,
3. expand to full handoff only when conflict, escalation, or closure proof requires it.

### Phase 4. Post-Step Reconciliation

After the step algorithm returns:

1. compare proposed changes against `protected_scope`,
2. verify that `must_do` constraints remain satisfied,
3. verify that `must_not` constraints remain unbroken,
4. update `fixed_facts` only when the step produced explicit evidence,
5. record new rejected paths when the step proved them invalid,
6. add regression watches when local success may threaten adjacent behavior.

### Phase 5. Session Update

Write an updated session packet that becomes the continuity source for the next step.

Delta-first update rule:

1. if invariants, scope, and facts are unchanged, write a delta receipt instead of restating the full packet,
2. if any required field changes, restate the affected packet slice explicitly,
3. if precedence resolution changed, record the resolution basis in the update receipt.

## Step Interface Contract

Session continuity sends:

1. session packet,
2. active step goal,
3. blocking smell findings if any,
4. required preservation constraints.

Step thinking returns:

1. decision or candidate result,
2. evidence references,
3. changed assumptions,
4. new confirmed facts,
5. violated constraints if any,
6. residual risks,
7. next-step hint.

## Allowed Smell Prevention Behavior

This protocol may:

1. request clarification when ambiguity blocks lawful continuation,
2. block synthesis when protected constraints would be violated,
3. force explicit acknowledgment of unresolved regression risk,
4. require a narrower modification scope before implementation or synthesis.

This protocol must not:

1. invent new user constraints,
2. rewrite historical facts without evidence,
3. bypass the step-thinking quality gates,
4. silently widen scope because the current step became inconvenient.

## Interaction Smell Mapping

This protocol is the canonical home for cross-step mitigation of:

1. `Ambiguous Instruction`
2. `Incomplete Instruction`
3. `Must-Do Omission`
4. `Must-Not Violate`
5. `Signature Mismatch`
6. `Cross-Turn Inconsistency`
7. `Partial Functionality Breakdown`
8. `Code Rollback`
9. `Repetitive Response`

Rule:

1. session continuity owns the preservation and early-detection layer,
2. step thinking owns the reasoning method used once the step is admissible.

## Compact Conflict Glossary

Use these compact fields consistently across packets and handoffs:

1. `history_conflicts`
   - prior facts, prior accepted decisions, or cross-turn state that the current step would contradict,
2. `preservation_conflicts`
   - active `must_do`, `must_not`, or `MUST_PRESERVE` constraints that the current step would weaken or break,
3. `scope_conflicts`
   - proposed changes that exceed `allowed_scope` or touch `protected_scope` without lawful override.

Rule:

1. omit these fields when empty in compact mode,
2. expand them only when they affect admissibility, escalation, or closure proof.

## Minimal Receipts

Each non-trivial session update should retain a concise receipt containing:

1. current step goal,
2. active invariants,
3. protected scope,
4. new confirmed facts,
5. rejected paths,
6. residual risks,
7. readiness of the next step.

## Integration Rule

When both protocols are active:

1. run session continuity first,
2. then run the selected step-thinking algorithm,
3. then reconcile and refresh the session packet.

Compact form:

1. `Capture -> Normalize -> Gate -> Step Think -> Reconcile -> Update`

## Closure Rule

Session continuity for a bounded task is closure-ready only when:

1. the final step result is admissible,
2. no material invariant remains unresolved,
3. protected scope violations are either absent or explicitly approved,
4. residual risks are carried forward explicitly when continuation is still required,
5. the session packet is either finalized or intentionally cleared at task boundary.

-----
artifact_path: config/instructions/instruction-contracts/overlay.session-context-continuity.protocol
artifact_type: instruction_contract
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/instruction-contracts/overlay.session-context-continuity-protocol.md
created_at: '2026-03-11T13:10:00+02:00'
updated_at: '2026-03-13T07:44:24+02:00'
changelog_ref: overlay.session-context-continuity-protocol.changelog.jsonl
