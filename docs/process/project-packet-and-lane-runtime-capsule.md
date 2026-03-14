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
5. coach-separated and verifier-backed before closure.

## Packet Minimum

Before dispatch, one packet must name at least:

1. `goal`
2. `scope_in`
3. `owned_paths` or `read_only_paths`
4. `definition_of_done`
5. `verification_command`
6. `proof_target`
7. one `blocking_question`

If any of those are missing, reshape before delegation.

## Default Lane Sequence

For normal write-producing work:

1. orchestrator shapes
2. runtime activates the cheapest capable carrier tier for `runtime_role=worker`
3. runtime activates the cheapest capable carrier tier for `runtime_role=coach`
4. runtime activates the cheapest capable carrier tier for `runtime_role=verifier`
5. orchestrator synthesizes

Read-only findings feed the next packet; they do not transfer root-session write ownership.

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
5. when one packet closes, immediately rebuild the parent bounded unit and continue to the next lawful packet unless a real blocker or escalation receipt exists.

## Routing

1. for full delegated-lane law and packet closure semantics, read `docs/process/team-development-and-orchestration-protocol.md`,
2. for routine packet rendering and prompt-layer precedence, read `docs/process/project-packet-rendering-runtime-capsule.md`.

-----
artifact_path: process/project-packet-and-lane-runtime-capsule
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-packet-and-lane-runtime-capsule.md
created_at: '2026-03-13T18:05:15+02:00'
updated_at: '2026-03-13T18:05:15+02:00'
changelog_ref: project-packet-and-lane-runtime-capsule.changelog.jsonl
