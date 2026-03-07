# Framework Memory Protocol (FMP)

Purpose: define one canonical, framework-owned memory ledger for lessons, corrections, and anomalies discovered during VIDA operation.

## Core Contract

1. Framework memory is durable state, not chat memory.
2. Framework memory is distinct from:
   - routing scorecards,
   - TODO execution logs,
   - one-off session reflections.
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
3. Later operator surfaces may summarize memory counts and recent entries, but the ledger itself remains the canonical store.

## Commands

```bash
python3 _vida/scripts/framework-memory.py record lesson --summary "<summary>" [--source-task <task_id>] [--details-json '{"pattern":"..."}']
python3 _vida/scripts/framework-memory.py record correction --summary "<summary>" [--source-task <task_id>] [--details-json '{"before":"...","after":"..."}']
python3 _vida/scripts/framework-memory.py record anomaly --summary "<summary>" [--source-task <task_id>] [--details-json '{"source":"..."}']
python3 _vida/scripts/framework-memory.py status
```

## Fail-Closed Rule

1. Do not treat scorecards or chat recap as a substitute for framework memory.
2. Do not hide durable framework learnings only inside TODO evidence when they are intended to shape future framework operation.
