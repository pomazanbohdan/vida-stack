# Semantic Testing And Local Gates Protocol

Status: active project process

Purpose: define repository-local semantic testing for TaskFlow state transitions,
persistence recovery, projections, malformed input, and targeted concurrency
without invoking VIDA runtime activation or dispatch.

## Purpose

Define repository-local semantic testing for TaskFlow state transitions,
persistence recovery, projections, malformed input, and targeted concurrency
without invoking VIDA runtime activation or dispatch.

## Trigger

Use this protocol when changing TaskFlow state, persistence, effects,
projection, parser/decoder, lease, reservation, or critical pure-function
surfaces. Run P0/P1 on pre-push; run P2-P4 only through their explicit manual
profiles.

## Scope

This protocol owns test harnesses and local proof commands. It does not create
authoritative TaskFlow receipts, execute runtime commands, mutate production
state, or replace the mutation-quality gate.

The test corpus follows the ZOMBIE-D matrix in
`docs/process/zombie-d-test-writing-protocol.md`. Replay, persistence, and
cross-surface rows require explicit evidence references in the focused gate.

Tool installation, host matrix, pinned Rust/nightly components, Kani 0.67
compatibility patch, and reproduction commands are canonical in
docs/process/rust-and-semantic-tooling-reproducibility-runbook.md. This
protocol owns semantic behavior and gate classification; the runbook owns
deployment and bootstrap reproduction.

## Authority

Rust unit/integration tests, pure parser targets, and bounded proof harnesses
are the authority for this protocol. The production TaskFlow runtime remains
outside this slice; no VIDA activation, dispatch, or runtime command is used.

## Inputs

Inputs are generated command sequences, in-memory test state, journal fault
plans, serialized JSONL/TOON values, projection fixtures, malformed parser
payloads, and bounded concurrency schedules. Inputs must remain test-only and
must not contain production receipts or effects.

## Outputs

Each profile emits typed `pass`, `blocked`, or `not_applicable` records,
duration, command logs, and JSON summaries under
`.vida/tmp/semantic-testing/<run-id>/`. Failing sequences and counterexamples
remain reproducible test artifacts.

## Rules

## Levels

| Level | Surface | Default execution |
| --- | --- | --- |
| P0 | state-machine reference model, transition invariants, fault-injection recovery | local pre-push |
| P1 | metamorphic persistence and management/dispatch projection parity | local pre-push |
| P2 | cargo-fuzz parser/config/protocol targets | manual Linux |
| P3 | Loom reservation/lease interleavings | manual Linux |
| P4 | bounded Kani proofs and targeted Miri checks | manual toolchain |

P0/P1 run through `scripts/vida-dev-gate.ps1 -Mode semantic-focused -Json`.
P2-P4 use the corresponding `semantic-*` modes. Missing tools are reported as
`blocked` or `not_applicable`; they are never silently treated as green.

## P0 state and recovery rules

The reference model generates bounded command sequences for Start, Dispatch,
CompleteLane, Block, Recover, Close, Fail, and RepairReopen. The concrete
aggregate must preserve state/version parity, event ordering, deterministic
replay, snapshot/replay hash equality, and terminal-transition version
stability. Shrinking must retain the smallest reproducing sequence.

Fault plans cover fail-before-write, fail-after-write, duplicate receipt,
stale lease, timeout/retry, and partial journal append followed by restart.
Retries may recover an accepted write, but may not duplicate semantic effects
or advance a rejected version.

## P1 metamorphic and differential rules

- path/status/issue normalization is idempotent;
- JSONL encode/decode produces one canonical line;
- TOON rendering is deterministic and scalar sanitization is stable;
- event replay and snapshot restore produce the same aggregate/hash;
- duplicate delivery is idempotent;
- management and dispatch projections differ only by the declared difference
  ledger and never perform writes/effects during comparison.

## Local gate contract

`pre-commit` remains the existing fast hygiene and script-check surface. The
`pre-push` hook runs P0/P1 serially through the project Cargo target-dir
policy. Fuzzing, Loom, Kani, and Miri stay manual so Windows developers are not
forced to install Linux-only or specialized proof toolchains.

Each semantic run writes compact JSON and raw command logs under
`.vida/tmp/semantic-testing/<run-id>/` and records duration, status,
`evidence_refs`, and artifact references. Focused P0/P1 records carry explicit
ZOMBIE-D R/P/C evidence objects pointing at those logs; a missing reference is
not green. Cargo mutation/quality commands remain a separate adequacy gate
(`quality-cycle`/`quality-pack`).

The checked-in pre-push wrapper is Windows-local (`.cmd`) to match the project
developer host. Linux/macOS operators invoke the same PowerShell gate directly;
P2-P4 remain manual platform/toolchain profiles.

## Forbidden

Do not call `vida orchestrator-init`, `project-activator`, runtime-dispatch,
or any execution surface for this protocol. Do not create authoritative
TaskFlow receipts/effects, write production state, silently skip missing tools,
or treat a `blocked`/`not_applicable` profile as green.

## Escalation

Classify missing toolchains as `blocked` or platform policy as
`not_applicable`, preserving the reason and artifact path. Escalate a failing
semantic invariant to the owning Rust crate and retain the minimized
counterexample before changing production semantics. Mutation testing remains
a separate adequacy gate.

## Validation

Required local checks are YAML/pre-commit validation, script-check, the focused
semantic gate, `git diff --check`, and DocFlow readiness for this document.
Manual Linux checks are bounded fuzz, Loom with `--cfg loom`, Kani proof, and
targeted Miri tests. Tool unavailability is an explicit typed result.

The fuzz profile copies checked-in malformed/valid seeds from `fuzz/seeds/` into
the semantic run directory, invokes each target with `-runs=64`, and routes
corpus/artifact output to that run directory so the worktree remains clean.

## Token Budget

Focused pre-push output is compact and machine-readable; raw logs stay in the
run artifact directory. Manual profiles may emit tool-native diagnostics but
must still write a summary record.

## Metadata

The footer below is the canonical artifact metadata and changelog binding for
DocFlow readiness checks.

-----
artifact_path: process/semantic-testing-and-local-gates-protocol
artifact_type: process_doc
artifact_version: 1
artifact_revision: 2026-08-12
schema_version: 1
status: canonical
source_path: docs/process/semantic-testing-and-local-gates-protocol.md
created_at: 2026-08-12T00:00:00+03:00
updated_at: 2026-08-12T00:00:00+03:00
changelog_ref: semantic-testing-and-local-gates-protocol.changelog.jsonl
