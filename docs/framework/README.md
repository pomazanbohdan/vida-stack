# docs/framework/ — Canonical VIDA Runtime Documentation

This directory is the canonical source for the agent runtime framework only.

Use `docs/framework/` for:

1. Boot, routing, reasoning, TODO, pack, and task-state protocols.
2. Runtime topology and agent behavior contracts.
3. Framework-owned helper policies for workers, logs, and protocol scripts.
4. Canonical runtime script architecture and ownership rules.
5. Project-bootstrap and self-reproduction contracts for the framework.
6. Framework product and release-target scope documents.
7. Framework implementation roadmaps for release delivery.

Do not use `docs/framework/` for:

1. Product architecture or feature specifications.
2. Project environments and access notes.
3. Project build, release, or observability runbooks.
4. App-specific commands whose executable entrypoints live in `scripts/`.
5. Historical project research or migration artifacts.

Canonical split:

1. `docs/framework/` -> framework runtime policy and protocol contracts.
2. `docs/` -> current project/domain documentation.
3. `docs/process/` -> canonical project operational runbooks.
4. `scripts/` -> executable project operations referenced by `docs/process/`.

Reasoning docs:

1. Canonical deep spec: `docs/framework/thinking-protocol.md`
2. One-screen reference: `docs/framework/algorithms-one-screen.md`
3. Operational quick reference: `docs/framework/algorithms-quick-reference.md`

Migration policy:

1. New framework/runtime docs belong in `docs/framework/`.
2. New project docs and build/ops runbooks belong in `docs/` or `docs/process/`.
3. New executable project workflows belong in `scripts/`, not `docs/framework/history/_vida-source/scripts/`.
4. Keep `AGENTS.md` references split by ownership instead of pointing everything to `docs/framework/`.
