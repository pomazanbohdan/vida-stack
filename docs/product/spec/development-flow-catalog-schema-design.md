# Development Flow Catalog Schema Design

Status: proposed

Use this document as the bounded schema contract for configurable development flows. The flow catalog must make role order, command templates, lifecycle hooks, proof gates, rework/resume transitions, approval pauses, and host-agent adapter projection data-driven rather than hardcoded in CLI/runtime code.

## Summary

- Feature / change: add a configurable development-flow catalog schema.
- Owner layer: mixed.
- Runtime surface: `vida.config.yaml | agent_extensions.flows | taskflow consume | agent dispatch-next | agent-init`.
- Status: proposed.

## Core Rule

VIDA owns flow law, task binding, packets, receipts, and closure. Host tools only provide an adapter capability selected by config. A flow step may request a host-agent adapter projection, but it must not name one vendor as runtime law.

## Configuration Shape

`dev_team` may define:

```yaml
dev_team:
  default_flow_id: default_delivery
  work_item_flow_bindings:
    epic: default_delivery
    defect: defect_repair_verified
    runtime_defect: runtime_defect_remediation
    pull_request: pr_repair_verified
    pr_repair: pr_repair_verified
    architecture: architecture_design
    release_readiness: release_readiness_gate
    service_tui: service_tui_orchestration
    internal_agent_development: hook_enabled_internal_agent_development
    task: default_delivery
    debug: debug_fast
  flows:
    default_delivery:
      enabled: true
      flow_class: development
      work_item_bindings: [epic, task]
      sequential: true
      allow_parallel_handoffs: false
      lifecycle_hook_templates: [command_timing_summary]
      adapter_projection:
        host_agent_bridge_contract: required
        process_carrier_requires_explicit_backend: true
      steps:
        - role_id: analyst
          runtime_role: business_analyst
          task_class: specification
          command_template:
            surface: vida agent-init
            args: [--role, business_analyst, "{{task_id}}", --json]
          lifecycle_hook_templates: [command_timing_summary]
          proof_gates:
            required_outputs: [detailed_task_brief, acceptance_contract]
          requires_user_approval: false
```

Legacy `steps: [developer, coach]` remains valid. Runtime projection must normalize both forms into:

```json
{
  "default_flow_id": "default_delivery",
  "sequence": ["analyst", "developer"],
  "flows": [
    {
      "flow_id": "default_delivery",
      "default": true,
      "work_item_bindings": ["epic", "task"],
      "ordered_steps": [
        {
          "step_id": "default_delivery-0",
          "order": 0,
          "role_id": "analyst",
          "runtime_role": "business_analyst",
          "task_class": "specification",
          "command_template": {},
          "lifecycle_hook_templates": [],
          "proof_gates": {},
          "resume_transitions": {},
          "rework_transitions": {},
          "adapter_projection": {},
          "requires_user_approval": false,
          "approval_policy": {}
        }
      ]
    }
  ]
}
```

## Flow Selection

Selection order:

1. Explicit task/epic binding when present.
2. `dev_team.work_item_flow_bindings.<issue_type>`.
3. `dev_team.default_flow_id`.
4. First enabled flow.

No runtime code may hardcode `default_delivery` as a semantic default. That id is a config value only.

## Work Item Taxonomy Registry

Flow binding keys are owned by the provider-neutral work item taxonomy registry in `state_store_task_models.rs`. The registry is additive over the persisted task-store `issue_type: String` field, so existing task records remain compatible while runtime code gains one owner contract for classification.

The registry separates four vocabularies:

1. Persisted work item type: the task-store `issue_type` value, normalized to a canonical taxonomy id.
2. Flow selection binding: the key used by `dev_team.work_item_flow_bindings` and flow `work_item_bindings`.
3. Runtime task class: selection and model-routing classes such as `implementation`, `test_authoring`, `verification`, and `architecture`.
4. Execution granularity: planning labels such as `delivery_task` and `execution_block`; these are not persisted work item types.

Phase 1 registry fields:

```json
{
  "schema_version": 1,
  "canonical_issue_type": "runtime_defect",
  "aliases": [],
  "category": "defect",
  "parent_required": true,
  "flow_bindable": true,
  "default_flow_binding": "runtime_defect_remediation",
  "source_tiers": ["runtime_status", "downstream_runtime_report"]
}
```

Unknown work item types are fail-closed for parent validation and must not silently become root-capable. New source systems may add aliases in later slices, but the canonical registry id remains provider-neutral and must not encode Jira, Linear, GitHub, or host-tool-specific names unless the work item class itself is a generic source tier. Compatibility aliases such as `bug` and `spike` may remain as aliases only; they must not become separate flow-driving provider-specific canonical ids.

## Standard Flow Presets

The root project config and generated template must keep the same standard flow presets unless a project explicitly opts out through a later accepted override contract:

| Work item | Flow id | Required role semantics |
| --- | --- | --- |
| `epic`, `task` | `default_delivery` | analyst, test author when required, developer, coach, verifier, prover, release closure |
| `defect` | `defect_repair_verified` | analyst, test author, developer, coach, verifier |
| `runtime_defect` | `runtime_defect_remediation` | runtime-surface analyst, regression author, developer, verifier, prover |
| `pull_request`, `pr_repair` | `pr_repair_verified` | PR triage analyst, CI/review verifier, repair/integration developer, coach, proof/disposition prover |
| `architecture` | `architecture_design` | analyst with user approval pause, execution-preparation worker, verifier |
| `release_readiness` | `release_readiness_gate` | verifier, prover |
| `service_tui` | `service_tui_orchestration` | analyst, developer, verifier |
| `internal_agent_development` | `hook_enabled_internal_agent_development` | analyst, developer, coach, verifier |

Every standard flow preset must be data-driven:

- It must be reachable through `dev_team.work_item_flow_bindings`.
- It must define ordered `steps`.
- Write-producing or review-producing steps must use a configured `command_template.surface`, normally `vida agent-init`.
- Flow-level `adapter_projection.host_agent_bridge_contract` must remain explicit.
- Process carriers must remain explicit through `process_carrier_requires_explicit_backend: true`.
- Hook references must resolve to `docs/product/spec/hook-templates.yaml`.

The PR preset is the canonical route for open pull-request handling. It may close stale/invalid PRs, merge valid PRs, or return a repair task, but only after triage, CI/review evidence, coach review, and final proof/disposition are represented in the flow evidence.

## Host-Agent Adapter Projection

Flow and step-level `adapter_projection` may require the generic host-agent bridge contract. Valid adapters are configured under `host_environment.host_agent_bridge_contract` and can include Codex host tools, Codex CLI process agents, Claude Code subagents, Pi plugin sub-agents, Vibe Kanban agents, OpenCode subagents, or custom adapters.

Process execution remains a child-process carrier and must be selected explicitly. Parent-session internal agents remain host bridge adapters and must produce result/receipt artifacts before closure.

## User Approval Gates

Phase 1 schema allows `requires_user_approval` and `approval_policy` on any ordered step. Runtime execution must treat these fields as a pause contract only after a later approval-gate implementation slice wires approval state, edit/rework loops, and resume commands.

The default delivery and architecture design presets use this field to model a user-editable analysis/specification document before downstream implementation roles continue. Approval hooks are diagnostic records until the approval-state implementation is accepted.

## Lifecycle Hooks

`lifecycle_hook_templates` are diagnostic hooks attached to flows or steps. Hooks must not change command semantics unless a future spec explicitly permits behavior-changing hooks. `command_timing_summary` is diagnostic-only and feeds operator self-diagnostics.

## Compatibility

- Existing `dev_team.roles` remains authoritative for role contracts.
- Existing legacy flow `steps` arrays continue to project into `ordered_steps`.
- Existing `agent_extensions.flow_sets[].lane_templates` remain valid and may carry the same generic fields.
- Unsupported or unknown role ids remain fail-closed validation blockers.

## Proof Targets

```powershell
cargo test -p vida development_flow_catalog -- --nocapture --test-threads=1
cargo test -p vida lifecycle_hook_contract -- --nocapture --test-threads=1
cargo test -p vida dev_team_sequence_uses_configured_flow_ordered_step_overrides -- --nocapture --test-threads=1
cargo test -p vida project_routing_shape_defines_configurable_pr_and_specialized_flow_presets -- --nocapture --test-threads=1
vida taskflow consume agent-system --json
vida docflow check --root . docs/product/spec/development-flow-catalog-schema-design.md docs/product/spec/current-spec-map.md docs/product/spec/current-spec-provenance-map.md
```

## External Reference Notes

These sources are reference patterns only, not VIDA runtime law:

- OpenAI Codex CLI: process and host-tool boundaries.
- Claude Code subagents: named subagent configuration and delegated context.
- Pi sub-agent package/docs: plugin-based sub-agent affordance.
- Vibe Kanban docs: multi-agent/profile orchestration surface.
- OpenCode agents docs: configured primary/subagent behavior and permission boundaries.

-----
artifact_path: product/spec/development-flow-catalog-schema-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-06-01'
schema_version: '1'
status: proposed
source_path: docs/product/spec/development-flow-catalog-schema-design.md
created_at: '2026-06-01T00:00:00+03:00'
updated_at: '2026-06-01T00:00:00+03:00'
changelog_ref: development-flow-catalog-schema-design.changelog.jsonl
