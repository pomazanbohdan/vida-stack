# Orchestrator Boot Flow Map

Purpose: provide the explicit step-by-step boot flow for the orchestrator lane so orchestrator startup does not need to be reconstructed from `AGENTS.md`, lane-entry prose, and activation law by inference.

## Activation Trigger

Read this map when:

1. worker-lane confirmation is absent,
2. the runtime explicitly places the agent in orchestrator lane,
3. bootstrap or compact recovery needs the exact orchestrator boot path,
4. framework work asks for the canonical orchestrator startup flow.

## Step 0. Shared Bootstrap

The orchestrator boot always begins with the mandatory shared bootstrap:

1. `AGENTS.md`
2. `AGENTS.sidecar.md`
3. `vida/root-map.md`

Rules:

1. lane resolution happens only after the shared bootstrap carriers and maps are read,
2. `AGENTS.sidecar.md` remains the project-doc bootstrap map,
3. `vida/root-map.md` remains the framework root map.

## Step 1. Enter The Orchestrator Lane

1. Read `vida/config/instructions/agent-definitions.orchestrator-entry.md`.
2. Apply `vida/config/instructions/instruction-contracts.instruction-activation-protocol.md`.
3. If the task is documentation-shaped, activate `vida/config/instructions/instruction-contracts.documentation-operation-protocol.md` immediately.

## Step 2. Select Boot Profile

Use the orchestrator entry contract to choose one boot profile:

1. `Lean`
2. `Standard`
3. `Full`

Profile selection authority:

1. `vida/config/instructions/agent-definitions.orchestrator-entry.md`

## Step 3. Lean Boot Minimum Read Set

The `Lean` boot profile requires:

1. `AGENTS.md`
2. this map
3. `vida/config/instructions/agent-definitions.orchestrator-entry.md`
4. compact task-state snapshot when the request is development-related
5. `vida/config/instructions/instruction-contracts.thinking-protocol.md` selected sections
6. `vida/config/instructions/runtime-instructions.web-validation-protocol.md`
7. `vida/config/instructions/runtime-instructions.project-overlay-protocol.md`
8. root `vida.config.yaml` when present
9. `vida/config/instructions/instruction-contracts.agent-system-protocol.md` when `protocol_activation.agent_system=true`
10. `vida/config/instructions/runtime-instructions.beads-protocol.md` only when non-`answer_only` flow and the compact snapshot is insufficient
11. `vida/config/instructions/diagnostic-instructions.silent-framework-diagnosis-protocol.md` only when silent diagnosis is enabled

## Step 4. Standard Boot Expansion

`Standard` boot executes `Lean` boot first, then adds only route-triggered surfaces:

1. `vida/config/instructions/runtime-instructions.taskflow-protocol.md` when TaskFlow is required
2. `vida/config/instructions/command-instructions.use-case-packs.md` when a pack path is required
3. `vida/config/instructions/command-instructions.implement-execution-protocol.md` when implementation flow is in scope

## Step 5. Full Boot Expansion

`Full` boot executes `Standard` boot first, then adds:

1. `vida/config/instructions/instruction-contracts.orchestration-protocol.md`
2. `vida/config/instructions/command-instructions.pipelines.md`

Use `Full` only for:

1. architecture/topology refactor,
2. unknown root cause,
3. cross-module integration,
4. security/data-safety decisions,
5. explicit meta-analysis,
6. confidence below `80%` after `Standard`.

## Step 6. Route To Execution Mode

After the boot profile is complete, the orchestrator must classify the request into:

1. `answer_only`
2. `artifact_flow`
3. `execution_flow`
4. `mixed`

Route authority:

1. `vida/config/instructions/agent-definitions.orchestrator-entry.md`
2. `vida/config/instructions/instruction-contracts.orchestration-protocol.md`

## Step 7. Route To Domain Protocols

After intent classification:

1. use `vida/config/instructions/system-maps.protocol-index.md` for domain protocol lookup,
2. use `vida/config/instructions/system-maps.runtime-family-index.md` for runtime-family routing,
3. keep additional reads trigger-bound under `vida/config/instructions/instruction-contracts.instruction-activation-protocol.md`.

## Boundary Rule

1. This map describes orchestrator startup only.
2. It does not replace the orchestrator entry contract.
3. It must remain a thin explicit boot route, not a second full orchestrator manual.

-----
artifact_path: config/system-maps/orchestrator-boot-flow
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps.orchestrator-boot-flow.md
created_at: '2026-03-10T15:05:00+02:00'
updated_at: '2026-03-10T14:41:13+02:00'
changelog_ref: system-maps.orchestrator-boot-flow.changelog.jsonl
