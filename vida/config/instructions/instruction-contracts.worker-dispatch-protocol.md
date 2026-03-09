# Worker Dispatch Protocol

Purpose: define dispatch invariants for every delegated worker packet.

This file is the canonical worker-dispatch protocol.

## Routing Boundary

This file defines dispatch invariants only.

Concrete backend/model choices are not hardcoded here.

Use:

1. `vida/config/instructions/instruction-contracts.agent-system-protocol.md` for system-level activation, routing, fallback, and scoring.
2. `vida/config/instructions/agent-backends.matrix.md` for generic backend classes and routing categories.
3. project overlay (`vida.config.yaml` + project docs) for concrete backends/models enabled in the current repository.

## Mandatory Packet Fields

0. Worker entry contract: external/delegated workers must receive `vida/config/instructions/agent-definitions.worker-entry.md` semantics instead of inheriting `AGENTS.md` orchestrator identity.
0.1. Worker thinking contract: external/delegated workers must receive `vida/config/instructions/instruction-contracts.worker-thinking.md` semantics and stay inside `STC|PR-CoT|MAR` unless explicitly escalated by the packet.
0.2. Worker-lane confirmation must be explicit in the rendered packet so the worker does not have to infer role from repository context.
1. Environment prerequisite: `Follow the active project preflight and command order declared by the host-project overlay.`
2. Working directory: current repository root (`<repo_root>` resolved at runtime).
3. Protocol unit when applicable: `<command>#CLx` plus whether the unit is read-only or mutation-owning.
4. Project-specific data/API quirks belong in the host-project overlay or task packet, not in framework dispatch policy.
5. Verification command: explicit command that proves success.
6. Code-modification constraints:
   - Read the target file first before editing.
   - Do not add dependencies absent from the host project's canonical dependency manifest.
   - Keep host-project serialization/data quirks inside the host overlay or task packet.

## Mandatory Dispatch Gate

Before dispatch:

1. Define bounded scope.
2. Name the protocol-scoped ownership unit when the work comes from command decomposition.
3. Define explicit verification command.
4. Define expected deliverable format.
5. Confirm dependency prerequisites are in the packet.
6. Prefer the canonical packet shape from `vida/config/instructions/prompt-templates.worker-packet-templates.md`.
7. If project overlay activates the agent system, consult the active routing snapshot before choosing backend class.
8. If routing metadata includes `fanout_workers`, dispatch only those backends for read-only work, require at least `fanout_min_results`, and merge results via the declared `merge_policy`.
9. If routing metadata marks `independent_verification_required=yes`, use `verification_plan` to select an independent verifier before orchestrator synthesis.
10. Include one explicit blocking question in the packet and require the worker to answer it directly.
11. Include worker-lane confirmation markers:
   - `worker_lane_confirmed: true`
   - `worker_role: worker`
12. Include impact-tail policy in the packet when non-STC workers must finish with bounded downstream analysis.

## Mandatory Return Contract

For code or docs tasks, the worker result is valid only if it includes a machine-readable delivery summary.

Required fields:

1. `status`
2. `question_answered`
3. `answer`
4. `evidence_refs`
5. `changed_files`
6. `verification_commands`
7. `verification_results`
8. `merge_ready`
9. `blockers`
10. `notes`
11. `recommended_next_action`
12. `impact_analysis`

## Role Boundary

1. `AGENTS.md` is for the orchestrator only.
2. Delegated workers should follow `vida/config/instructions/agent-definitions.worker-entry.md` as their entry contract.
3. Delegated workers should use `vida/config/instructions/instruction-contracts.worker-thinking.md` as their default reasoning subset.
4. Do not proxy the full orchestrator boot/governance layer into worker packets unless the task explicitly audits that framework layer.
5. Worker packets must identify worker-lane semantics explicitly instead of relying on repository-global instruction inheritance.

-----
artifact_path: config/instructions/instruction-contracts/worker.dispatch
artifact_type: instruction_contract
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/instruction-contracts.worker-dispatch-protocol.md
created_at: 2026-03-09T22:51:59+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: instruction-contracts.worker-dispatch-protocol.changelog.jsonl
