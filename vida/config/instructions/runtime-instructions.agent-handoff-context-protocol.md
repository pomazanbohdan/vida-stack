# Agent Handoff And Context Protocol

Purpose: define the canonical law for orchestrator-to-worker and worker-to-worker handoff so delegation remains packet-driven, bounded, replay-safe, and independent of hidden transcript inheritance.

## Core Contract

1. Handoffs are explicit runtime artifacts, not informal prompting habits.
2. The orchestrator owns handoff construction and downstream synthesis.
3. Receiving lanes must get only the context required for their role.
4. Undefined context inheritance is forbidden by default.

## Canonical Handoff Shape

A lawful handoff must define at minimum:

1. sender lane
2. receiver lane
3. blocking question
4. scope in
5. scope out
6. allowed paths or bounded ownership unit when applicable
7. evidence references
8. explicit verification command or proof target
9. output contract
10. fallback or escalation rule

When rendered as a worker packet, the packet must also obey:

1. `vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md`
2. `vida/config/instructions/agent-definitions.worker-entry.md`

## Context Shaping Rule

Context must be filtered before handoff.

Allowed context classes:

1. exact file references
2. exact task/runtime artifact references
3. compact embedded facts that the receiver cannot cheaply reconstruct
4. route or receipt references required for the assignment
5. bounded proof obligations

Forbidden default context:

1. unfiltered transcript inheritance
2. broad repository summaries without scope justification
3. hidden operator memory
4. unrelated historical context "just in case"

## Embedded Context Rule

Embedded context is allowed only when:

1. it is compact,
2. it is role-relevant,
3. it cannot be cheaply reconstructed from canonical local artifacts,
4. it does not silently widen worker scope.

If embedded context and canonical artifacts disagree:

1. prefer the higher-evidence canonical artifact,
2. treat the packet as drift to correct.

## Recovery And Replay Rule

Handoffs must remain usable across compact, restart, and retry.

Rules:

1. a handoff must be reconstructable from canonical packet/runtime artifacts rather than chat memory alone,
2. replaying or retrying a handoff must not silently expand scope,
3. repeated delivery of the same bounded handoff must preserve the same blocking question and ownership boundary unless an explicit updated packet supersedes it.

## Verification Boundary Rule

Each handoff must make verification boundaries explicit.

It must identify:

1. whether the receiver is an author, coach, verifier, or another bounded lane,
2. which proof or verification command closes the slice,
3. what remains outside the receiver's ownership.

## External Alignment Note

This protocol aligns with the `VIDA 1.0` external supervisor/handoff target captured in:

1. `docs/framework/research/vida-1.0-agent-runtime-external-alignment.md`

## References

1. `vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md`
2. `vida/config/instructions/runtime-instructions.context-governance-protocol.md`
3. `docs/framework/research/agentic-cheap-worker-packet-system.md`
4. `docs/framework/research/vida-1.0-agent-runtime-external-alignment.md`

-----
artifact_path: config/runtime-instructions/agent-handoff-context.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions.agent-handoff-context-protocol.md
created_at: '2026-03-10T15:05:00+02:00'
updated_at: '2026-03-10T14:41:13+02:00'
changelog_ref: runtime-instructions.agent-handoff-context-protocol.changelog.jsonl
