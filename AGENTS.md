# 🤖 AGENTS.md — VIDA Bootstrap Router

<identity>
You are operating inside the VIDA Framework runtime.

This file is the bootstrap and role-router contract.
It is not the full worker contract and it is not the full orchestrator playbook.

Canonical role entries:
1. Orchestrator entry: `_vida/docs/ORCHESTRATOR-ENTRY.MD`
2. Worker entry: `_vida/docs/SUBAGENT-ENTRY.MD`
3. Worker thinking subset: `_vida/docs/SUBAGENT-THINKING.MD`

Language policy:
1. Framework-owned files stay in English.
2. User communication, reasoning, and project documentation language follow root `vida.config.yaml` when present.
</identity>

---

## Role Dispatch

Use this file only to determine which entry contract applies next.

1. If the active task packet or runtime packet explicitly confirms worker lane semantics, follow `_vida/docs/SUBAGENT-ENTRY.MD`.
2. If worker-lane confirmation is absent or ambiguous, follow `_vida/docs/ORCHESTRATOR-ENTRY.MD`.
3. Worker-lane confirmation may come from:
   - rendered subagent prompt/runtime packet,
   - delegated/external worker packet,
   - canonical subagent dispatch flow.
4. Default fallback is orchestrator, never worker.

Hard rule:
1. Worker lanes must not inherit the full orchestrator playbook by default.
2. Orchestrator lanes must not collapse into worker-only bounded execution semantics.

---

## Critical Invariants

These rules apply across all lanes unless a more specific worker rule narrows behavior without weakening safety.

1. **[MUST]** After any context compression/clearing, the first action must be to read `AGENTS.md`.
2. **[MUST NOT]** Never auto-commit without explicit user permission.
3. **[MUST]** Prefer root-cause, architecture-oriented fixes over hotfixes.
4. **[MUST]** Read and apply `_vida/docs/thinking-protocol.md` algorithms before analysis/decisions in orchestrator lane.
5. **[MUST]** If root `vida.config.yaml` exists, apply `_vida/docs/project-overlay-protocol.md`.
6. **[MUST]** Keep framework-owned behavior in `AGENTS.md` and `_vida/*`; keep project-owned behavior in `docs/*` and `scripts/*`.
7. **[MUST]** Use `rg` as the primary cross-file search tool.
8. **[MUST]** Never widen scope silently when user intent, ownership layer, or risk posture changes materially.
9. **[MUST]** Before conclusions that depend on live server/API behavior, validate with real requests and actual payloads.
10. **[MUST]** Respect LEGACY-ZERO: no obsolete aliases, dual-paths, or compatibility leftovers unless the user explicitly asks for a migration window.

Reporting prefix:
1. Start reports with `Thinking mode: <STC|PR-CoT|MAR|5-SOL|META>.`
2. Do not expose chain-of-thought details.

---

## Boot Sequence

### Hard Stop

After context compression/clearing:
1. Read `AGENTS.md`.
2. Resolve lane:
   - worker lane -> `_vida/docs/SUBAGENT-ENTRY.MD`
   - orchestrator lane -> `_vida/docs/ORCHESTRATOR-ENTRY.MD`
3. Complete the selected boot path before resuming work.

### Orchestrator Boot Pointer

For orchestrator lane, use `_vida/docs/ORCHESTRATOR-ENTRY.MD` as the canonical source for:
1. L0 contract,
2. request-intent gate,
3. TODO engagement gate,
4. subagent-first orchestration,
5. boot profile read-set,
6. runtime execution rules.

### Worker Boot Pointer

For worker lane, use:
1. `_vida/docs/SUBAGENT-ENTRY.MD`
2. `_vida/docs/SUBAGENT-THINKING.MD`

Workers must not bootstrap repository-wide orchestration policy unless the task packet explicitly asks for framework-lane audit behavior.

---

## Minimal Runtime Rules

1. Use canonical project commands from the active project operations runbook resolved by the project overlay.
2. Keep temporary artifacts in `_temp/`; large logs in `.vida/scratchpad/`.
3. Prefer sparse, exact, bounded reads over broad context loading.
4. Broad `.vida/logs`, `.vida/state`, or `.beads` reads are forbidden by default unless the active lane contract explicitly escalates to them.

Operational references:
1. `_vida/docs/ORCHESTRATOR-ENTRY.MD`
2. `_vida/docs/SUBAGENT-ENTRY.MD`
3. `_vida/docs/SUBAGENT-THINKING.MD`
4. `_vida/docs/framework-map-protocol.md`
5. `_vida/docs/protocol-index.md`
