# Environments

Purpose: keep the current project process environment assumptions compact,
portable, and aligned with config-driven runtime ownership.

This file describes environment boundaries for operators and agents. It does
not own product runtime law, host-carrier selection, or release packaging rules.

## Current Environment Assumptions

1. Project root is the Git checkout containing `vida.config.yaml`.
   - Do not hardcode an absolute developer-machine path in this document.
2. Runtime directories are managed under `.vida/`.
3. Default long-lived authoritative local state root:
   - `.vida/data/state/`
4. Disposable proof roots may use:
   - `VIDA_STATE_DIR=<temp-dir>`
5. Generated files under `.vida/data/state/**` are operational runtime
   artifacts, not product-doc inputs.
6. Host CLI system selection is config-derived:
   - `vida.config.yaml -> host_environment.cli_system`
7. Canonical agent/carrier registries are config-derived:
   - `vida.config.yaml -> agent_system`
   - `docs/process/agent-extensions/**`
8. Route policy should use explicit executor/backend fields and treat legacy
   subagent hints as compatibility-only.
9. Hybrid mode means internal and external executors remain admissible only when
   policy selects them.
10. Internal backends remain internal-only even when hybrid mode is active.
11. If sandbox or network posture blocks an external CLI, runtime/status
    surfaces should report a fail-closed preflight blocker with next actions.

## State Hygiene

1. Use `.vida/data/state/` for normal project-local continuity.
2. Use `VIDA_STATE_DIR=<temp-dir>` for isolated proof when long-lived local
   state must not influence the result.
3. Do not manually delete backing-store files as routine cleanup.
4. If long-lived state appears broken, classify it as a runtime/state recovery
   problem and use the documented recovery path or a fresh temp-state proof.
5. Keep generated runtime state out of commits unless an explicitly documented
   fixture or product-owned artifact path requires it.

Detailed state policy lives in
`docs/product/spec/ops-state-runtime-evidence-hygiene-contract.md`.

## Config Portability Rule

Environment-specific values belong in config, environment variables, installer
state, or runtime state, not in hardcoded process prose. This matches the
project's config-driven host-system contract and the broader Twelve-Factor
config principle that deploy-specific configuration should stay outside code.

## Related Owners

1. Host system and carrier selection:
   - `docs/product/spec/config-driven-host-system-runtime-contract.md`
   - `docs/process/agent-system.md`
2. State-root hygiene:
   - `docs/product/spec/ops-state-runtime-evidence-hygiene-contract.md`
   - `docs/process/project-operations.md`
3. Proof ladder and timing:
   - `docs/process/command-timing-and-gate-optimization-protocol.md`
4. Project bootstrap paths:
   - `vida.config.yaml -> project_bootstrap`

-----
artifact_path: process/environments
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-06-13'
schema_version: '1'
status: canonical
source_path: docs/process/environments.md
created_at: '2026-04-04T20:24:09+03:00'
updated_at: 2026-06-13T02:05:00+03:00
changelog_ref: environments.changelog.jsonl
