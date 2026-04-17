# Export Canonical Operator Command Map Through Design

Status: `proposed`

Use this document for one bounded feature/change design before implementation.

## Summary
- Feature / change: export the canonical operator command map through `vida orchestrator-init` and `vida agent-init` so both surfaces expose the current command families in machine-readable JSON and aligned human-readable help/report output.
- Owner layer: `mixed`
- Runtime surface: `launcher`
- Status: `proposed`

## Current Context
- Existing system overview:
  - `vida orchestrator-init` and `vida agent-init` already render bounded startup projections over the compiled runtime bundle.
  - Root help already exposes the major command families through `crates/vida/src/surface_render.rs`.
  - The init views already export startup metadata such as `minimum_commands`, runtime bundle summary, activation semantics, and selected startup capsules.
- Key components and relationships:
  - `crates/vida/src/taskflow_runtime_bundle.rs` builds the canonical `orchestrator_init_view` and `agent_init_view` JSON payloads.
  - `crates/vida/src/init_surfaces.rs` merges project activation state into those views and renders both JSON and plain-text envelopes.
  - `crates/vida/src/surface_render.rs` owns the frozen root help family wording that currently lists the operator command families separately from init-view output.
  - `crates/vida/tests/boot_smoke.rs` proves current init-surface behavior and activation-view semantics.
- Current pain point or gap:
  - The runtime already knows the operator command families, but the init surfaces do not export one canonical command-family map that downstream operators, agents, and tests can reuse.
  - `minimum_commands` is useful but too narrow: it is a startup checklist, not a stable operator command catalog grouped by family.
  - Human-readable init output and machine-readable init JSON can drift from root help because the command-family model is not exported from one shared view contract.

## Goal
- What this change should achieve:
  - Define a bounded command-map projection that both `vida orchestrator-init` and `vida agent-init` must export.
  - Keep root help, init JSON, and init plain-text output aligned around the same command-family vocabulary.
  - Give the orchestrator enough structure to shape later work-pool and development packets without re-deriving the command surface from scattered help text.
- What success looks like:
  - Both init JSON views expose a stable operator-command-map block covering the current command/help surfaces.
  - Plain-text init output summarizes the same families in a compact human-readable section.
  - Tests prove parity between root help wording and init-surface command-family projections for the bounded families in scope.
- What is explicitly out of scope:
  - Adding new root commands or changing command semantics.
  - Replacing `minimum_commands`; this feature augments, not removes, the startup checklist.
  - Extending the contract to every TaskFlow or DocFlow subcommand.
  - Any implementation beyond the bounded design/spec artifact in this packet.

## Requirements

### Functional Requirements
- Must define one canonical operator-command-map projection for the current root/init/help surfaces.
- Must cover the current in-scope command families already visible at the root operator layer:
  - bootstrap/init family
  - protocol/help discovery family
  - status/doctor family
  - task/taskflow family
  - docflow/documentation family
  - lane/approval/recovery family
  - project activation family
  - agent feedback / memory family when already present on the root surface
- Must add machine-readable JSON fields to both `vida orchestrator-init --json` and `vida agent-init --json`.
- Must add compact human-readable command-family reporting to the plain-text init surfaces.
- Must keep `vida --help` as the authoritative human-readable root command-family wording, while exporting that wording through the init views rather than re-inventing separate prose.
- Must preserve the current distinction that `vida agent-init` is an activation/view surface and does not itself execute the packet unless an explicit execution surface is used.

### Non-Functional Requirements
- Performance:
  - The new command-map export must be derived from already available launcher/runtime data and must not trigger broad repo scans.
- Scalability:
  - The schema must allow adding future command families without renaming existing keys.
- Observability:
  - The exported map must be explicit enough for smoke tests and operator diagnostics to assert family presence and surface parity.
- Security:
  - The export must stay descriptive only; it must not imply new execution authority or bypass existing fail-closed routing.

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/export-canonical-operator-command-map-through-design.md`
  - `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`
  - `docs/product/spec/compiled-runtime-bundle-contract.md`
  - `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
- Framework protocols affected:
  - none in this packet; later implementation may need bounded updates only if runtime/help wording ownership is promoted out of launcher code.
- Runtime families affected:
  - launcher root help
  - orchestrator-init view
  - agent-init view
- Config / receipts / runtime surfaces affected:
  - `vida --help`
  - `vida orchestrator-init`
  - `vida agent-init`
  - no new receipts or state-store artifacts in scope

## Design Decisions

### 1. Export one shared operator-command-map projection instead of duplicating family prose
Will implement / choose:
- Add one canonical command-family projection that both init views can embed.
- Treat root help as the human-readable wording source and init-view exports as structured projections of that source.
- Keep `minimum_commands` for bounded startup actions while adding a separate grouped map for broader operator discovery.
- ADR link if this must become a durable decision record:
  - none currently required

Why:
- The current split between root help and init views causes discoverability drift risk.
- Operators and delegated lanes need a compact grouped map, not only a flat command checklist.

Trade-offs:
- Slightly larger init JSON payloads.
- A shared projection helper introduces one more launcher-owned contract that tests must keep stable.

Alternatives considered:
- Expanding `minimum_commands` only.
  - Rejected because it conflates startup necessities with a canonical operator-family map.
- Leaving the command catalog only in `vida --help`.
  - Rejected because delegated/runtime consumers need a machine-readable init-surface export.

### 2. Scope the first slice to current root/init/help families only
Will implement / choose:
- Limit the design to families already surfaced by root help and relevant init/startup usage.
- Exclude deep TaskFlow/DocFlow subcommand catalogs from this slice.
- Keep the exported family members at the command-family level, with optional representative commands/examples instead of exhaustive subcommand lists.
- ADR link if needed:
  - none

Why:
- The packet explicitly asks for the current init/help surfaces, not a full runtime command encyclopedia.
- Bounded family export is enough for orchestrator shaping and parity proofs.

Trade-offs:
- Consumers still need family-specific help for deep subcommands.
- Later follow-up may be needed if the product wants subcommand-level catalogs.

Alternatives considered:
- Exporting the full subcommand tree now.
  - Rejected as too broad for the current bounded packet.

### 3. Keep orchestrator and agent command maps aligned but role-filtered where required
Will implement / choose:
- Both init views export the same top-level family schema.
- `vida orchestrator-init` may include orchestrator-only guidance families such as status/recovery/continue-oriented inspection.
- `vida agent-init` must either omit or explicitly mark root-only families as not lawful from the delegated lane.
- The shared schema must support `availability` or `lane_scope` style markers rather than silently dropping meaning.
- ADR link if needed:
  - none

Why:
- The orchestrator and delegated lane need parity on vocabulary, but not parity on lawful actions.
- The packet itself requires preserving the delegated-lane/root-lane boundary.

Trade-offs:
- Slightly more schema complexity than one flat list.

Alternatives considered:
- Emitting identical unrestricted command lists for both surfaces.
  - Rejected because it would blur root-only versus delegated-lane-safe actions.

## Technical Design

### Core Components
- Main components:
  - shared launcher helper that materializes the canonical operator command-family map
  - `build_orchestrator_init_view` export path
  - `build_agent_init_view` export path
  - plain-text init renderers in `init_surfaces.rs`
  - root help renderer in `surface_render.rs`
- Key interfaces:
  - one JSON block attached to each init surface, tentatively named `operator_command_map`
  - one compact plain-text section rendered from the same projection, tentatively named `command families`
- Bounded responsibilities:
  - `surface_render.rs`: authoritative human-readable family labels on the root help surface
  - `taskflow_runtime_bundle.rs`: canonical machine-readable family projection for init views
  - `init_surfaces.rs`: envelope rendering and lane-specific visibility hints
  - `boot_smoke.rs`: proof that root help and init exports stay aligned

### Data / State Model
- Important entities:
  - `operator_command_map`
  - `command_family`
  - lane/surface availability markers
- Receipts / runtime state / config fields:
  - no new runtime receipts
  - no new state-store rows
  - no new `vida.config.yaml` policy required for this bounded slice
- Migration or compatibility notes:
  - this is additive schema growth for `orchestrator-init --json` and `agent-init --json`
  - current consumers of `minimum_commands`, `selection`, `activation_semantics`, and `runtime_bundle_summary` must remain compatible

Proposed JSON shape:

```json
{
  "surface": "vida orchestrator-init",
  "init": {
    "...": "existing fields remain",
    "operator_command_map": {
      "schema_version": "v1",
      "source_surface": "vida --help",
      "families": [
        {
          "family_id": "bootstrap",
          "label": "Bootstrap",
          "commands": ["init", "boot", "orchestrator-init", "agent-init", "project-activator"],
          "lane_scope": "shared",
          "notes": "bootstrap and activation entry surfaces"
        },
        {
          "family_id": "runtime_status",
          "label": "Runtime Status",
          "commands": ["status", "doctor"],
          "lane_scope": "orchestrator_preferred",
          "notes": "bounded runtime inspection surfaces"
        }
      ]
    }
  }
}
```

Agent-surface rule:
- `vida agent-init --json` should reuse the same shape but may mark selected families with `lane_scope = "root_only"` or `availability = "view_only_reference"` when the delegated lane must not run those commands directly.

### Integration Points
- APIs:
  - root help printing in `crates/vida/src/surface_render.rs`
  - init JSON assembly in `crates/vida/src/taskflow_runtime_bundle.rs`
  - init plain-text rendering in `crates/vida/src/init_surfaces.rs`
- Runtime-family handoffs:
  - no new handoff surfaces
  - exported families may point operators toward `vida taskflow help` and `vida docflow help` as family entrypoints rather than enumerating full subcommands
- Cross-document / cross-protocol dependencies:
  - `bootstrap-carriers-and-project-activator-model.md` owns the startup-surface meaning
  - `compiled-runtime-bundle-contract.md` owns the view/projection model
  - `user-facing-runtime-flow-and-operating-loop-model.md` may need later wording alignment if user-visible operator journey text cites the new export

### Bounded File Set
- Spec/design file for this packet:
  - `docs/product/spec/export-canonical-operator-command-map-through-design.md`
- Expected owner/runtime files for later implementation:
  - `crates/vida/src/surface_render.rs`
  - `crates/vida/src/taskflow_runtime_bundle.rs`
  - `crates/vida/src/init_surfaces.rs`
  - `crates/vida/tests/boot_smoke.rs`
- Possible follow-up owner-doc alignment files if implementation changes product wording:
  - `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`
  - `docs/product/spec/compiled-runtime-bundle-contract.md`
  - `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`

## Fail-Closed Constraints
- Forbidden fallback paths:
  - do not synthesize command-family truth independently in multiple renderers
  - do not treat the exported command map as authorization to run root-only commands from a delegated lane
  - do not widen this slice into full TaskFlow/DocFlow subcommand export
- Required receipts / proofs / gates:
  - smoke tests must prove init JSON contains the new command-map block
  - smoke tests must prove `agent-init` still reports view-only activation semantics where applicable
  - plain-text output must continue to describe `agent-init` as a view/activation surface, not implicit execution evidence
- Safety boundaries that must remain true during rollout:
  - root help remains the canonical human-readable root operator surface
  - `vida agent-init` remains non-executing unless an explicit execution path is invoked
  - lane legality must remain explicit even when family names are shared across surfaces

## Implementation Plan

### Phase 1
- Introduce one shared launcher-owned operator-command-family projection for the bounded root families.
- Add the projection to `orchestrator_init_view` and `agent_init_view`.
- First proof target:
  - JSON smoke tests assert `operator_command_map.schema_version` and family presence on both init surfaces.

### Phase 2
- Render compact `command families` output in plain-text `vida orchestrator-init` and `vida agent-init`.
- Align root help family labels with the exported projection.
- Land one bounded operator-facing help slice on `vida task` / `vida taskflow` so task sequencing and parallel-safe scheduling are discoverable from primary help entrypoints and deterministic query routing.
- Second proof target:
  - output-parity tests assert representative family names/commands stay aligned between root help and init output.
  - `vida task --help`, `vida taskflow --help`, `vida task help parallelism`, and `vida taskflow query "what can run in parallel with the current task"` stay aligned with live `graph-summary` scheduling fields.

### Phase 3
- Harden lane-scope markers and prove that delegated-lane exports do not blur root-only authority.
- Finalize related product wording if implementation changes user-visible operator guidance.
- Final proof target:
  - view-only `agent-init` semantics remain explicit alongside the new command-map export.

## Validation / Proof
- Unit tests:
  - helper-level tests for command-family projection shape and lane-scope markers
- Integration tests:
  - `crates/vida/tests/boot_smoke.rs`
  - bounded init-surface JSON assertions
  - bounded root-help/init-output parity assertions
- Runtime checks:
  - `cargo test -p vida boot_smoke orchestrator_init_renders_compiled_startup_view_json`
  - `cargo test -p vida boot_smoke agent_init_renders_worker_startup_view_json_for_explicit_role`
  - `cargo test -p vida boot_smoke agent_init_dispatch_packet_reports_view_only_activation_semantics`
  - `cargo test -p vida taskflow_query_answer -- --nocapture`
- Canonical checks:
  - `vida docflow check --root . docs/product/spec/export-canonical-operator-command-map-through-design.md`
  - `vida docflow doctor --root . --layer 3`

## Observability
- Logging points:
  - no new dedicated logs required for the design slice
- Metrics / counters:
  - none required; this is a render/export contract change
- Receipts / runtime state written:
  - none

## Rollout Strategy
- Development rollout:
  - implement behind the existing init/help surfaces with additive JSON keys and compact plain-text additions
- Migration / compatibility notes:
  - preserve all current top-level JSON fields
  - keep `minimum_commands` unchanged for consumers already using it
- Operator or user restart / restart-notice requirements:
  - none beyond rebuilding the local binary during implementation

## Future Considerations
- Follow-up ideas:
  - promote the shared command-family projection into a dedicated launcher/operator-contract helper if more surfaces need it
  - add subcommand-family drill-down for `taskflow` and `docflow` only if product demand appears
- Known limitations:
  - this slice does not attempt full root-command or runtime-family command introspection
- Technical debt left intentionally:
  - current root help wording is still string-rendered in launcher code rather than sourced from one declarative registry

## References
- Related specs:
  - `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`
  - `docs/product/spec/compiled-runtime-bundle-contract.md`
  - `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
  - `docs/product/spec/clarify-enforce-immediate-project-agent-first-design.md`
- Related protocols:
  - `AGENTS.md`
  - `AGENTS.sidecar.md`
- Related ADRs:
  - none
- External references:
  - `crates/vida/src/surface_render.rs`
  - `crates/vida/src/taskflow_runtime_bundle.rs`
  - `crates/vida/src/init_surfaces.rs`
  - `crates/vida/tests/boot_smoke.rs`

-----
artifact_path: product/spec/export-canonical-operator-command-map-through-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-15
schema_version: 1
status: canonical
source_path: docs/product/spec/export-canonical-operator-command-map-through-design.md
created_at: 2026-04-14T07:40:56.253573048Z
updated_at: 2026-04-17T09:20:04.289039599Z
changelog_ref: export-canonical-operator-command-map-through-design.changelog.jsonl
