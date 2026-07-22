# Development Flow Catalog Schema Contract

Status: active product contract

## Summary

TeamFlow options are declared once in the master `vida.config.yaml` template, selected by project config, validated by the TeamFlow authority schema, and projected as complete typed lanes. Runtime code consumes the projection and must not reconstruct missing authority.

## Canonical Sources

The authority order is:

1. `docs/framework/templates/vida.config.yaml.template -> dev_team.authority_catalog` is the exhaustive human-readable option and capability/admissibility catalog.
2. `vida/config/schemas/team-flow-authority.schema.json` is the machine validation contract for selections and resolved projections.
3. Project `vida.config.yaml -> dev_team.authority_selection` chooses declared options and `dev_team.roles` / `dev_team.flows` provide project instances.
4. Agent-extension registries provide referenced roles, profiles, flows, packs, commands, and dispatch aliases.
5. The compiled bundle is a deterministic derived projection, never a second authoring surface.

The persisted `team_flow_authority.resolved_all_flow_payload` is the schema-complete
authority source consumed after compilation. The payload is hashed with
`canonical_json_blake3_v1`; `team_flow_authority.resolved_all_flow_payload_blake3`
and `team_flow_authority.authority_source.payload_blake3` must match before reload or
dispatch. A current implementation example contains 13 resolved flows and 51
resolved lanes; the schema remains extensible for project-declared counts.

Other ADRs and process docs explain use and ownership only. They must not enumerate an independent option catalog.

## Configuration Rules

Project config may:

- select an option defined by the master catalog;
- define or override a project role or flow instance using schema-supported fields;
- bind a work-item classification to a configured flow;
- reference a command, profile, pack, or alias declared in its configured registry.

Project config must not:

- introduce an undeclared projection mode or transition policy;
- treat a selected model profile as the team role/profile authority;
- encode terminal behavior only through a conventional id or sequence position;
- rely on a command string when a registry `command_ref` is required;
- resolve duplicate or shadowing aliases by order.

Unknown selections, missing refs, malformed conditions, ambiguous aliases, and incomplete terminal/rework/approval definitions are validation blockers.

## Field-Source and Exact-One Rules

The master catalog is the only exhaustive documentation owner for supported values, source paths, and variants. A project flow step selects one `dispatch_alias`, task class, command ref, inclusion/proof/approval/lifecycle contract, and explicit transitions. Its referenced team role supplies runtime role, packet template kind, closure class, stage, completion blocker, and carrier policy. Dispatch-alias, command, and profile registries supply their declared rows and identities. The carrier catalog supplies the selected carrier/model-profile relation. The selected host system supplies the executor-backend relation.

Every authority-affecting field has exactly one source. Every alias, command, profile, and model-profile ref resolves to exactly one effective row. The resolved alias must admit the selected runtime role and task class. Empty refs, duplicate/shadowed rows, conflicting step/role values, or an unlisted architect context fail closed. Architect aliases are selected by flow context from the master catalog: evidence-triggered architecture is escalation; planned runtime-remediation and architecture-design work is execution preparation.

Carrier relation and executor-backend relation are separate typed fields. A carrier identifies the admissible agent/profile/model catalog entry; an executor backend identifies the configured host execution mechanism. Matching names, defaults, or implementation convenience never permit one relation to fill the other.

The relation shapes are strict and non-substitutable:

- `carrier_relation` is exactly `{relation_kind, source_path, selected_id}` with `relation_kind: carrier_catalog`.
- `executor_backend_relation` is exactly `{relation_kind, source_path, selected_id, backend_class, required_backend_class}` with `relation_kind: executor_backend`.
- Loose `executor_backend`, `backend_id`, `backend_class`, and `required_backend_class` fields are forbidden only in resolved lane/runtime-assignment authority objects once the typed relation is present. The host-system source DSL may retain `backend_id` and `required_backend_class`; materialization resolves those inputs into the strict five-field lane relation.

## Deterministic Authority Identity

The compiled bundle binds every authority-affecting registry before hashing: roles, skills, profiles, flows, packs, commands, and dispatch aliases. It also hashes the selected `dev_team` config.

Canonicalization rules:

- object keys are sorted recursively;
- registry rows are sorted by the registry's declared id key;
- ordered arrays such as flow steps and proof gates preserve order;
- identities use canonical JSON plus the algorithm selected by config;
- the aggregate TeamFlow identity covers the config identity and all component registry identities.

Identity construction is two-phase to avoid circular references:

1. Phase 1 hashes the selected config and registry identities without embedding the resolved payload.
2. Phase 2 canonicalizes and persists the resolved all-flow payload, then hashes that payload and records it through `authority_source` with `identity_phase: phase_2_persisted_payload`.

Identity references point to phase-1 ids or phase-2 payload hashes; a payload may not recursively embed its own aggregate identity. Circular or mismatched references are migration/dispatch blockers.

A changed pack or command must change its component identity and the aggregate identity. Mapping-key or registry-row reordering alone must not.

## Resolved Lane Contract

Every resolved lane is typed and includes all schema-required fields. In addition to its lane id and routing metadata, it carries:

- `included`: evaluated conditional-admission result;
- `required`: whether the selected flow requires completion of the lane;
- `evidence_requirements`: concrete outputs required for completion;
- `proof_gates`: configured proof contract;
- `command_ref`: registry id or null when the declared capability allows no command;
- `command_mapping`: the resolved registry row or null under the same schema condition;
- `rework`: typed rework kind and explicit target;
- `terminal`: typed terminal kind plus proof that it was config-declared;
- `profile_authority`: configured team role, runtime role, task class, and source path;
- `selected_model_profile`: runtime-selected model profile id and selection source.
- `carrier_relation` and `executor_backend_relation`: distinct typed refs to carrier selection and host execution mechanism;
- `authority_identities`: config and effective registry/profile/model identity refs;
- `execution_identity`: the stable identity derived from the selected flow/node and bound authority.

`profile_authority` and `selected_model_profile` are intentionally distinct. Carrier/model selection can change without changing the team role or flow authority.

No untyped legacy fallback projection is executable. A compatibility parser may emit a typed blocker describing the missing field.

## Transition, Approval, Rework, and Terminal Rules

Transitions come from explicit configured edges. Sequence order can help display the plan but does not independently authorize an edge.

Approval-enabled steps pause before their outgoing edge. The authority owner records the configured approval outcome; pending and rework outcomes are not successful completion.

Rework and resume transitions name explicit configured targets. Missing targets, targets outside the selected flow, or conflicting edge declarations block projection or transition.

Terminal closure is lawful only from a step declared terminal in the selected config. Runtime code must not infer terminal state from a role id, name, last array position, missing command, or missing successor.

## Command and Alias Resolution

`command_ref` resolves through the configured command registry. A required lane with an unresolved command fails closed unless the capability/admissibility matrix explicitly permits a commandless lane.

Aliases are identifiers, not fallback command text. Duplicate ids, competing targets, or a project alias that shadows another effective alias are conflicts and must be rejected. Source order is never a conflict-resolution policy.

Each executable configured step must declare a non-empty alias. Alias resolution validates runtime-role and task-class admissibility before carrier selection. Command mapping resolves separately from the command registry; alias text cannot act as command text or as a model/profile selector.

## Conditional Inclusion and Evidence

The master catalog owns the supported inclusion rules. Runtime evaluates only the selected configured rule. An unknown or malformed condition is blocked rather than treated as included or skipped.

If a lane is both included and required, its evidence list must be non-empty and its proof gates must be structurally valid. A skipped conditional lane remains visible in the plan with `included: false`; it is not silently removed from authority evidence.

## Work-Item Flow Selection

Flow selection is data-driven. Explicit work-item binding wins over configured project bindings, and the configured project default is the final ordinary source. If no enabled configured flow resolves, selection fails closed. Runtime code must not embed a semantic default flow id.

Work-item binding vocabulary remains provider-neutral and separate from runtime task class and execution granularity. Unknown work-item types do not become root-capable or flow-bindable through fallback inference.

## Host Adapter Boundary

Flow steps may require a generic host-agent bridge capability. Host adapter choice is config/runtime capability data, not TeamFlow transition law. The adapter must return receipt evidence bound to the same aggregate TeamFlow authority identity before closure.

## Compatibility and Migration

Existing configs remain readable only when they can be normalized without inventing authority. Any legacy shorthand that omits an explicit edge, approval result, terminal declaration, rework target, evidence requirement, or command mapping returns a typed migration blocker.

Migration order follows the TeamFlow authority ADR: establish template/schema/identity, migrate the TaskFlow owner, adapt consumers, remove fallbacks, then enable closure proof.

Migration notes for persisted authority:

- A legacy selected-flow-only snapshot must be expanded to `resolved_all_flow_payload` before it can be reloaded as executable authority.
- Missing payload hash, stale hash, missing `authority_source`, or a circular identity reference returns a typed blocker; do not recompute a replacement from loose lane fields.
- During rollout, retain old data only as read-only migration input and never as a second executable projection.

## Proof Targets

Proof must include template/schema parity and cases for missing, malformed, conditional, approval, terminal, rework/resume, evidence, command, alias-conflict, architect-context resolution, exact-one field sources, distinct carrier/backend relations, complete node identity, registry-identity behavior, persisted all-flow payload hash/reload, two-phase identity, and circular-reference rejection. Static hardcode checks must cover routing, dispatch, resume, state, and status consumers. Executable test commands are recorded by the active implementation packet after all shared owner types are stable.

-----
artifact_path: product/spec/development-flow-catalog-schema-contract
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-07-20'
schema_version: '1'
status: canonical
source_path: docs/product/spec/development-flow-catalog-schema-contract.md
created_at: '2026-06-01T00:00:00+03:00'
updated_at: '2026-07-20T00:00:00+03:00'
changelog_ref: development-flow-catalog-schema-contract.changelog.jsonl
