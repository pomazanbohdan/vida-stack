# Runtime Operator Tooling Map

Purpose: provide one discoverability map for operator-facing runtime commands, workflow entrypoints, and practical tooling surfaces without turning those examples into competing protocol-law owners.

## Activation Triggers

Read this map when:

1. the user asks how to run a health check, workflow wrapper, or boot preflight command,
2. operator-facing runtime entrypoints are needed for tracked execution,
3. GitHub CLI operations are needed as part of the current runtime workflow,
4. migration-only wrapper lookup is needed without changing canonical law owners.

## Routing

1. health-check semantics:
   - continue to `vida/config/instructions/runtime-instructions/work.execution-health-check-protocol.md`
2. command discipline / temp artifacts / command boundary:
   - continue to `vida/config/instructions/runtime-instructions/work.command-execution-discipline-protocol.md`
3. tracked execution lifecycle and TaskFlow law:
   - continue to `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
4. task-state workflow wrapper law:
   - continue to `vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md`
5. script/runtime implementation boundary:
   - continue to `vida/config/instructions/system-maps/migration.script-runtime-architecture-map.md`
6. migration-only wrapper catalog:
   - continue to `vida/config/instructions/command-instructions/migration.pack-wrapper-note.md`
   - continue to `vida/config/instructions/system-maps/migration.runtime-transition-map.md`

## Operator Entry Surfaces

### Health Check

Fast operator sanity check:

```bash
nim c taskflow-v0/src/vida.nim
nim c -r taskflow-v0/tests/test_boot_profile.nim
nim c -r taskflow-v0/tests/test_worker_packet.nim
nim c -r taskflow-v0/tests/test_kernel_runtime.nim
```

### Workflow Wrappers

```bash
bash beads-workflow.sh ready
bash beads-workflow.sh start <id>
bash beads-workflow.sh pack-start <id> <pack_id> "goal" "constraints"
bash beads-workflow.sh block-plan <id> B01 "goal"
bash beads-workflow.sh block-start <id> B01 "goal"
bash beads-workflow.sh block-finish <id> B01 done "B02" "actions" "artifacts" - - "evidence" "85"
bash beads-workflow.sh pack-end <id> <pack_id> done "summary" "next"
bash beads-workflow.sh finish <id> "All ACs met"
```

### Boot Preflight

```bash
taskflow-v0 boot run lean <task_id>
taskflow-v0 boot verify-receipt <task_id> [profile]
```

### Context Compression

```bash
bash beads-compact.sh pre <id> "done" "next" "risk"
bash beads-compact.sh post <task_after>
```

### Optional Backup

```bash
bash beads-bg-sync.sh start --interval 600
bash beads-bg-sync.sh status
bash beads-bg-sync.sh stop
```

### Evaluation Pack

```bash
bash eval-pack.sh run <task_id>
python3 worker-eval-pack.py run <task_id>
```

### GitHub Operations

1. `gh` CLI is the preferred operator path for PR, review, workflow, and run inspection.
2. Prefer `gh pr`, `gh run`, and `gh workflow` over browser-only actions when the CLI can perform the task.

## Boundary Rule

1. This file is a discoverability/tooling map, not a law owner.
2. It must not redefine health-check, command-discipline, TaskFlow, or migration law.
3. When an example implies a gate or blocker, defer to the canonical protocol owner instead of copying the law here.

-----
artifact_path: config/system-maps/runtime-operator-tooling.map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/tooling.runtime-operator-tooling-map.md
created_at: '2026-03-11T00:00:00+02:00'
updated_at: '2026-03-11T13:41:21+02:00'
changelog_ref: tooling.runtime-operator-tooling-map.changelog.jsonl
