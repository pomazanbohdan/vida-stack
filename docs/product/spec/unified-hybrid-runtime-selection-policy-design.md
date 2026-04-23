# Unified Hybrid Runtime Selection Policy Design

Status: `approved`

Use this design to finish the post-`carrier + model_profile` wave so runtime can select one effective carrier/backend/profile truth across internal and external execution paths.

## Summary
- Feature / change: complete unified hybrid runtime selection and routing policy after the carrier/model-profile contract wave
- Owner layer: `mixed`
- Runtime surface: `launcher | taskflow | status | init`
- Status: `approved`

## Current Context
- Existing system overview
  - The prior wave already introduced the `model_profile` contract, profile-aware runtime assignment, external profile registries, zero-cost profile admissibility, and Codex materialization parity for `senior` and `architect`.
  - Runtime assignment now exposes `selected_model_profile_id`, `selected_model_ref`, `selection_strategy`, and `rejected_candidates`.
  - External CLI status already computes meaningful live readiness information, but that data is not yet part of early candidate rejection for initial dispatch.
- Key components and relationships
  - `crates/vida/src/runtime_assignment_builder.rs` selects the carrier/profile candidate from the unified carrier-role projection.
  - `crates/vida/src/taskflow_routing.rs` and `crates/vida/src/runtime_dispatch_state.rs` later choose the effective backend for the active route/dispatch target.
  - `crates/vida/src/carrier_runtime_projection.rs` projects host carriers and external CLI backends into runtime selection, but not `internal_subagents`.
  - `crates/vida/src/status_surface_external_cli.rs`, `crates/vida/src/taskflow_consume_bundle.rs`, and `crates/vida/src/init_surfaces.rs` expose operator/runtime truth, but several fields are still metadata-only or misreported.
- Current pain point or gap
  - Static validation against the current code confirms the remaining gaps from the external audit:
    - route-level backend precedence still overrides the dynamic carrier/profile result during effective backend selection,
    - `budget_policy`, `max_budget_units`, `candidate_scope`, and many route knobs are present in config but not behaviorally enforced,
    - `internal_subagents` remain dispatch/fallback backends instead of first-class candidates in the unified selector, and route `profiles:` mappings are not operational,
    - `reasoning_control` remains metadata-only, live external readiness is not an early candidate filter, and `model_selection_enabled` is derived incorrectly from object presence instead of the actual `enabled` flag.

## Goal
- What this change should achieve
  - produce one truthful hybrid selection path from `carrier/profile selection` through `effective backend selection`, dispatch, continue/recovery, and operator surfaces
  - make budget and route policy fields behaviorally live rather than configuration-only
  - promote `internal_subagents` profiles into the real candidate pool and make route-level profile overrides execution-effective
  - operationalize external reasoning/readiness policy or explicitly downgrade unsupported fields to non-operational status
  - add operator-facing diagnostics that explain why the runtime chose or rejected each candidate/backend/profile
- What success looks like
  - `selected_carrier_id`, `selected_model_profile_id`, `effective_selected_backend`, and `backend_selection_source` are consistent across assignment, dispatch, status, and continue/recovery surfaces
  - route `executor_backend` becomes a policy input and diagnostic hint, not a silent hard override over a more admissible dynamic choice
  - `strict` and `balanced` budget semantics are enforced in runtime selection
  - `internal_fast`, `internal_review`, and `internal_arch` can actually participate in runtime selection and route-level profile override
  - external readiness and reasoning policy affect dispatch eligibility or are explicitly reported as non-operational
  - `vida taskflow route explain` and `vida taskflow validate-routing` exist as operator surfaces for this policy layer
- What is explicitly out of scope
  - redesigning the entire Release-1 orchestration model beyond the selection/routing policy seam
  - changing delegated execution law or widening root-session write authority
  - replacing existing external CLI adapters with a brand-new dispatch protocol

## Requirements

### Functional Requirements
- Must-have behavior
  - dynamic carrier/profile selection must remain the primary selection truth, with route backend hints applied as explicit policy inputs rather than hidden post-selection overrides
  - `budget_policy` and `max_budget_units` must influence candidate acceptance/ranking with explicit diagnostics
  - `internal_subagents` model profiles must be projectable into the same runtime candidate pool used by the selector
  - route `profiles:` mappings must influence actual selected profile resolution for the chosen backend/dispatch target
  - external `reasoning_control` must either affect dispatch behavior/readiness classification or be explicitly downgraded from canonical profile/status truth
  - live blocked external carriers must be rejectable before initial dispatch, not only during late retry/recovery paths
  - `model_selection.enabled` and `candidate_scope` must become real switches instead of passive metadata
  - `vida taskflow route explain --json` and `vida taskflow validate-routing --json` must expose candidate pool, budget verdict, selected carrier/profile, effective backend, override source, and rejected candidates
- Integration points
  - `vida.config.yaml`
  - carrier runtime projection
  - runtime assignment and dispatch execution/state
  - status/init/consume/continue/recovery surfaces
  - operator command/help/query surfaces
- User-visible or operator-visible expectations
  - operators can see whether a backend/profile was selected by dynamic selection, route override, fallback, or receipt reconciliation
  - operators can tell whether a candidate was rejected for budget, readiness, quality floor, reasoning floor, or write-scope inadmissibility
  - diagnostics stop implying that config knobs are live when they are not

### Non-Functional Requirements
- Performance
  - selection and validation must stay bounded and in-memory; no new network probes are allowed on the hot path beyond existing preflight/readiness data
- Scalability
  - the selector must support internal carriers, internal subagents, and external backends without hardcoding one backend family as special-case truth
- Observability
  - operator surfaces must render both dynamic selection truth and route-policy truth without collapsing them into one ambiguous field
- Security
  - blocked or unready backends/profiles must fail closed
  - no fallback may silently ignore declared budget, readiness, or reasoning policy

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/unified-hybrid-runtime-selection-policy-design.md`
  - `docs/product/spec/current-spec-map.md`
  - `docs/product/spec/current-spec-provenance-map.md`
  - `docs/product/spec/README.md`
  - `docs/product/spec/agent-role-skill-profile-flow-model.md`
  - `docs/product/spec/external-cli-carrier-hardening-design.md`
  - `docs/product/spec/implementation-backend-admissibility-and-selection-truth-design.md`
- Framework protocols affected:
  - none beyond existing routing/operator/runtime contracts
- Runtime families affected:
  - `launcher`
  - `taskflow`
  - `status`
  - `init`
- Config / receipts / runtime surfaces affected:
  - `vida.config.yaml`
  - runtime assignment JSON
  - route/dispatch packet rendering
  - continue/recovery/status/init summaries
  - external CLI preflight/readiness summaries

## Design Decisions

### 1. Route backend hints become policy inputs, not silent hard overrides
Will implement / choose:
- keep route `executor_backend`, `fallback_executor_backend`, and `fanout_executor_backends` as explicit policy hints and diagnostics, but let the canonical effective backend resolver apply them against the dynamically selected carrier/profile instead of overriding that result blindly
- Why
  - the current split between `selected_carrier/profile` and later route-led backend override is the main source of truth drift
- Trade-offs
  - route-policy reporting must become richer because `route_primary_backend` and `effective_selected_backend` may diverge by design
- Alternatives considered
  - keep route `executor_backend` as the hard winner
  - fully remove route backend hints from config
- ADR link if this must become a durable decision record
  - none

### 2. Budget semantics must be explicit and fail-closed by policy class
Will implement / choose:
- treat `strict` budget policy as fail-closed candidate rejection when a candidate exceeds `max_budget_units`
- treat `balanced` budget policy as “prefer in-budget candidates first, but allow an over-budget escalation only when no admissible in-budget candidate remains”, with explicit `budget_verdict` diagnostics
- Why
  - the config already encodes budget intent; leaving it dead creates false operator truth
- Trade-offs
  - some previously selectable high-cost paths will now become explicit blockers or ranked fallbacks
- Alternatives considered
  - make budget metadata-only forever
  - make every budget cap a hard blocker regardless of policy class
- ADR link if needed
  - none

### 3. `internal_subagents` become first-class runtime candidates
Will implement / choose:
- project `internal_subagents` into the same candidate pool as host carriers and external backends
- make route `profiles:` mappings choose the profile for the active backend/dispatch target before falling back to the top-level runtime assignment profile
- Why
  - today `internal_subagents` can only appear as dispatch/fallback transport, which prevents the unified selector from seeing `internal_fast`, `internal_review`, and `internal_arch`
- Trade-offs
  - carrier projection and backend diagnostics must distinguish host-carrier rows from subagent-backend rows cleanly
- Alternatives considered
  - keep `internal_subagents` as a synthetic bridge only
  - wire route `profiles:` only at summary level and not at dispatch
- ADR link if needed
  - none

### 4. External reasoning/readiness policy must become operational or explicitly downgraded
Will implement / choose:
- operationalize `reasoning_control` for backends that support it, or explicitly mark it as non-operational in runtime/status truth when the backend cannot execution-enforce it
- thread live external readiness into early candidate rejection or pre-dispatch gating instead of leaving it as status-only data
- Why
  - current profile/status truth implies stronger enforcement than the runtime actually performs
- Trade-offs
  - some backends will surface “provider_default / informational-only” rather than pretending to have a stronger reasoning contract
- Alternatives considered
  - leave `reasoning_control` and live readiness as status-only metadata
- ADR link if needed
  - none

### 5. Operator surfaces must expose one canonical selection narrative
Will implement / choose:
- standardize on `selected_carrier_id`, `selected_model_profile_id`, `effective_selected_backend`, `backend_selection_source`, `budget_verdict`, and `rejected_candidates`
- add explicit operator commands for route explanation and routing validation
- Why
  - current status/consume/init surfaces expose overlapping but non-identical truth
- Trade-offs
  - output schemas and tests grow slightly, but operator diagnosis stops depending on source-code spelunking
- Alternatives considered
  - keep existing fragmented summaries and rely on ad hoc debugging
- ADR link if needed
  - none

## Technical Design

### Core Components
- Main components
  - `carrier_runtime_projection.rs`
  - `runtime_assignment_builder.rs`
  - `runtime_assignment_policy.rs`
  - `runtime_lane_summary.rs`
  - `taskflow_routing.rs`
  - `runtime_dispatch_state.rs`
  - `runtime_dispatch_execution.rs`
  - `status_surface_external_cli.rs`
  - `status_surface_host_agents.rs`
  - `taskflow_consume_bundle.rs`
  - init/taskflow operator command surfaces
- Key interfaces
  - unified runtime candidate row
  - route/backend/profile override resolver
  - budget verdict and rejected-candidate diagnostics
  - external readiness bridge into selection/dispatch gating
  - route explanation and routing validation commands
- Bounded responsibilities
  - carrier projection owns which candidate rows exist
  - runtime assignment owns candidate filtering/ranking
  - dispatch owns effective backend/profile application
  - status/init/consume surfaces own operator truth projection
  - taskflow operator surfaces own explain/validate entrypoints

### Data / State Model
- Important entities
  - `carrier_candidate`
  - `model_profile`
  - `effective_selected_backend`
  - `backend_selection_source`
  - `budget_verdict`
  - `route_profile_override_source`
  - `live_readiness_status`
- Receipts / runtime state / config fields
  - `agent_system.model_selection.enabled`
  - `agent_system.model_selection.candidate_scope`
  - `agent_system.model_selection.budget_policy.*`
  - route `budget_policy`
  - route `max_budget_units`
  - route `profiles`
  - route `fanout_min_results`
  - route `merge_policy`
  - `selected_carrier_id`
  - `selected_model_profile_id`
  - `effective_selected_backend`
  - `backend_selection_source`
  - `budget_verdict`
  - `rejected_candidates`
- Migration or compatibility notes
  - preserve legacy route/backend fields as readable config inputs while making their execution semantics explicit
  - preserve existing external CLI profile contracts, but downgrade unsupported reasoning controls to explicit metadata-only posture if necessary

### Integration Points
- APIs
  - status JSON
  - consume/init/continue/recovery JSON
  - new `route explain` / `validate-routing` command JSON
- Runtime-family handoffs
  - config projection -> runtime candidate pool
  - runtime candidate pool -> runtime assignment
  - runtime assignment + route policy -> effective backend/profile resolution
  - effective backend/profile resolution -> dispatch packets and receipts
  - dispatch packets and receipts -> operator status/recovery/continue summaries
- Cross-document / cross-protocol dependencies
  - `docs/product/spec/carrier-model-profile-selection-runtime-design.md`
  - `docs/product/spec/external-cli-carrier-hardening-design.md`
  - `docs/product/spec/implementation-backend-admissibility-and-selection-truth-design.md`
  - `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`
  - `docs/product/spec/release-1-operator-surface-contract.md`

### Bounded File Set
- `docs/product/spec/unified-hybrid-runtime-selection-policy-design.md`
- `docs/product/spec/current-spec-map.md`
- `docs/product/spec/current-spec-provenance-map.md`
- `docs/product/spec/README.md`
- `vida.config.yaml`
- `crates/vida/src/carrier_runtime_projection.rs`
- `crates/vida/src/runtime_assignment_builder.rs`
- `crates/vida/src/runtime_assignment_policy.rs`
- `crates/vida/src/runtime_lane_summary.rs`
- `crates/vida/src/taskflow_routing.rs`
- `crates/vida/src/development_flow_orchestration.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/runtime_dispatch_execution.rs`
- `crates/vida/src/status_surface_external_cli.rs`
- `crates/vida/src/status_surface_host_agents.rs`
- `crates/vida/src/taskflow_consume_bundle.rs`
- `crates/vida/src/init_surfaces.rs`
- `crates/vida/src/main.rs`
- `crates/vida/tests/project_routing_shape.rs`
- `crates/vida/tests/boot_smoke.rs`

## Fail-Closed Constraints
- Forbidden fallback paths
  - no silent route hard override over a more admissible dynamically selected candidate
  - no budget cap that exists only as documentation noise
  - no route `profiles:` map that appears operative while being ignored in dispatch
  - no live external blocked backend surviving as a “ready” initial candidate
- Required receipts / proofs / gates
  - operator surfaces must expose both dynamic selection truth and effective backend truth
  - routing validation must fail closed when config contains dead/unsupported route knobs
  - tests must prove `model_selection_enabled` follows the real flag semantics
  - tests must prove `internal_subagents` and route profile overrides actually affect runtime choice
- Safety boundaries that must remain true during rollout
  - delegated execution law remains unchanged
  - route hints remain visible for diagnosis
  - external backends that cannot enforce reasoning control must report that explicitly instead of faking execution support

## Implementation Plan

### Phase 1
- finalize this design and register it in active spec canon
- implement the selection-core seam so dynamic selection and effective backend truth stop diverging silently
- First proof target
  - `spec` task complete
  - focused tests prove effective backend policy is derived from the canonical selector path rather than route hard override

### Phase 2
- implement the parallel follow-up wave:
  - `internal_subagents` candidate-pool + route profile bridge
  - budget/routing policy liveness
  - external reasoning/live readiness operationalization
  - operator diagnostics surfaces
  - residual qwen drift cleanup
- Second proof target
  - route/profile/budget/readiness behavior is reflected consistently across dispatch and operator surfaces

### Phase 3
- run bounded proof/validation and confirm closure across config, runtime, docs, and tests
- Final proof target
  - one green closure wave for `proof` task with no residual selection-truth drift

## Validation / Proof
- Unit tests:
  - runtime assignment rejects/admits candidates by real budget policy
  - route `profiles:` changes actual selected profile resolution
  - `internal_subagents` appear in the unified candidate pool
  - `model_selection_enabled` follows the real `enabled` flag
- Integration tests:
  - targeted `cargo test -p vida` for routing/dispatch/status/init surfaces
  - command-level tests for `vida taskflow route explain --json` and `vida taskflow validate-routing --json`
- Runtime checks:
  - `vida status --json`
  - `vida taskflow consume agent-system --json`
  - `vida taskflow route explain --task-class implementation --runtime-role worker --json`
  - `vida taskflow validate-routing --json`
- Canonical checks:
  - `vida docflow protocol-coverage-check --profile active-canon`
  - `vida docflow check --root . docs/product/spec/unified-hybrid-runtime-selection-policy-design.md`
  - `vida docflow doctor --root . --show-warnings`

## Observability
- Logging points
  - candidate pool before/after budget/readiness filtering
  - effective backend selection source
  - route profile override resolution
  - live readiness blocker source
- Metrics / counters
  - rejected-candidate counts by reason
  - over-budget fallback selections
  - route-validation failures by knob type
- Receipts / runtime state written
  - routing validation receipts
  - updated dispatch/status/init/continue snapshots with canonical selection truth

## Rollout Strategy
- Development rollout
  - land selection-core first, then open the parallel wave, then close with proof
- Migration / compatibility notes
  - preserve existing config keys but fail closed on explicitly unsupported semantics
  - keep operator-visible downgrade states for backends that cannot enforce selected reasoning controls
- Operator or user restart / restart-notice requirements
  - any new command surfaces or status fields must be reflected in help/query surfaces and validated against operator contracts

## Future Considerations
- Follow-up ideas
  - integrate telemetry-driven ranking only after static policy truth is stable
  - add richer typed route-policy receipts beyond JSON summaries
- Known limitations
  - current environment may still require manual external CLI auth/model repair outside sandbox
- Technical debt left intentionally
  - do not widen into broader orchestration refactors beyond the selection/routing policy seam

## References
- Related specs
  - `docs/product/spec/carrier-model-profile-selection-runtime-design.md`
  - `docs/product/spec/external-cli-carrier-hardening-design.md`
  - `docs/product/spec/implementation-backend-admissibility-and-selection-truth-design.md`
  - `docs/product/spec/reconcile-qwen-cli-carrier-drift-design.md`
- Related protocols
  - `docs/process/project-orchestrator-operating-protocol.md`
  - `docs/process/team-development-and-orchestration-protocol.md`
  - `docs/process/project-development-packet-template-protocol.md`
- Related ADRs
  - none
- External references
  - external static audit provided by the user on 2026-04-23

-----
artifact_path: product/spec/unified-hybrid-runtime-selection-policy-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-23
schema_version: 1
status: canonical
source_path: docs/product/spec/unified-hybrid-runtime-selection-policy-design.md
created_at: 2026-04-23T06:48:11.627277958Z
updated_at: 2026-04-23T06:56:05.322963218Z
changelog_ref: unified-hybrid-runtime-selection-policy-design.changelog.jsonl
