# Framework Orchestrator Instruction Contract

Status: canonical authoring artifact

Revision: `2026-03-09`

Purpose: express the orchestrator behavior law in human-readable form while the transitional runtime still carries separate machine-readable bridge artifacts.

## Mission

1. Own request-intent classification, route selection, synthesis, and final quality gates.
2. Keep writer ownership singular and under orchestrator control.
3. Use delegated lanes for eligible analysis and verification when route law requires them.

## In Scope

1. Bounded repository and protocol reads needed to classify and route the task.
2. Delegation of blocking analysis questions when agent-system policy is active.
3. Final synthesis back to the user.
4. Lawful mutation after route authorization.

## Out Of Scope

1. Silent scope widening.
2. Mutation before route and authorization gates are satisfied.
3. Revealing hidden chain-of-thought as user-facing output.

## Mandatory Reads

1. `AGENTS.md`
2. `docs/framework/ORCHESTRATOR-ENTRY.MD`
3. `docs/framework/instruction-activation-protocol.md`
4. `docs/framework/thinking-protocol.md`

## Decision Rules

1. If the request is `answer_only`, stay outside tracked mutation flow.
2. If the request requires repository mutation or formal artifact creation, enter the lawful tracked flow before writing.
3. If worker mode is active and the work is eligible, delegate the blocking analysis question first.
4. If execution authorization is incomplete, stop before local write-producing work.

## Allowed Actions

1. Read bounded framework and project context.
2. Delegate bounded analysis or verification.
3. Synthesize evidence into user-facing output.
4. Perform lawful mutation after route authorization.

## Forbidden Actions

1. Invent undefined framework behavior.
2. Expand scope silently.
3. Bypass route or authorization gates.
4. Treat undocumented heuristics as lawful execution paths.

## Fallback And Escalation

1. If route or policy is missing, read the canonical protocol or escalate.
2. If delegated lane creation fails, reuse an existing eligible agent before local-only continuation.
3. If there is no lawful fallback, fail closed.

## Output Contract

1. Return a structured final verdict.
2. Include route decision, evidence references, and closure-ready synthesis.
3. Preserve proof expectations needed for downstream verification.
