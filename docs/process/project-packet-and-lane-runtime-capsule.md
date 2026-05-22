# Project Packet And Lane Runtime Capsule

Status: active project process doc

Purpose: provide a compact runtime-facing projection of project packet and delegated-lane law for routine orchestrator startup and continuation.

## Use

Use this capsule when the project orchestrator needs the high-frequency packet/lane rules without rereading the full owner protocol.

Owner law remains in:

1. `docs/process/team-development-and-orchestration-protocol.md`
2. `docs/process/project-packet-rendering-runtime-capsule.md`

Consult those owner documents when an edge case, conflict, or packet-shaping question is not settled by this capsule.

## Runtime Summary

Project development stays:

1. orchestrator-led,
2. delegation-first for normal write-producing work,
3. `delivery_task` as the default leaf,
4. `execution_block` only when one-owner bounded closure still fails,
5. coach-separated and verifier-backed before closure,
6. session-scoped when multiple orchestrators share one project root,
7. source-neutral across PRs, defects, external downstream reports, CI/release work, optimization, documentation/process work, diagnostics, and operator-surface gaps.

Session-scoped shorthand:

1. the current session must know its `orchestrator_session_id` before shaping write-producing work,
2. every active packet must map to a claim kind, conflict domain, and path scope,
3. foreign blocked lanes are reported separately from current-session blockers,
4. only same task/run, path intersection, exclusive conflict domain, or global state-integrity blockers stop the current session.

## Packet Minimum

Before dispatch, the active packet must satisfy the template-specific minimum from `docs/process/project-development-packet-template-protocol.md`.

Runtime shorthand:

1. `delivery_task_packet` and `execution_block_packet` must name `goal`, `scope_in`, `owned_paths` or `read_only_paths`, `definition_of_done`, `verification_command`, `proof_target`, `stop_rules`, and one `blocking_question`,
2. `coach_review_packet` must name `review_goal`, `owned_paths` or `read_only_paths`, `definition_of_done`, `proof_target`, and one `blocking_question`,
3. `verifier_proof_packet` must name `proof_goal`, `verification_command`, `proof_target`, `owned_paths` or `read_only_paths`, and one `blocking_question`,
4. `escalation_packet` must name `decision_needed`, `options`, `constraints`, and one `blocking_question`.

If the active packet template is missing any mandatory field, dispatch must fail closed and the packet must be reshaped first.

## Default Lane Sequence

For normal write-producing work:

1. orchestrator shapes
2. analyst validates source intent, current code/spec behavior, owned paths, acceptance, and duplication risks
3. test_author/autotester writes or specifies regression proof when test-first proof is required
4. coach_test_gate validates test quality when a new failing test gates the packet
5. developer/implementer writes through the cheapest eligible configured write carrier
6. coach_implementation_gate reviews implementation conformance
7. duplication_reviewer checks reuse and duplicate active paths
8. tester/prover verifies the proof target
9. release_closure runs when the packet or task pool has release impact
10. orchestrator synthesizes and updates TaskFlow

Read-only findings feed the next packet; they do not transfer root-session write ownership.
The canonical delegated execution surface is the runtime lane flow through `vida agent-init`; host subagent APIs may exist under the selected carrier system, but they do not replace the project runtime contract.
Host-local shell or patch capability is not a receipt and does not transfer write ownership back to the root session.
An activation/view-only internal-host handoff without execution evidence is a blocker/reroute condition, not an executing delegated lane.
If that blocker still leaves a bounded read-only diagnostic path, continue diagnosis to a code-level blocker or next bounded fix before asking the user to choose a route.
That bounded fix does not itself unlock local mutation; root-session write remains forbidden until an explicit exception-path receipt or receipt-backed delegated execution evidence is present for the active packet.

Source-neutral intake shorthand:

1. Before any bounded item enters implementation, classify the source type and consult the mapped spec/process surface for that source class.
2. Record the active TaskFlow item, source evidence, priority reason, owned/read-only paths, role chain, proof target, and sequential/parallel posture.
3. If VIDA cannot execute the configured role chain, record the runtime defect separately and use bounded Defective Runtime Emulation Mode without dropping the same evidence chain.

## Local-Work Boundary

Keep work local only for:

1. shaping only,
2. bounded read-only analysis,
3. proof-only verification,
4. explicit exception-path handling.

Local write work still requires an explicit exception-path receipt and remains blocked while the same packet has an open delegated lane or unresolved handoff.

## Continuation Summary

1. partial implementer return means reroute, not implicit root-session completion,
2. review-found compile blocker in a mutated packet still stays under reroute/exception law,
3. if delegated state is still open, packet closure and root takeover are both blocked,
4. worker timeout or empty poll window does not authorize generic single-agent fallback or root-session self-development,
5. commentary, status output, and intermediate reports are visibility only; they never create a lawful pause boundary.
6. when one packet closes or a runtime handoff returns bounded evidence, immediately rebuild the parent bounded unit and continue to the next lawful packet in the same cycle unless a real blocker or escalation receipt exists.
7. if closure-style wording/reporting is emitted by mistake under active continuation intent, the recovery action is to return to commentary mode and bind the already-known next lawful packet immediately.
8. when recording task progress from shell during orchestration, prefer file-backed text arguments such as `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.
9. packet closure does not by itself authorize binding a different sibling bounded unit; if the next lawful unit is not explicitly evidenced, continuation must fail closed to ambiguity instead of widening by inertia.

## Routing

1. for full delegated-lane law and packet closure semantics, read `docs/process/team-development-and-orchestration-protocol.md`,
2. for routine packet rendering and prompt-layer precedence, read `docs/process/project-packet-rendering-runtime-capsule.md`,
3. for packet-family field ownership, read `docs/process/project-development-packet-template-protocol.md`.

-----
artifact_path: process/project-packet-and-lane-runtime-capsule
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-packet-and-lane-runtime-capsule.md
created_at: '2026-03-13T18:05:15+02:00'
updated_at: 2026-05-22T02:44:40Z
changelog_ref: project-packet-and-lane-runtime-capsule.changelog.jsonl
