# Framework Protocol Layers Map

Purpose: provide one thin framework-owned routing map for protocol-bearing layers so agents can place an artifact into the correct owner layer without turning this map into a second law-bearing source.

## Scope

This map covers placement for:

1. framework canon,
2. agent-role protocol surfaces,
3. bootstrap/environment protocol surfaces,
4. human-governance protocol surfaces,
5. project documentation surfaces adjacent to framework protocol discovery.

Primary law owner for the layering model:

1. `docs/product/spec/framework-project-documentation-layer-model.md`

This map is routing-only.

## Activation Triggers

Read this map when:

1. a task asks where a protocol-bearing artifact belongs in the layer model,
2. a task audits layer mixing or ownership drift,
3. a task needs one-pass routing before deeper reads,
4. a task asks whether an artifact is framework truth, role-derived, bootstrap/environment, governance, or project-owned.

## Layer Map

### 1. Framework Canon

Canonical homes:

1. `vida/config/**`
2. `vida/config/instructions/**`

Owns:

1. framework behavior law,
2. routing and escalation law,
3. state, receipt, and transition law,
4. framework-owned maps and instruction artifacts.

### 2. Agent Role Layer

Canonical homes:

1. `agent-definitions/**`
2. `instruction-contracts/role.*`

Owns:

1. role-specific contracts,
2. role participation boundaries,
3. worker-entry and worker-role interaction law derived from framework canon.

### 3. Bootstrap / Environment Layer

Canonical homes:

1. `AGENTS.md`
2. `AGENTS.sidecar.md`
3. root maps and bootstrap flow maps
4. `bridge.project-overlay-protocol.md`

Owns:

1. bootstrap carriers,
2. local environment entry/hosting surfaces,
3. shell/runtime integration,
4. environment binding without redefining framework law.

### 4. Human Governance Layer

Canonical homes:

1. `system-maps/governance.map`
2. governance/approval protocols
3. root human-governance docs when they are the canonical owner

Owns:

1. human approval rules,
2. contribution/edit policy,
3. governance exceptions and publication rules.

### 5. Project Documentation Layer

Canonical homes:

1. `docs/product/**`
2. `docs/process/**`
3. `docs/project-memory/**`

Owns:

1. product specs,
2. project process docs,
3. project memory docs,
4. project-facing maps and readiness/alignment views.

## Core Placement Rule

The `core cluster` belongs to `Framework Canon`.

It does not belong to:

1. tooling,
2. bootstrap/environment notes,
3. project process docs,
4. backend-specific lifecycle law.

Use `system-maps/framework.core-protocols-map` when the task is specifically about how the `core cluster` fits together.

## Routing

1. if the artifact defines framework truth:
   - continue to `system-maps/framework.map`
2. if the artifact is specifically about the `core cluster`:
   - continue to `system-maps/framework.core-protocols-map`
3. if the artifact is role-derived:
   - continue to the relevant `agent-definitions/**` or `role.*` owner
4. if the artifact is bootstrap/environment:
   - continue to `AGENTS.md`, `AGENTS.sidecar.md`, or the relevant bootstrap/overlay owner
5. if the artifact is human governance:
   - continue to `system-maps/governance.map`
6. if the artifact is project-owned:
   - hand off to `AGENTS.sidecar.md` and then the project maps under `docs/**`
7. if the layering law itself is in question:
   - continue to `docs/product/spec/framework-project-documentation-layer-model.md`

-----
artifact_path: config/system-maps/framework.protocol-layers-map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/framework.protocol-layers-map.md
created_at: '2026-03-11T15:48:00+02:00'
updated_at: '2026-03-13T23:20:00+02:00'
changelog_ref: framework.protocol-layers-map.changelog.jsonl
