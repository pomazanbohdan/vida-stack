# Pack Completion Gate Protocol

Purpose: define the canonical close gate for declaring a routed pack complete so pack coverage, completion evidence, strict verification, and terminal completion verdict stay under one bounded owner.

## Scope

This protocol applies when an active use-case pack is about to be declared complete.

It owns:

1. pack start/end balance as a completion prerequisite,
2. minimum completion evidence required at pack close,
3. strict verification requirement before a pack-complete claim,
4. terminal completion verdicts for pack lifecycle closure.

It does not own:

1. pack taxonomy or pack selection,
2. inter-pack handoff admissibility,
3. generic health-check mode definitions,
4. task-state SSOT,
5. generic TaskFlow planning or task-close law.

## Core Contract

A routed pack may be declared complete only when all are true:

1. the active pack was explicitly opened with `pack-start`,
2. the pack is being closed with a matching `pack-end`,
3. pack events remain balanced for the current routed scope,
4. completion evidence for the pack's bounded outputs is present,
5. strict verification passed for the pack-complete claim,
6. the pack receives one explicit terminal completion verdict.

If any item fails, the pack must remain open or blocked rather than silently treated as complete.

## Completion Prerequisites

Before claiming pack completion:

1. the pack must have one explicit `pack_id`,
2. all mandatory outputs expected for the current pack route must be present or explicitly blocked,
3. completion evidence must be visible in TaskFlow/block evidence or bounded pack-close summary,
4. required runtime proof must satisfy `vida/config/instructions/runtime-instructions/work.execution-health-check-protocol.md` in `full` mode.

Pack-specific output expectations remain owned by the deeper canonical runtime/command owners for that pack.

## Completion Evidence

At minimum, pack completion evidence must make visible:

1. `pack_id`,
2. completion summary,
3. completion or blocker evidence for the pack's mandatory outputs,
4. strict verification status,
5. resulting terminal verdict.

Evidence may live in:

1. `pack-end` payload,
2. TaskFlow block evidence,
3. bounded verification artifacts referenced by the active pack owner.

## Terminal Verdicts

Allowed terminal verdicts:

1. `done`
   - the pack completed lawfully and may proceed to handoff or closure.
2. `blocked`
   - the pack cannot complete because a required output, approval, or proof item is missing.
3. `partial`
   - the pack must not be treated as complete; further work or reconciliation is required before handoff.

Rule:

1. `done` is lawful only when all completion prerequisites are satisfied,
2. `blocked` and `partial` must preserve blocker visibility rather than collapsing into implicit completion.

## Boundary To Adjacent Owners

Ownership boundaries:

1. `vida/config/instructions/command-instructions/routing.use-case-packs-protocol.md`
   - owns pack taxonomy and routing intent only.
2. `vida/config/instructions/runtime-instructions/work.pack-handoff-protocol.md`
   - owns cross-pack admissibility after a pack has a lawful completion verdict.
3. `vida/config/instructions/runtime-instructions/work.execution-health-check-protocol.md`
   - owns health-check modes and verification proof expectations used by this gate.
4. `vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md`
   - owns pack event logging/wrapper usage, not pack completion law.
5. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
   - owns generic planning/execution/task gates, not pack-complete admissibility.

## Fail-Closed Rule

1. Do not treat balanced `pack-start`/`pack-end` events alone as proof of pack completion.
2. Do not treat a `pack-end` wrapper call alone as a lawful completion verdict.
3. Do not hand off to the next pack when completion evidence or strict verification is missing.
4. Do not reuse task-close proof as implicit pack-complete proof unless the active pack evidence explicitly covers the pack outputs.

-----
artifact_path: config/runtime-instructions/pack-completion-gate.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.pack-completion-gate-protocol.md
created_at: '2026-03-11T00:00:00+02:00'
updated_at: '2026-03-11T13:03:27+02:00'
changelog_ref: work.pack-completion-gate-protocol.changelog.jsonl
