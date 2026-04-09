# Config Driven Host System Runtime Keep Design

Status: implemented

Bounded feature/change design implemented in the current project runtime/config surfaces.

## Summary
- Feature / change: Keep framework templates available for all host CLI systems, but derive the active host-system list, runtime roots, and selection behavior only from `vida.config.yaml`; remove hardcoded host-system inventories from runtime logic; add Hermes as a template/config-backed external system.
- Owner layer: `mixed`
- Runtime surface: `project activation | launcher | taskflow`
- Status: implemented

## Current Context
- Existing system overview
  - Project activation already reads `host_environment.systems` from `vida.config.yaml` and uses each selected entry's `template_root`, `runtime_root`, `materialization_mode`, and `execution_class`.
  - Runtime dispatch already resolves the selected host system from config and uses `agent_system.subagents` as the external backend registry.
  - Framework templates exist as inventory roots such as `.codex`, `.qwen`, `.kilo`, and `.opencode`.
- Key components and relationships
  - `crates/vida/src/project_activator_surface.rs` owns host-system registry parsing and materialization.
  - `crates/vida/src/status_surface_host_cli_system.rs` and `crates/vida/src/runtime_dispatch_state.rs` resolve the selected active system from config.
  - `crates/vida/src/host_runtime_registry.rs` and `crates/vida/src/carrier_runtime_projection.rs` still contain hardcoded runtime-root assumptions.
  - `vida.config.yaml` and `docs/framework/templates/vida.config.yaml.template` define the user-visible system catalog.
- Current pain point or gap
  - Some runtime/bootstrap paths still assume a fixed built-in host-system set instead of deriving the active inventory from config.
  - Docs and tests still overstate named systems as framework-owned canonical lists.
  - Adding Hermes currently requires code changes in places that should be config-driven.

## Goal
- What this change should achieve
  - Make active host-system discovery and runtime-root lookup derive from `vida.config.yaml`.
  - Preserve framework template roots as reusable inventory surfaces without making them the active-system source of truth.
  - Introduce Hermes through template/config/runtime wiring without adding a new named-system registry in code.
- What success looks like
  - The selected and enabled host systems come from `vida.config.yaml -> host_environment.systems`.
  - Runtime code no longer relies on a fixed `[".codex", ".qwen", ".kilo", ".opencode"]` list to detect host runtime roots.
  - Hermes can be selected and materialized through project activation using config and template surfaces only.
  - Tests prove config-driven host-system discovery and Hermes template/config behavior.
- What is explicitly out of scope
  - Reworking delegated lane law or carrier selection strategy.
  - Building a Hermes-specific execution bridge beyond the existing external CLI adapter model.
  - Removing the framework template inventory for legacy systems.

## Requirements

### Functional Requirements
- Must derive the active host-system registry from `vida.config.yaml -> host_environment.systems`.
- Must treat template roots as inventory/materialization sources, not as the active-system authority.
- Must let runtime-root detection succeed when at least one configured runtime root is materialized, without requiring a hardcoded built-in list.
- Must add Hermes as a selectable external host system and external CLI subagent/backend through config/template surfaces.
- Must keep existing config-driven systems functional after the refactor.

### Non-Functional Requirements
- Performance
  - Registry/runtimes lookup must remain lightweight and local-file based.
- Scalability
  - Adding another system after Hermes should not require a new Rust constant list.
- Observability
  - Status and activation views must continue to report the selected system, execution class, and runtime root.
- Security
  - The change must not widen root-session write authority or bypass delegated execution rules.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/process/agent-system.md`
  - `docs/process/environments.md`
  - `docs/product/spec/config-driven-host-system-runtime-keep-design.md`
- Framework protocols affected:
  - none
- Runtime families affected:
  - `launcher`
  - `taskflow`
- Config / receipts / runtime surfaces affected:
  - `vida.config.yaml`
  - `docs/framework/templates/vida.config.yaml.template`
  - `vida project-activator`
  - `vida status --json`
  - `vida taskflow consume agent-system --json`

## Design Decisions

### 1. Active host-system inventory stays config-driven
Will implement / choose:
- Use `host_environment.systems` as the only active host-system registry for selection, enablement, runtime-root lookup, and host-system reporting.
- Why
  - The user-visible active system catalog already lives in config, so runtime should not maintain a second canonical list.
- Trade-offs
  - Some bootstrap fallback helpers will need config-aware lookup instead of fixed root-name iteration.
- Alternatives considered
  - Keep a built-in fallback list and only append Hermes.
  - Rejected because that preserves dual authority.
- ADR link if this must become a durable decision record
  - none

### 2. Template inventory remains broad, but not authoritative
Will implement / choose:
- Keep framework templates for all supported systems and add Hermes template assets, while clarifying in docs that templates are inventory/materialization sources only.
- Why
  - This preserves reusable built-in scaffolds without conflating them with the active runtime registry.
- Trade-offs
  - There will still be template directories for systems that are disabled in a given project.
- Alternatives considered
  - Delete unused template roots and force projects to vendor their own copies.
  - Rejected because template inventory is still useful framework bootstrap content.
- ADR link if needed
  - none

## Technical Design

### Core Components
- Main components
  - host-system registry parsing and runtime-root lookup
  - project activation materialization
  - carrier/runtime projection
  - external CLI backend config for Hermes
- Key interfaces
  - `host_cli_system_registry_with_fallback`
  - `looks_like_host_runtime_source_root`
  - `build_carrier_runtime_projection`
  - `vida project-activator --host-cli-system <system>`
- Bounded responsibilities
  - config owns active host systems and their runtime roots
  - template roots remain framework inventory sources
  - runtime projection must stop assuming Codex-only fixed roots where a config-derived path exists

### Data / State Model
- Important entities
  - host-system registry entry
  - template root
  - runtime root
  - external backend entry
- Receipts / runtime state / config fields
  - `host_environment.cli_system`
  - `host_environment.systems.<system>.template_root`
  - `host_environment.systems.<system>.runtime_root`
  - `host_environment.systems.<system>.execution_class`
  - `agent_system.subagents.<backend>`
- Migration or compatibility notes
  - Existing systems remain readable from config.
  - Hermes is additive.
  - Runtime-root detection should tolerate projects that only materialize one configured system.

### Integration Points
- APIs
  - none external beyond existing CLI adapters
- Runtime-family handoffs
  - project activation materializes the selected runtime root
  - status/dispatch resolve the active system from config
- Cross-document / cross-protocol dependencies
  - project activation model
  - agent-system/project-operations docs
  - delegated execution law remains unchanged

### Bounded File Set
- `vida.config.yaml`
- `docs/framework/templates/vida.config.yaml.template`
- `.hermes/README.md`
- `crates/vida/src/host_runtime_registry.rs`
- `crates/vida/src/carrier_runtime_projection.rs`
- `crates/vida/src/project_activator_surface.rs`
- `crates/vida/src/status_surface_host_cli_system.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/init_surfaces.rs`
- `crates/vida/src/main.rs`
- `docs/process/agent-system.md`
- `docs/process/environments.md`
- `docs/product/spec/config-driven-host-system-runtime-keep-design.md`

## Fail-Closed Constraints
- Forbidden fallback paths
  - No new hardcoded named-system registry in Rust as the active source of truth.
  - No Hermes-specific bypass path that skips config-backed system entries.
- Required receipts / proofs / gates
  - Config-backed project activation must still report the selected system and runtime root.
  - Tests must prove host-runtime detection succeeds from config-derived roots.
  - Hermes activation/dispatch config must be representable without a new built-in constant list.
- Safety boundaries that must remain true during rollout
  - Delegated execution law stays unchanged.
  - Existing external and internal systems remain compatible.
  - Template inventory can stay broader than the enabled active list.

## Implementation Plan

### Phase 1
- Fill this design with the config-driven authority rules and bounded file set.
- Update config/template inventory with Hermes and remove Kilo from the active project config where needed.
- First proof target
  - `vida docflow check --root . docs/product/spec/config-driven-host-system-runtime-keep-design.md`

### Phase 2
- Refactor runtime-root discovery and carrier projection to stop depending on fixed host-root constants.
- Keep selection/materialization surfaces config-driven end-to-end.
- Second proof target
  - targeted `cargo test -p vida ...` for host-system discovery, activation, and dispatch helpers

### Phase 3
- Update docs/tests to describe template inventory vs active config authority.
- Rebuild the binary and continue the next delegated cycle.
- Final proof target
  - green bounded tests plus `cargo test -p vida`

## Validation / Proof
- Unit tests:
  - host-system registry and runtime-root detection tests
  - Hermes config/template activation tests
  - runtime dispatch helper tests for config-derived external systems
- Integration tests:
  - `vida project-activator --host-cli-system hermes --json`
  - `vida status --json`
- Runtime checks:
  - `vida taskflow consume agent-system --json`
  - `vida project-activator --json`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/config-driven-host-system-runtime-keep-design.md`
  - `cargo test -p vida`

## Implementation Outcome
- Implemented in bounded runtime, config, and documentation surfaces.
- Hermes is selectable and materializable through config/template surfaces only.
- Active host-system discovery and runtime-root selection no longer depend on a fixed built-in root list.
- Proof was completed with targeted docflow, activation, dispatch, and smoke checks; the remaining long-running `memory_surface_fails_closed_when_governance_linkage_missing` smoke is outside this epic's bounded file set.

## Observability
- Logging points
  - none new required
- Metrics / counters
  - none
- Receipts / runtime state written
  - project activation receipt
  - runtime consumption/status snapshots

## Rollout Strategy
- Development rollout
  - land config/template additions and runtime refactor in one bounded change set
- Migration / compatibility notes
  - framework inventory may still ship templates for systems not enabled in a given project
  - project config decides the active list
- Operator or user restart / restart-notice requirements
  - re-run project activation if selecting Hermes
  - rebuild installed `vida` binary after the code change

## Future Considerations
- Follow-up ideas
  - move more Codex-specific carrier projection logic behind config-shaped adapters
  - add a generic helper for template inventory enumeration from config + framework source roots
- Known limitations
  - carrier projection still has Codex-specific behavior where the selected materialization mode is `codex_toml_catalog_render`
- Technical debt left intentionally
  - deeper genericization of carrier-role projection beyond this host-system inventory slice

## References
- Related specs
  - `docs/product/spec/internal-codex-agent-execution-fail-closed-design.md`
  - `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- Related protocols
  - `docs/process/project-orchestrator-operating-protocol.md`
  - `docs/process/documentation-tooling-map.md`
- Related ADRs
  - none
- External references
  - none

-----
artifact_path: product/spec/config-driven-host-system-runtime-keep-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-04-09'
schema_version: '1'
status: canonical
source_path: docs/product/spec/config-driven-host-system-runtime-keep-design.md
created_at: '2026-04-08T21:02:35.297108952Z'
updated_at: 2026-04-08T21:30:25.30536044Z
changelog_ref: config-driven-host-system-runtime-keep-design.changelog.jsonl
