# Dev Agents Matrix — Generic Routing Classes

> Generic provider classes only. Concrete models/providers are project-owned.

## Provider Classes

| Provider Class | Best Use | Write Mode | Notes |
|---|---|---|---|
| `internal` | Default framework-native implementation lane | ✅ | Runtime-managed inside the current platform |
| `external_cli` | Cheap or specialized CLI-driven execution lane | ✅ with scoped ownership | Must never own workflow state |
| `external_review` | Independent review or validation lane | Read-only by default | Prefer when separation of judgement matters |

## Routing Rules

1. Provider selection must come from active subagent-system state, not from hardcoded framework docs.
2. Project overlay may map task classes to provider order and optional provider-specific model/profile policy, but framework chooses only among generic provider classes.
3. Use strong or promoted providers for architecture/high-risk tasks.
4. Use cheap or review providers only when bounded scope and verification contract are explicit.
5. Cheap/fast model lanes and native role profiles should stay project-owned; framework only carries the generic route contract that can return a selected model/profile for an eligible provider.
6. Read-only task classes may additionally return advisory fanout metadata (`fanout_providers`, `fanout_min_results`, `merge_policy`) for orchestrator-managed ensemble dispatch.

## Delegation Gate

Dispatch only if all are true:

1. Task is atomic and bounded.
2. Success criteria are explicit and testable.
3. Verification command is defined.
4. Failure can be contained without breaking active flow.

## Mandatory Prompt Contract

Every subagent prompt must include:

1. Working directory: current repository root (`<repo_root>` resolved at runtime).
2. Environment prerequisite: `Follow project preflight from docs/process/project-operations.md before analyze/test/build`.
3. Protocol-scoped ownership unit when applicable (`/vida-*#CLx`).
4. Odoo JSON note (if API/JSON touched): `Odoo returns false instead of null for empty fields`.
5. Verification command(s): exact commands and expected outcome.
6. Edit constraints:
   - Read target files before editing.
   - Do not add packages absent in `pubspec.yaml`.
   - Keep changes scoped to requested files.

Reference:

1. `_vida/docs/subagents.md`
2. `_vida/docs/subagent-system-protocol.md`
