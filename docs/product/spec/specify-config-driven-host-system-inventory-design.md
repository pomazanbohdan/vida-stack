# Specify Config Driven Host System Inventory Design

## Summary
- Change: make the active host-system inventory fully config-driven from `vida.config.yaml`, keep framework template roots available for all supported systems, and add `hermes` without introducing new hardcoded runtime-root or system lists.
- Owner layer: `launcher | project activation | host runtime`
- Status: proposed

## Current Context
- Active host-system selection is already read from `vida.config.yaml -> host_environment.systems` and `host_environment.cli_system`.
- Template materialization already uses per-system `template_root`, `runtime_root`, and `materialization_mode`.
- Some runtime/bootstrap paths still hardcode named runtime roots or a specific system runtime surface instead of reading the configured inventory.

## Goal
- The active host-system list must come only from `vida.config.yaml`.
- Built-in framework templates may still exist for every supported host system.
- Adding `hermes` must be possible by updating config/template inventory and bounded runtime plumbing, not by extending another hardcoded system list.

## Requirements
- Host runtime discovery must derive runtime roots from config/template inventory, not a fixed Rust array.
- Status/project-activation/runtime-dispatch must continue to use the selected configured system.
- Framework template inventory may include non-active systems.
- `hermes` must be available as:
  - host system template inventory entry,
  - external backend/subagent entry,
  - project-local template root.
- Documentation must distinguish:
  - template inventory for all systems,
  - active configured systems in `vida.config.yaml`.

## Bounded File Set
- `vida.config.yaml`
- `docs/framework/templates/vida.config.yaml.template`
- `.hermes/README.md`
- `crates/vida/src/host_runtime_registry.rs`
- `crates/vida/src/project_activator_surface.rs`
- `crates/vida/src/status_surface_host_cli_system.rs`
- `crates/vida/src/runtime_dispatch_state.rs`
- `crates/vida/src/carrier_runtime_projection.rs`
- `crates/vida/src/init_surfaces.rs`
- `crates/vida/src/main.rs`
- `docs/process/agent-system.md`
- `docs/process/environments.md`

## Design Decisions
### 1. Template inventory and active inventory are separate
- Framework template roots remain available as install/materialization sources.
- Runtime selection, status, and dispatch treat only configured systems as active.

### 2. Runtime-root discovery becomes config-backed
- Replace fixed runtime-root enumeration with inventory derived from `vida.config.yaml`, with template-config fallback where the project config is unavailable.

### 3. Hermes is added as config inventory, not as a new special case
- Add `hermes` entries to config/template inventory and project template roots.
- Keep dispatch wiring declarative through the existing host-system/subagent config model.

## Implementation Plan
1. Update config/template inventory to include `hermes` and stop using `kilo` as the active external-planning placeholder.
2. Remove or narrow hardcoded runtime-root/system lists in launcher/runtime bootstrap code.
3. Update docs and tests so they describe template inventory vs active configured systems correctly.

## Validation / Proof
- `cargo test -p vida`
- targeted launcher/runtime tests for host-system selection and external backend dispatch
- `vida docflow check --root . docs/product/spec/specify-config-driven-host-system-inventory-design.md`

-----
artifact_path: product/spec/specify-config-driven-host-system-inventory-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-08
schema_version: 1
status: canonical
source_path: docs/product/spec/specify-config-driven-host-system-inventory-design.md
created_at: 2026-04-08T21:02:49.540320126Z
updated_at: 2026-04-08T21:03:29.677602577Z
changelog_ref: specify-config-driven-host-system-inventory-design.changelog.jsonl
