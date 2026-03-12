# Log File Policy

Purpose: define what runtime/protocol logs are local artifacts vs versioned documentation.

## 1) Non-Versioned Runtime Logs

These files are local execution artifacts and MUST NOT be committed:

1. `.vida/logs/*.jsonl`
2. `.vida/logs/context-capsules/*.json`
3. `.vida/scratchpad/*`
4. ad-hoc command outputs in `_temp/`

Rationale:

1. High churn and low signal in VCS.
2. Environment-specific data and timestamps.
3. Can leak internal operational context.

## 2) Versioned Protocol Sources

These files define process and MUST be committed:

1. `AGENTS.md`
2. `vida/config/instructions/*.md` instruction documents
3. `vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md`
4. `scripts/*.sh` protocol tooling

## 3) Escalation Exception

If a runtime log must be shared for incident analysis:

1. Store a sanitized snapshot under `_temp/`.
2. Summarize in issue notes (`br update <id> --notes ...`).
3. Do not promote raw logs into VCS.

## 4) Console Output Policy

1. TaskFlow runtime scripts must minimize non-essential console chatter by default.
2. Keep machine-readable output stable on `stdout` (`taskflow-tool ui-json`, `list`, `current`, `next`).
3. Keep user-facing diagnostics in `quality-health-check.sh` and explicit status commands.
4. Use explicit verbose mode (`VIDA_TASKFLOW_VERBOSE=1`) only for debugging.

-----
artifact_path: config/runtime-instructions/log.policy
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/runtime.log-policy.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-11T12:52:33+02:00'
changelog_ref: runtime.log-policy.changelog.jsonl
