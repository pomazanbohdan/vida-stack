# Agent Backend Lifecycle Protocol

Purpose: define the canonical lifecycle for adding, probing, promoting, degrading, cooling down, and recovering external CLI backends in VIDA.

This file is the canonical agent-backend lifecycle protocol.

## Scope

This protocol applies to CLI backend onboarding inside:

1. `vida.config.yaml`
2. `docs/framework/templates/vida.config.yaml.template`
3. `taskflow-v0 system ...`
4. `taskflow-v0 prepare-execution ...`
5. `taskflow-v0 registry ...`

## Lifecycle

Canonical lifecycle:

1. `declared`
2. `detected`
3. `probed`
4. `probation`
5. `promoted`
6. `degraded`
7. `cooldown`
8. `recovered`
9. `retired`

## Minimum Onboarding Contract

When adding a new CLI backend:

1. declare the backend in `vida.config.yaml`,
2. mirror the backend in `docs/framework/templates/vida.config.yaml.template`,
3. declare realistic runtime limits,
4. declare dispatch fields that actually work in headless mode,
5. add probe settings when supported,
6. validate config before use,
7. run backend probe before promoting it into critical fanout.

## Lane Fitness

CLI backends are not universally good or bad.

Lane fitness should be evaluated independently for:

1. `analysis`
2. `review`
3. `meta_analysis`
4. `verification`
5. `implementation` when applicable

-----
artifact_path: config/instructions/agent-backends/lifecycle.protocol
artifact_type: agent_backend_instruction
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/agent-backends.lifecycle-protocol.md
created_at: '2026-03-09T22:51:59+02:00'
updated_at: '2026-03-10T03:06:28+02:00'
changelog_ref: agent-backends.lifecycle-protocol.changelog.jsonl
