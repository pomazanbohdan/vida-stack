# Agentic Cheap Worker Packet System

Purpose: define the minimum packet and prompt system that lets cheap subagents execute bounded `1.0` implementation work with near-zero chat context and no loss of deterministic behavior.

Status: research-to-execution bridge artifact for packetized cheap-agent implementation.

Date: 2026-03-08

---

## 1. Core Rule

Cheap workers must not infer architecture.

They must receive:

1. one bounded objective,
2. one exact write scope,
3. one exact proof target,
4. one explicit output contract,
5. one explicit fallback/escalation rule.

Rule:

1. no packet may depend on chat memory,
2. no packet may ask the worker to “figure out the architecture”,
3. undefined behavior is forbidden by default.

---

## 2. Canonical Packet Stack

The full execution stack is:

1. `Program Plan`
2. `Epic Spec`
3. `Family Spec`
4. `Child Task Packet`
5. `Worker Prompt`

Cheap workers should usually receive only:

1. the child task packet,
2. the worker prompt,
3. a minimal reference bundle,
4. exact target files/tests.

---

## 3. Canonical Child Task Packet

### 3.0 Required Packet Meta

1. `packet_id`
2. `version`
3. `task_id`
4. `parent_task_id`
5. `epic_id`
6. `spec_family`
7. `work_class`
8. `slice_type`
9. `created_at`
10. `producer`

### 3.1 Required Identity

1. `packet_id`
2. `epic_id`
3. `task_id`
4. `spec_family`
5. `work_type`

### 3.2 Required Goal Contract

1. `objective`
2. `scope_in`
3. `scope_out`
4. `definition_of_done`
5. `non_goals`

### 3.3 Required Context Contract

1. `why_this_exists`
2. `current_behavior`
3. `target_behavior`
4. `reference_bundle`
5. `terminology_normalization`
6. `embedded_context`

Rule:

1. `embedded_context` should carry only the compact facts the worker cannot cheaply reconstruct,
2. it must have an explicit size budget and must not turn into transcript replay.

### 3.4 Required Write Contract

1. `allowed_paths`
2. `forbidden_paths`
3. `expected_new_files`
4. `expected_modified_files`
5. `shared_contracts_read_only`

### 3.5 Required Proof Contract

1. `tests_to_add_or_update`
2. `commands_to_run`
3. `expected_pass_conditions`
4. `expected_receipts_or_outputs`
5. `local_proof_obligations`

### 3.6 Required Deterministic Constraints

1. `allowed_actions`
2. `forbidden_actions`
3. `decision_rules`
4. `fallback_ladder`
5. `escalation_rules`

### 3.6.1 Required Routing Context

1. `route_class`
2. `route_receipt_ref`
3. `worker_mode`
4. `track_id`
5. `owner`
6. `read_or_write`

### 3.7 Required Delivery Contract

1. `output_format`
2. `summary_requirements`
3. `file_reference_requirements`
4. `verification_report_requirements`

---

## 4. Canonical Worker Prompt Structure

Use one prompt with these sections:

1. `Role`
2. `Objective`
3. `Inputs`
4. `Write Scope`
5. `Required Steps`
6. `Verification`
7. `Constraints`
8. `Error Handling`
9. `Final Output Contract`

Compact structure:

```md
## Role
<role>Bounded implementation worker for one exact slice.</role>

## Objective
<objective>Implement the packet exactly; no scope expansion.</objective>

## Inputs
<inputs>
- child task packet
- listed reference docs only
</inputs>

## Write Scope
<write_scope>
- allowed paths
- forbidden paths
</write_scope>

## Required Steps
<required_steps>
1. read packet
2. read only referenced files
3. implement exact slice
4. run required verification
5. report only what the packet asks for
</required_steps>

## Verification
<verification>
- run exact commands
- report pass/fail
</verification>

## Constraints
<constraints>
- no chat-memory dependency
- no architectural invention
- no scope widening
- no undeclared fallback
</constraints>

## Error Handling
<error_handling>
- if packet missing required input -> stop and report blocker
- if scope conflict appears -> stop and escalate
- if tests fail outside scope -> report exact blocker, do not widen scope silently
</error_handling>

## Final Output Contract
<final_output_contract>
1. outcome
2. changed files
3. verification results
4. blockers or residual risks
</final_output_contract>
```

---

## 5. Required Gates

Before dispatching a cheap worker, the orchestrator must verify:

1. spec family exists,
2. child task packet exists,
3. allowed write scope is bounded,
4. expected proof is explicit,
5. the task is cheap-agent-ready,
6. no unresolved architecture choice is hidden inside the task.

If any item fails:

1. do not dispatch the cheap worker,
2. route back to spec work or senior integration.

### 5.1 Packet Validation Gate

The packet is invalid if any of these are missing:

1. bounded objective
2. scope in/out
3. allowed/forbidden paths
4. proof contract
5. fallback ladder
6. escalation rules
7. output contract
8. route context
9. anti-drift context

### 5.2 Anti-Drift Gate

The packet is incomplete for guidance-sensitive work if any of these are missing:

1. `assumptions`
2. `invalidation_triggers`
3. `freshness_basis`
4. `source_delta_status`
5. `no_chat_memory=true`

---

## 6. Cheap-Agent-Ready Checklist

A task is cheap-agent-ready only if all are true:

1. one bounded objective
2. one bounded write scope
3. exact file set is known or tightly discoverable
4. exact tests or proofs are known
5. no open architecture decision remains
6. no overlapping writer ownership exists
7. fallback and escalation are explicit

If a task is missing any of those:

1. it stays with the orchestrator or senior integrator,
2. or it is converted into an upstream spec task.

---

## 7. Anti-Drift Rules

Cheap workers must not:

1. infer extra scope from nearby code,
2. rewrite contracts not named in the packet,
3. widen tests far beyond the objective,
4. substitute their own architecture,
5. silently ignore missing inputs,
6. use provider-specific prompt freedom as authority.

Orchestrator must not:

1. send vague packets,
2. hide unresolved decisions inside implementation packets,
3. treat a good-looking answer as proof,
4. merge overlapping cheap-worker patches without a senior integrator.

---

## 8. Minimal Reference Bundle For Cheap Workers

Default minimum:

1. child task packet
2. relevant family spec
3. relevant exact source files
4. relevant exact tests
5. one terminology/glossary reference if terms are non-trivial

Optional:

1. one higher-level epic spec
2. one parity fixture or golden receipt

Do not send:

1. the entire research bundle by default
2. broad protocol sets the worker does not need
3. long chat transcripts

---

## 9. Missing Pieces Still To Formalize

Still missing or only partially formalized:

1. dedicated role-profile source registry
2. source delta log
3. role-profile eval plan
4. formal cheap-worker packet schema in template form
5. formal cheap-worker readiness gate in protocol/test form
6. packet-to-prompt rendering template
7. minimal machine-readable worker output schema

---

## 10. Promotion Path

This artifact should promote next into:

1. packet template
2. worker prompt template set
3. cheap-worker readiness gate
4. conformance tests for packet completeness

---

## 11. Final Rule

Cheap workers become reliable when:

1. the packet is stronger than the prompt,
2. the proof contract is stronger than the prose,
3. the scope is narrower than the temptation to improvise.
