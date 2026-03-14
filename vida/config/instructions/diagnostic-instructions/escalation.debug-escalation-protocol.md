# Debug Escalation Protocol

Purpose: define when repeated technical failures must stop blind local trial-and-error and enter bounded escalation through the canonical validation and dispatch owners.

## Core Contract

1. One bounded local fix attempt is normal.
2. If the same technical error repeats a second time, escalate.
3. If an external API, crate format, or protocol surface is uncertain, escalate as soon as uncertainty is material.

## Escalation Trigger

Escalation is mandatory when any are true:

1. the same compile/runtime error appears twice,
2. the same class of fix fails twice,
3. the API/crate/format/version behavior is not confidently known,
4. the failure concerns external library semantics rather than only local code logic.

## Mandatory Escalation Sequence

When escalation is triggered, use this order unless a stronger law already narrows the path:

1. capture the repeated error and failed local hypotheses in task evidence,
2. activate the canonical delegated-diagnosis path through `instruction-contracts/core.agent-system-protocol` and `instruction-contracts/lane.worker-dispatch-protocol` when worker mode supports it,
3. activate `runtime-instructions/work.web-validation-protocol` when external-fact uncertainty is material,
4. synthesize the next bounded fix attempt only after the required escalation evidence exists.

Hard rule:

1. after the same technical error appears twice, do not continue with solo local trial-and-error only,
2. at least one lawful escalation owner must run before the next substantive fix attempt,
3. for external crate/API/version semantics, prefer both delegated diagnosis and web validation when available.

## Bridge Rules

Web-validation bridge:

1. when external-fact uncertainty is material, this protocol must activate `runtime-instructions/work.web-validation-protocol`,
2. this file does not own source-quality hierarchy, live-check workflow, or web-search completeness law,
3. store bounded WVP evidence in task artifacts before the next substantive fix attempt when WVP triggers fired.

Delegated-diagnosis bridge:

1. when worker mode is not `disabled`, dispatch behavior must follow `instruction-contracts/core.agent-system-protocol` and `instruction-contracts/lane.worker-dispatch-protocol`,
2. this file does not own backend choice, packet shape, or verifier-selection law,
3. if no eligible delegated diagnosis lane exists, record explicit evidence and continue with the remaining lawful escalation path.

## Fail-Closed Rule

1. Do not keep repeating blind edits after repeated API drift failures.
2. Do not pretend confidence about unknown external formats or APIs.

-----
artifact_path: config/diagnostic-instructions/debug-escalation.protocol
artifact_type: diagnostic_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/diagnostic-instructions/escalation.debug-escalation-protocol.md
created_at: '2026-03-09T12:00:46+02:00'
updated_at: '2026-03-11T13:34:00+02:00'
changelog_ref: escalation.debug-escalation-protocol.changelog.jsonl
