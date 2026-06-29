# Project ZOMBIE-D Test-Writing Protocol

Purpose: define the project-local protocol that agents must use before writing
or updating tests in `vida-stack`.

## Activation Triggers

Activate this protocol for any task that writes, rewrites, deletes, or plans:

1. Rust unit, contract, smoke, or integration tests.
2. CLI smoke or integration tests.
3. Fixture, snapshot, or golden tests.
4. Coverage-gate tests or coverage proof plans.
5. Runtime defect proof tests.
6. Test task planning, test-batch shaping, or test-matrix design.

## Required Read Path

Before writing or updating tests, read:

1. `AGENTS.md`.
2. `AGENTS.sidecar.md`.
3. This protocol.
4. The relevant TaskFlow task, step, and parent epic.
5. Relevant existing tests, fixtures, snapshots, and golden outputs for the
   target behavior.
6. `docs/process/runtime-command-authority-inventory.md` when public CLI,
   runtime, TaskFlow, DocFlow, lane, run-graph, recovery, receipt, or operator
   surfaces are involved.

## ZOMBIE-D Matrix

Every test-writing task must plan the target behavior through the ZOMBIE-D
matrix before broad verification:

| Code | Category | Planning question |
| --- | --- | --- |
| Z | Zero | What happens with no rows, no task, no fixture, no state, no input, or no match? |
| O | One | What is the smallest valid case proving the intended behavior? |
| M | Many | What changes with multiple rows, tasks, fixtures, options, agents, or surfaces? |
| B | Boundary | What limit, edge, ordering, timeout, missing field, or malformed state matters? |
| I | Interface | Which public CLI, JSON, TOON/plain, fixture, API, or file contract exposes the behavior? |
| E | Exceptions | How does the behavior fail closed, report blockers, and preserve actionable next actions? |
| S | Simple | What pure helper or smallest contract can be tested without runtime setup? |

Add a doubt-driven row whenever a requirement rests on an assumption:

```text
Assumption:
Doubt:
Test:
```

## VIDA Runtime Test Standard

Runtime behavior must be proved through public CLI integration or smoke tests
when the contract is visible to operators.

1. Use unit tests only for pure helpers, deterministic parsers, renderers,
   fixture builders, and contract functions that do not require project runtime
   state.
2. Use matrix or table-driven tests for CLI command families, especially when
   options, output modes, and blocker states vary.
3. Cover default TOON/plain output for human-facing commands.
4. Cover explicit JSON output when a command exposes `--json` or another
   machine-readable mode.
5. Cover `--help` when options, output modes, defaults, or next-action guidance
   change.
6. Cover fail-closed blocker shape, including `blocker_codes`, `next_actions`,
   and `artifact_refs` where the command surface exposes them.
7. Cover `--state-dir` override behavior when the public command supports it.
8. Use persisted-state fixtures when behavior depends on TaskFlow, DocFlow,
   run-graph, recovery, receipts, lane state, or runtime store truth.
9. Cover cross-surface parity when more than one command reports the same
   invariant.
10. Prefer public command assertions over private implementation assertions for
    runtime defects.

## Code-Level Test Standard

Code-level tests must protect behavior and contracts, not private incidental
layout.

1. Use pure helper unit tests for deterministic helper logic.
2. Use contract tests for shared invariants, renderers, parsers, fixtures, and
   state transition helpers.
3. Use fixture or golden tests for durable output formats, snapshots, and
   generated operator artifacts.
4. Use property or matrix tests when transitions, state combinations, ordering,
   or option combinations are involved.
5. Avoid implementation-detail-only assertions unless the implementation detail
   is itself the public or persisted contract.

## Batch Discipline

1. Plan the full test batch for the target file, behavior, or public surface
   before broad verification.
2. While shaping the batch, run focused tests that give fast feedback on the
   current invariant.
3. After the planned batch is complete, run broader or full suites required by
   the task proof targets.
4. If focused tests expose a production defect, fix the production contract and
   continue completing the planned batch before broad verification.
5. Do not close a task after only the first green focused test when the planned
   batch still has uncovered ZOMBIE-D categories.

## Task-Shaping Template

Copy this shape into TaskFlow tasks that plan or write tests:

```text
Behavior:
Z/O/M/B/I/E/S:
Doubts:
Test Matrix:
Fixtures:
Proof Targets:
Non-goals:
Stop/If blocked:
```

## Closure Gate

No test-writing task is complete without:

1. a filled ZOMBIE-D matrix for the in-scope behavior, or
2. an explicit documented reason why a category is not applicable.

The closure evidence must name the focused tests run during shaping, the broader
or full verification run after the batch, and any remaining uncovered category
with a TaskFlow follow-up or documented non-goal.

## Relationship To Existing Docs

This protocol extends the project-local testing and runtime proof rules without
replacing their owner documents:

1. `docs/process/command-timing-and-gate-optimization-protocol.md` owns proof
   timing, focused-vs-broad gate discipline, and slow-gate classification.
2. `docs/process/runtime-defect-function-option-matrix-protocol.md` owns runtime
   defect matrix rows that connect invariants, command surfaces, options,
   output contracts, owning functions, fixtures, and proof tests.
3. `docs/process/runtime-command-authority-inventory.md` owns the public command
   authority inventory for runtime and CLI surfaces.
4. `docs/process/documentation-tooling-map.md` owns DocFlow validation and
   activation checks for documentation-shaped protocol work.

-----
artifact_path: process/zombie-d-test-writing-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-06-30'
schema_version: '1'
status: canonical
source_path: docs/process/zombie-d-test-writing-protocol.md
created_at: '2026-06-30T00:00:00+03:00'
updated_at: 2026-06-30T00:00:00+03:00
changelog_ref: zombie-d-test-writing-protocol.changelog.jsonl
