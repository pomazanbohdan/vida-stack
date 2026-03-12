# Command Execution Discipline Protocol

Purpose: define the canonical shell and runtime discipline for temporary artifacts, command serialization, project command boundaries, code search, and bounded log reads.

## Scope

This protocol applies when VIDA chooses, sequences, or interprets shell/runtime commands during execution.

It owns:

1. temp artifact placement,
2. handoff artifact handling,
3. project command boundary,
4. project preconditions boundary,
5. command serialization policy,
6. code-search policy,
7. log-read budget.

It does not own:

1. project-specific build/run instructions,
2. operator command examples,
3. wrapper migration status,
4. execution health-check gate law.

## Handoffs And Temp Artifacts

1. TDC v3.1 handoff writes artifacts to file or `br` issue body, not chat summary.
2. Large command output over bounded human-read size should be redirected to `.vida/scratchpad/` and inspected with focused reads.
3. Temporary artifacts belong only under `_temp/`.

## Project Command Boundary

1. Use project-documented canonical commands from the active host-project operations runbook.
2. Do not invent ad hoc build/deploy/audit commands when a project script already exists.
3. If project operational guidance changes, update project-owned docs and scripts, not framework-owned instruction canon.

## Project Preconditions

1. Framework policy stays generic in framework-owned surfaces.
2. Project-specific preflight order belongs only in the active host-project operations runbook.
3. If analyzer/build/test behavior depends on project environment preparation, document that sequence in project runbooks, not framework-owned runtime law.

## Command Serialization Policy

1. Parallelize read-only discovery commands when scopes are independent.
2. Do not parallelize stateful commands by default.
3. Stateful commands include:
   - task-state mutation,
   - language/runtime execution, tests, builds, dependency resolution,
   - project scripts that mutate cache/runtime state,
   - live API mutations,
   - DB/schema/cache/storage mutations.
4. If a command may take a lock or mutate shared runtime state, serialize it.

## Code Search Policy

1. Use `rg` as the primary cross-file search tool.
2. Use `rg --files` for fast file discovery.
3. Use `grep` or exact-match tools only for exact string or filename pattern matching.
4. Full tooling/search orientation belongs to `vida/config/instructions/system-maps/tooling.search-guide.md`.

## Log-Read Budget

Broad runtime-log inspection is forbidden by default.

Rules:

1. start with exact-key lookup against a specific file when possible,
2. prefer one manifest/state file over broad `.vida/logs`, `.vida/state`, or `.beads` scans,
3. prefer short window reads over large dumps,
4. do not emit raw JSONL/JSON dumps unless an explicit escalation reason is recorded,
5. if worker evidence already identifies the relevant file/line or artifact reference, do not repeat wide local log inspection without a new blocker or conflict,
6. `answer_only` flows should avoid runtime log sweeps unless bounded evidence is insufficient.

## Fail-Closed Rule

1. Do not let operator convenience override serialization safety.
2. Do not let broad log sweeps become a default substitute for bounded evidence.
3. Do not treat framework-owned instruction canon as the home for project-specific runbook sequencing.

Operator examples and concrete command snippets may live in:

1. `vida/config/instructions/command-instructions/operator.runtime-pipeline-guide.md`
2. project runbooks
3. runtime-family maps and runtime homes

-----
artifact_path: config/runtime-instructions/command-execution-discipline.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.command-execution-discipline-protocol.md
created_at: '2026-03-11T00:00:00+02:00'
updated_at: '2026-03-11T13:02:48+02:00'
changelog_ref: work.command-execution-discipline-protocol.changelog.jsonl
