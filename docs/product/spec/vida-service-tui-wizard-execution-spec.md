# VIDA Service, TUI, And Wizard Execution Spec

Status: accepted for staged implementation

Purpose: turn the approved service/TUI/wizard research Sets 1-22 into an execution-ready specification packet with scope, exclusions, operation families, first-wave task order, owned paths, proof targets, and approval evidence.

Source research:

1. `docs/product/research/vida-service-tui-wizard-architecture-research.md`
2. Approved operator clarifications Sets 1-22 dated 2026-05-21.
3. TaskFlow item `vida-service-tui-wizard-spec-pack`.

## Scope

This spec covers the first executable architecture packet for:

1. shared service/client/wizard contracts,
2. command envelope and response/problem/event/receipt types,
3. operation catalog and command-family mapping,
4. project registry and project selection model,
5. wizard domain state machine and option graph,
6. service-local coordination state,
7. fixture and in-process client proof path,
8. later TUI screens as clients of the same contracts.

The first implementation target is not a finished daemon or terminal UI. The first target is the contract and proof chain that prevents CLI, service, TUI, and future dashboard surfaces from growing separate semantics.

## Explicit Exclusions

These are out of scope for the first packet:

1. Ratatui screen implementation before contract and fixture-client proof.
2. Full daemon command surface before `VidaClient` and in-process client proof.
3. Dashboard, browser API, and `JsonRpseeTransport`.
4. Remote or multi-user auth.
5. Native Windows Service apply path before foreground/session daemon and install diagnostics are proven.
6. Destructive project-root deletion.
7. TaskFlow/DocFlow service migration.
8. Hardcoded host CLI, provider, model, role sequence, flow sequence, or carrier names.
9. UI-local config parsing, materialization drift decisions, or mutation legality.
10. Multisession repair as a child task of this epic; multisession/session-scoped continuation remains an external prerequisite and runtime defect track.

## Approved Architecture

The accepted architecture is envelope/core-first with adapters:

```text
client action
  -> VidaCommandEnvelope
  -> VidaClient
  -> transport adapter
  -> vida-service command handler
  -> project-scoped state/router
  -> receipt + events + response
```

Rules:

1. CLI, TUI, daemon, and future dashboard share this semantic path.
2. Transport adapters carry `VidaCommandEnvelope`; transport-specific RPC traits do not become product semantics.
3. TUI never writes files, project DB, or service-home state directly.
4. Service/client plan, diff, validate, apply, receipts, jobs, and events are the mutation authority.
5. Existing direct CLI mutation paths remain only until service/client equivalents are proven and explicit fallback policy exists.

## First Crate And Module Boundary

First crate:

1. Add `vida-contracts`.
2. Keep it pure serde/contracts.
3. Do not add Ratatui, tarpc, service runtime, filesystem mutation, or project DB mutation to `vida-contracts`.

Initial contract families:

1. identity:
   - `VidaSessionId`
   - `VidaRequestId`
   - `VidaProjectId`
   - `VidaProjectRef`
   - `VidaClientKind`
2. envelope:
   - `VidaCommandEnvelope`
   - `VidaCommandResponse`
   - `VidaProblem`
   - `VidaOperation`
   - `VidaIdempotencyKey`
3. events and receipts:
   - `VidaEvent`
   - `VidaEventCursor`
   - `VidaReceiptRef`
   - `VidaReceiptSummary`
4. project registry:
   - `ServiceProjectRegistryEntry`
   - `ProjectRegistryStatus`
   - `ProjectActivationStatus`
   - `ServiceBindingStatus`
   - `ProjectHealthSummary`
5. wizard:
   - `WizardKind`
   - `WizardSessionId`
   - `WizardOptionSpec`
   - `WizardOptionValue`
   - `WizardOptionState`
   - `WizardSessionState`
   - `WizardValidationFinding`
   - `WizardReadinessFinding`
6. config and materialization:
   - `VidaConfigManifest`
   - `VidaProjectionManifest`
   - `MaterializationArtifactStatus`
   - `ActivationUpdateReport`
7. planning and jobs:
   - `VidaPlanRef`
   - `VidaPlanSummary`
   - `VidaDiffSummary`
   - `VidaApplyToken`
   - `VidaJobRef`
   - `WizardApplyJob`
   - `VidaJobStatus`

Internal `crates/vida` modules follow after the shared contracts:

1. `vida_client`
2. `vida_config_graph`
3. `vida_materialization`
4. `vida_wizard_core`
5. `vida_project_registry`
6. `vida_service_state`
7. `vida_jobs`
8. `vida_events`

Crate extraction for service/TUI/dashboard waits until these APIs stabilize.

## Command Envelope

`VidaCommandEnvelope` must carry:

```text
schema_version
protocol_version
operation
session_id
request_id
project_ref
claim_kind
payload
correlation
idempotency_key optional
apply_token optional
```

Rules:

1. Mutating operations require `session_id`, `request_id`, `project_ref`, `claim_kind`, and idempotency/apply-token fields where operation metadata requires them.
2. Read-only operations still carry session/request identity for observability and multi-session correctness.
3. `client_kind` and carrier/provider/model references are registry/config-derived values, not hardcoded product names.
4. JSON output from service-backed CLI surfaces includes `request_id`, `session_id`, `operation_id`, and service/direct/fallback mode.

## Operation Families

First-wave read-heavy operation catalog:

```text
vida.service.hello
vida.service.status
vida.service.capabilities
vida.service.endpoint.status
vida.events.since
vida.session.resolve
vida.project.resolve
vida.project.status
vida.project.registry.list
vida.project.registry.get
vida.project.registry.discover
vida.receipts.get
vida.wizard.schema.get
vida.wizard.session.start
vida.wizard.session.get
vida.wizard.session.update_input
vida.wizard.session.validate
vida.wizard.session.diff
vida.jobs.get
```

Apply-capable operations wait until registry, service state, idempotency, claim admission, and apply-token proof exist:

```text
vida.service.install.plan
vida.service.install.apply
vida.project.registry.register
vida.project.registry.update
vida.project.registry.reconcile
vida.project.registry.set_active
vida.project.registry.detach
vida.project.registry.archive
vida.project.registry.restore
vida.project.registry.forget
vida.project.config.update.plan
vida.project.config.update.apply
vida.project.materialization.update.plan
vida.project.materialization.update.apply
vida.wizard.session.plan
vida.wizard.session.apply
vida.wizard.session.cancel
```

Rules:

1. Operation ids are stable string constants in `vida-contracts`.
2. Operation metadata declares required project, claim, idempotency, capability, and apply-token fields.
3. Deprecated operation ids require an explicit compatibility window.
4. TUI controls and CLI commands use operation metadata rather than hardcoded flow legality.

## Wizard Domain State

The wizard is a persisted domain state machine, not TUI-local screen state.

States:

```text
created
inspecting
drafting
validating
invalid
diff_ready
approval_required
apply_queued
applying
applied
stale
cancelled
blocked
failed
```

Modes:

1. `project_init`
2. `project_register`
3. `reconfigure`
4. `materialization_update`
5. `service_install`
6. `repair`

Rules:

1. The service owns wizard draft state after attach.
2. TUI and CLI edit drafts only through wizard operations.
3. A draft stores `base_config_revision` and `draft_revision`.
4. Changed config or materialization state marks the draft stale.
5. Apply is forbidden unless validation and diff are bound to the current draft revision.
6. Draft handoff or reclaim requires explicit session/lease-aware transition evidence.

## Wizard Option Graph

Core option groups:

```text
project_identity
language_policy
docs_roots
host_system
execution_class
agent_system_mode
agent_registries
enabled_roles
enabled_skills
enabled_profiles
enabled_flows
dev_team_flow
model_profiles
pricing_policy
scoring_policy
parallelism_policy
materialization_targets
service_install_mode
update_policy
```

Rules:

1. Option dependencies are explicit.
2. Disabled, hidden, invalid, defaulted, and blocked options expose structured reasons.
3. TUI widgets, CLI prompts, and future dashboard controls derive from the same option metadata where practical.
4. Model and carrier choices resolve through configured registries and profile compatibility, never hardcoded provider/model names.
5. Flow sequences are config-derived; task/epic classes bind to configured flow ids and roles.

## Service State And Project Authority

Service home owns coordination state only:

1. service manifest,
2. endpoint metadata,
3. project registry,
4. sessions,
5. jobs,
6. events,
7. receipts,
8. idempotency,
9. materialization manifests,
10. recovery logs.

Project-local authority remains in the project:

1. project config,
2. project DB/state,
3. project docs,
4. generated artifacts,
5. project receipts for project-scoped mutation.

Rules:

1. Service-home state is single-writer by the service process.
2. CLI/TUI never write service-home state directly.
3. MVP service state uses append-only JSONL plus immutable receipt JSON files.
4. Derived compact projections can exist, but replay semantics are the authority.
5. No service-home or project DB lock may be held across long-running jobs, external process waits, or client IPC waits.

## TUI Scope

TUI MVP is a client over `VidaClient` and uses Ratatui only after fixture and in-process client proof.

Top-level shell:

1. header with service status, selected project, session id, active job, update status,
2. navigation for Projects, Overview, Wizard, Update Center, Config, Agent Topology, Materialization, Jobs, Receipts, Logs, Service,
3. main pane for active screen,
4. contextual sidecar pane for events/logs/findings/receipt preview,
5. footer command bar.

Rules:

1. Read-only screens can refresh directly.
2. Mutating actions open a workflow and require plan/diff/validate/apply.
3. Dangerous lifecycle actions require explicit confirmation and remain non-destructive to project root in MVP.
4. TUI reconnect/resume restores service connection, selected project, wizard session, active jobs, and event cursor.
5. Screens are snapshot-tested against fixture `VidaClient` states before live daemon smoke tests.

## CLI Migration Scope

CLI migration is staged:

1. `direct_only_bootstrap`: init/boot/service lifecycle/offline repair.
2. `service_first`: project, wizard, config, materialization, jobs, events, receipts.
3. `service_preferred_with_direct_fallback`: status, doctor, selected read-only project activation surfaces after routing exists.
4. `direct_until_taskflow_service_adapter_exists`: TaskFlow, agent, lane, approval, recovery, consume proxies.
5. `external_family_direct`: DocFlow/protocol/release families until separate service adapters exist.

Rules:

1. Bootstrap and service lifecycle commands cannot depend on a running service.
2. New project/wizard/config/materialization commands are service-first.
3. Existing TaskFlow mutations remain direct until service routing preserves TaskFlow law, claims, receipts, and performance.
4. Fallback mode is visible in output and receipts.
5. `--direct`, `--offline`, and `--service-required` are explicit modes.

## First-Wave Task Order

1. `vida-contracts-core-types`
   - create `vida-contracts` with pure serde contract types and golden fixtures.
2. `vida-operation-registry-golden-fixtures`
   - define operation ids, operation metadata, and fixture responses/problems/events.
3. `vida-client-fixture-inprocess-adapters`
   - add `VidaClient`, `FixtureVidaClient`, and `InProcessVidaClient` behavior parity tests.
4. `vida-service-state-hello-status`
   - add service-home layout, service state records, hello/status/capabilities, endpoint metadata contracts.
5. `vida-project-registry-per-project-actors`
   - add project registry records, project resolution, lifecycle states, and per-project queue/actor boundaries.
6. `vida-wizard-core-state-diff-engine`
   - add persisted wizard draft state, option graph, validation, diff, stale detection, and apply-token placeholder.
7. `vida-materialization-manifest-drift-engine`
   - add config/materialization manifests, drift classification, update modes, structured diff, and receipt shape.
8. `vida-tui-fixture-shell-snapshots`
   - add Ratatui shell and snapshot tests against fixture `VidaClient` only after Gate 6.
9. `vida-tarpc-local-ipc-envelope-smoke`
   - add tarpc-over-local-IPC smoke carrying `VidaCommandEnvelope` after in-process proof.

## Proof Gates

Gate 1: contracts

1. `vida-contracts` compiles.
2. Golden JSON fixtures cover envelope, response, problem, event, receipt, option graph, project registry, wizard session, apply job.

Gate 2: operation registry

1. Operation ids and metadata are stable constants.
2. Claim/capability/project/idempotency requirements validate.
3. Unsupported operation or unsupported knob returns `VidaProblem`.

Gate 3: client abstraction

1. `FixtureVidaClient` and `InProcessVidaClient` pass the same behavior tests.
2. CLI/TUI-facing code depends on `VidaClient`, not direct state writes.

Gate 4: service state

1. Service registry, jobs, events, receipts, and idempotency records persist and replay.
2. Startup recovery rebuilds derived projections and classifies non-terminal jobs.
3. Locks are not held across waits.

Gate 5: wizard core

1. Start/update/validate/diff wizard session with fixture project.
2. Disabled options expose blocked reasons.
3. Draft becomes stale after base config/materialization revision changes.

Gate 6: materialization

1. Generated-clean artifact plans as `safe_update`.
2. User-modified artifact plans as `manual_conflict` or `skip`.
3. Apply token signs exact diff hash.

Gate 7: TUI fixtures

1. AppShell, Projects, Wizard, Update Center, Agent Topology, Jobs, Logs render from fixture client.
2. Narrow terminal snapshots remain readable.
3. Mutating actions route to workflow screens.

Gate 8: transport smoke

1. Tarpc local IPC carries `VidaCommandEnvelope`.
2. `vida.service.hello`, `vida.service.status`, and `vida.events.since` work through the adapter.

Gate 9: live daemon smoke

1. service hello,
2. project registry list,
3. wizard schema get,
4. update inspect,
5. events since.

## Owned Paths

Specification packet:

1. `docs/product/spec/vida-service-tui-wizard-execution-spec.md`
2. `docs/product/spec/current-spec-map.md`
3. `docs/product/spec/current-spec-provenance-map.md`

First-wave implementation paths:

1. `crates/vida-contracts/**`
2. `crates/vida/src/vida_client*.rs`
3. `crates/vida/src/vida_config_graph*.rs`
4. `crates/vida/src/vida_materialization*.rs`
5. `crates/vida/src/vida_wizard_core*.rs`
6. `crates/vida/src/vida_project_registry*.rs`
7. `crates/vida/src/vida_service_state*.rs`
8. `crates/vida/src/vida_jobs*.rs`
9. `crates/vida/src/vida_events*.rs`
10. `crates/vida/tests/**` for fixture, contract, and command-family proof.

Excluded from first implementation packet:

1. dashboard crates,
2. separate `vida-service` binary,
3. separate `vida-tui` binary,
4. TaskFlow/DocFlow service adapters,
5. native Windows Service apply path.

## Acceptance Criteria

AC-1: No semantic drift

CLI, TUI, service, and future dashboard surfaces must consume the same contracts and operation metadata.

AC-2: No hardcoded host/provider/model/flow names

Host systems, carriers, providers, model profiles, roles, skills, and flows are resolved from configured registries and compatibility rules.

AC-3: No UI-local mutation authority

TUI and CLI surfaces do not compute write legality or mutate files/state directly for service-owned operations.

AC-4: Read-heavy MVP

First wizard/TUI MVP remains read-heavy and apply-limited until service state, idempotency, claim admission, and apply-token proof are green.

AC-5: Project authority preserved

Service home coordinates projects, but project-local config/DB/docs/generated artifacts remain project authority.

AC-6: Proof order enforced

Ratatui work starts only after fixture/in-process client proof; live daemon tests start only after transport smoke; mutating apply tests wait for idempotency and receipts.

AC-7: Multisession stays external

Session-scoped continuation and multisession ownership defects are tracked in the runtime defect epic and are not hidden as service/TUI child tasks.

-----
artifact_path: product/spec/vida-service-tui-wizard-execution-spec
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-05-26'
schema_version: '1'
status: accepted
source_path: docs/product/spec/vida-service-tui-wizard-execution-spec.md
created_at: 2026-05-26T00:00:00+03:00
updated_at: 2026-05-26T00:00:00+03:00
changelog_ref: vida-service-tui-wizard-execution-spec.changelog.jsonl
