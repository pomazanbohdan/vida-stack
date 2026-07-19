# Architecture Decision: TeamFlow State-Machine Authority Boundary

## ID

`ADR-team-flow-state-machine-owner-20260701`

## Status

ACCEPTED — migration is fail-closed until every consumer uses the shared authority.

## Date

2026-07-19

## Context

TeamFlow behavior was previously reconstructed from runtime literals, partial dispatch projections, legacy lane templates, and independently interpreted role sequences. That distributed authority allowed the configured flow, the persisted TaskFlow state, and the operator projection to disagree about the next edge, approval pause, rework target, terminal state, command, or evidence gate.

The runtime also lacked deterministic identities covering the selected `dev_team` config and every registry that can affect dispatch. A receipt could therefore name a flow while omitting the exact packs or commands used to compile it.

## Decision

TeamFlow uses one authority chain:

1. The master template at `docs/framework/templates/vida.config.yaml.template -> dev_team.authority_catalog` enumerates the supported options and capability/admissibility matrix.
2. `vida/config/schemas/team-flow-authority.schema.json` defines the machine-valid authority selection and resolved lane projection.
3. Project `vida.config.yaml -> dev_team.authority_selection` selects only options declared by the master catalog and supplies project flow/role definitions.
4. The compiled agent-extension bundle resolves registry-backed commands and emits deterministic config/registry identities.
5. The TaskFlow authority owner validates explicit edges, approvals, rework/resume, and config-declared terminal transitions.
6. Runtime dispatch and status surfaces consume that authority; they do not infer or redefine it.

The template and schema are co-versioned canonical surfaces. This ADR explains their ownership boundary and must not repeat their option lists.

## Ownership Boundary

### TaskFlow authority owner

The shared TaskFlow authority owns transition validation and state changes. It must:

- validate the requested next node against an explicit configured edge;
- distinguish approval pending, approval accepted, rework requested, and rejection outcomes;
- accept terminal closure only when the selected config explicitly marks it terminal;
- reject ambiguous aliases and conflicting edge definitions;
- persist one transition outcome before downstream projection proceeds.

No consumer may infer terminal state from a literal lane id, position, role name, or an empty next-node value.

### Projection/config owner

The compiled bundle and development-flow projection are read-only derived surfaces. They must:

- bind roles, skills, profiles, flows, packs, commands, and dispatch aliases before computing authority identity;
- canonicalize mapping keys and registry row order before hashing;
- expose separate team/profile authority and selected model-profile fields;
- resolve `command_ref` through the command registry;
- emit complete typed lane fields for inclusion, requirement, evidence, proof, command mapping, rework, and terminal state;
- return a typed blocker when required or malformed authority data is encountered.

They must not synthesize legacy lane shapes or silently substitute default roles, models, reasoning settings, team profiles, commands, approval results, edges, or terminal nodes.

### Runtime consumers

Routing, resume, dispatch, status, and receipt consumers may use provider-neutral routing/status enums. They must not embed concrete agent, model, reasoning, runtime-role, team-profile, command, next-node, or terminal literals as authority.

## Identity Contract

Each registry and the selected `dev_team` config receives a content identity over canonical JSON. The aggregate TeamFlow authority identity covers the config identity and all registry identities, including packs and commands. Ordered arrays remain ordered because step order is semantic; registry rows are sorted by their declared identifier before hashing because registry source order is not semantic.

Receipts and projections may carry the aggregate identity and component identities. A mismatch is a stale-authority blocker, not a compatibility fallback.

## Migration

1. Materialize the catalog, schema, project selection, registry identities, and complete typed projection.
2. Move transition validation to the shared TaskFlow authority owner.
3. Adapt each routing, dispatch, resume, state, and status consumer to the shared authority.
4. Remove legacy synthesized lane templates and hardcoded transition/terminal fallbacks.
5. Add malformed, missing, conditional, approval, terminal, rework, evidence, alias-conflict, and identity-drift proof.
6. Enable closure only after all consumers pass the same authority identity through their receipts.

During migration, missing or inconsistent authority blocks execution. Compatibility code may parse an old input only to return a typed migration blocker; it may not create an executable fallback shape.

## Consequences

- Config changes can alter TeamFlow without code changes when they remain within the declared catalog/schema.
- Project config stays concise: it selects and overrides declared options rather than documenting the option universe.
- Adding a new option requires a template/schema revision before a project can select it.
- Deterministic identities make config/registry drift observable in receipts and tests.
- Runtime consumers become simpler because transition semantics remain in one owner.

## Verification

Static and executable proof must cover:

- master-template and project-selection schema validity;
- complete lane projection fields;
- missing and malformed authority failure;
- conditional inclusion and required evidence;
- explicit approval outcomes;
- config-only terminal admission;
- explicit rework/resume targets;
- duplicate or shadowing alias rejection;
- packs/commands participation in deterministic authority identity;
- absence of concrete authority literals in runtime consumers.

-----
artifact_path: product/spec/adr-team-flow-state-machine-owner
artifact_type: architecture_decision
artifact_version: '1'
artifact_revision: '2026-07-19'
schema_version: '1'
status: accepted
source_path: docs/product/spec/adr-team-flow-state-machine-owner.md
created_at: '2026-07-01T00:00:00+03:00'
updated_at: '2026-07-19T00:00:00+03:00'
changelog_ref: adr-team-flow-state-machine-owner.changelog.jsonl
