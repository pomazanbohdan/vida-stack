# VIDA Scripts — Nim Runtime Experiment

Experimental Nim port of selected VIDA script-runtime surfaces.

Current status:
- not a canonical runtime replacement
- partial parity work only
- build/test surface is still under active development

## Збірка

```bash
cd _vida/scripts-nim
nim c -d:release -o:vida-legacy src/vida.nim
```

## Команди

Nim binary name for this lane: `vida-legacy`.

### `config` — Конфігурація

```bash
vida-legacy config validate                    # Валідація vida.config.yaml
vida-legacy config dump                        # Дамп конфігу як JSON
vida-legacy config protocol-active agent_system  # Чи активний протокол?
```

Приклади:
```bash
$ vida-legacy config validate
✅ vida.config.yaml is valid

$ vida-legacy config protocol-active agent_system
active

$ vida-legacy config dump | jq '.agent_system.mode'
"hybrid"
```

### `boot` — Boot профілі та пакети

```bash
vida-legacy boot run <lean|standard|full> [task_id] [--non-dev]
vida-legacy boot verify-receipt <subject> [profile]
vida-legacy boot read-contract <lean|standard|full> [--non-dev]
vida-legacy boot summary <subject>
vida-legacy boot snapshot [--json] [--top-limit N] [--ready-limit N]
```

Приклади:
```bash
$ vida-legacy boot run standard TASK-42
✅ Boot profile: standard
  receipt: .vida/logs/boot-receipts/TASK-42.latest.json
  snapshot: available

$ vida-legacy boot read-contract lean
AGENTS.md
_vida/docs/thinking-protocol.md#section-algorithm-selector
_vida/docs/thinking-protocol.md#section-stc
...

$ vida-legacy boot snapshot --json --top-limit 3
{ "generated_at": "2026-03-08T20:00:00Z", ... }
```

### `run-graph` — Граф виконання

```bash
vida-legacy run-graph init <task_id> <task_class> [route_task_class]
vida-legacy run-graph update <task_id> <task_class> <node> <status> [route_task_class] [meta_json]
vida-legacy run-graph status <task_id>
```

Ноди: `analysis`, `writer`, `coach`, `problem_party`, `verifier`, `approval`, `synthesis`
Статуси: `pending`, `ready`, `running`, `completed`, `blocked`, `failed`, `skipped`

Приклади:
```bash
$ vida-legacy run-graph init TASK-42 implementation
.vida/state/run-graphs/TASK-42.json

$ vida-legacy run-graph update TASK-42 implementation analysis running
.vida/state/run-graphs/TASK-42.json

$ vida-legacy run-graph status TASK-42
{
  "task_id": "TASK-42",
  "present": true,
  "resume_hint": { "next_node": "analysis", "status": "running" }
}
```

### `beads` — JSONL Runtime

```bash
vida-legacy beads mode                  # Поточний режим
vida-legacy beads set-mode <mode>       # Зміна режиму (jsonl_safe / direct)
vida-legacy beads stats                 # Статистика issues
vida-legacy beads snapshot-age          # Вік snapshot (секунди)
vida-legacy beads verify [--strict]     # Верифікація логів
```

### `todo` — TODO views from beads execution log

```bash
vida-legacy todo ui-json <task_id>
vida-legacy todo list <task_id>
vida-legacy todo current <task_id>
vida-legacy todo next <task_id>
vida-legacy todo board <task_id>
vida-legacy todo compact <task_id> [limit]
vida-legacy todo tracks <task_id>
```

Приклади:
```bash
$ vida-legacy beads mode
jsonl_safe

$ vida beads stats
{ "total": 47, "unique": 23, "by_status": { "open": 5, "in_progress": 3 } }

$ vida beads verify --strict
{ "status": "ok", "issues_checked": 47, "warnings": [] }
```

### `task` — DB-backed task surface

Runtime SSOT for this lane lives in `.vida/state/vida-legacy.db`.
`.beads/issues.jsonl` is treated as an ingest/bootstrap artifact, not the live read store.

Output policy:
- `task list` and `task ready` default to `jsonl`
- `task show` defaults to `TOON`
- `--json` and `--jsonl` stay available as explicit fallbacks

Display planning format:
- epic: `vida-2d9`
- task: `vida-2d9.1`
- subtask: `vida-2d9.1.3`

```bash
vida-legacy task import-jsonl .beads/issues.jsonl
vida-legacy task create vd_epic_demo "Demo epic" --type epic --display-id vida-2d9
vida-legacy task create vd_task_demo "Demo task" --parent-display-id vida-2d9 --auto-display-from vida-2d9
vida-legacy task update vd_task_demo --status in_progress --notes "working" --add-label mode:autonomous
vida-legacy task close vd_task_demo --reason "done"
vida-legacy task list
vida-legacy task ready
vida-legacy task show vida-stack-2d9.10
vida-legacy task show vida-2d9.1
vida-legacy task show vida-stack-2d9.10 --json
vida-legacy task next-display-id vida-2d9 --json
vida-legacy task next-display-id vida-2d9.1 --json
```

### `system` — Субагент система

```bash
vida system snapshot [task_id]
vida system detect
vida system mode
vida system budget-summary [task_class]
```

Приклади:
```bash
$ vida system mode
{ "effective_mode": "hybrid", "reasons": ["requested_mode=hybrid"] }

$ vida system detect
{
  "claude_cli": { "enabled": true, "available": true, "role": "primary" },
  "gemini_cli": { "enabled": true, "available": true, "role": "secondary" }
}

$ vida system budget-summary implementation
{ "run_count": 12, "cheap_lane_attempted": 8, "bridge_fallback_used": 2 }
```

### `registry` — Реєстр можливостей

```bash
vida registry build
vida registry check <task_class> <subagent>
```

Приклади:
```bash
$ vida registry build
.vida/state/capability-registry.json

$ vida registry check analysis claude_cli
{
  "compatible": true,
  "reason": "ok",
  "task_class": "analysis",
  "subagent": "claude_cli",
  "required_artifacts": ["analysis_receipt"]
}

$ vida registry check implementation gemini_cli
{
  "compatible": false,
  "reason": "write_scope_mismatch",
  ...
}
```

### `route` — Маршрутизація

```bash
vida route resolve <task_id> <task_class> [--write-scope <scope>]
vida route receipt <task_id>
vida route mutation-snapshot <task_id>
```

Приклади:
```bash
$ vida route resolve TASK-42 implementation --write-scope scoped_only
{
  "task_id": "TASK-42",
  "dispatch_policy": "external_first",
  "risk_class": "R2",
  "selected_subagent": "claude_cli"
}
```

### `lease` — Управління лізами

```bash
vida lease acquire <resource_type> <resource_id> <holder> [--ttl-seconds N]
vida lease renew <resource_type> <resource_id> <holder> [--ttl-seconds N]
vida lease release <resource_type> <resource_id> <holder>
vida lease list
```

Приклади:
```bash
$ vida lease acquire subagent_pool claude_cli orchestrator --ttl-seconds 1800
{ "status": "acquired", "lease": { "fencing_token": 1, ... } }

$ vida lease list
{
  "leases": [...],
  "summary": { "active": 1, "released": 0, "expired": 0 }
}

$ vida lease release subagent_pool claude_cli orchestrator
{ "status": "released" }
```

### `pool` — Пул субагентів

```bash
vida pool borrow <task_class> <holder> [--ttl-seconds N]
vida pool release <subagent> <holder>
vida pool status
```

Приклади:
```bash
$ vida pool borrow analysis orchestrator
{
  "status": "acquired",
  "selected_subagent": "claude_cli",
  "task_class": "analysis"
}

$ vida pool status
{ "active_pool_leases": [...] }

$ vida pool release claude_cli orchestrator
{ "status": "released", "subagent": "claude_cli" }
```

### `auth` — Авторизація виконання

```bash
vida auth check <task_id> <task_class> [--local-write] [--block-id <id>]
vida auth authorize-local <task_id> <task_class> <reason> <scope> <notes> [evidence] [actor]
vida auth authorize-internal <task_id> <task_class> <reason> <scope> <notes> [evidence] [actor]
vida auth authorize-skip <task_id> <task_class> <reason> <notes> [evidence] [actor]
```

Приклади:
```bash
$ vida auth check TASK-42 implementation
{
  "status": "ok",
  "analysis_prereq_via": "analysis_receipt",
  "blockers": []
}

$ vida auth authorize-local TASK-42 implementation emergency_override scoped_only "hotfix needed"
.vida/logs/route-receipts/TASK-42.implementation.local-exec.json

$ vida auth authorize-skip TASK-42 implementation no_eligible_analysis_lane "framework-only override"
.vida/logs/execution-auth-overrides/TASK-42.implementation.json
```

### `worker` — Валідація worker пакетів

```bash
vida worker check <prompt_file|->
vida worker check-output <prompt_file|-> <output_file|->
```

Приклади:
```bash
$ vida worker check worker-prompt.md
{ "status": "ok", "errors": [] }

$ vida worker check - <<< "incomplete packet text"
{ "status": "blocked", "errors": ["missing worker_lane_confirmed marker", ...] }

$ vida worker check-output prompt.md output.json
{ "status": "ok", "errors": [] }
```

### `coach` — Coach review gate

```bash
vida coach check <task_id>
vida coach authorize-skip <task_id> <reason> <notes> [evidence] [actor]
```

Приклади:
```bash
$ vida coach check TASK-42
{ "status": "ok", "authorized_via": "", "blockers": [] }

$ vida coach authorize-skip TASK-42 no_eligible_coach "no coach available"
.vida/logs/coach-review-overrides/TASK-42.json
```

### `memory` — Пам'ять фреймворку

```bash
vida memory status
vida memory record <lesson|correction|anomaly> --summary <text> [--source-task <id>] [--details-json <json>]
```

Приклади:
```bash
$ vida memory status
{ "entries": [...], "summary": { "lesson_count": 5, "correction_count": 2 } }

$ vida memory record lesson --summary "Always verify imports in Nim" --source-task TASK-42
{ "ts": "2026-03-08T20:00:00Z", "kind": "lesson", "summary": "Always verify imports in Nim" }
```

### `context` — Context governance

```bash
vida context status
```

### `status` — Огляд системи

```bash
$ vida status
VIDA Runtime v0.3.0
VIDA_ROOT: /home/unnamed/project/vida-stack
Config: /home/unnamed/project/vida-stack/vida.config.yaml
Beads mode: jsonl_safe
Issues: 47 total, 23 unique
Snapshot age: 120s
```

## Глобальні флаги

```bash
vida-legacy --help       # Допомога
vida-legacy --version    # Версія (v0.3.0)
```

## Змінні середовища

| Змінна | Опис |
|---|---|
| `VIDA_ROOT` | Override project root (або через `.env`) |
| `VIDA_RUN_GRAPH_STATE_DIR` | Override run-graph state directory |

## Структура

```
src/
├── vida.nim           # CLI entry point (16 subcommands)
├── core/
│   ├── types.nim      # Typed data models
│   ├── config.nim     # YAML config loader + validation
│   ├── utils.nim      # Shared helpers (now_utc, load_json, etc.)
│   └── turso_task_store.nim  # DB-backed task-store bridge
├── boot/
│   ├── packet.nim     # Boot packet generation
│   ├── snapshot.nim   # Task state snapshot
│   └── profile.nim    # Boot profile + receipt writer
├── agents/
│   ├── system.nim     # Subagent detection, scoring, mode
│   ├── registry.nim   # Capability registry
│   ├── leases.nim     # Resource lease management
│   ├── pool.nim       # Subagent pool (borrow/release)
│   └── route.nim      # Route resolution + receipts
├── gates/
│   ├── execution_auth.nim  # Execution authorization gate
│   ├── worker_packet.nim   # Worker packet validation
│   └── coach_review.nim    # Coach review gate
└── state/
    ├── run_graph.nim  # Run graph ledger
    └── beads.nim      # Beads JSONL runtime
```

## Примітки

- This directory is an experiment, not the active production framework runtime.
- Claims in the Python/shell framework remain canonical until a tracked parity migration explicitly replaces them.
- task read surfaces now come from the DB-backed `vida-legacy task` store
- Rust `vida` binary (`crates/vida/`) підтримує `--state-dir` / `VIDA_STATE_DIR`
- `.beads/issues.jsonl` is an ingest/export artifact; primary task reads in this lane come from `.vida/state/vida-legacy.db`
