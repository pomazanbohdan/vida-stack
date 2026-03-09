# _vida/docs/ — Canonical VIDA Runtime Documentation

This directory is the canonical source for the agent runtime framework only.

Use `_vida/docs/` for:

1. Boot, routing, reasoning, TODO, pack, and task-state protocols.
2. Runtime topology and agent behavior contracts.
3. Framework-owned helper policies for subagents, logs, and protocol scripts.
4. Canonical runtime script architecture and ownership rules.
5. Project-bootstrap and self-reproduction contracts for the framework.
6. Framework product and release-target scope documents.
7. Framework implementation roadmaps for release delivery.

Do not use `_vida/docs/` for:

1. Product architecture or feature specifications.
2. Project environments and access notes.
3. Project build, release, or observability runbooks.
4. App-specific commands whose executable entrypoints live in `scripts/`.
5. Historical project research or migration artifacts.

Canonical split:

1. `_vida/docs/` -> framework runtime policy and protocol contracts.
2. `docs/` -> current project/domain documentation.
3. `docs/process/` -> canonical project operational runbooks.
4. `scripts/` -> executable project operations referenced by `docs/process/`.

Reasoning docs:

1. Canonical deep spec: `_vida/docs/thinking-protocol.md`
2. One-screen reference: `_vida/docs/algorithms-one-screen.md`
3. Operational quick reference: `_vida/docs/algorithms-quick-reference.md`

Migration policy:

1. New framework/runtime docs belong in `_vida/docs/`.
2. New project docs and build/ops runbooks belong in `docs/` or `docs/process/`.
3. New executable project workflows belong in `scripts/`, not `_vida/scripts/`.
4. Keep `AGENTS.md` references split by ownership instead of pointing everything to `_vida/docs/`.
