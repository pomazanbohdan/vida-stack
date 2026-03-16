# Bootstrap Router Guide

Purpose: provide a framework-owned bootstrap-router read surface so runtime/help/package delivery can expose bootstrap law through canonical framework discovery without treating the root `AGENTS.md` carrier as a raw filesystem-only entrypoint.

## Core Contract

1. Root `AGENTS.md` remains the L0 bootstrap carrier and stronger live bootstrap surface.
2. This guide is a framework-owned discovery/read surface for bootstrap routing and framework-map routing.
3. This guide must not become a second competing root bootstrap carrier.
4. When bootstrap behavior and this guide disagree, obey root `AGENTS.md` and repair this guide in the same change.

## Bootstrap Identity

1. framework bootstrap carrier:
   - root `AGENTS.md`
2. project docs bootstrap carrier:
   - root `AGENTS.sidecar.md`
3. framework bootstrap routing is command-first:
   - `vida orchestrator-init`
   - `vida agent-init`
   - `vida project-activator`

Two-map initialization rule:

1. after `AGENTS.md`, bootstrap executes the bounded runtime routing command for the active lane or activation state,
2. `AGENTS.sidecar.md` remains the project-document map,
3. framework-owned discovery should use `vida ...-init` and bounded framework canonical ids interpreted through `vida protocol view` rather than raw framework Markdown as the primary path in initialized downstream projects.

## Runtime Routing

Canonical runtime init targets:

1. `vida orchestrator-init`
2. `vida agent-init`
3. `vida project-activator` when onboarding/activation is still pending

Execution-carrier model:

1. `agent` in host routing means execution carrier tier/model with cost and telemetry-backed effectiveness.
2. runtime role is a separate activation state and is not replaced by carrier identity.
3. runtime may bind any admissible carrier to any runtime role when role/task-class constraints allow it.
4. carrier selection is capability/admissibility first, then local score/telemetry guard, then cheapest eligible carrier.

Bounded protocol inspection surfaces:

1. `bootstrap/router`
2. `agent-definitions/entry.orchestrator-entry`
3. `agent-definitions/entry.worker-entry`
4. `instruction-contracts/role.worker-thinking`

Reference grammar:

1. in framework routing prose, a backticked canonical id means the bounded inspection target for `<canonical_id>`,
2. keep the full command form only in runnable shell examples or explicit operator commands,
3. do not use `.md` suffixes in ordinary framework routing prose.

Carrier split:

1. root `AGENTS.md` is the stronger live bootstrap carrier,
2. this guide is the synchronized framework-owned bootstrap-router read surface,
3. packaged/generated bootstrap carriers are delivery surfaces only and must not become a second owner layer,
4. packaged delivery uses the dedicated generated root bootstrap carrier under `install/assets/AGENTS.scaffold.md`,
5. when these carriers diverge, repair them in the same change.

## Optimization Posture

1. use command-first runtime bootstrap surfaces first,
2. use compact runtime capsules and bounded shorthand-target inspection when they settle the startup question,
3. use owner-layer Markdown only on demand for edge cases, ambiguity, or law mutation,
4. do not bulk-read framework protocol stacks merely because they exist.

## Discovery Routing

1. for main-lane startup routing:
   - `vida orchestrator-init`
2. for bounded worker-lane startup routing:
   - `vida agent-init`
3. for pending onboarding or activation posture:
   - `vida project-activator`
4. while activation is pending:
   - collect the bounded activation interview inputs first (`project id`, language policy, supported host CLI system),
   - fail closed on partial activation submissions; do not materialize host templates or write activation state from only one interview field,
   - prefer `vida docflow` for documentation/readiness inspection,
   - do not enter `vida taskflow` or any non-canonical external TaskFlow runtime
5. for framework bootstrap inspection:
   - `bootstrap/router`
5. for exact orchestrator or worker protocol inspection:
   - `agent-definitions/entry.orchestrator-entry`
   - `agent-definitions/entry.worker-entry`
   - `instruction-contracts/role.worker-thinking`

## Boundary Rule

1. this guide is for framework discovery/readability and synchronized bootstrap routing,
2. it does not replace root bootstrap-carrier generation during `vida init`,
3. it does not move project-doc routing into framework-owned instruction homes,
4. it makes command-first routing explicit and keeps bounded shorthand-target inspection through `vida protocol view` rather than as the primary bootstrap execution path.
5. pending project activation is a bounded onboarding/configuration slice, not tracked execution.

-----
artifact_path: config/system-maps/bootstrap.router-guide
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/bootstrap.router-guide.md
created_at: '2026-03-14T01:20:00+02:00'
updated_at: '2026-03-14T09:00:57+02:00'
changelog_ref: bootstrap.router-guide.changelog.jsonl
