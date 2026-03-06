# Runtime Pipeline And Tooling Policy

Framework-owned execution and verification policy lives here. Project-specific build/run/audit commands live in `docs/process/project-operations.md` and `scripts/`.

## One-Command Health Check

Run this before handoff or finish when you want a fast protocol sanity check:

```bash
bash _vida/scripts/quality-health-check.sh [task_id]
bash _vida/scripts/quality-health-check.sh --mode quick [task_id]
bash _vida/scripts/quality-health-check.sh --mode strict-dev [task_id]
bash _vida/scripts/quality-health-check.sh [task_id] --mode quick
```

If `task_id` is provided, strict execution-log verification and pack-coverage checks are enforced for that task.
For tasks with external assumptions, health-check also reports WVP evidence warnings.

Mode notes:

1. Default mode is `full` (strict gate for close/handoff).
2. `--mode quick` is for intermediate checks and skips strict TODO snapshot gates.

Mandatory gate:

1. Before `br close` of an active task, run `bash _vida/scripts/quality-health-check.sh <task_id>`.
2. Before subagent-result handoff, run `bash _vida/scripts/quality-health-check.sh <task_id>`.
3. If root `vida.config.yaml` exists, health-check must validate overlay schema before passing.
4. If WVP triggers fired, record evidence per `_vida/docs/web-validation-protocol.md`.
4.1. Prefer structured WVP markers via `bash _vida/scripts/wvp-evidence.sh ...` to reduce heuristic false positives in health checks.
4.2. Health-check should ignore soft WVP keywords for framework-scope diagnosis/overlay tasks unless a strong external-fact trigger is also present.
5. During in-flight execution, prefer `--mode quick`; use `--mode strict-dev` for development-cycle close checks; reserve `full` mode for final post-`pack-end`, pre-close/handoff checks.

TODO decomposition and parallel execution policy:

```bash
_vida/docs/todo-protocol.md
```

## Handoffs And Temp Artifacts

1. TDC v3.1 handoff: write artifacts to file or `br` issue body; do not hand off via chat summary.
2. Large command output (>100 lines): redirect to `.vida/scratchpad/` and inspect with focused `grep`.
3. Temporary artifacts: store only under `_temp/`.

## Project Command Boundary

1. Use project-documented canonical commands from `docs/process/project-operations.md`.
2. Do not invent ad hoc build/deploy/audit commands when a project script already exists.
3. If project operational guidance changes, update `docs/process/*` and `scripts/*`, not `_vida/docs/*`.

## Project Preconditions

1. Framework policy stays generic here.
2. Project-specific preflight order (for example dependency resolution, build sequencing, or product runbooks) belongs only in `docs/process/project-operations.md`.
3. If analyzer/build/test behavior depends on project environment preparation, the canonical sequence must be documented in project runbooks, not `_vida/docs/*`.

## Command Serialization Policy

1. Parallelize read-only discovery commands (`rg`, `sed`, `cat`, `find`, JSON inspection) when scopes are independent.
2. Do not parallelize stateful commands.
3. Stateful commands include:
   - task-state mutation (`br`, `beads-workflow`, TODO sync/index writes),
   - `flutter`, `dart run`, tests, builds, dependency resolution,
   - project scripts that mutate cache/runtime state,
   - live API mutations,
   - DB/schema/cache/storage mutations.
4. If a command may take a lock or mutate shared runtime state, serialize it by default.

## Code Search Policy (Mandatory)

1. Use `rg` as the primary cross-file search tool.
2. Use `rg --files` for fast file discovery.
3. Use `grep`/`Glob` only for exact string or filename pattern matching.
4. Full MCP search guide: `_vida/docs/tooling.md`.

## Script Runtime Architecture

Framework runtime scripts follow one hybrid rule:

1. shell entrypoints keep the stable CLI surface,
2. Python engines own complex parsing, derivation, validation, and scoring logic,
3. do not keep duplicate jq/bash logic after a Python migration.

Canonical source:

```bash
_vida/docs/script-runtime-architecture.md
```

Operational expectation:

1. verify both the wrapper and the Python engine,
2. verify at least one real consumer path after migration,
3. keep shell-first orchestration scripts as the owner of lock-sensitive sequencing.

## Beads Workflow Automation

Validate skill availability before skill-driven flows:

```bash
bash _vida/scripts/validate-skills.sh
```

Use wrappers to keep task state consistent and reduce protocol drift:

```bash
bash _vida/scripts/beads-workflow.sh ready
bash _vida/scripts/beads-workflow.sh start <id>
bash _vida/scripts/beads-workflow.sh pack-start <id> <pack_id> "goal" "constraints"
bash _vida/scripts/beads-workflow.sh block-plan <id> B01 "goal"
bash _vida/scripts/beads-workflow.sh block-start <id> B01 "goal"
bash _vida/scripts/beads-workflow.sh block-finish <id> B01 done "B02" "actions" "artifacts" - - "evidence" "85"
bash _vida/scripts/beads-workflow.sh pack-end <id> <pack_id> done "summary" "next"
bash _vida/scripts/beads-workflow.sh finish <id> "All ACs met"
```

Shortcut for standard non-dev initialization:

```bash
bash _vida/scripts/nondev-pack-init.sh <task_id> <pack_id> "<goal>" [constraints]
```

Boot profile preflight:

```bash
bash _vida/scripts/boot-profile.sh run lean <task_id>
bash _vida/scripts/boot-profile.sh verify-receipt <task_id> [profile]
```

Escalate profile (`standard`/`full`) only when risk/complexity justifies extra read-set.

Quiet background backup (optional):

```bash
bash _vida/scripts/beads-bg-sync.sh start --interval 600
bash _vida/scripts/beads-bg-sync.sh status
bash _vida/scripts/beads-bg-sync.sh stop
```

Policy: prefer sparse interval (default 10 min) to avoid high operational overhead. The worker snapshots `.beads/issues.jsonl`; it is not a DB-authoritative sync loop.

Hard execution rule:

1. Do not run implementation or research steps outside an active TODO block.
2. For non-trivial work, pre-register planned blocks with `block-plan` before execution.
3. For pack-oriented flow, keep `pack-start`/`pack-end` balanced.

Execution log is stored at:

```bash
.vida/logs/beads-execution.jsonl
```

`finish` enforces a strict verification gate and refuses to close the issue if critical log contradictions are found.
Strict gate also requires at least one self-reflection entry for the task.

Self-reflection spec:

```bash
_vida/docs/self-reflection-protocol.md
```

Use-case routing spec:

```bash
_vida/docs/use-case-packs.md
bash _vida/scripts/vida-pack-helper.sh detect "<request>"
```

For context compression checkpoints:

```bash
bash _vida/scripts/beads-compact.sh pre <id> "done" "next" "risk"
bash _vida/scripts/beads-compact.sh post <task_after>
```

Compact safety gate:

1. `pre` writes context capsule for `<id>`.
2. `post` runs hydration gate before restoring execution.
3. On hydration failure, stop with `BLK_CONTEXT_NOT_HYDRATED` and do not continue implementation.

Evaluation pack (learning loop baseline):

```bash
bash _vida/scripts/eval-pack.sh run <task_id>
python3 _vida/scripts/subagent-eval-pack.py run <task_id>
```

Use generated scorecards (`.vida/logs/eval-pack-<task_id>.json`, `.vida/logs/subagent-review-<task_id>.json`) and strategy snapshot (`.vida/state/subagent-strategy.json`) for telemetry-driven improvement decisions.

## GitHub Operations

1. `gh` CLI is available in this environment and is the preferred path for GitHub operations.
2. Prefer `gh` for PR/review/workflow operations (`gh pr`, `gh run`, `gh workflow`) to keep commands auditable in shell history.
3. Use browser/manual GitHub actions only when `gh` cannot perform the required action.
