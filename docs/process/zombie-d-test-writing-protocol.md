# Project ZOMBIE-D Test-Writing Protocol

Purpose: define the project-local protocol that agents must use before writing
or updating tests in `vida-stack`.

## Purpose

Make test planning, proof coverage, and closure evidence explicit across the
Z/O/M/B/I/E/S categories, with additive R/P/C coverage for replay, persistence,
and cross-surface consistency.

## Trigger

Use this protocol for any test, fixture, snapshot, golden, coverage, or runtime
defect proof change.

## Scope

Rust, CLI, persisted-state, TaskFlow, runtime, and operator-surface test work in
this repository.

## Authority

This document owns the project ZOMBIE-D test-writing matrix; command timing and
runtime authority remain with their mapped process documents.

## Inputs

Read the active task, owned paths, acceptance targets, existing tests, fixtures,
runtime commands, and current state evidence.

## Outputs

Produce focused tests, the Z/O/M/B/I/E/S/R/P/C matrix, proof commands, artifact
refs, and a closure-ready TaskFlow evidence record.

## Rules

Use public contract tests for operator-visible behavior and focused unit tests
for pure helpers; batch related proofs before broad verification.

## Forbidden

Do not mark an uncovered category as pass, infer evidence from a green build, or
replace a blocked proof with an unverified note.

## Escalation

Create a follow-up or stop when a category remains blocked, authority conflicts,
or the required runtime fixture cannot be reproduced.

## Validation

Run the focused proof, the required broader suite, `git diff --check`, and the
mapped DocFlow/runtime checks before closure.

## Token Budget

No fixed token target; compactness is preferred only while preserving proof and
authority atoms.

## Metadata

Canonical artifact: `process/zombie-d-test-writing-protocol`; source and
changelog are the owning surfaces.

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
| R | Replay | Does replay/rebuild/deterministic re-execution reproduce the same result, version, events, and proof hash? |
| P | Persistence | Does durable state survive restart/recovery/restore, and does the fixture prove the persisted source of truth? |
| C | Cross-surface consistency | Do every affected operator or public surfaces expose the same verdict, blockers, next actions, and artifact refs? |

R/P/C are additive to the legacy matrix. They are applicable when the TaskFlow
title, description, labels, notes, owned paths, acceptance targets, or proof
targets identify replay/rebuild, durable state/recovery, or multiple operator
surfaces/parity. An applicable facet must contain a row with `status: pass` and
non-empty `evidence_refs`, or `status: na` with a concrete `reason`; omission is
blocked. Non-applicable facets may remain absent for legacy records.

### Required-category profiles and configuration authority

The `dev_team.zombie_d_gate` block in the project configuration is the runtime
policy source. When `required_categories` is omitted, the gate uses the legacy
profile in this exact order: `[Z, O, M, B, I, E, S]`. An explicit list must
match one complete ordered profile:

| Profile | Exact required categories |
| --- | --- |
| `legacy` | `[Z, O, M, B, I, E, S]` |
| `canonical` | `[Z, O, M, B, I, E, S, R, P, C]` |

Duplicates, unknown category codes, reordered values, incomplete subsets, or
any other explicit list fail closed with typed configuration blockers. Projection
and evaluation preserve the selected exact list; they do not silently drop
`R`, `P`, or `C`. The projected `required_categories_profile` is one of
`legacy`, `canonical`, or `invalid`. `optional_categories` remains `[R, P, C]`
for applicability detection when the legacy profile is selected; the canonical
profile requires all ten categories. The gate's `enabled`, `applies_to`, and
`enforcement_points` options remain independently configurable and are subject
to the same fail-closed validation at dispatch, handoff, and closure.

The canonical matrix metadata for an applicable facet is:

```json
{
  "schema_version": 1,
  "metadata": {
    "schema_version": 1,
    "applicable_categories": ["R", "P", "C"]
  }
}
```

TaskFlow planner metadata is part of the applicability contract: preserve
`owned_paths`, `acceptance_targets`, and `proof_targets` on the task so the
validator can derive and report the required facets in `artifact_refs`.

### Fresh/Explicit/Persisted/Replay mode matrix

When a test crosses authority, projection, persistence, or replay boundaries,
plan the applicable execution modes explicitly:

| Mode | Fixture source | Required assertion |
| --- | --- | --- |
| `Fresh` | current config/authority inputs, no prior persisted artifact | first-build result and generated identity are correct |
| `Explicit` | caller-supplied ids/options/overrides | explicit values win only after typed validation |
| `Persisted` | reloaded authoritative state and projections | restart/reload preserves version, identity, and field parity |
| `Replay` | recorded events/receipts/artifacts | deterministic replay matches result, sequence, and proof digest |

Each applicable row needs `status: pass` plus `evidence_refs`, or `status: na`
with a concrete reason. Do not infer a mode from a green broad suite.

### Deliberately non-coincident cross-domain fixture IDs

Fixture builders must assign deliberately non-coincident cross-domain fixture
IDs: authority, projection, persisted, and replay identifiers must be distinct
and their mapping must be recorded. Assert inequality across domains; if the
product intentionally aliases an id, model that alias explicitly and test the
alias contract instead of allowing accidental id coincidence to hide a routing
or projection defect.

### Typed authority→projection schema completeness and field-parity gate

For every authority-to-projection fixture, persist typed schema metadata with
`schema_version`, `authority_fields`, `projection_fields`, `missing_fields`,
`extra_fields`, and `type_mismatches`. The gate passes only when required field
sets and types have exact parity (or an explicitly versioned, documented
projection exception); missing, extra, unknown, or type-mismatched fields are
typed blockers and must fail closed in default and JSON evidence.

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

Follow the canonical `Focused Batch Before Broad Gate` in
`docs/process/command-timing-and-gate-optimization-protocol.md`: plan one
complete focused batch for the target file/surface, run it before any broad
gate, finish the planned batch after any production fix, then run the broader
suite. Keep uncovered categories explicit until the batch is complete.

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

### Canonical Evidence Record

Before retrying closure after a ZOMBIE-D gate failure:

1. inspect `vida task proof status <task-id> --json` and the task notes;
2. keep one canonical `zombie_d_matrix` pass record before closure;
3. encode `schema_version: 1`, `categories: {Z,O,M,B,I,E,S,R,P,C}`, and non-empty
   `evidence_refs` for every `pass` category;
4. include `metadata.applicable_categories` for every applicable R/P/C facet;
5. replace stale or invalid earlier pass records, because the runtime parser
   selects the latest target-specific pass record;
6. retry `vida task close` and require `closed=true` with `proof_verdict=pass`.

Evidence normalization must not promote a category from `blocked` to `pass`
without a concrete test, artifact, or explicit non-applicable reason. Legacy
Z/O/M/B/I/E/S records remain readable; migration is complete when triggered
R/P/C facets are added to the next canonical evidence record.

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
artifact_revision: '2026-07-22'
schema_version: '1'
status: canonical
source_path: docs/process/zombie-d-test-writing-protocol.md
created_at: '2026-06-30T00:00:00+03:00'
updated_at: 2026-07-22T00:00:00+03:00
changelog_ref: zombie-d-test-writing-protocol.changelog.jsonl
