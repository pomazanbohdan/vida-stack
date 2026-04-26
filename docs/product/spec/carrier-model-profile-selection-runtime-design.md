# Carrier Model Profile Selection Runtime Design

Status: `approved`

Use this design to land one bounded carrier/model-profile contract slice for runtime selection, dispatch truth, and operator proof.

## Summary
- Feature / change: unify carrier + model_profile selection/runtime truth for Codex carriers, internal subagent profiles, and external CLI backends
- Owner layer: `mixed`
- Runtime surface: `launcher | project activation | taskflow | status`
- Status: `approved`

## Current Context
- Existing system overview
  - `vida.config.yaml` already carries concrete Codex carrier metadata (`model`, `model_reasoning_effort`, `sandbox_mode`, `rate`, `runtime_roles`, `task_classes`) for `junior`, `middle`, `senior`, and `architect`.
  - external CLI backends already carry `default_model`, `models_hint`, readiness, and dispatch pinning hooks, but not a canonical `model_profiles` registry.
  - internal subagents already expose profile ids (`internal_fast`, `internal_arch`, `internal_review`), but those ids are not normalized into one model-profile contract with cost/reasoning/readiness metadata.
- Key components and relationships
  - `crates/vida/src/host_runtime_materialization.rs` materializes carrier catalogs into `.codex/agents/*.toml` and currently reads legacy `model` / `model_reasoning_effort` fields directly.
  - `crates/vida/src/runtime_assignment_builder.rs` selects internal carriers from `carrier_runtime.roles`, but exposes only tier-level truth and currently drops `rate == 0` candidates.
  - `crates/vida/src/runtime_assignment_policy.rs` still models task-class/runtime-role inference around tier selection, not profile-level selection.
  - `crates/vida/src/runtime_dispatch_execution.rs` and `crates/vida/src/runtime_dispatch_state.rs` already know how to pass `model`, `sandbox_mode`, and `model_reasoning_effort`, but only from legacy carrier/backend fields.
  - `crates/vida/src/status_surface_host_agents.rs`, `crates/vida/src/status_surface_external_cli.rs`, and `crates/vida/src/taskflow_consume_bundle.rs` surface carrier/runtime state to operators, but they do not yet emit model-profile truth.
- Current pain point or gap
  - runtime proof still tells only `selected_tier` / `model_reasoning_effort`, not `selected_model_profile_id`, `selected_model_ref`, or rejected candidate reasons.
  - external and internal profile metadata live in incompatible shapes, so selection, status, and dispatch do not share one contract.
  - rendered `.codex` parity is now correct in the current workspace for `senior` and `architect`, but the renderer still lacks a profile-aware contract and explicit parity proof against new-style `model_profiles`.

## Goal
- What this change should achieve
  - normalize legacy carrier/backend/profile metadata into one canonical `model_profile` contract
  - expose selected model-profile truth in runtime assignment, dispatch receipts, and operator status
  - make internal and external dispatch consume the resolved model profile instead of ad hoc legacy fields
  - keep legacy config fields readable through synthetic default-profile normalization
- What success looks like
  - every carrier/backend/profile-bearing surface has at least one resolved `model_profile`
  - runtime assignment exposes `selected_model_profile_id`, `selected_model_ref`, `selected_reasoning_effort`, `selection_strategy`, and `rejected_candidates`
  - external CLI status shows profile readiness and selected/default profile truth
  - `.codex` materialization remains parity-safe under legacy and new-style profile config
- What is explicitly out of scope
  - a full global cross-backend route optimizer that replaces current route-based backend selection
  - per-model telemetry learning beyond bounded proof/diagnostic visibility
  - changing delegated-execution law or root-session write authority

## Requirements

### Functional Requirements
- Must-have behavior
  - support both legacy carrier fields (`model`, `model_reasoning_effort`, `sandbox_mode`) and new-style `default_model_profile` plus `model_profiles`
  - synthesize one default profile for legacy carriers/backends so runtime logic sees one uniform shape
  - expose profile truth for internal Codex carriers, internal subagent profiles, and external CLI backends
  - keep zero-cost/free model profiles admissible instead of dropping them by `rate == 0`
  - use the resolved model profile for internal/external dispatch pinning and status visibility
  - emit selected and rejected candidate diagnostics in runtime assignment and status/operator surfaces
- Integration points
  - `vida.config.yaml`
  - host template materialization
  - runtime assignment and consume snapshots
  - external/internal dispatch command generation
  - operator-facing status JSON and TaskFlow agent-system snapshots
- User-visible or operator-visible expectations
  - `vida status --json` must show default/available model profiles and current selected/default profile truth
  - `vida taskflow consume final ... --json` and related snapshots must show profile-level assignment evidence
  - `vida project-activator` plus `.codex` render must stay aligned with source config under the new schema

### Non-Functional Requirements
- Performance
  - profile normalization must stay in-memory and bounded; no new network probes or process hops
- Scalability
  - one carrier may expose multiple profiles without requiring per-profile bespoke code branches
- Observability
  - status and runtime assignment must explain why a profile was selected or rejected
- Security
  - unsupported or unready profiles must fail closed and remain explicit in diagnostics rather than silently falling back

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/agent-role-skill-profile-flow-model.md`
  - `docs/product/spec/compiled-runtime-bundle-contract.md`
  - `docs/product/spec/external-cli-carrier-hardening-design.md`
  - `docs/process/codex-agent-configuration-guide.md`
- Framework protocols affected:
  - none beyond existing runtime/operator contracts
- Runtime families affected:
  - `launcher`
  - `project activation`
  - `taskflow`
  - `status`
- Config / receipts / runtime surfaces affected:
  - `vida.config.yaml`
  - `.codex/agents/*.toml`
  - runtime assignment JSON
  - external CLI preflight/status JSON
  - dispatch packets and receipts

## Design Decisions

### 1. Legacy fields normalize into synthetic default model profiles
Will implement / choose:
- keep legacy config readable, but convert it immediately into synthetic `model_profiles` rows
- expose `default_model_profile` on every normalized carrier/backend/profile-bearing row
- Why
  - this avoids a breaking config migration and gives one uniform runtime contract
- Trade-offs
  - some fields will exist both as compatibility aliases and canonical normalized profile metadata during the bridge window
- Alternatives considered
  - hard-cut to `model_profiles` only and reject legacy config
- ADR link if this must become a durable decision record
  - none

### 2. Runtime assignment becomes profile-aware inside the current carrier ladder
Will implement / choose:
- keep current carrier ladder selection semantics, but choose a concrete profile within the selected carrier and emit selected/rejected profile diagnostics
- keep external backend routing route-driven for this slice; external backends gain profile-level truth once the backend is selected
- Why
  - this lands the profile contract without rewriting the whole planner in one step
- Trade-offs
  - profile-aware routing across all backends remains a follow-up rather than part of this bounded slice
- Alternatives considered
  - rewrite routing and runtime assignment into one global cross-backend optimizer immediately
- ADR link if needed
  - none

### 3. `architect` defaults through the configured model-profile catalog
Will implement / choose:
- use the active configured `architect` default profile from `vida.config.yaml`
- keep lower-cost architecture profiles available only when a bounded route explicitly selects them through the configured catalog
- Why
  - framework behavior must stay model-agnostic while the project config remains the concrete source of model/profile/cost truth
- Trade-offs
  - ordinary architecture work follows the configured default cost unless a route explicitly selects another configured profile
- Alternatives considered
  - keep `architect` permanently pinned to `high` with no `xhigh` option
  - promote all architect work to `xhigh`
- ADR link if needed
  - none

### 4. Zero-cost profiles stay eligible and quality/reasoning floors remain explicit
Will implement / choose:
- stop treating `rate == 0` as automatic rejection
- add bounded selection-policy fields for strategy, quality floors, and reasoning floors where available
- Why
  - free external profiles are valid candidates and must remain visible in diagnostics
- Trade-offs
  - some quality-floor behavior may begin as diagnostics-first before broader route-level adoption
- Alternatives considered
  - leave free profiles hidden from runtime selection and rely on route hardcoding only
- ADR link if needed
  - none

## Technical Design

### Core Components
- Main components
  - profile normalization helpers in `host_runtime_materialization.rs`
  - runtime assignment/profile selection in `runtime_assignment_builder.rs` and `runtime_assignment_policy.rs`
  - dispatch/profile resolution in `runtime_dispatch_execution.rs` and `runtime_dispatch_state.rs`
  - operator/status snapshots in `status_surface_host_agents.rs`, `status_surface_external_cli.rs`, and `taskflow_consume_bundle.rs`
- Key interfaces
  - normalized carrier row with `default_model_profile` and `model_profiles`
  - normalized external backend entry with the same profile contract
  - runtime assignment projection fields for selected/rejected profiles
  - dispatch/profile resolver returning `model_ref`, `reasoning_effort`, `provider`, and `sandbox_mode`
- Bounded responsibilities
  - materialization owns profile normalization and `.codex` parity
  - runtime assignment owns candidate evaluation and selected/rejected diagnostics
  - dispatch owns command rendering from the resolved profile
  - status/consume surfaces own operator-proof projection

### Data / State Model
- Important entities
  - `carrier`
  - `model_profile`
  - `reasoning_control`
  - `selection_strategy`
  - `rejected_candidate`
- Receipts / runtime state / config fields
  - `default_model_profile`
  - `model_profiles`
  - `selected_model_profile_id`
  - `selected_model_ref`
  - `selected_reasoning_effort`
  - `selection_strategy`
  - `rejected_candidates`
  - `normalized_cost_units`
- Migration or compatibility notes
  - old-style carrier/backends normalize into one synthetic profile keyed from legacy model/reasoning fields
  - `default_model` and `models_hint` remain readable compatibility input for external backends, but no longer act as the only source of truth

### Integration Points
- APIs
  - status JSON
  - runtime assignment JSON
  - dispatch result/receipt payloads
- Runtime-family handoffs
  - project activation/materialization -> carrier catalog rows
  - carrier catalog rows -> runtime assignment
  - selected profile -> dispatch and status
- Cross-document / cross-protocol dependencies
  - `feature-carrier-model-profile-selection-runtime-spec`
  - `feature-carrier-model-profile-selection-runtime-schema`
  - `feature-carrier-model-profile-selection-runtime-normalization`
  - `feature-carrier-model-profile-selection-runtime-selection`
  - `feature-carrier-model-profile-selection-runtime-dispatch-status`
  - `feature-carrier-model-profile-selection-runtime-proof`

### Bounded File Set
- `vida.config.yaml`
- `crates/vida/src/host_runtime_materialization.rs`
- `crates/vida/src/runtime_assignment_builder.rs`
- `crates/vida/src/runtime_assignment_policy.rs`
- `crates/vida/src/carrier_runtime_metadata.rs`
- `crates/vida/src/runtime_dispatch_execution.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/status_surface_host_agents.rs`
- `crates/vida/src/status_surface_external_cli.rs`
- `crates/vida/src/taskflow_consume_bundle.rs`
- `crates/vida/tests/boot_smoke.rs`
- `docs/process/codex-agent-configuration-guide.md`
- `docs/product/spec/agent-role-skill-profile-flow-model.md`
- `docs/product/spec/compiled-runtime-bundle-contract.md`
- `docs/product/spec/external-cli-carrier-hardening-design.md`

## Fail-Closed Constraints
- Forbidden fallback paths
  - no silent drop of zero-cost profiles
  - no silent fallback from a selected profile to ambient backend state when the profile is present and admissible
  - no status surface that hides rejected or blocked profiles behind one generic pass/fail
- Required receipts / proofs / gates
  - runtime assignment must expose selected profile fields and rejected candidate reasons
  - `.codex` parity must stay test-backed under the normalized profile contract
  - external/internal dispatch tests must prove selected profile pinning
- Safety boundaries that must remain true during rollout
  - current route-based external backend selection stays intact
  - delegated execution law remains unchanged
  - compatibility aliases remain readable while canonical profile fields become authoritative

## Implementation Plan

### Phase 1
- land `Task A` schema/doc contract and normalize `vida.config.yaml` into profile-aware shapes
- add synthetic-profile compatibility helpers for legacy carrier/backend fields
- First proof target
  - design doc, owner docs, and normalized config shapes all agree on the profile contract

### Phase 2
- land `Task B` and `Task C`: runtime normalization plus profile-aware assignment and diagnostics
- expose `selected_model_profile_id`, `selected_model_ref`, `selected_reasoning_effort`, `selection_strategy`, and `rejected_candidates`
- Second proof target
  - runtime assignment snapshots and tests show concrete selected profile truth and free profiles stay eligible

### Phase 3
- land `Task D` and `Task E`: dispatch/status wiring, `.codex` parity enforcement, and proof fixtures
- keep `Task E` parallel-safe only for isolated proof work once `Task C` is stable; closure still depends on `Task D`
- Final proof target
  - dispatch, status, consume, and materialization surfaces all agree on the resolved model profile

## Validation / Proof
- Unit tests:
  - runtime assignment profile selection and rejected-candidate reasons
  - synthetic-profile normalization from legacy config
  - external backend selected-profile resolution and zero-cost admissibility
- Integration tests:
  - `.codex` render parity through `boot_smoke`
  - TaskFlow agent-system/runtime-assignment snapshot projection
  - status JSON profile visibility
- Runtime checks:
  - `cargo test -p vida runtime_assignment_builder`
  - `cargo test -p vida runtime_dispatch_state`
  - `cargo test -p vida status_surface_host_agents`
  - `cargo test -p vida status_surface_external_cli`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/carrier-model-profile-selection-runtime-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md`
  - `vida docflow proofcheck --profile active-canon docs/product/spec/carrier-model-profile-selection-runtime-design.md`
  - `vida docflow readiness-check --profile active-canon`

## Observability
- Logging points
  - selected carrier id
  - selected model profile id
  - selected model ref / provider / reasoning effort
  - rejected candidates with reasons
- Metrics / counters
  - none new beyond existing score/budget surfaces in this bounded slice
- Receipts / runtime state written
  - runtime assignment JSON
  - dispatch packet/receipt profile metadata
  - host-agent and external-cli status snapshots

## Rollout Strategy
- Development rollout
  - docs/schema first, then normalization/assignment, then dispatch/status/proofs
- Migration / compatibility notes
  - legacy fields stay accepted until the profile contract is fully propagated through status and dispatch
  - route-based backend selection remains the planner boundary for this slice
- Operator or user restart / restart-notice requirements
  - rerun `vida project-activator --host-cli-system codex --json` after profile-aware materialization changes when `.codex` output must be refreshed

## Future Considerations
- Follow-up ideas
  - extend profile-aware selection into route-level external backend choice
  - add per-model telemetry learning instead of per-carrier-only scoring
  - promote quality-floor and risk-escalation policy from diagnostics-first into route-level selection law
- Known limitations
  - this slice does not yet replace route-based backend selection with one global cross-backend optimizer
- Technical debt left intentionally
  - compatibility aliases for legacy fields remain until downstream consumers are fully profile-aware

## References
- Related specs
  - `docs/product/spec/agent-role-skill-profile-flow-model.md`
  - `docs/product/spec/release-1-carrier-neutral-runtime-and-host-materialization-design.md`
  - `docs/product/spec/external-cli-carrier-hardening-design.md`
- Related protocols
  - `docs/process/codex-agent-configuration-guide.md`
- Related ADRs
  - none
- External references
  - user handoff report: `model/reasoning profile behind carriers`, provided on 2026-04-22
  - OpenAI Codex config reference and config sample cited in the handoff report

-----
artifact_path: product/spec/carrier-model-profile-selection-runtime-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-22
schema_version: 1
status: canonical
source_path: docs/product/spec/carrier-model-profile-selection-runtime-design.md
created_at: 2026-04-22T09:43:31.446725801Z
updated_at: 2026-04-22T10:13:29.70930412Z
changelog_ref: carrier-model-profile-selection-runtime-design.changelog.jsonl
