# Project Development Packet Template Protocol

Status: active project process doc

Purpose: define the canonical project-side packet template family so orchestrator, implementer, coach, verifier, and escalation lanes all exchange bounded work in one stable shape instead of inventing packet structure ad hoc.

## Scope

This protocol defines:

1. the minimum packet family for project development work,
2. the required fields for each packet shape,
3. when `delivery_task` is enough and when `execution_block` is required,
4. the minimum handoff structures for coach and verifier lanes.

This protocol does not define:

1. framework worker-lane law,
2. one specific backlog item,
3. one specific role prompt,
4. product capability ownership.

## Core Rule

Every delegated lane must receive one bounded packet in a canonical shape.

Project rule:

1. free-form delegation is forbidden,
2. packet law is stronger than reusable prompt wording,
3. one lane must receive one bounded unit, one proof target, and one stop boundary,
4. if a bounded unit cannot be expressed in one lawful packet, reshape it before dispatch.
5. commentary/progress updates are visibility only and must not be used as a substitute for rendering or dispatching a lawful packet.

## Packet Family

The minimum project packet family is:

1. `session_frame`
2. `delivery_task_packet`
3. `execution_block_packet`
4. `coach_review_packet`
5. `verifier_proof_packet`
6. `escalation_packet`

Rule:

1. `delivery_task_packet` is the default working leaf,
2. `execution_block_packet` is just-in-time refinement only,
3. `coach_review_packet`, `verifier_proof_packet`, and `escalation_packet` narrow the same bounded unit rather than creating a second feature scope.

## Session Frame Template

Use this before first write-producing work in a session:

```text
session_frame:
  request_class: answer_only | artifact_flow | execution_flow | mixed
  active_unit: <backlog_id or bounded ask>
  next_leaf: delivery_task | execution_block
  next_lane_mode: local_shaping | delegated_implementation | verifier_only | escalation
  proof_target: <bounded proof target>
  active_skills: <skill names or no_applicable_skill>
  readiness_status: ready | reshape_required | bootstrap_blocked
```

## Delivery-Task Packet Template

Use this as the default delegated write packet:

```text
delivery_task_packet:
  packet_id: <stable packet id>
  backlog_id: <backlog item id>
  release_slice: <release slice or none>
  owner: <taskflow | docflow | seam | other bounded owner>
  closure_class: law | implementation | proof | refactor | hardening
  goal: <one dominant goal>
  non_goals:
    - <explicit non-goal>
  scope_in:
    - <allowed scope item>
  scope_out:
    - <forbidden scope item>
  owned_paths:
    - <writable path>
  read_only_paths:
    - <optional bounded read path>
  inputs:
    - <required artifact, command output, or state>
  outputs:
    - <expected changed artifact, code, or receipt>
  definition_of_done:
    - <closure criterion>
  verification_command: <one bounded verification command or procedure>
  proof_target: <what must be proven>
  active_skills: <required skills or no_applicable_skill>
  stop_rules:
    - <stop boundary>
  blocking_question: <one blocking question>
  handoff_runtime_role: worker
  handoff_task_class: implementation
  handoff_selection: runtime_selected_tier
```

Readiness rule:

1. `goal`, `owned_paths` or `read_only_paths`, `definition_of_done`, `verification_command`, `proof_target`, `stop_rules`, and `blocking_question` are mandatory,
2. if more than one mutable owner exists, do not dispatch this packet yet.
3. if the active bounded unit is write-producing and this readiness rule is satisfied, the next lawful action is dispatch unless an explicit blocker is recorded.

## Execution-Block Packet Template

Use only when the parent `delivery_task` still fails one-owner bounded closure:

```text
execution_block_packet:
  packet_id: <stable packet id>
  parent_packet_id: <parent delivery task packet>
  backlog_id: <backlog item id>
  owner: <one bounded owner surface>
  closure_class: <one narrowed class>
  goal: <one narrowed goal>
  scope_in:
    - <allowed scope item>
  scope_out:
    - <forbidden scope item>
  owned_paths:
    - <writable path>
  definition_of_done:
    - <narrow closure criterion>
  verification_command: <one bounded verification command or procedure>
  proof_target: <narrow proof target>
  active_skills: <required skills or no_applicable_skill>
  stop_rules:
    - <stop boundary>
  blocking_question: <one blocking question>
  handoff_runtime_role: worker
  handoff_task_class: implementation
  handoff_selection: runtime_selected_tier
```

JIT rule:

1. create an `execution_block_packet` only for the next active item or a near-critical-path item about to dispatch,
2. do not pre-split the full backlog into execution blocks.

## Coach Review Packet Template

Use when the implementer result is ready for bounded formative review:

```text
coach_review_packet:
  packet_id: <stable packet id>
  source_packet_id: <delivery task or execution block packet>
  review_goal: <what the coach must judge>
  owned_paths:
    - <paths under review>
  definition_of_done:
    - <same bounded done rule>
  proof_target: <same bounded proof target>
  active_skills: <required skills or no_applicable_skill>
  review_focus:
    - <quality gate>
  blocking_question: <one review question>
  handoff_runtime_role: coach
  handoff_task_class: coach
  handoff_selection: runtime_selected_tier
```

## Verifier Proof Packet Template

Use when bounded independent proof is required:

```text
verifier_proof_packet:
  packet_id: <stable packet id>
  source_packet_id: <delivery task or execution block packet>
  proof_goal: <what must be verified>
  verification_command: <one bounded verification command or procedure>
  proof_target: <what closure depends on>
  owned_paths:
    - <paths or artifacts to inspect>
  active_skills: <required skills or no_applicable_skill>
  blocking_question: <one proof question>
  handoff_runtime_role: verifier
  handoff_task_class: verification
  handoff_selection: runtime_selected_tier
```

## Escalation Packet Template

Use only when normal closure cannot be made coherent:

```text
escalation_packet:
  packet_id: <stable packet id>
  source_packet_id: <delivery task or execution block packet>
  conflict_type: boundary | architecture | write_scope | unresolved_decision
  decision_needed: <one explicit decision>
  options:
    - <option>
  constraints:
    - <must preserve rule>
  active_skills: <required skills or no_applicable_skill>
  blocking_question: <one escalation question>
  handoff_runtime_role: solution_architect
  handoff_task_class: architecture
  handoff_selection: runtime_selected_tier
```

## Packet Selection Rule

Use this selection order:

1. `session_frame` at session start,
2. `delivery_task_packet` by default,
3. `execution_block_packet` only when the delivery task still crosses mutable contracts or proof classes,
4. `coach_review_packet` after implementer completion,
5. `verifier_proof_packet` before closure,
6. `escalation_packet` only when normal packet closure cannot be made coherent.

## Fail-Closed Rule

If a packet:

1. has more than one mutable owner,
2. mixes implementation and unrelated proof/hardening,
3. has no single blocking question,
4. has no bounded proof target,
5. has no explicit stop rule,

then it is not lawful and must be reshaped before dispatch.

## Routing

1. for top-level orchestration, read `docs/process/project-orchestrator-operating-protocol.md`,
2. for session startup, read `docs/process/project-orchestrator-session-start-protocol.md`,
3. for team/lane semantics, read `docs/process/team-development-and-orchestration-protocol.md`,
4. for skill activation, read `docs/process/project-skill-initialization-and-activation-protocol.md`,
5. for framework worker packet home, read `prompt-templates/worker.packet-templates.md`.

-----
artifact_path: process/project-development-packet-template-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-development-packet-template-protocol.md
created_at: '2026-03-13T21:30:00+02:00'
updated_at: '2026-03-13T21:30:00+02:00'
changelog_ref: project-development-packet-template-protocol.changelog.jsonl
