# Transition Rules (Slim)

Command transitions are protocol-driven, not menu-driven.

Canonical routing:

1. `_vida/docs/use-case-packs.md`
2. `_vida/docs/orchestration-protocol.md`
3. `_vida/docs/protocol-index.md`

Standard transition chain:

1. `/vida-research` -> `/vida-spec`
2. `/vida-spec` -> `/vida-form-task`
3. `/vida-form-task` -> `/vida-implement` (only after explicit launch confirmation)

State gates:

1. Track transitions via TODO blocks (`block-plan`/`block-start`/`block-end`).
2. Use `verify` + health-check before close/handoff.
