# Runtime Transition Map

Purpose: provide one canonical transition registry from legacy helper surfaces to current runtime homes and make migration-only or historical-only status explicit without turning this map into a second owner of execution or verification law.

## Active Transitional Runtime

Use `vida taskflow` as the canonical runtime surface for these domains:

1. `boot-packet.py` / `boot-profile.sh` / `vida-boot-snapshot.py` -> `vida taskflow boot ...`
2. `worker-packet-gate.py` -> `vida taskflow worker ...`
3. route snapshot/receipt helpers -> `vida taskflow route ...`
4. kernel config introspection -> `vida taskflow kernel ...`
5. task store and import/export -> `vida taskflow task ...`
6. TaskFlow/readiness views -> `vida taskflow todo ...`
7. run-graph -> `vida taskflow run-graph ...`
8. execution auth / coach / verification prompt -> `vida taskflow auth ...`, `vida taskflow coach ...`, `vida taskflow coach-decision ...`, `vida taskflow verification-prompt ...`
9. worker runtime inventory and leases -> `vida taskflow system ...`, `vida taskflow registry ...`, `vida taskflow lease ...`, `vida taskflow pool ...`
10. context/memory/spec surfaces -> `vida taskflow context ...`, `vida taskflow context-capsule ...`, `vida taskflow memory ...`, `vida taskflow spec-intake ...`, `vida taskflow spec-delta ...`, `vida taskflow draft-execution-spec ...`

Interpretation rule:

1. concrete operator commands for `run-graph`, capability-registry, and context-governance belong to the active `vida taskflow` runtime surface or its help surface, not to the peer `core` owner protocols,
2. the peer `core` protocols keep semantic ownership of law, boundaries, and proof conditions above those concrete runtime commands.

## Historical-Only Until Retired Or Reimplemented

These surfaces still exist as migration sources but are not the target canonical home:

1. `beads-workflow.sh`
2. `quality-health-check.sh`
3. `beads-bg-sync.sh`
4. `vida-pack-helper.sh`
5. `vida-pack-router.sh`
6. `nondev-pack-init.sh`
7. `framework-wave-start.sh`
8. `framework-task-sync.py`
9. `skill-discovery.py`
10. `doc-lifecycle.py`
11. `problem-party.py`
12. `render-worker-prompt.sh`
13. `framework-memory.py`
14. `trace-eval.py`

Rule:

1. historical-only commands may be referenced only as migration sources or temporary gaps,
2. they must not be treated as the long-term active runtime home,
3. removal is blocked only until replacement behavior is either implemented in the active TaskFlow runtime family or intentionally retired.

Pack/wave rule:

1. `vida-pack-helper`, `vida-pack-router`, `nondev-pack-init`, `framework-wave-start`, and `framework-task-sync` remain legacy orchestration wrappers only.
2. They are not authorized as the long-term canonical runtime surface.
3. Framework docs may mention them only when explicitly marked `migration-only` or `historical-only`.

## Boundary Rule

This map owns only:

1. transition mapping,
2. migration-only or historical-only status,
3. pointers from legacy wrapper surfaces to current runtime homes.

This map does not own:

1. framework-wave execution semantics,
2. task-state sync semantics,
3. runtime verification baseline law,
4. close or handoff admissibility rules.

Current owner notes:

1. transitioned-slice verification baseline lives in `runtime-instructions/work.taskflow-protocol`,
2. `framework-wave-start.sh` remains a migration-only helper shortcut to existing owners in `core.orchestration-protocol.md`, `command-instructions/routing.use-case-packs-protocol.md`, and `diagnostic-instructions/analysis.framework-self-analysis-protocol.md`; no separate framework-wave-start law is promoted yet,
3. `framework-task-sync.py` remains a migration-only helper shortcut to `runtime-instructions/work.task-state-reconciliation-protocol`; no separate framework-wave task-sync law is promoted yet,
4. framework-wave wrappers remain migration-only references until a stronger canonical owner is promoted.

-----
artifact_path: config/system-maps/runtime-transition.map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/migration.runtime-transition-map.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: '2026-03-11T15:17:28+02:00'
changelog_ref: migration.runtime-transition-map.changelog.jsonl
