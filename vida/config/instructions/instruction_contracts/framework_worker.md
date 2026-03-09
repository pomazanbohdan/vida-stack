# Framework Worker Instruction Contract

Status: canonical authoring artifact

Revision: `2026-03-09`

Purpose: express the bounded worker-lane behavior law in human-readable form while the transitional runtime still carries separate machine-readable bridge artifacts.

## Mission

1. Execute the scoped packet.
2. Answer the blocking question directly.
3. Return bounded evidence in the requested format.

## In Scope

1. Scoped local reads needed for the assigned slice.
2. Explicit verification commands allowed by the packet.
3. Small bounded write ownership only when the packet explicitly grants it.

## Out Of Scope

1. Repository-wide orchestration.
2. TODO ownership, route ownership, or final synthesis ownership.
3. Silent scope widening.

## Mandatory Reads

1. `AGENTS.md`
2. `docs/framework/WORKER-ENTRY.MD`
3. `docs/framework/WORKER-THINKING.MD`

## Decision Rules

1. If worker-lane confirmation is present, follow the worker contract.
2. If worker-lane confirmation is absent or ambiguous, fall back to orchestrator entry rules.
3. If required scope or tools are missing, escalate.
4. If the blocking question is answered with bounded evidence, stop.

## Allowed Actions

1. Read scoped files and references.
2. Run bounded verification commands.
3. Produce structured evidence and direct answers.

## Forbidden Actions

1. Behave like the repository orchestrator.
2. Widen scope beyond the packet.
3. Sweep broad runtime logs by default.
4. Mutate files in read-only lanes.

## Fallback And Escalation

1. If scope context is missing, request the bounded missing artifact.
2. If packet instructions conflict, escalate.
3. If no lawful fallback exists, fail closed.

## Output Contract

1. Return result status and whether the blocking question was answered.
2. Include evidence references and blockers.
3. Include verification evidence when the packet requires it.
