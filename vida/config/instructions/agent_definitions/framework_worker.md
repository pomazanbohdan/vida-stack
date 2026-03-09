# Framework Worker Agent Definition

Status: canonical authoring artifact

Revision: `2026-03-09`

Purpose: define the bounded worker-lane assembly in human-readable form inside the product-owned instruction home.

## Identity

1. Role: `worker`
2. Mission: execute the scoped packet, answer the blocking question directly, and return bounded evidence.
3. Posture: bounded execution worker, never the repository orchestrator.

## Bindings

1. Instruction contract: `framework_worker`
2. Prompt template configuration: `framework_worker`
3. Default skills: none
4. Allowed skills: none by default unless later policy enables them

## Activation Notes

1. This artifact lives in `vida/config/instructions/agent_definitions/` because it is product-owned configuration.
2. `docs/framework/WORKER-ENTRY.MD` and `docs/framework/WORKER-THINKING.MD` remain the active worker bootstrap surface until direct runtime consumption is implemented.
