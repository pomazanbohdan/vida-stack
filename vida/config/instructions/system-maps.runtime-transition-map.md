# Runtime Transition Map

Purpose: provide one canonical map from legacy `docs/framework/history/_vida-source/scripts/*` helpers to the `vida-v0` transitional runtime or to explicit historical-only status.

## Active Transitional Runtime

Use `vida-v0` as the canonical runtime surface for these domains:

1. `boot-packet.py` / `boot-profile.sh` / `vida-boot-snapshot.py` -> `vida-v0 boot ...`
2. `worker-packet-gate.py` -> `vida-v0 worker ...`
3. route snapshot/receipt helpers -> `vida-v0 route ...`
4. kernel config introspection -> `vida-v0 kernel ...`
5. task store and import/export -> `vida-v0 task ...` and `vida-v0 br ...`
6. TaskFlow/readiness views -> `vida-v0 todo ...`
7. run-graph -> `vida-v0 run-graph ...`
8. execution auth / coach / verification prompt -> `vida-v0 auth ...`, `vida-v0 coach ...`, `vida-v0 coach-decision ...`, `vida-v0 verification-prompt ...`
9. worker runtime inventory and leases -> `vida-v0 system ...`, `vida-v0 registry ...`, `vida-v0 lease ...`, `vida-v0 pool ...`
10. context/memory/spec surfaces -> `vida-v0 context ...`, `vida-v0 context-capsule ...`, `vida-v0 memory ...`, `vida-v0 spec-intake ...`, `vida-v0 spec-delta ...`, `vida-v0 draft-execution-spec ...`

## Historical-Only Until Retired Or Reimplemented

These surfaces still exist as migration sources but are not the target canonical home:

1. `docs/framework/history/_vida-source/scripts/beads-workflow.sh`
2. `docs/framework/history/_vida-source/scripts/quality-health-check.sh`
3. `docs/framework/history/_vida-source/scripts/beads-bg-sync.sh`
4. `docs/framework/history/_vida-source/scripts/vida-pack-helper.sh`
5. `docs/framework/history/_vida-source/scripts/vida-pack-router.sh`
6. `docs/framework/history/_vida-source/scripts/nondev-pack-init.sh`
7. `docs/framework/history/_vida-source/scripts/framework-wave-start.sh`
8. `docs/framework/history/_vida-source/scripts/framework-task-sync.py`
9. `docs/framework/history/_vida-source/scripts/skill-discovery.py`
10. `docs/framework/history/_vida-source/scripts/doc-lifecycle.py`
11. `docs/framework/history/_vida-source/scripts/problem-party.py`
12. `docs/framework/history/_vida-source/scripts/render-worker-prompt.sh`
13. `docs/framework/history/_vida-source/scripts/framework-memory.py`
14. `docs/framework/history/_vida-source/scripts/trace-eval.py`

Rule:

1. historical-only commands may be referenced only as migration sources or temporary gaps,
2. they must not be treated as the long-term active runtime home,
3. removal is blocked only until replacement behavior is either implemented in `vida-v0` or intentionally retired.

Pack/wave rule:

1. `vida-pack-helper`, `vida-pack-router`, `nondev-pack-init`, `framework-wave-start`, and `framework-task-sync` remain legacy orchestration wrappers only.
2. They are not authorized as the long-term canonical runtime surface.
3. Framework docs may mention them only when explicitly marked `migration-only` or `historical-only`.

## Verification Baseline

Minimum transitional runtime proof:

1. `nim c vida-v0/src/vida.nim`
2. `nim c -r vida-v0/tests/test_boot_profile.nim`
3. `nim c -r vida-v0/tests/test_worker_packet.nim`
4. `nim c -r vida-v0/tests/test_kernel_runtime.nim`

-----
artifact_path: config/system-maps/runtime-transition.map
artifact_type: system_map
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/system-maps.runtime-transition-map.md
created_at: 2026-03-09T20:28:59+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: system-maps.runtime-transition-map.changelog.jsonl
