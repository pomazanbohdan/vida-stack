# Framework Memory Protocol (FMP)

Purpose: define one canonical, framework-owned memory ledger for lessons, corrections, and anomalies discovered during VIDA operation.

## Core Contract

1. Framework memory is durable state, not chat memory.
2. Framework memory is distinct from:
   - routing scorecards,
   - TaskFlow execution logs,
   - one-off session reflections,
   - project memory,
   - instruction memory.
3. The minimum memory kinds are:
   - `lesson`
   - `correction`
   - `anomaly`

## Canonical Artifact

1. `.vida/state/framework-memory.json`

Minimum shape:

1. `entries`
2. `summary.lesson_count`
3. `summary.correction_count`
4. `summary.anomaly_count`

## Capture Rules

1. `lesson`
   - reusable positive pattern or confirmed fix strategy
2. `correction`
   - explicit human/operator correction that changed framework behavior or decision policy
3. `anomaly`
   - framework friction, failure mode, or recurrent instability signal

## Integration Rules

1. Silent framework diagnosis may record anomalies automatically when framework bugs are captured.
2. Session reflection may record anomalies automatically for newly detected framework gaps.
3. Later operator surfaces may summarize memory counts, repeated anomaly clusters, task-level anomaly concentration, and recent entries, but the ledger itself remains the canonical store.
4. Framework memory is not the canonical durable store for `Agent Definition`, `Instruction Contract`, `Prompt Template Configuration`, or their sidecars.

## Commands

```bash
python3 framework-memory.py record lesson --summary "<summary>" [--source-task <task_id>] [--details-json '{"pattern":"..."}']
python3 framework-memory.py record correction --summary "<summary>" [--source-task <task_id>] [--details-json '{"before":"...","after":"..."}']
python3 framework-memory.py record anomaly --summary "<summary>" [--source-task <task_id>] [--details-json '{"source":"..."}']
python3 framework-memory.py status
```

## Fail-Closed Rule

1. Do not treat scorecards or chat recap as a substitute for framework memory.
2. Do not hide durable framework learnings only inside TaskFlow evidence when they are intended to shape future framework operation.

-----
artifact_path: config/runtime-instructions/framework-memory.protocol
artifact_type: runtime_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/runtime-instructions.framework-memory-protocol.md
created_at: 2026-03-07T22:08:06+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: runtime-instructions.framework-memory-protocol.changelog.jsonl
