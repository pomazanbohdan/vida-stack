# Subagent Dispatch Protocol

Use this protocol for every subagent dispatch.

## Routing Boundary

This file defines dispatch invariants only.

Concrete provider/model choices are not hardcoded here.

Use:

1. `_vida/docs/subagent-system-protocol.md` for system-level activation, routing, fallback, and scoring.
2. `_vida/docs/DEV-AGENTS-MATRIX.md` for generic provider classes and routing categories.
3. project overlay (`vida.config.yaml` + project docs) for concrete providers/models enabled in the current repository.

## Mandatory Prompt Fields

0. Worker entry contract: external/delegated providers must receive `_vida/docs/SUBAGENT-ENTRY.MD` semantics instead of inheriting `AGENTS.md` orchestrator identity.
0.1. Worker thinking contract: external/delegated providers must receive `_vida/docs/SUBAGENT-THINKING.MD` semantics and stay inside `STC|PR-CoT|MAR` unless explicitly escalated by the packet.
1. Environment prerequisite: `Follow the active project preflight and command order declared by the host-project overlay.`
2. Working directory: current repository root (`<repo_root>` resolved at runtime).
3. Protocol unit when applicable: `<command>#CLx` plus whether the unit is read-only or mutation-owning.
4. Project-specific data/API quirks belong in the host-project overlay or task packet, not in framework dispatch policy.
5. Verification command: explicit command that proves success (for example, a project analyzer/test command exits 0).
6. Code-modification constraints:
   - Read the target file first before editing.
   - Do not add dependencies absent from the host project's canonical dependency manifest.
   - Keep host-project serialization/data quirks inside the host overlay or task packet.

## Mandatory Dispatch Gate

Before dispatch:

1. Define bounded scope (files/directories).
2. Name the protocol-scoped ownership unit when the work comes from command decomposition (`/vida-*#CLx`).
3. Define explicit verification command.
4. Define expected deliverable format.
5. Confirm dependency prerequisites are in prompt.
6. Prefer `bash _vida/scripts/render-subagent-prompt.sh ...` to render the baseline prompt with `<repo_root>`, worker entry contract, protocol-unit hint, and project preflight already filled in.
7. If project overlay activates the subagent system, consult the active routing snapshot before choosing provider class.
8. If routing metadata includes `fanout_providers`, dispatch only those providers for read-only work, require at least `fanout_min_results`, and merge results via the declared `merge_policy`.

## Mandatory Return Contract

For code or docs tasks, the subagent result is valid only if it includes a machine-readable delivery summary.

Required fields:

1. `status` — `done|partial|blocked`
2. `changed_files` — explicit path list (empty list allowed only for read-only tasks)
3. `verification_commands` — every command the subagent actually ran
4. `verification_results` — pass/fail plus short result per command
5. `merge_ready` — `yes|no`
6. `blockers` — empty list or concrete blocker list
7. `notes` — optional concise integration notes

Preferred format:

```json
{
  "status": "done",
  "changed_files": ["path/a", "path/b"],
  "verification_commands": ["project verification command"],
  "verification_results": ["project verification command -> pass"],
  "merge_ready": "yes",
  "blockers": [],
  "notes": "short note"
}
```

Text-only summaries without `changed_files` and verification evidence are invalid for write tasks.

## Orchestrator Validation After Return

1. Confirm output followed the active project preflight declared by the host-project overlay.
2. Confirm verification command was actually executed (not only claimed).
3. If analyzer errors are reported, classify environment/toolchain vs real code errors.
4. Confirm the return contract is present before accepting or merging results.

## Failure Handling

1. Read full error output.
2. Classify root cause: environment/toolchain or code.
3. If environment/toolchain: re-dispatch with corrected prerequisites, without code edits.
4. If code: run systematic-debugging Phase 1 before implementing fixes.
5. If the provider fails repeatedly, record provider failure in the subagent-system scorecard and re-route through the next eligible provider.

## Prompt Templates

Use ready templates from:

1. `_vida/docs/subagent-prompt-templates.md`
2. `bash _vida/scripts/render-subagent-prompt.sh <audit|implementation|decision|patch> ...`
3. `_vida/docs/SUBAGENT-ENTRY.MD` for the worker-lane entry contract
4. `_vida/docs/SUBAGENT-THINKING.MD` for the worker-lane thinking subset

Protocol-unit routing rule:

1. Delegate `CL1`, `CL2`, and read-heavy `CL3` units freely when scope is bounded.
2. Keep `CL4` in the orchestrator lane unless isolated write scope is explicitly granted.
3. Treat `CL5` as orchestrator-owned for final gate decisions, even when evidence gathering is delegated.

## Role Boundary

1. `AGENTS.md` is for the orchestrator only.
2. External providers and delegated workers should follow `_vida/docs/SUBAGENT-ENTRY.MD` as their entry contract.
3. External providers and delegated workers should use `_vida/docs/SUBAGENT-THINKING.MD` as their default reasoning subset.
4. Do not proxy the full orchestrator boot/governance layer into external worker prompts unless the task explicitly audits that framework layer.
