# VIDA Autonomous Execution Protocol Next-Step After Compact Instruction

Purpose: provide the exact next-step instruction for enabling and applying autonomous follow-through mode after the canonical plan/spec/task surfaces are already settled.

Use when:

1. the active request is already `execution_flow` or tracked `artifact_flow`,
2. a lawful `br` task or pool exists,
3. the user explicitly wants the agent to keep executing through the plan to completion unless blocked.

## Exact Next Step

1. confirm the active task/pool is in tracked TODO/`br` flow,
2. confirm the next lawful work is defined by canonical sources,
3. enable autonomous follow-through mode through `docs/framework/autonomous-execution-protocol.md`,
4. continue through the next ready blocks/tasks until blocker or pool completion,
5. record stop reasons explicitly when autonomy halts.

## Hard Constraints

1. do not use this mode to widen scope,
2. do not bypass verification/review gates,
3. do not continue when next-task authority is ambiguous,
4. do not leave task/block transitions only in chat memory.
