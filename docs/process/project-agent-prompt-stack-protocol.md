# Project Agent Prompt Stack Protocol

Status: active project process doc

Purpose: define the canonical project-side prompt stack model so orchestrator and delegated lanes understand how framework bootstrap, project role posture, packet data, skill activation, and runtime state combine without ambiguity.

## Scope

This protocol defines:

1. the ordered prompt stack for project agent lanes,
2. which layers are mandatory and which are bounded overlays,
3. how packet data and skills narrow the active lane,
4. how to resolve stack conflicts.

This protocol does not define:

1. one specific skill body,
2. one specific packet instance,
3. one specific model provider,
4. framework-owned role law.

## Core Rule

Project agent behavior is produced by a stack, not by one prompt string.

Project rule:

1. no single layer may silently replace the rest of the stack,
2. lower layers may narrow behavior but must not weaken higher-precedence safety and routing rules,
3. the active packet is bounded by the whole stack, not by chat wording alone.

## Canonical Stack

The canonical stack order is:

1. framework bootstrap and lane entry
2. project docs map and project process posture
3. project role-specific static prompt
4. dynamic bounded packet
5. active relevant skill overlay
6. current runtime/task state

## Layer Meanings

### 1. Framework Bootstrap And Lane Entry

Mandatory sources:

1. `AGENTS.md`
2. `vida/root-map.md`
3. lane entry contract chosen by bootstrap

Role:

1. bootstrap routing,
2. lane selection,
3. invariant safety rules,
4. framework protocol activation.

### 2. Project Docs Map And Project Process Posture

Mandatory sources:

1. `AGENTS.sidecar.md`
2. `docs/process/project-orchestrator-operating-protocol.md` for orchestrator lane
3. `docs/process/team-development-and-orchestration-protocol.md` for development lanes

Role:

1. project-local routing,
2. project-specific decomposition depth,
3. project taskflow/bootstrap posture,
4. project launch-readiness rules.

### 3. Project Role-Specific Static Prompt

Mandatory sources when a delegated lane is active:

1. `.codex/agents/junior.toml`
2. `.codex/agents/middle.toml`
3. `.codex/agents/senior.toml`
4. `.codex/agents/architect.toml`
5. activation-time `runtime_role` / `task_class` packet fields

Role:

1. role identity,
2. lane-specific boundaries,
3. role-local bootstrap reminders,
4. failure rules for missing packet data.

### 4. Dynamic Bounded Packet

Mandatory source:

1. one lawful packet shaped according to `docs/process/project-development-packet-template-protocol.md`

Role:

1. one bounded unit of work,
2. one owner,
3. one proof target,
4. one blocking question,
5. one stop boundary.

### 5. Active Relevant Skill Overlay

Mandatory source when applicable:

1. active skill catalog exposed in the session,
2. relevant `SKILL.md` files selected under `docs/process/project-skill-initialization-and-activation-protocol.md`

Role:

1. specialized workflow knowledge,
2. domain-specific constraints,
3. execution shortcuts that do not bypass higher-precedence law.

### 6. Current Runtime/Task State

Mandatory sources when execution state matters:

1. `vida status --json`
2. `vida orchestrator-init --json`
3. bounded `vida taskflow task` views when a specific active unit exists

Role:

1. current active queue and lifecycle truth,
2. bounded current state,
3. proof that the session is attached to the right runtime root.

## Conflict Rule

When stack layers conflict, use this precedence:

1. framework bootstrap and lane entry,
2. project process posture,
3. role-specific static prompt,
4. dynamic packet,
5. skill overlay,
6. chat phrasing or stale recollection.

Interpretation rule:

1. lower layers may narrow scope,
2. lower layers must not override safety, ownership, or fail-closed rules from higher layers.

## Prompt Construction Rule

At dispatch time, the effective prompt must be interpretable as:

1. who this lane is,
2. which bounded packet it owns,
3. which skills are active,
4. which runtime/task state is relevant,
5. which proof target closes the work.

If any of those are missing, the lane is not ready.

## Boot Visibility Rule

After normal bootstrap, the active lane should be able to answer:

1. which role prompt layer is active,
2. which packet layer is active,
3. which skill overlay is active or that `no_applicable_skill` applies,
4. which runtime state confirms the bounded unit,
5. which layer would win if a conflict appears.

## Routing

1. for session startup, read `docs/process/project-orchestrator-session-start-protocol.md`,
2. for top-level routing, read `docs/process/project-orchestrator-operating-protocol.md`,
3. for packet family rules, read `docs/process/project-development-packet-template-protocol.md`,
4. for skill activation, read `docs/process/project-skill-initialization-and-activation-protocol.md`,
5. for Codex role configuration, read `docs/process/codex-agent-configuration-guide.md`.

-----
artifact_path: process/project-agent-prompt-stack-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/project-agent-prompt-stack-protocol.md
created_at: '2026-03-13T21:30:00+02:00'
updated_at: '2026-03-13T21:30:00+02:00'
changelog_ref: project-agent-prompt-stack-protocol.changelog.jsonl
