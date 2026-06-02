# WORKFLOW Policy Loader For Service-Orchestrated Runs

Status: proposed

Use this document as the bounded design contract for adding a repo-owned `WORKFLOW.md` policy loader to service-orchestrated VIDA runs. The loader enriches the existing `vida.config.yaml` and agent-extension flow registry path; it must not become a second runtime authority.

## Summary

- Feature / change: load optional service-orchestration policy from `WORKFLOW.md`.
- Owner layer: mixed.
- Runtime surface: `vida.config.yaml | docs/process/agent-extensions/flows.yaml | WORKFLOW.md | taskflow consume | status/TUI projections`.
- Status: proposed.

## Runtime Request Evidence

The active TaskFlow run `vida-symphony-workflow-md-policy-loader` selected an internal subagent specification lane:

- dispatch target: `specification`
- runtime role: `business_analyst`
- backend: `internal_subagents`
- owned path: `docs/product/spec/add-workflow-policy-loader-service-orchestrated-design.md`
- read-only evidence paths: `.vida/data/state/runtime-consumption`, `docs/product/spec`, and `docs/process`

The host-bridge result was specification evidence only. It did not authorize implementation, runtime state closure without handoff evidence, or mutation outside this document.

## Problem Statement

Service-orchestrated development flows must not depend on hardcoded Rust fallback sequences, role chains, prompt names, command templates, or carrier assumptions. VIDA already has configuration owners for this:

- `vida.config.yaml` owns active registries, backend/carrier posture, role selection, host bridge contracts, and enabled flow sets.
- `docs/process/agent-extensions/flows.yaml` owns project flow definitions, ordered role steps, command templates, proof gates, and adapter projection hints.
- Runtime projections expose compiled truth to operators and TUI surfaces.

`WORKFLOW.md` should provide a project-local policy overlay for service orchestration only when explicitly enabled by configuration. It must not silently redefine TaskFlow law, proof law, carrier selection, or prompt-template authority.

## Design

Add a typed loader that reads `WORKFLOW.md` as Markdown with strict YAML front matter.

The front matter may declare:

```yaml
schema_version: 1
policy_id: service-orchestrated-default
enabled: true
workflow_class: service_orchestration
flow_bindings:
  service_tui: service_tui_orchestration
prompt_templates:
  specification: prompts/service-specification.md
reload:
  mode: snapshot_per_run
projection:
  expose_to_tui: true
```

The Markdown body remains human documentation unless a front-matter key maps a named section to a configured prompt-template reference. Inline prose must not become unvalidated runtime law.

The loader returns a typed policy object with explicit defaults. Missing optional fields are defaulted in code through typed values; missing required fields, unknown mandatory keys, unresolved flow ids, unresolved prompt-template refs, or unsupported reload modes become machine-readable validation errors.

## Configuration Ownership

`WORKFLOW.md` is an overlay. Authority order is:

1. Runtime law and TaskFlow/DocFlow receipts.
2. `vida.config.yaml` activation and registry configuration.
3. `docs/process/agent-extensions/flows.yaml` flow catalog.
4. Enabled `WORKFLOW.md` service-orchestration overlay.
5. Runtime/TUI projections.

TUI surfaces may display policy source, active snapshot id, reload state, and validation errors. TUI must not select roles, command order, carriers, model profiles, or closure proof independently.

## Reload Semantics

Active runs use a policy snapshot. A changed `WORKFLOW.md` creates one of these states:

- `current`: active snapshot matches source.
- `pending_reload`: source changed after active snapshot creation.
- `validation_blocked`: source changed but validation failed.

No active lane may silently switch policy mid-run. New dispatches may use the latest valid snapshot after validation.

## TUI And Status Projection

Expose a read-side projection with:

```json
{
  "workflow_policy": {
    "enabled": true,
    "source_path": "WORKFLOW.md",
    "policy_id": "service-orchestrated-default",
    "snapshot_state": "current",
    "validation_status": "pass",
    "validation_errors": []
  }
}
```

Validation errors must include source path, key path, severity, and next action. Unsupported knobs fail closed when they would alter runtime behavior.

## Implementation Slices

1. Loader/schema slice:
   - add a typed workflow policy module under `crates/vida/src/`
   - parse Markdown front matter
   - validate schema version, known keys, flow ids, prompt-template refs, and reload mode

2. Runtime compilation slice:
   - merge the policy into the existing configured flow projection after `vida.config.yaml` and registry loading
   - keep service-orchestrated defaults explicit
   - remove service-orchestration reliance on Rust-only role/command fallback literals

3. TUI/status projection slice:
   - expose active policy id, source, snapshot state, and validation errors
   - keep the projection read-only

4. Test/docs slice:
   - add focused loader tests
   - add integration tests proving flow sequence comes from config/registry/policy
   - update spec map and provenance after the design is accepted

## Acceptance Criteria

- Missing `WORKFLOW.md` is allowed only when configuration marks the policy optional.
- Invalid front matter fails closed with machine-readable validation errors.
- Unknown behavior-changing keys fail validation.
- Service-orchestrated role order and command sequence are derived from config/registry/policy, not hardcoded fallback literals.
- Prompt templates are resolved references, not hidden inline runtime law.
- Active runs use stable policy snapshots.
- TUI/status surfaces expose source, active policy id, snapshot state, and validation blockers.
- Existing `vida.config.yaml` and `flows.yaml` behavior remains compatible when `WORKFLOW.md` is absent or disabled.

## Proof Targets

```powershell
cargo test -p vida workflow_policy_loader -- --nocapture --test-threads=1
cargo test -p vida development_flow_catalog -- --nocapture --test-threads=1
cargo test -p vida dev_team_sequence_uses_configured_flow_ordered_step_overrides -- --nocapture --test-threads=1
vida taskflow consume agent-system --json
vida docflow check --root . docs/product/spec/add-workflow-policy-loader-service-orchestrated-design.md
```

## Risks And Controls

- Risk: `WORKFLOW.md` becomes a second runtime law surface.
  Control: treat it as an enabled overlay compiled through existing runtime policy owners.
- Risk: prompt prose bypasses prompt-template validation.
  Control: only configured prompt-template refs may affect runtime prompts.
- Risk: dynamic reload changes an active run mid-lane.
  Control: snapshot per run and expose pending reload state.
- Risk: TUI becomes authority.
  Control: TUI consumes projections only.

## Next Handoff

After this design is finalized, continue with execution preparation and test-authoring slices before implementation. The first implementation packet should own only the typed loader/schema module and its unit tests.

-----
artifact_path: product/spec/add-workflow-policy-loader-service-orchestrated-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-06-02'
schema_version: '1'
status: proposed
source_path: docs/product/spec/add-workflow-policy-loader-service-orchestrated-design.md
created_at: '2026-06-02T00:00:00+03:00'
updated_at: '2026-06-02T00:00:00+03:00'
changelog_ref: add-workflow-policy-loader-service-orchestrated-design.changelog.jsonl
