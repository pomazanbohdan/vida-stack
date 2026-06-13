# Decisions

Purpose: keep a compact project process decision index for bootstrap-visible
project choices that are not already better owned by a product spec, runtime
instruction, or executable config.

This file is not a release log and not a replacement for Architecture Decision
Records. Significant architecture decisions should live in the owning product
spec or linked ADR-style design artifact. This process file records only the
current project bootstrap decisions needed by the runtime and operators.

## Current Project Decisions

1. Project identity is config-owned:
   - source: `vida.config.yaml -> project.id`
   - current value: `vida-stack`
2. Project bootstrap document paths are config-owned:
   - source: `vida.config.yaml -> project_bootstrap`
   - decisions doc: `docs/process/decisions.md`
   - environments doc: `docs/process/environments.md`
   - process root: `docs/process`
3. Host CLI system selection is config-owned:
   - source: `vida.config.yaml -> host_environment.cli_system`
   - current value: `codex`
4. Active host/carrier behavior is config- and runtime-derived:
   - source: `vida.config.yaml`, `docs/process/agent-system.md`,
     `docs/product/spec/config-driven-host-system-runtime-contract.md`
   - do not hardcode model, carrier, or host-system authority in this file.
5. Language policy is config-owned:
   - user communication: `uk`
   - reasoning: `en`
   - documentation: `en`
   - todo protocol: `en`
6. Release-specific facts are provenance, not current process law:
   - use release notes, Git tags, installer proof, and
     `docs/process/codex-agent-configuration-guide.md` for release evidence.

## Decision Retention Rule

Keep only decisions that are:

1. current,
2. needed during bootstrap or process routing,
3. not already fully owned by `vida.config.yaml`, a product spec, or a runtime
   instruction.

Remove or replace entries when they become stale host facts, old release notes,
or one-off troubleshooting evidence.

External documentation practice also supports this compression: ADR guidance
keeps significant decisions tied to context and consequences, while minor or
environment-specific facts should remain in the owning config or runbook.

-----
artifact_path: process/decisions
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-06-13'
schema_version: '1'
status: canonical
source_path: docs/process/decisions.md
created_at: '2026-04-04T20:24:09+03:00'
updated_at: 2026-06-13T02:05:00+03:00
changelog_ref: decisions.changelog.jsonl
