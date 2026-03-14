# Execution Health-Check Protocol

Purpose: define the canonical verification and health-check gates before close, handoff, and strict execution completion.

## Scope

This protocol applies when a bounded execution slice needs a health-check or close/handoff gate.

It owns:

1. health-check modes,
2. required proof before close/handoff on transitioned slices,
3. overlay-schema validation gate during health-check,
4. WVP evidence expectations during health-check,
5. admissibility rules for quick vs strict execution checks.

It does not own:

1. operator command examples,
2. wrapper migration status,
3. generic shell-discipline law,
4. project-specific build/run command selection.

## Core Contract

Health-check is the canonical runtime sanity gate for bounded execution slices.

It must be able to:

1. run in bounded intermediate mode,
2. run in stricter development-close mode,
3. run in final pre-close / pre-handoff mode,
4. surface missing WVP evidence when external-fact triggers fired,
5. fail closed when required proof or overlay validation is missing.

## Modes

Supported health-check modes:

1. `quick`
   - intermediate checks during active work,
   - skips the strictest close/handoff gates.
2. `strict-dev`
   - development-cycle close checks before claiming the current slice is locally ready.
3. `full`
   - final post-`pack-end`, pre-close/handoff verification.

Rule:

1. use `quick` during in-flight execution,
2. use `strict-dev` before development-cycle closure,
3. reserve `full` for final close/handoff state.

## Mandatory Gate

Before `vida taskflow task close` of an active task on transitioned slices:

1. run the relevant verification set defined by the transitioned runtime surfaces,
2. require the bounded health-check to pass,
3. block closure if required proof is missing.

Before worker-result handoff on transitioned slices:

1. run the relevant verification set defined by the transitioned runtime surfaces,
2. require the bounded health-check to pass,
3. block handoff if required proof is missing.

## Overlay Validation Gate

If root `vida.config.yaml` exists:

1. health-check must validate overlay schema before passing,
2. parse success alone is insufficient,
3. invalid overlay schema is a blocking failure.

## WVP Evidence Gate

If WVP triggers fired:

1. record evidence per `runtime-instructions/work.web-validation-protocol`,
2. prefer structured WVP evidence markers when available,
3. do not pass final health-check without required WVP evidence.

Framework-scope diagnosis/overlay note:

1. soft WVP keywords alone should not fail health-check for framework-scope diagnosis/overlay tasks unless a stronger external-fact trigger is also present.

## Runtime Proof Boundary

The transitioned runtime proof set is owned by the active transitioned runtime surfaces.

At minimum, health-check must confirm:

1. the relevant proof commands/tests for the transitioned slice passed,
2. no blocking contradiction remains in the current execution state,
3. close/handoff is lawful for the current mode.

Operator examples and concrete command snippets may live in:

1. `command-instructions/operator.runtime-pipeline-guide`
2. runtime-family maps and runtime homes

## Fail-Closed Rule

1. Do not treat a wrapper or command example as proof by itself.
2. Do not treat `quick` mode as equivalent to final close/handoff proof.
3. Do not allow close/handoff when overlay validation or required WVP evidence is still missing.

-----
artifact_path: config/runtime-instructions/execution-health-check.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.execution-health-check-protocol.md
created_at: '2026-03-11T00:00:00+02:00'
updated_at: '2026-03-11T13:03:05+02:00'
changelog_ref: work.execution-health-check-protocol.changelog.jsonl
