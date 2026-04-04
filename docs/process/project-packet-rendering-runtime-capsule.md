# Project Packet Rendering Runtime Capsule

Status: active project process doc

Purpose: provide a compact runtime-facing projection of project packet rendering and prompt-stack interpretation for routine orchestrator startup and dispatch preparation.

## Use

Use this capsule when the project orchestrator needs the minimum packet-rendering and prompt-stack rules without rereading the full owner protocols.

Owner law remains in:

1. `docs/process/project-development-packet-template-protocol.md`
2. `docs/process/project-agent-prompt-stack-protocol.md`

Consult those owner documents when a packet-shaping edge case, role-layer conflict, or template-family question is not settled by this capsule.

## Packet Rendering Minimum

Before dispatch, the active bounded work must be renderable as:

1. one bounded packet,
2. one owner,
3. one proof target,
4. one blocking question,
5. one stop boundary.

For routine write-producing work:

1. use `delivery_task_packet` by default,
2. refine to `execution_block_packet` only when one-owner bounded closure still fails,
3. keep coach/verifier/escalation packets tied to the same bounded unit rather than widening scope.

## Required Packet Fields

The runtime-visible packet minimum is template-specific and must match `docs/process/project-development-packet-template-protocol.md`.

Dispatch-readiness summary:

1. `delivery_task_packet` and `execution_block_packet` require `goal`, `scope_in`, `owned_paths` or `read_only_paths`, `definition_of_done`, `verification_command`, `proof_target`, `stop_rules`, and `blocking_question`,
2. `coach_review_packet` requires `review_goal`, `owned_paths` or `read_only_paths`, `definition_of_done`, `proof_target`, and `blocking_question`,
3. `verifier_proof_packet` requires `proof_goal`, `verification_command`, `proof_target`, `owned_paths` or `read_only_paths`, and `blocking_question`,
4. `escalation_packet` requires `decision_needed`, `options`, `constraints`, and `blocking_question`.

If the active template minimum is missing, the packet is not dispatch-ready and runtime must fail closed instead of emitting a `packet_ready` handoff.

## Prompt-Stack Minimum

Interpret the active lane through this order:

1. framework bootstrap and lane entry,
2. project process posture,
3. role-specific static prompt,
4. dynamic bounded packet,
5. active relevant skill overlay,
6. current runtime/task state.

Lower layers may narrow behavior but must not weaken higher-precedence safety, routing, or ownership law.

## Dispatch Readiness Summary

Before delegation, the session should be able to answer:

1. which role layer is active,
2. which packet layer is active,
3. which skill overlay is active or that `no_applicable_skill` applies,
4. which runtime state confirms the bounded unit,
5. which proof target closes the packet.

If those answers are missing, continue shaping instead of dispatching.

## Routing

1. for the full packet-template family, read `docs/process/project-development-packet-template-protocol.md`,
2. for the full prompt-stack law, read `docs/process/project-agent-prompt-stack-protocol.md`,
3. for session-start routing, read `docs/process/project-orchestrator-session-start-protocol.md`.

-----
artifact_path: process/project-packet-rendering-runtime-capsule
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-packet-rendering-runtime-capsule.md
created_at: '2026-03-13T18:05:15+02:00'
updated_at: '2026-03-13T18:05:15+02:00'
changelog_ref: project-packet-rendering-runtime-capsule.changelog.jsonl
