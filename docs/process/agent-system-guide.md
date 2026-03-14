# VIDA Project Agent System Guide

Status: active project process doc

Purpose: describe project-specific agent operating notes without owning the framework bootstrap path.

## Scope

This file defines only the project-facing agent-system process surface for the active repository:

1. where the project binds into the framework agent-system,
2. where project-owned extension data is configured,
3. which project surfaces remain authoritative for project-specific agent posture.

This file does not define:

1. framework bootstrap routing,
2. framework role or lane law,
3. backend lifecycle mechanics,
4. product-law semantics for role, skill, profile, flow, or lane selection.

## Boundary Rule

1. Framework bootstrap ownership remains in `AGENTS.md`, `AGENTS.sidecar.md`, and `system-maps.*`.
2. This file must not become a second bootstrap router or canonical owner of framework/product discovery paths.
3. Project-specific operating notes may live here only when they do not compete with framework-owned bootstrap routing.

## Canonical Project-Facing Surfaces

The active project agent-system surface is split across:

1. root overlay activation in `vida.config.yaml`,
2. project extension bridge map in `docs/process/agent-extensions/README.md`,
3. active runtime projection family in `.vida/project/agent-extensions/**`,
4. product-law model in `docs/product/spec/agent-role-skill-profile-flow-model.md`,
5. product-law lane-selection model in `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`,
6. framework/runtime validation and activation law in `runtime-instructions/work.project-agent-extension-protocol.md`,
7. framework generic routing law in `instruction-contracts/core.agent-system-protocol.md`.

## Current Project Posture

For the active repository, project-specific agent behavior is expected to be:

1. overlay-driven rather than shell-habit driven,
2. registry-backed rather than prose-defined,
3. validated through framework/runtime activation before use,
4. fail-closed on unresolved project extension wiring.

## Project Operating Rule

1. project-specific roles, skills, profiles, and flows belong in the project registries, not in this prose document,
2. project-specific activation choices belong in `vida.config.yaml`,
3. this file may record the project process boundary and routing posture for the agent system, but it must not duplicate framework law already owned elsewhere,
4. when a project note becomes stable product law, promote it into `docs/product/spec/**` instead of expanding this file into a second product-law home.

## Routing

1. for project extension bridge registries and ids, read `docs/process/agent-extensions/README.md`,
2. for the active runtime-owned role/skill/profile/flow projections, inspect `.vida/project/agent-extensions/**`,
3. for canonical role/skill/profile/flow semantics, read `docs/product/spec/agent-role-skill-profile-flow-model.md`,
4. for auto lane selection and conversational-mode law, read `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`,
5. for runtime activation and validation of project extensions, read `runtime-instructions/work.project-agent-extension-protocol.md`,
6. for generic framework agent-system routing and backend-class law, read `instruction-contracts/core.agent-system-protocol.md`.

-----
artifact_path: process/agent-system-guide
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/process/agent-system-guide.md
created_at: '2026-03-10T04:40:00+02:00'
updated_at: '2026-03-13T11:20:00+02:00'
changelog_ref: agent-system-guide.changelog.jsonl
