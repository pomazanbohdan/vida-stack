# Host CLI Agent Setup Protocol

Purpose: define the framework-owned activation boundary for host CLI agent templates during project onboarding.

## Core Contract

1. Host CLI agent setup is framework-owned activation work, not project-owned process drift.
2. Project initialization must not silently copy one host CLI runtime tree into every new repository.
3. `vida init` may scaffold the minimum framework/bootstrap carriers, but host CLI agent templates must be selected and materialized through `vida project-activator`.
4. Host CLI selection must be explicit and durable in `vida.config.yaml` before the project is treated as ready for normal agent-backed work.

## Selection Rule

1. `vida project-activator` must ask for or accept an explicit host CLI system selection when activation is still pending.
2. The current framework-supported host CLI list is:
   - `codex`
3. If no supported host CLI system is selected, project activation remains pending.
4. Unsupported host CLI values are invalid and must fail closed.
5. The bounded activation interview should surface the supported host CLI list directly so the agent/operator does not have to infer it from repository files.

## Materialization Rule

1. After a supported host CLI system is selected, the activator may materialize the matching runtime template into the project.
2. The current framework-owned materialization path is:
   - `codex` -> `.codex/**`
3. Materialization must use a framework-owned template source, not a mutable project-local workaround.
4. When overlay metadata already declares host-agent tiers in `vida.config.yaml -> host_environment.codex.agents`, activation must render `.codex/**` from that overlay catalog while preserving framework-owned template instruction bodies.
5. When overlay metadata also declares internal dispatch aliases in `vida.config.yaml -> host_environment.codex.dispatch_aliases`, activation must render those alias carriers into `.codex/**` from overlay/template owner state instead of Rust-owned fallback catalogs.
6. Existing project-local host CLI configuration must not be overwritten silently.
7. When the selected template exposes default agent definitions, the activator should surface those defaults in its activation view so the operator knows which agents become available after restart.

## Configuration State

1. The selected host CLI system must be recorded in root `vida.config.yaml`.
2. Host-executor carrier-tier metadata should be recorded in `vida.config.yaml -> host_environment.codex.agents`.
3. Internal dispatch aliases should be recorded in `vida.config.yaml -> host_environment.codex.dispatch_aliases`.
4. Host CLI selection must remain separate from project docs mapping and from framework owner law.
5. `.codex/**` is a rendered host surface; it must not become the owner of tier rates, runtime-role fit, task-class fit, or dispatch-alias instruction bodies when those values are already declared in the project overlay.
6. Project-specific runtime tuning for a selected CLI system may live in project-owned docs/process surfaces, but the selection/materialization boundary itself remains framework-owned.

## Restart Rule

1. When host CLI agent configuration is newly materialized, the activator must tell the operator to close and restart the selected tool.
2. For `codex`, that means closing and restarting Codex so the activated agent configuration becomes visible in the runtime execution environment.

Activation-path rule:

1. Host CLI setup runs inside the bounded activation/onboarding path, not through TaskFlow.
2. Companion documentation/config readiness during this path should use `vida docflow`, not `vida taskflow`.
3. Host CLI materialization should write an activation receipt under `.vida/receipts/` so the change is durable and reviewable.

## Current Codex Binding

1. Framework activation owns `codex` selection and `.codex/**` template materialization.
2. The current framework-owned Codex template materializes a four-tier ladder:
   - `junior` -> rate `1`
   - `middle` -> rate `4` and default `coach` runtime-role support for bounded spec-conformance review
   - `senior` -> rate `16`
   - `architect` -> rate `32`
3. The same rendered Codex surface may also materialize internal dispatch aliases from `vida.config.yaml -> host_environment.codex.dispatch_aliases`.
4. Those aliases are compatibility/internal activation surfaces only, not the project-visible primary agent model:
   - `development_implementer` -> carried by `junior`; owns one bounded write-producing packet
   - `development_coach` -> carried by `middle`; owns formative packet-local review against approved spec, acceptance criteria, definition of done, and visible residual risks, then either approves forward or returns explicit rework
   - `development_verifier` -> carried by `senior`; owns independent proof and closure readiness
   - `development_escalation` -> carried by `architect`; owns architecture preparation and hard-conflict arbitration only when the normal packet cycle cannot close coherently
5. Project-visible carrier selection, reporting, and activation should still resolve to `junior|middle|senior|architect` plus explicit runtime role.
6. The framework-owned Codex template must also initialize the local score-state surfaces:
   - `.vida/state/worker-scorecards.json`
   - `.vida/state/worker-strategy.json`
7. Runtime feedback writeback for the selected host agent should use:
   - `vida agent-feedback --agent-id <tier> --score <0-100> --task-class <task_class> [--outcome <success|failure|neutral>] [--notes "..."]`
8. The active Codex path must also maintain the local observability/history and budget-rollup surface:
   - `.vida/state/host-agent-observability.json`
9. `vida taskflow task ...` must use the native Rust StateStore bridge for core lifecycle operations; non-canonical external helper paths are forbidden in the active runtime.
10. `vida taskflow task close ...` should refresh the same host-agent score/observability loop automatically when the bounded task can be mapped back into one lawful Codex task class.
11. `vida status --json` should expose a bounded `host_agents` summary so the operator can inspect tier state, local stores, recent events, and total estimated budget units without reading raw state files.
12. Project-local role/runtime tuning for Codex remains in `docs/process/codex-agent-configuration-guide.md`.
13. That project guide must not be treated as the framework owner for selection/materialization.
14. Rust runtime code must not hardcode dispatch-alias catalogs that duplicate overlay-owned alias definitions; activation must render from template/overlay owner state.

## Verification

Minimum proof:

1. `vida init` does not materialize `.codex/**` by default.
2. `vida project-activator` reports pending activation when host CLI selection is missing.
3. `vida project-activator --host-cli-system codex` records the selected host CLI in `vida.config.yaml`.
4. `vida project-activator --host-cli-system codex` materializes `.codex/**` from the framework template source when the project-local tree is missing.
5. If `vida.config.yaml -> host_environment.codex.agents` is present, the rendered `.codex/**` metadata must reflect that overlay catalog rather than stale template rates/role mappings.
6. After successful materialization, the activator tells the operator to restart Codex.

-----
artifact_path: config/runtime-instructions/host-cli-agent-setup.protocol
artifact_type: runtime_instruction
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/runtime-instructions/work.host-cli-agent-setup-protocol.md
created_at: '2026-03-14T14:30:00+02:00'
updated_at: '2026-03-14T14:30:00+02:00'
changelog_ref: work.host-cli-agent-setup-protocol.changelog.jsonl
