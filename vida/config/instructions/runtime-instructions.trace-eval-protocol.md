# Trace Eval Protocol (TEP)

Purpose: add a first-class local trace-grading surface for routed VIDA execution without requiring an external eval backend.

## Core Contract

1. `eval-pack` remains the compact task-close execution summary.
2. `run-graph` remains the canonical node-level resumability ledger.
3. `trace-eval` is the canonical local grading layer that scores the routed execution trace across route law, fallback law, budget law, and approval law.

## Canonical Artifacts

1. `.vida/logs/trace-evals/trace-eval-<task_id>.json`
2. `.vida/logs/trace-datasets/trace-dataset-<task_id>.json`

## Minimum Grader Surface

Each trace-eval artifact must grade at least:

1. `route_correctness`
2. `fallback_correctness`
3. `budget_correctness`
4. `approval_correctness`

Allowed grade values:

1. `pass`
2. `partial`
3. `fail`

## Local Evidence Sources

`trace-eval` should bind only to canonical local artifacts:

1. `.vida/logs/route-receipts/<task_id>.*.json`
2. `.vida/state/run-graphs/<task_id>.json`
3. `.vida/logs/eval-pack-<task_id>.json`

## Commands

```bash
python3 docs/framework/history/_vida-source/scripts/trace-eval.py grade <task_id>
python3 docs/framework/history/_vida-source/scripts/trace-eval.py dataset <task_id>
```

## Dataset Contract

The dataset export should stay compact and replay-friendly:

1. task id
2. dataset version
3. grader labels
4. artifact paths
5. compact trace summary

Rule:

1. local dataset export is for regression reuse and grader comparison,
2. it must not duplicate full raw receipts when artifact references are sufficient.

-----
artifact_path: config/runtime-instructions/trace-eval.protocol
artifact_type: runtime_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/runtime-instructions.trace-eval-protocol.md
created_at: 2026-03-08T02:15:22+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: runtime-instructions.trace-eval-protocol.changelog.jsonl
