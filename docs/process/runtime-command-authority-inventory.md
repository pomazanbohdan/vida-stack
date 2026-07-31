# Runtime Command Authority Inventory

Purpose: provide the VH-00 baseline matrix for public VIDA command surfaces. This is a project-owned inventory artifact for runtime hardening work; it does not create framework law or replace executable command routing.

Source evidence:

1. `crates/vida/src/cli.rs` root `Command` enum and `TaskCommand` enum.
2. `crates/vida/src/root_command_router.rs` command dispatch and state-dir preparation helpers.
3. `vida task --help` public operator surface.

State access modes:

| Mode | Meaning |
| --- | --- |
| HelpOnly | Help or parser-only surface; no project root or state store is required. |
| ParseOnly | CLI normalization or dispatch parsing before the owned family executes. |
| ReadOnly | Reads authoritative runtime/project state without mutating it. |
| Mutation | Mutates authoritative state, runtime receipts, project docs, release artifacts, or local service state. |
| Proxy | Delegates to another owned command family; authority remains with the target family. |
| ExternalProxy | Delegates to service/client or external provider surfaces. |
| SnapshotRead | Reads latest runtime projections, receipts, or cached summaries; must not outrank authoritative state. |

## Root Command Matrix

| Command | Authority owner | State access | State-dir behavior | JSON contract | Workflow role | Recommended proof owner |
| --- | --- | --- | --- | --- | --- | --- |
| `vida` | root help renderer | HelpOnly | no project root | plain help | operator discovery | CLI help snapshot |
| `vida init` | bootstrap/init surfaces | Mutation | project root materialization | mixed human/JSON by args | project bootstrap | init smoke |
| `vida boot` | bootstrap/init surfaces | Mutation | project root materialization | mixed human/JSON by args | state/framework bootstrap | boot smoke |
| `vida orchestrator-init` | init surfaces | SnapshotRead | project-bound runtime state | JSON startup envelope | root lane binding | startup contract smoke |
| `vida agent-init` | init surfaces and dispatch packet owner | SnapshotRead/Mutation | project-bound runtime state | JSON activation/dispatch envelope | worker lane activation | agent-init dispatch tests |
| `vida agent` | agent dispatch surface | ReadOnly/Mutation | project-bound unless explicit state-dir | JSON preview/dispatch contract | dispatch planning | agent dispatch smoke |
| `vida coder` | coder provider surface | ExternalProxy | provider/config dependent | JSON when requested | provider inspection/invocation | provider capability tests |
| `vida protocol` | protocol surface | ReadOnly | project/framework instruction state | JSON when requested | instruction lookup | protocol view tests |
| `vida project-activator` | project activator surface | Mutation | project root materialization | JSON activation report | onboarding/repair | activator smoke |
| `vida agent-feedback` | agent feedback surface | Mutation | project-bound strategy state | JSON when requested | local score update | feedback surface tests |
| `vida task` | TaskFlow authoritative state store | ReadOnly/Mutation | preserves explicit `--state-dir`; otherwise project-bound | JSON operator contract when `--json` | backlog authority | task smoke and graph validation |
| `vida memory` | memory surface | ReadOnly/Mutation | project-bound memory state | JSON when requested | instruction memory inspection | memory smoke |
| `vida status` | status surface | SnapshotRead | project-bound runtime state | JSON status envelope | operator projection | status/doctor parity tests |
| `vida state` | state surface | Mutation | explicit reset archive/reinit target | JSON when requested | state maintenance | state reset tests |
| `vida runtime` | runtime web/service surface | Mutation | runtime service config | JSON when requested | local service operation | runtime web tests |
| `vida doctor` | doctor surface | SnapshotRead | project-bound runtime state | JSON diagnostics envelope | integrity check | doctor contract smoke |
| `vida diagnostics` | diagnostics surface | SnapshotRead/Mutation | explicit state-dir for subcommands when present | JSON diagnostics envelope | post-commit/self-diagnostic | diagnostics tests |
| `vida proof` | proof surface | ReadOnly/Mutation | proof artifact dependent | JSON when requested | proof evidence | proof surface tests |
| `vida service` | VidaClient service proxy | ExternalProxy | service runtime dependent | service/client contract | service operations | client conformance |
| `vida project` | VidaClient project proxy | ExternalProxy | service runtime dependent | service/client contract | project operations | client conformance |
| `vida wizard` | VidaClient wizard proxy | ExternalProxy | service runtime dependent | service/client contract | guided flows | client conformance |
| `vida job` | VidaClient job proxy | ExternalProxy | service runtime dependent | service/client contract | job operations | client conformance |
| `vida receipt` | VidaClient receipt proxy | ExternalProxy | service runtime dependent | service/client contract | receipt operations | client conformance |
| `vida docs` | docs surface | Mutation | project docs root | JSON when requested | docs carrier update | DocFlow/doc tests |
| `vida orchestrator-session` | orchestrator session surface | ReadOnly/Mutation | project-bound session state | JSON session contract | session ownership | session reclaim tests |
| `vida session` | session surface | SnapshotRead | project-bound runtime state | JSON when requested | bounded-unit triage | session triage tests |
| `vida quality` | quality surface | ReadOnly | project-bound quality state | JSON when requested | quality advice | quality gate tests |
| `vida consume` | TaskFlow consume alias | Proxy | forwarded to TaskFlow consume | TaskFlow JSON contract | consume continuation | consume continue tests |
| `vida lane` | lane surface | ReadOnly/Mutation | project-bound lane/run-graph state | JSON lane envelope | lane/takeover authority | lane surface tests |
| `vida approval` | approval surface | ReadOnly/Mutation | project-bound approval/run-graph state | JSON approval envelope | approval inspection/mutation | approval tests |
| `vida recovery` | TaskFlow recovery alias | Proxy | forwarded to TaskFlow recovery | TaskFlow JSON contract | recovery inspection | recovery tests |
| `vida route` | TaskFlow route alias | Proxy | forwarded to TaskFlow route | TaskFlow JSON contract | route diagnostics | route tests |
| `vida release` | release surface | Mutation | release/install paths | JSON when requested | binary release/install | release smoke |
| `vida taskflow` | TaskFlow management runtime plus optional dispatch runtime | Proxy | family-owned state-dir rules | TaskFlow JSON contract | runtime workflow | TaskFlow smoke |
| `vida docflow` | DocFlow runtime family | Proxy | family-owned docs state | DocFlow JSON contract | documentation workflow | DocFlow validation |
| external subcommand | root external fallback | ExternalProxy | external/provider dependent | external contract | compatibility escape hatch | explicit adapter tests |

## Task Command Matrix

All `vida task` subcommands are owned by the TaskFlow authoritative state store. Each subcommand preserves an explicit `--state-dir` when the CLI struct exposes one; otherwise project binding is prepared by the root router. `help` and `adaptive-preview` are parse/read surfaces that do not need project-root binding.

## TaskFlow Runtime Split

| Surface | Authority | Disabled behavior |
| --- | --- | --- |
| `vida task create|update|close|reparent|deps` | Task Management Runtime | Always available; management-only close does not reconcile dispatch artifacts. |
| `vida taskflow dispatch status|adopt` | Task Dispatch Runtime | Returns `dispatch_runtime_disabled` when `taskflow.dispatch.enabled` is false; `adopt --apply` never writes in that mode. |
| `vida taskflow scheduler dispatch`, `consume`, run-graph mutation aliases | Task Dispatch Runtime | Compatibility aliases fail closed with `dispatch_runtime_disabled`. |

Management close authority is shared by `vida task close` and `vida task update --status closed`. In management-only mode it ignores proof-target presence but still enforces graph, child, and reopen guards; it does not touch dispatch receipts, run graphs, continuation bindings, or closure artifacts. With dispatch enabled, execution-bound lifecycle transitions require the dispatch runtime and a validated `{run_id, receipt_id}` source.

TeamFlow bootstrap is independent of dispatch: `dev_team.enabled: false` is a stable `team_flow_disabled` execution-surface result and does not require `authority_catalog`; `enabled: true` keeps strict catalog validation.

| Task subcommand | State access | Authority/role | Recommended proof |
| --- | --- | --- | --- |
| `help` | HelpOnly | TaskFlow help topics and aliases | help snapshot |
| `import` / `create-bulk` / `bulk-create` | Mutation | bulk task creation | import dry-run and mutation tests |
| `import-jsonl` | Mutation | JSONL backlog import | JSONL fixture test |
| `replace-jsonl` | Mutation | authoritative backlog replacement | replacement fixture test |
| `export-jsonl` | ReadOnly | backlog export | export fixture test |
| `list` | ReadOnly | task listing | list filter tests |
| `search` | ReadOnly | task search | search filter tests |
| `show` | ReadOnly | single task metadata | show contract test |
| `progress` | ReadOnly | task/epic progress projection | progress basis tests |
| `closure-ready` | ReadOnly | closure gate inspection | closure gate tests |
| `proof status` | ReadOnly | proof target/evidence status | proof status tests |
| `proof attach-browser` | Mutation | browser proof evidence attach | proof attach tests |
| `ready` | ReadOnly | ready queue from graph truth | ready graph tests |
| `next` | ReadOnly | next task selection | next selection tests |
| `next-lawful` | ReadOnly | next lawful continuation without heuristics | next-lawful parity tests |
| `next-display-id` | ReadOnly/Mutation | display id allocation | allocation tests |
| `create` | Mutation | one task create | create mutation tests |
| `ensure` | Mutation | idempotent task create | ensure idempotency tests |
| `update` | Mutation | task metadata/status update | update mutation tests |
| `note append` | Mutation | append-only task notes | note append tests |
| `block` | Mutation | task blocker recording | block tests |
| `verify` | Mutation | partial verification evidence | verify tests |
| `attempt dispatch/status/collect/consolidate/record/transition/summary` | ReadOnly/Mutation | per-stage attempt ledger | attempt ledger tests |
| `stage status` | ReadOnly | per-stage execution status | stage status tests |
| `owned-status` | ReadOnly | dirty files vs owned paths | owned-status tests |
| `handoff accept` | Mutation | delegated handoff receipts | handoff receipt tests |
| `takeover status` | ReadOnly | task-scoped exception takeover status | takeover status tests |
| `close` | Mutation | task closure gate and release automation | close gate tests |
| `reconcile` | Mutation | close complete open epics | reconcile tests |
| `reconcile-closed-runs` | Mutation | retire historical runs for closed tasks | closed-run reconcile tests |
| `prune-closed-epics` | Mutation | archive/prune closed epics | prune safety tests |
| `split` | Mutation | split oversized task | split tests |
| `spawn-blocker` | Mutation | create blocker dependency | blocker spawn tests |
| `adaptive-preview` | ReadOnly | preview replanner classification | adaptive preview tests |
| `deps` | ReadOnly | direct dependency edges | dependency read tests |
| `reverse-deps` | ReadOnly | reverse dependency edges | dependency read tests |
| `blocked` | ReadOnly | blocked graph listing | blocked listing tests |
| `children` | ReadOnly | direct children listing | children tests |
| `reparent-children` | Mutation | bulk parent-child move | reparent tests |
| `defect-batch-rehome` | Mutation | atomic defect batch rehome | defect batch tests |
| `tree` | ReadOnly | recursive subtree inspection | tree tests |
| `validate-graph` | ReadOnly | graph consistency validation | graph validation tests |
| `dep add/ensure/add-bulk/remove` | Mutation | dependency edge mutation | dependency mutation tests |
| `critical-path` | ReadOnly | critical path report | critical-path tests |

## Proxy Family Rows

| Family | Root alias | Authority owner | Notes |
| --- | --- | --- | --- |
| TaskFlow consume | `vida consume`, `vida taskflow consume ...` | TaskFlow runtime consumption | Alias must not invent authority; run-id, packet, receipt, and continuation binding remain TaskFlow-owned. |
| TaskFlow recovery | `vida recovery`, `vida taskflow recovery ...` | TaskFlow recovery projection | Recovery output is evidence for operator action, not a replacement for authoritative task state. |
| TaskFlow route | `vida route`, `vida taskflow route ...` | TaskFlow route diagnostics | Route diagnostics are read/projection surfaces unless the forwarded subcommand mutates state. |
| Lane/takeover | `vida lane ...` | lane surface plus run-graph receipts | Local write authority requires active exception metadata and path-scoped `owned_write_scope`. |
| Approval | `vida approval ...` | approval surface plus run-graph approval law | Approval mutation must preserve receipt-backed state transitions. |
| DocFlow | `vida docflow ...` | DocFlow runtime | Documentation state and checks are DocFlow-owned, even when invoked from root. |

## Coverage Gaps For Follow-Up

1. Promote this matrix into generated or checked fixtures once VH-01/VH-02 introduce the golden operator JSON harness.
2. Add a drift test that compares `Command` and `TaskCommand` variants against this inventory.
3. Add state-dir parity tests for command families that preserve explicit state-dir and for parse-only families that intentionally do not bind project root.

-----
artifact_path: process/runtime-command-authority-inventory
artifact_type: process_doc
artifact_version: '1'
artifact_revision: 2026-06-18
schema_version: '1'
status: canonical
source_path: docs/process/runtime-command-authority-inventory.md
created_at: '2026-06-18T00:00:00+03:00'
updated_at: 2026-06-18T00:00:00+03:00
changelog_ref: runtime-command-authority-inventory.changelog.jsonl
