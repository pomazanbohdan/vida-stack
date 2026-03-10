# Runtime Family Map: Codex

Purpose: define the bounded `codex` runtime family surface used for documentation-system operations and documentation-aware repository tooling.

## Runtime Identity

1. runtime family: `codex`
2. root surface: `codex-v0/`
3. current role: bounded documentation/operator runtime surface
4. framework relationship: independently usable runtime family under the unified VIDA framework map

## Canonical Surfaces

1. executable/tool entrypoint:
   - `codex-v0/codex.py`
2. bounded config/policy surfaces:
   - `codex-v0/docsys_policy.yaml`
   - `codex-v0/docsys_project.yaml`
   - `codex-v0/docsys_schema.yaml`
3. governing framework protocol:
   - `vida/config/instructions/instruction-contracts.documentation-operation-protocol.md`

## Activation Triggers

Read this map when:

1. the task is documentation-shaped,
2. documentation operator commands, proof checks, or inventory checks are needed,
3. documentation runtime readiness or documentation tooling ownership must be resolved,
4. a task asks about the `codex` runtime family directly.

## Routing

1. Documentation operation law:
   - continue to `vida/config/instructions/instruction-contracts.documentation-operation-protocol.md`
2. Repository documentation commands:
   - continue to `AGENTS.sidecar.md`
3. Low-call operator/tooling path:
   - continue to `codex-v0/codex.py`

## Boundary Rule

1. `codex` is a runtime/tooling surface, not the owner of framework-wide truth.
2. Documentation law remains in framework and project canonical docs.
3. `codex-v0/**` provides the bounded operational/runtime surface for that law.

-----
artifact_path: config/system-maps/runtime-family.codex
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps.runtime-family-codex.md
created_at: '2026-03-10T08:45:00+02:00'
updated_at: '2026-03-10T08:45:00+02:00'
changelog_ref: system-maps.runtime-family-codex.changelog.jsonl
