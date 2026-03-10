# Worker Boot Flow Map

Purpose: provide the explicit step-by-step boot flow for delegated worker lanes so worker startup remains bounded and distinct from orchestrator boot.

## Activation Trigger

Read this map when:

1. the task packet/runtime confirms worker-lane semantics,
2. a delegated worker needs the exact bounded startup path,
3. framework work asks for the canonical worker boot route.

## Step 0. Shared Bootstrap

The worker boot inherits only the mandatory shared bootstrap:

1. `AGENTS.md`
2. `AGENTS.sidecar.md`
3. `vida/root-map.md`

Rules:

1. shared bootstrap does not authorize repository-wide orchestration behavior,
2. the worker must not continue past this point without explicit worker-lane confirmation.

## Step 1. Confirm Worker Lane

Required packet/runtime markers:

1. `worker_lane_confirmed: true`
2. `worker_role: worker`

If those markers are absent or ambiguous:

1. stop using this map,
2. fall back to `vida/config/instructions/agent-definitions.orchestrator-entry.md`.

## Step 2. Read Worker Entry Surfaces

The canonical worker boot read set is:

1. `vida/config/instructions/agent-definitions.worker-entry.md`
2. `vida/config/instructions/instruction-contracts.worker-thinking.md`

When packet construction or packet validation is active, also read:

1. `vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md`

## Step 3. Apply Bounded Packet Context

The worker may then read only:

1. the task packet,
2. scoped files/directories,
3. explicit verification commands,
4. packet-linked local references,
5. project preflight doc only when the packet requires command execution.

Forbidden by default:

1. broad framework docs,
2. broad repo scanning,
3. repository-wide orchestration policy,
4. log sweeps over `.vida/logs`, `.vida/state`, or `.beads`.

## Step 4. Execute The Blocking Question

The worker must:

1. answer the packet's blocking question directly,
2. stay inside packet scope,
3. obey the packet output contract,
4. stop once the bounded deliverable is produced or an escalation condition is hit.

## Step 5. Escalate Or Return

Escalate only when:

1. required scope is missing,
2. required tool is unavailable,
3. packet instructions conflict,
4. completion would require widened ownership.

Otherwise return the required bounded result with evidence and verification.

## Boundary Rule

1. This map is the worker startup route, not the worker packet itself.
2. It must not inherit the full orchestrator boot/governance stack.
3. It must remain a thin explicit startup map above the worker entry and worker dispatch contracts.

-----
artifact_path: config/system-maps/worker-boot-flow
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-10'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps.worker-boot-flow.md
created_at: '2026-03-10T15:05:00+02:00'
updated_at: '2026-03-10T14:41:13+02:00'
changelog_ref: system-maps.worker-boot-flow.changelog.jsonl
