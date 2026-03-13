# Runtime Kernel Bundle Protocol

Purpose: define the canonical runtime-owned bundle that composes kernel law, route law, receipt law, instruction bundle metadata, and project agent extensions into one executable payload for `taskflow`.

## Core Contract

1. `taskflow` must be able to compose one runtime-owned kernel bundle without broad manual document traversal.
2. The bundle must expose machine, route, receipt, instruction, and runtime-agent inventory boundaries explicitly.
3. The bundle must remain executable and inspectable before direct runtime consumption is attempted.

## Canonical Runtime Surfaces

1. `taskflow-v0/src/core/runtime_bundle.nim`
2. `vida orchestrator-init`
3. `vida agent-init`
4. `taskflow-v0 bundle check --json`
5. `taskflow-v0 kernel summary --json`
6. `vida/config/instructions/bundles/default_runtime.yaml`
7. `docs/product/spec/partial-development-kernel-model.md`
8. `docs/product/spec/canonical-machine-map.md`
9. `docs/product/spec/receipt-and-proof-law.md`

## Bundle Minimum

The runtime kernel bundle must expose all of:

1. kernel family summary,
2. canonical machine-map binding,
3. route catalog artifact,
4. receipt taxonomy artifact,
5. instruction bundle ordering,
6. runtime agent inventory,
7. compiled project role/skill/profile/flow extensions.
8. compiled init views and compact startup projections when the local runtime exposes them.

## Closure Rule

The runtime kernel bundle is closed enough for current `taskflow` consumption only when:

1. bundle ordering is explicit,
2. route and receipt artifacts are present,
3. runtime inventory is present,
4. compiled agent extensions are present,
5. the bundle passes `taskflow-v0 bundle check --json`.

## Boundary Rule

1. The runtime kernel bundle is runtime-owned executable composition, not a replacement for product law.
2. Product law remains in canonical specs and root config artifacts.
3. The bundle is the lawful runtime consumption surface that `taskflow` reads directly.

## References

1. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
2. `vida/config/instructions/runtime-instructions/work.agent-lane-selection-protocol.md`
3. `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`
4. `docs/product/spec/canonical-runtime-layer-matrix.md`

-----
artifact_path: config/runtime-instructions/runtime-kernel-bundle.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md
created_at: '2026-03-10T16:00:00+02:00'
updated_at: '2026-03-12T07:58:34+02:00'
changelog_ref: runtime.runtime-kernel-bundle-protocol.changelog.jsonl
