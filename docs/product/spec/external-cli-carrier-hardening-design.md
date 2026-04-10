# External CLI Carrier Hardening Design

Status: draft

Use this design to record the bounded runtime hardening slice for external CLI carriers before implementation.

## Summary
- Feature / change: harden external CLI carrier dispatch so VIDA can execution-enforce model/provider intent, report truthful carrier readiness, and keep project config aligned with the actually working external carriers.
- Owner layer: `mixed`
- Runtime surface: `launcher | taskflow | status`
- Status: draft

## Current Context
- Existing system overview
  - `agent_system.subagents` is already the canonical executor registry in `vida.config.yaml`.
  - external CLI dispatch already reads `dispatch.command`, `dispatch.static_args`, `dispatch.workdir_flag`, and `dispatch.prompt_mode`.
  - external CLI preflight already reports `sandbox_active`, `network_reachable`, and tool-contract completeness.
- Key components and relationships
  - `crates/vida/src/runtime_dispatch_state.rs` builds the actual external activation command.
  - `crates/vida/src/status_surface_external_cli.rs` emits transport-level readiness for external CLI use.
  - `crates/vida/src/release1_contracts.rs` owns blocker-code vocabulary and tool-contract helpers.
  - `vida.config.yaml` already declares `default_model` and `models_hint` for some carriers, but dispatch does not execution-enforce them.
- Current pain point or gap
  - project config can declare a canonical model, but runtime dispatch still falls through to carrier-local recent/default state.
  - preflight can say `pass` even when the only reachable provider/model path is not actually runnable.
  - `opencode` demonstrates the defect clearly: an explicit pinned model works, but the default provider/model path can still fail due to carrier-local drift.
  - `kilo` and `vibe` have working runtime behavior, but the active project config does not yet reflect that reality as first-class bounded carrier policy.

## Goal
- What this change should achieve
  - make external CLI model/provider selection execution-enforced when the carrier exposes CLI flags for it
  - distinguish transport readiness from carrier readiness in status/preflight
  - normalize the project carrier registry so `opencode`, `kilo`, and `vibe` are represented according to the researched working paths
  - give operators one bounded smoke-proof path for sandbox posture, auth state, model fixation, and one-shot validation
- What success looks like
  - `default_model` stops being metadata-only for carriers that support CLI model flags
  - operator status can distinguish `sandbox_blocked`, `interactive_auth_required`, `provider_auth_failed`, and `model_not_pinned`
  - `opencode` dispatch is forced onto the canonical working model instead of ambient recent-model drift
  - `kilo` is represented as a first-class external carrier in config and routing
  - the project has one repeatable proof path for external carrier readiness
- What is explicitly out of scope
  - redesigning carrier pricing/scoring strategy
  - implementing a new external dispatch protocol beyond the existing external CLI adapter
  - widening root-session write authority or bypassing delegated execution law

## Requirements

### Functional Requirements
- Must let external CLI dispatch inject `default_model` into the actual activation command when the carrier supports a model flag.
- Must support optional provider pinning for carriers that expose a provider flag.
- Must keep carriers without direct CLI model/provider flags admissible through config-driven readiness rules rather than forcing fake flags.
- Must extend external CLI readiness so operator-visible status is not limited to sandbox/network/tool-contract checks.
- Must keep existing `qwen`, `hermes`, and `opencode` flows compatible while adding `kilo` and deciding `vibe` posture explicitly.

### Non-Functional Requirements
- Clarity
  - runtime state must make it obvious whether a failure is transport, auth, provider, or model-drift related
- Compatibility
  - legacy config entries without explicit model/provider flags must continue to parse
- Observability
  - readiness surfaces must emit stable blocker/status values suitable for operator automation
- Security
  - no new fallback may silently ignore declared model/provider policy

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/external-cli-carrier-hardening-design.md`
  - `docs/process/agent-system.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `launcher`
  - `taskflow`
  - `status`
- Config / receipts / runtime surfaces affected:
  - `vida.config.yaml`
  - `vida status --json`
  - external CLI dispatch command rendering

## Design Decisions

### 1. Dispatch must execution-enforce pinned model/provider intent
Will implement / choose:
- add explicit dispatch-level model/provider flag support and wire `default_model` into the actual command line for carriers that support it
- Why
  - project-owned carrier intent must beat carrier-local recent/default state
- Trade-offs
  - dispatch schema becomes slightly richer and scaffold generation must stay in sync
- Alternatives considered
  - rely on carrier-local config/state normalization only
  - rejected because it does not make project config authoritative at execution time
- ADR link if this must become a durable decision record
  - none

### 2. Carrier readiness must be richer than transport preflight
Will implement / choose:
- separate transport/tool-contract pass from carrier-specific readiness classifications
- Why
  - a reachable CLI with incomplete provider auth is not actually ready for execution
- Trade-offs
  - status surfaces and blocker vocab need expansion
- Alternatives considered
  - keep a single `pass/blocked` surface and leave deeper diagnosis to logs
  - rejected because it hides actionable operator state
- ADR link if needed
  - none

### 3. `vibe` remains config-driven even if other carriers are flag-driven
Will implement / choose:
- allow carriers to declare config-driven readiness when they do not expose direct CLI model/provider flags
- Why
  - `vibe` has a real working path, but its model selection lives in config rather than programmatic CLI flags
- Trade-offs
  - readiness logic must handle two valid carrier modes: flag-driven and config-driven
- Alternatives considered
  - force every carrier into one CLI-flag abstraction
  - rejected because that would misrepresent working carrier behavior
- ADR link if needed
  - none

## Technical Design

### Core Components
- Main components
  - external CLI dispatch schema
  - runtime dispatch command builder
  - external CLI readiness/preflight classifier
  - carrier registry entries in `vida.config.yaml`
- Key interfaces
  - `configured_external_activation_command`
  - `configured_external_activation_parts`
  - `external_cli_preflight_summary`
  - release blocker-code / tool-contract helpers
- Bounded responsibilities
  - config declares canonical carrier intent
  - runtime injects admissible flags into real execution commands
  - readiness surfaces classify actual carrier usability
  - docs describe operator procedure and proof targets

### Data / State Model
- Important entities
  - external CLI backend entry
  - flag-driven carrier
  - config-driven carrier
  - transport readiness
  - carrier readiness
- Receipts / runtime state / config fields
  - `agent_system.subagents.<backend>.default_model`
  - `agent_system.subagents.<backend>.models_hint`
  - `agent_system.subagents.<backend>.dispatch.model_flag`
  - `agent_system.subagents.<backend>.dispatch.provider_flag`
  - `agent_system.subagents.<backend>.dispatch.provider_value`
  - `host_agents.external_cli_preflight`
- Migration or compatibility notes
  - missing `dispatch.model_flag` / `dispatch.provider_flag` remains legal
  - carriers that do not declare flags remain config-driven rather than broken

### Integration Points
- APIs
  - external carrier CLIs only
- Runtime-family handoffs
  - status/preflight must stay aligned with dispatch semantics
  - config scaffold must emit the same dispatch schema runtime expects
- Cross-document / cross-protocol dependencies
  - `docs/process/agent-system.md`
  - `docs/product/spec/config-driven-host-system-runtime-keep-design.md`
  - `docs/product/spec/hybrid-host-executor-semantics-host-environment-design.md`

### Bounded File Set
- `docs/product/spec/external-cli-carrier-hardening-design.md`
- `vida.config.yaml`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/status_surface_external_cli.rs`
- `crates/vida/src/release1_contracts.rs`
- `crates/vida/src/main.rs`
- `docs/process/agent-system.md`

## Fail-Closed Constraints
- Forbidden fallback paths
  - no external dispatch path may silently ignore declared model/provider intent when the carrier supports flag-driven pinning
  - no readiness surface may report `pass` when the working provider/model path is known broken
  - no carrier-local auth/state file becomes the canonical project authority
- Required receipts / proofs / gates
  - dispatch tests must prove model/provider pinning enters the rendered command line
  - status tests must prove carrier readiness states differentiate transport/auth/model failures
  - operator proof must include at least one working one-shot path per enabled external carrier
- Safety boundaries that must remain true during rollout
  - delegated execution law remains unchanged
  - sandbox-off remains an explicit operator prerequisite for interactive auth/model repair work
  - carriers without direct CLI flags remain config-driven rather than force-fit into a false schema

## Implementation Plan

### Phase 1
- add dispatch-level model/provider flag support
- harden `opencode` around execution-enforced pinned model behavior
- First proof target
  - targeted dispatch-builder tests plus one working `opencode` one-shot through the pinned model path

### Phase 2
- extend readiness/blocker classification for sandbox, auth, provider, and model drift
- wire the new vocabulary into status/preflight surfaces
- Second proof target
  - targeted status/preflight tests covering transport pass vs provider/model failure states

### Phase 3
- normalize carrier profiles for `kilo` and `vibe`
- update route fanout and operator procedure/proof paths
- Final proof target
  - bounded smoke checks for enabled carriers plus docflow and cargo test coverage

## Validation / Proof
- Unit tests:
  - dispatch command generation for qwen/hermes/opencode/kilo
  - blocker-code/status mapping for external CLI readiness
- Integration tests:
  - `vida status --json`
  - bounded external CLI one-shot runs for enabled carriers
- Runtime checks:
  - `vida taskflow consume agent-system --json`
  - `vida task ready --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/external-cli-carrier-hardening-design.md`
  - `cargo test -p vida`

## Observability
- Logging points
  - selected carrier/backend id
  - selected model/provider pin
  - readiness classification transitions
- Metrics / counters
  - none new in this bounded slice beyond existing status evidence
- Receipts / runtime state written
  - external CLI preflight/status snapshots
  - taskflow/runtime dispatch evidence that reflects the selected backend configuration

## Rollout Strategy
- Development rollout
  - land design doc first, then dispatch/runtime changes, then carrier-config normalization, then proofs
- Migration / compatibility notes
  - legacy config remains readable while richer dispatch keys are additive
  - carrier-local state may still exist, but project config becomes the execution authority
- Operator or user restart / restart-notice requirements
  - external CLI auth/model repair still requires the operator to run outside sandbox when interactive activation is needed

## Future Considerations
- Follow-up ideas
  - deeper provider-health checks beyond bounded one-shot smoke validation
  - first-class typed carrier readiness receipts instead of JSON-only summary state
- Known limitations
  - `vibe` remains structurally different from qwen/hermes/opencode/kilo because its model selection is config-driven
- Technical debt left intentionally
  - no new global carrier orchestration layer beyond the existing external CLI adapter family

## References
- Related specs
  - `docs/product/spec/config-driven-host-system-runtime-keep-design.md`
  - `docs/product/spec/hybrid-host-executor-semantics-host-environment-design.md`
- Related protocols
  - `docs/process/agent-system.md`
- Related ADRs
  - none
- External references
  - CLI help/output and local carrier config/state inspection recorded during the bounded external-carrier research cycle on 2026-04-10

-----
artifact_path: product/spec/external-cli-carrier-hardening-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: 2026-04-10
schema_version: '1'
status: canonical
source_path: docs/product/spec/external-cli-carrier-hardening-design.md
created_at: '2026-04-10T08:05:00+03:00'
updated_at: 2026-04-10T07:52:55.524470406Z
changelog_ref: external-cli-carrier-hardening-design.changelog.jsonl
