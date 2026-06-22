# LDRK Operation Catalog Before After Command Tree

Status: generated proof artifact for TaskFlow task `ldr-003`.

Source: `crates/vida/src/cli.rs` plus `docs/product/spec/ldrk-baseline/baseline.json`.
Baseline current leaf command candidates: `160`.
Source parser first-class root leaves: `96`.
Baseline command-specific option candidates: `527`.

## Before Tree

- `vida init` -> `vida.apply` (canonical_operation)
- `vida boot` -> `vida.apply` (canonical_operation)
- `vida orchestrator-init` -> `vida.plan` (canonical_operation)
- `vida agent-init` -> `vida.plan` (canonical_operation)
- `vida agent dispatch-next` -> `vida.get` (canonical_operation)
- `vida agent select` -> `vida.get` (canonical_operation)
- `vida agent host-bridge` -> `vida.apply` (canonical_operation)
- `vida agent status` -> `vida.get` (canonical_operation)
- `vida coder capabilities` -> `vida.get` (canonical_operation)
- `vida coder provider-check` -> `vida.get` (canonical_operation)
- `vida coder run` -> `vida.apply` (canonical_operation)
- `vida protocol view` -> `vida.get` (canonical_operation)
- `vida project-activator` -> `vida.plan` (canonical_operation)
- `vida agent-feedback` -> `vida.apply` (canonical_operation)
- `vida task help` -> `vida.apply` (canonical_operation)
- `vida task import` -> `vida.apply` (canonical_operation)
- `vida task import-jsonl` -> `vida.apply` (canonical_operation)
- `vida task replace-jsonl` -> `vida.apply` (canonical_operation)
- `vida task export-jsonl` -> `vida.apply` (canonical_operation)
- `vida task list` -> `vida.get` (canonical_operation)
- `vida task search` -> `vida.get` (canonical_operation)
- `vida task show` -> `vida.get` (canonical_operation)
- `vida task progress` -> `vida.get` (canonical_operation)
- `vida task closure-ready` -> `vida.get` (canonical_operation)
- `vida task proof status` -> `vida.apply` (canonical_operation)
- `vida task proof attach-browser` -> `vida.apply` (canonical_operation)
- `vida task ready` -> `vida.get` (canonical_operation)
- `vida task next` -> `vida.get` (canonical_operation)
- `vida task next-lawful` -> `vida.get` (canonical_operation)
- `vida task next-display-id` -> `vida.apply` (canonical_operation)
- `vida task create` -> `vida.apply` (canonical_operation)
- `vida task ensure` -> `vida.apply` (canonical_operation)
- `vida task update` -> `vida.apply` (canonical_operation)
- `vida task note append` -> `vida.apply` (canonical_operation)
- `vida task block` -> `vida.apply` (canonical_operation)
- `vida task verify` -> `vida.apply` (canonical_operation)
- `vida task attempt dispatch` -> `vida.plan` (canonical_operation)
- `vida task attempt status` -> `vida.plan` (canonical_operation)
- `vida task attempt collect` -> `vida.apply` (canonical_operation)
- `vida task attempt consolidate` -> `vida.apply` (canonical_operation)
- `vida task attempt record` -> `vida.apply` (canonical_operation)
- `vida task attempt transition` -> `vida.apply` (canonical_operation)
- `vida task attempt summary` -> `vida.apply` (canonical_operation)
- `vida task stage status` -> `vida.apply` (canonical_operation)
- `vida task owned-status` -> `vida.apply` (canonical_operation)
- `vida task handoff accept` -> `vida.apply` (canonical_operation)
- `vida task takeover status` -> `vida.apply` (canonical_operation)
- `vida task close` -> `vida.apply` (canonical_operation)
- `vida task reconcile` -> `vida.apply` (canonical_operation)
- `vida task reconcile-closed-runs` -> `vida.apply` (canonical_operation)
- `vida task prune-closed-epics` -> `vida.apply` (canonical_operation)
- `vida task split` -> `vida.apply` (canonical_operation)
- `vida task spawn-blocker` -> `vida.apply` (canonical_operation)
- `vida task adaptive-preview` -> `vida.plan` (canonical_operation)
- `vida task deps` -> `vida.get` (canonical_operation)
- `vida task reverse-deps` -> `vida.get` (canonical_operation)
- `vida task blocked` -> `vida.get` (canonical_operation)
- `vida task children` -> `vida.get` (canonical_operation)
- `vida task reparent-children` -> `vida.apply` (canonical_operation)
- `vida task defect-batch-rehome` -> `vida.apply` (canonical_operation)
- `vida task tree` -> `vida.get` (canonical_operation)
- `vida task validate-graph` -> `vida.apply` (canonical_operation)
- `vida task dep add` -> `vida.apply` (canonical_operation)
- `vida task dep ensure` -> `vida.apply` (canonical_operation)
- `vida task dep add-bulk` -> `vida.apply` (canonical_operation)
- `vida task dep remove` -> `vida.apply` (canonical_operation)
- `vida task critical-path` -> `vida.plan` (canonical_operation)
- `vida memory` -> `vida.get` (canonical_operation)
- `vida status` -> `vida.get` (canonical_operation)
- `vida state reset` -> `vida.get` (canonical_operation)
- `vida runtime web status` -> `vida.get` (canonical_operation)
- `vida runtime web restart` -> `vida.plan` (canonical_operation)
- `vida doctor` -> `vida.get` (canonical_operation)
- `vida diagnostics post-commit` -> `vida.apply` (canonical_operation)
- `vida diagnostics evidence-check` -> `vida.get` (canonical_operation)
- `vida diagnostics rules-check` -> `vida.get` (canonical_operation)
- `vida proof browser` -> `vida.get` (canonical_operation)
- `vida service` -> `vida.service` (adapter_alias_family)
- `vida project` -> `vida.service` (adapter_alias_family)
- `vida wizard` -> `vida.service` (adapter_alias_family)
- `vida job` -> `vida.service` (adapter_alias_family)
- `vida receipt` -> `vida.service` (adapter_alias_family)
- `vida docs update` -> `vida.apply` (canonical_operation)
- `vida orchestrator-session show` -> `vida.get` (canonical_operation)
- `vida orchestrator-session reclaim` -> `vida.apply` (canonical_operation)
- `vida orchestrator-session transfer` -> `vida.apply` (canonical_operation)
- `vida session triage` -> `vida.plan` (canonical_operation)
- `vida quality gate` -> `vida.get` (canonical_operation)
- `vida consume` -> `vida.repair` (adapter_alias_family)
- `vida lane` -> `vida.repair` (adapter_alias_family)
- `vida approval` -> `vida.repair` (adapter_alias_family)
- `vida recovery` -> `vida.repair` (adapter_alias_family)
- `vida route` -> `vida.repair` (adapter_alias_family)
- `vida release install` -> `vida.apply` (canonical_operation)
- `vida taskflow` -> `vida.repair` (adapter_alias_family)
- `vida docflow` -> `vida.repair` (adapter_alias_family)

## Delegated Proxy Coverage

- `vida taskflow ...` -> `vida.repair` family adapter boundary; expanded leaves counted in baseline candidate total until owning runtime emits a generated family tree.
- `vida docflow ...` -> `vida.repair` family adapter boundary; expanded leaves counted in baseline candidate total until owning runtime emits a generated family tree.
- `vida consume ...` -> `vida.repair` family adapter boundary; expanded leaves counted in baseline candidate total until owning runtime emits a generated family tree.
- `vida lane ...` -> `vida.repair` family adapter boundary; expanded leaves counted in baseline candidate total until owning runtime emits a generated family tree.
- `vida approval ...` -> `vida.repair` family adapter boundary; expanded leaves counted in baseline candidate total until owning runtime emits a generated family tree.
- `vida recovery ...` -> `vida.repair` family adapter boundary; expanded leaves counted in baseline candidate total until owning runtime emits a generated family tree.
- `vida route ...` -> `vida.repair` family adapter boundary; expanded leaves counted in baseline candidate total until owning runtime emits a generated family tree.
- `vida service ...` -> `vida.service` family adapter boundary; expanded leaves counted in baseline candidate total until owning runtime emits a generated family tree.
- `vida project ...` -> `vida.service` family adapter boundary; expanded leaves counted in baseline candidate total until owning runtime emits a generated family tree.
- `vida wizard ...` -> `vida.service` family adapter boundary; expanded leaves counted in baseline candidate total until owning runtime emits a generated family tree.
- `vida job ...` -> `vida.service` family adapter boundary; expanded leaves counted in baseline candidate total until owning runtime emits a generated family tree.
- `vida receipt ...` -> `vida.service` family adapter boundary; expanded leaves counted in baseline candidate total until owning runtime emits a generated family tree.

## After Tree

- `vida get <operation> --payload <file|->`
- `vida plan <operation> --payload <file|->`
- `vida apply <operation> --payload <file|->`
- `vida watch <operation> --payload <file|->`
- `vida service <operation> --payload <file|->`
- `vida repair <operation> --payload <file|->`

## Reduction Proof

- Leaf commands: `160` -> `6` = `96.25%` reduction.
- Command-specific options: `527` -> `0` = `100.0%` reduction.
- Global context moves to common options: `--project`, `--session`, `--format`, `--endpoint`, `--offline`, `--state-dir`, `--render`, `--json`.
- Mutation semantics move into typed payloads; host-bridge completion uses one structured outcome payload instead of independent decision/verdict/blocker fields.

-----
artifact_path: product/spec/ldrk-operation-catalog/before-after-command-tree
artifact_type: product_spec
artifact_version: "1"
source_path: docs/product/spec/ldrk-operation-catalog/before-after-command-tree.md
created_at: 2026-06-22T00:00:00+03:00
updated_at: 2026-06-22T00:00:00+03:00
changelog_ref: current-spec-catalog.changelog.jsonl
