# Host-Agent Layer Status Matrix

Status: active product spec

Purpose: record the current closure status of the host-agent ladder across `L0-L6`, keep the owner/code/proof split explicit, and make the remaining closure order reviewable without re-deriving it from chat history.

## Scope

This matrix covers the active host-agent runtime loop for the current `codex`-backed execution path:

1. activation and rendered host surface,
2. overlay-owned tier metadata,
3. lane/role routing into executable tiers,
4. pricing and tier selection,
5. local score and adaptive strategy state,
6. observability, history, automatic feedback, and budget rollup.

It does not define future multi-host expansion beyond the current supported host CLI list.

## Layer Matrix

| Layer | Focus | Owner surfaces | Runtime/code surfaces | Current status |
| --- | --- | --- | --- | --- |
| `L0` | bootstrap and host-template activation | `bootstrap-carriers-and-project-activator-model.md`, `work.host-cli-agent-setup-protocol.md`, `vida.config.yaml.template` | `vida init`, `vida project-activator`, `crates/vida/src/main.rs` | `green` |
| `L1` | overlay-owned ladder and admissibility metadata | `vida.config.yaml`, `bridge.project-overlay-protocol.md` | `crates/vida/src/main.rs` | `green` |
| `L2` | rendered host executor surface | `.codex/config.toml`, `.codex/agents/*.toml`, `codex-agent-configuration-guide.md` | activation render path in `crates/vida/src/main.rs` | `green` |
| `L3` | conversation-role to execution-role routing | `agent-lane-selection-and-conversation-mode-model.md`, `entry.orchestrator-entry.md` | `vida taskflow consume final`, `crates/vida/src/main.rs` | `green` |
| `L4` | tier selection and per-task economics | `agent-role-skill-profile-flow-model.md`, `codex-agent-configuration-guide.md` | compiled bundle selection and cost estimation in `crates/vida/src/main.rs` | `green` |
| `L5` | local adaptive score state | `work.host-cli-agent-setup-protocol.md`, `vida.config.yaml` | `.vida/state/worker-scorecards.json`, `.vida/state/worker-strategy.json`, `vida agent-feedback` | `green` |
| `L6` | observability, history, automatic feedback ingestion, budget rollup | this matrix, `work.host-cli-agent-setup-protocol.md`, `documentation-tooling-map.md` | `.vida/state/host-agent-observability.json`, `vida taskflow task close`, `vida status --json` | `green` |

## Current Green Closure

### `L0`

1. `vida init` leaves host-agent activation pending instead of copying a host template blindly.
2. `vida project-activator --host-cli-system codex` records the selected host CLI system and renders `.codex/**`.

### `L1`

1. `vida.config.yaml -> host_environment.codex.agents` owns tier metadata, runtime-role fit, and task-class fit.
2. `.codex/**` is a rendered executor surface, not the owner of rates or admissibility.

### `L2`

1. project activation renders `.codex/config.toml` and `.codex/agents/*.toml` from overlay metadata while preserving template instruction bodies,
2. the root session remains outside the delegated tier list.

### `L3`

1. runtime maps request intent into bounded execution posture before tier selection,
2. design-first feature requests stay on the `business_analyst -> spec-pack -> docflow` path before delegated development.

### `L4`

1. the runtime chooses the cheapest healthy capable tier that satisfies task-class and runtime-role constraints,
2. per-task estimate is exposed as `estimated_task_price_units`,
3. cost-quality constraints remain part of the compiled runtime identity.

### `L5`

1. manual feedback uses `vida agent-feedback`,
2. local scorecards and strategy remain under `.vida/state/**`,
3. tier lifecycle remains derived from local score and failure history.

### `L6`

1. `vida taskflow task close ...` now records automatic host-agent feedback using the same score/strategy path as manual feedback,
2. host-agent activity and budget rollup are persisted locally in `.vida/state/host-agent-observability.json`,
3. `vida status --json` now surfaces `host_agents` with tier metadata, local stores, budget totals, and recent events.

## Proof Surface

The current bounded proof set for this matrix is:

1. `cargo test -q -p vida --bin vida tests::agent_feedback_records_scorecard_and_refreshes_strategy -- --exact`
2. `cargo test -q -p vida --test boot_smoke status_json_exposes_host_agent_summary -- --exact`
3. `cargo test -q -p vida --test boot_smoke taskflow_task_close_records_auto_feedback_and_budget -- --exact`
4. `cargo test -q -p vida --test boot_smoke taskflow_bootstrap_spec_creates_epic_spec_task_and_design_doc -- --exact`

## Update Rule

1. If a host-agent layer regresses from `green`, update this matrix in the same bounded change that introduces or repairs the regression.
2. Do not treat chat summaries as the owner of layer status once this matrix exists.
3. Keep the owner/code/proof split explicit; do not collapse this matrix into a changelog or one-off implementation note.

-----
artifact_path: product/spec/host-agent-layer-status-matrix
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: docs/product/spec/host-agent-layer-status-matrix.md
created_at: '2026-03-14T17:55:00+02:00'
updated_at: '2026-03-14T17:55:00+02:00'
changelog_ref: host-agent-layer-status-matrix.changelog.jsonl
