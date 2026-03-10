# Runtime Transition Map

Purpose: provide one canonical map from legacy `*` helpers to the `taskflow-v0` transitional runtime or to explicit historical-only status.

## Active Transitional Runtime

Use `taskflow-v0` as the canonical runtime surface for these domains:

1. `boot-packet.py` / `boot-profile.sh` / `vida-boot-snapshot.py` -> `taskflow-v0 boot ...`
2. `worker-packet-gate.py` -> `taskflow-v0 worker ...`
3. route snapshot/receipt helpers -> `taskflow-v0 route ...`
4. kernel config introspection -> `taskflow-v0 kernel ...`
5. task store and import/export -> `taskflow-v0 task ...` and `taskflow-v0 br ...`
6. TaskFlow/readiness views -> `taskflow-v0 todo ...`
7. run-graph -> `taskflow-v0 run-graph ...`
8. execution auth / coach / verification prompt -> `taskflow-v0 auth ...`, `taskflow-v0 coach ...`, `taskflow-v0 coach-decision ...`, `taskflow-v0 verification-prompt ...`
9. worker runtime inventory and leases -> `taskflow-v0 system ...`, `taskflow-v0 registry ...`, `taskflow-v0 lease ...`, `taskflow-v0 pool ...`
10. context/memory/spec surfaces -> `taskflow-v0 context ...`, `taskflow-v0 context-capsule ...`, `taskflow-v0 memory ...`, `taskflow-v0 spec-intake ...`, `taskflow-v0 spec-delta ...`, `taskflow-v0 draft-execution-spec ...`

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
3. removal is blocked only until replacement behavior is either implemented in `taskflow-v0` or intentionally retired.

Pack/wave rule:

1. `vida-pack-helper`, `vida-pack-router`, `nondev-pack-init`, `framework-wave-start`, and `framework-task-sync` remain legacy orchestration wrappers only.
2. They are not authorized as the long-term canonical runtime surface.
3. Framework docs may mention them only when explicitly marked `migration-only` or `historical-only`.

## Verification Baseline

Minimum transitional runtime proof:

1. `nim c taskflow-v0/src/vida.nim`
2. `nim c -r taskflow-v0/tests/test_boot_profile.nim`
3. `nim c -r taskflow-v0/tests/test_worker_packet.nim`
4. `nim c -r taskflow-v0/tests/test_kernel_runtime.nim`

-----
artifact_path: config/system-maps/runtime-transition.map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps.runtime-transition-map.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: '2026-03-10T03:06:28+02:00'
changelog_ref: system-maps.runtime-transition-map.changelog.jsonl
