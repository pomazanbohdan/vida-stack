# VIDA 1 Development Conditions

Purpose: record the currently proven development, build, install, and runtime-entry conditions for active `VIDA 1` work so implementation can move from successful milestone to successful milestone without rediscovering local execution rules.

## Scope

This file tracks only conditions that are already proven in the current repository state.

It does not replace canonical product law in `docs/product/spec/**`.

## Retention And Compression Rule

1. this file is a proven-condition ledger for active `VIDA 1` development, not a second owner of product law,
2. retain only conditions that are already proven in the current repository state,
3. command families may remain enumerated here while runtime surfaces are still actively converging and no stricter generated proof inventory exists,
4. compress large command inventories only after another retained canonical surface can carry the same proved execution conditions without loss.

## Active Proven Conditions

### Rust Workspace

The current Rust workspace is green for the active modernization bootstrap.

Proven commands:

1. `cargo test --workspace`
2. `cargo test -p vida`
3. `cargo fmt --all`
4. `cargo test -p taskflow-format-jsonl -p taskflow-format-toon -p docflow-format-jsonl -p docflow-format-toon`
5. `cargo test -p taskflow-config -p docflow-config`
6. `cargo test -p taskflow-state`
7. `cargo test -p taskflow-state-fs`
8. `cargo test -p taskflow-state-surreal`
9. `cargo test -p docflow-markdown`
10. `cargo test -p docflow-inventory`
11. `cargo test -p docflow-validation`
12. `cargo test -p docflow-readiness`
13. `cargo test -p docflow-relations`
14. `cargo test -p docflow-operator`
15. `cargo test -p docflow-cli`
16. `cargo run -p docflow-cli -- overview --registry-count 4 --relation-count 2`
17. `cargo run -p vida -- docflow overview --registry-count 5 --relation-count 2`
18. `cargo run -p vida -- docflow validate-footer --path docs/process/test.md --content '# title\n'`
19. `cargo run -p vida -- docflow readiness --path docs/process/test.md --content '# title\n'`
20. `cargo run -p vida -- docflow check-file --path <markdown-file>`
21. `cargo run -p vida -- docflow readiness-file --path <markdown-file>`
22. `cargo run -p vida -- docflow registry-scan --root <scan-root>`
23. `cargo run -p vida -- docflow overview-scan --root <scan-root>`
24. `cargo run -p vida -- docflow validate-tree --root <scan-root>`
25. `cargo run -p vida -- docflow readiness-tree --root <scan-root>`
26. `cargo run -p vida -- docflow relations-scan --root <scan-root>`
27. `cargo run -p vida -- docflow registry-write --root <scan-root> --output <jsonl-path>`
28. `cargo run -p vida -- docflow readiness-write --root <scan-root> --output <jsonl-path>`
29. `cargo run -p vida -- docflow registry --root <scan-root>`
30. `cargo run -p vida -- docflow readiness-check --root <scan-root>`
31. `cargo run -p vida -- docflow layer-status --layer <N>`
32. `cargo run -p vida -- docflow summary --root <scan-root>`
33. `cargo run -p vida -- docflow scan --root <scan-root>`
34. `cargo run -p vida -- docflow fastcheck --root <scan-root>`
35. `cargo run -p vida -- docflow doctor --root <scan-root>`
36. `cargo run -p vida -- docflow activation-check --root <scan-root>`
37. `cargo run -p vida -- docflow protocol-coverage-check --root <scan-root>`
38. `cargo run -p vida -- docflow proofcheck --layer <N>`
39. `cargo run -p vida -- docflow registry-write --root <scan-root> --canonical`
40. `cargo run -p vida -- docflow readiness-write --root <scan-root> --canonical`
41. `cargo run -p vida -- docflow links --path <markdown-file>`
42. `cargo run -p vida -- docflow deps-map --path <path>`
43. `cargo run -p vida -- docflow artifact-impact --artifact <artifact-path> --root <scan-root>`
44. `cargo run -p vida -- docflow task-impact --task-id <task-id> --root <scan-root>`
45. `cargo run -p vida -- docflow help`
46. `cargo run -p vida -- docflow check --profile active-canon`
47. `cargo run -p vida -- docflow fastcheck --profile active-canon`
48. `cargo run -p vida -- docflow activation-check --profile active-canon`
49. `cargo run -p vida -- docflow protocol-coverage-check --profile active-canon`
50. `cargo run -p vida -- docflow readiness-check --profile active-canon`
51. `cargo run -p vida -- docflow proofcheck --profile active-canon-strict`
52. `cargo run -p vida -- docflow finalize-edit <markdown-file> "<change-note>" [--status <value>] [--artifact-revision <value>] [--set key=value]`
53. `cargo run -p vida -- docflow touch <markdown-file> "<change-note>"`
54. `cargo run -p vida -- docflow rename-artifact <markdown-file> <artifact-path> "<change-note>" [--artifact-type <value>] [--bump-version]`
55. `cargo run -p vida -- docflow init <markdown-file> <artifact-path> <artifact-type> "<change-note>" [--title <value>] [--purpose <value>]`
56. `cargo run -p vida -- docflow move <markdown-file> <destination> "<change-note>"`

Current focused Rust test harness condition:

1. the format-foundation crates already have green package-local golden tests for JSONL and TOON fixtures under `tests/golden/**`
2. this is a bounded Wave 1 test-first foundation, not yet the full parity/recovery matrix for `taskflow-rs` and `docflow-rs`
3. the initial config-foundation crates also have green package-local validation/load tests for `taskflow-config` and `docflow-config`
4. the initial `taskflow-state` kernel crate has green package-local trait/in-memory store tests before filesystem and SurrealDB adapters are introduced
5. the first `taskflow-state-fs` adapter is green as a snapshot-backed filesystem proof surface before richer adapter behavior is introduced
6. the initial `taskflow-state-surreal` adapter bootstrap is green as an adapter-owned target/defaults surface before full embedded SurrealDB runtime wiring is introduced
7. the initial `docflow-markdown` kernel crate is green for footer split/render and changelog append primitives before higher mutation, inventory, and validation semantics are layered on top
8. the initial `docflow-inventory` service crate is green for bounded markdown-tree inventory materialization with include/exclude scope control and deterministic row ordering
9. the initial `docflow-validation` service crate is green for bounded footer-presence and registry-row validation before readiness/proof aggregation is layered on top
10. the initial `docflow-readiness` aggregation crate is green for deterministic readiness-row materialization and verdict summarization over validation issues
11. the initial `docflow-relations` service crate is green for bounded artifact-identity and reverse-reference edge materialization with deterministic ordering
12. the initial `docflow-operator` service crate is green for compact overview and relation-summary rendering over the new Rust DocFlow surfaces
13. the initial `docflow-cli` thin shell is green for Rust-native `overview` and `relations` command routing over the new DocFlow operator surface
14. the `docflow-cli` package now also exposes a real standalone `docflow` binary surface in addition to the library shell used by `vida`
15. supported `vida docflow` commands now run in-process by default for bounded `overview`, `validate-footer`, and `readiness` surfaces over the new Rust crates
16. the same in-process Rust `vida docflow` surface is now proven for file-backed `check-file` and `readiness-file` commands over real markdown files
17. the in-process Rust `vida docflow` surface is now also proven for bounded `registry-scan` inventory materialization over a real markdown tree
18. the same in-process Rust `vida docflow` surface is now also proven for bounded `overview-scan` synthesis over a real markdown tree with real relation-edge counting
19. the same in-process Rust `vida docflow` surface is now also proven for bounded `validate-tree` aggregation over a real markdown tree
20. the same in-process Rust `vida docflow` surface is now also proven for bounded `readiness-tree` aggregation over a real markdown tree
21. the same in-process Rust `vida docflow` surface is now also proven for bounded `relations-scan` aggregation over a real markdown tree
22. the same in-process Rust `vida docflow` surface is now also proven for bounded `registry-write` export of canonical registry rows into JSONL
23. the same in-process Rust `vida docflow` surface is now also proven for bounded `readiness-write` export of canonical readiness rows into JSONL
24. the same in-process Rust `vida docflow` surface is now also proven for bounded `registry` JSONL streaming of canonical registry rows
25. the same in-process Rust `vida docflow` surface is now also proven for bounded `readiness-check` JSONL streaming of canonical readiness rows
26. the same in-process Rust `vida docflow` surface is now also proven for bounded `layer-status` reads over the canonical documentation layer matrix
27. the same in-process Rust `vida docflow` surface is now also proven for bounded `summary` synthesis over a real markdown tree
28. the same in-process Rust `vida docflow` surface is now also proven for bounded `scan` JSONL streaming of markdown inventory rows with footer/changelog state
29. the same in-process Rust `vida docflow` surface is now also proven for bounded `fastcheck` JSONL streaming of validation issues over a real markdown tree
30. the same in-process Rust `vida docflow` surface is now also proven for bounded `doctor` JSONL streaming of error rows over a real markdown tree
31. the same in-process Rust `vida docflow` surface is now also proven for bounded `activation-check` JSONL streaming of missing activation-binding rows
32. the same in-process Rust `vida docflow` surface is now also proven for bounded `protocol-coverage-check` JSONL streaming of activation and protocol-index coverage rows
33. the same in-process Rust `vida docflow` surface is now also proven for bounded `proofcheck --layer` aggregation over already implemented fastcheck, protocol-coverage, readiness, and doctor surfaces
34. the same in-process Rust `vida docflow` surface is now also proven for canonical-path `registry-write --canonical` export into `vida/config/codex-registry.current.jsonl`
35. the same in-process Rust `vida docflow` surface is now also proven for canonical-path `readiness-write --canonical` export into `vida/config/codex-readiness.current.jsonl`
36. the same in-process Rust `vida docflow` surface is now also proven for bounded `links` JSONL streaming over markdown-body link extraction and path resolution
37. the same in-process Rust `vida docflow` surface is now also proven for bounded `deps-map` JSONL streaming over markdown-link and footer-ref relation edges
38. the same in-process Rust `vida docflow` surface is now also proven for bounded `artifact-impact` impact tracing over footer refs and markdown-link references
39. the same in-process Rust `vida docflow` surface is now also proven for bounded `task-impact` tracing over changelog task rows and indirect artifact impacts
40. `vida docflow help` now renders the real in-process DocFlow command map instead of the generic proxy-argument shell, while still disclosing donor fallback posture for unsupported commands
41. `vida docflow check --profile active-canon` now runs in-process against the current policy profile roots instead of delegating to the Python donor runtime
42. `vida docflow fastcheck --profile active-canon` now runs in-process against the same policy profile roots instead of delegating to the Python donor runtime
43. `vida docflow activation-check --profile active-canon` now runs in-process against the same policy profile roots instead of delegating to the Python donor runtime
44. `vida docflow protocol-coverage-check --profile active-canon` now runs in-process against the same policy profile roots instead of delegating to the Python donor runtime
45. `vida docflow readiness-check --profile active-canon` now runs in-process against the same policy profile roots instead of delegating to the Python donor runtime
46. `vida docflow proofcheck --profile active-canon-strict` now runs in-process with donor-matching profile totals instead of delegating to the Python donor runtime
47. `vida docflow finalize-edit` now runs in-process for bounded footer metadata updates, changelog append, and quiet changed-file validation instead of delegating to the Python donor runtime
48. `vida docflow touch` now runs in-process for bounded changelog append and quiet changed-file validation instead of delegating to the Python donor runtime
49. `vida docflow rename-artifact` now runs in-process for bounded artifact-path/type updates, optional version bump, changelog append, and quiet changed-file validation instead of delegating to the Python donor runtime
50. `vida docflow init` now runs in-process for bounded artifact creation, footer/bootstrap metadata initialization, changelog append, and quiet changed-file validation instead of delegating to the Python donor runtime
51. `vida docflow move` now runs in-process for bounded markdown/changelog relocation, footer source-path/changelog-ref updates, move-event append, and quiet changed-file validation instead of delegating to the Python donor runtime
52. `vida docflow changelog` now runs in-process for bounded changelog sidecar reads instead of delegating to the Python donor runtime
53. `vida docflow changelog-task` now runs in-process for bounded task-id changelog scans instead of delegating to the Python donor runtime
54. `vida docflow task-summary` now runs in-process for bounded task event aggregation instead of delegating to the Python donor runtime
55. `vida docflow migrate-links` now runs in-process for bounded markdown link rewrites, changelog append, and quiet changed-file validation instead of delegating to the Python donor runtime
56. `docflow-cli` link existence validation now resolves against the active runtime root (`VIDA_ROOT`) instead of the repository root, so bounded link mutation and validation stay correct in temp/runtime-scoped trees
57. `vida docflow` launcher routing is now Rust-first and fail-closed for unsupported commands; it no longer silently falls through to the Python donor wrapper path during bridge-wave execution
58. `vida doctor` now reports bounded TaskFlow task-store and run-graph health summaries in addition to storage, spine, compatibility, migration, and effective-bundle checks
59. `docflow-cli` activation-check and protocol-coverage-check now resolve the canonical activation protocol from the current bridge path when the legacy flat file path is absent, keeping existing in-process validation surfaces green
60. `vida taskflow help` is now launcher-owned and layered: the default surface exposes purpose, source-of-truth notes, canonical command homes, operator recipes, and fail-closed posture, while `vida taskflow help <topic>` and `vida taskflow <command> --help` provide bounded task/consume/run-graph/doctor command contracts without delegating help rendering to the donor runtime
61. `vida taskflow query` is now launcher-owned and deterministic: it accepts bounded workflow questions, returns one recommended TaskFlow command with intent/why/failure-mode guidance, and keeps the operator query surface local to the `vida` binary without model-, role-, or project-specific hardcoding
62. `vida taskflow` now resolves the project root from the current working directory by walking ancestor markers when `VIDA_ROOT` is unset, exports that resolved root into the delegated runtime, and fails closed on ambiguous multi-root ancestry instead of silently guessing
63. an installed `vida` binary now resolves its sibling `taskflow-v0` runtime from the active executable `bin/` root while still deriving the delegated `VIDA_ROOT` project context from the current working directory, so binary placement and project-PWD bootstrap no longer depend on one manually exported path doing both jobs
64. `vida task deps <task-id>`, `vida task reverse-deps <task-id>`, and `vida task blocked` now read dependency edges directly from the authoritative TaskFlow runtime store, returning dependency-aware planning views without export/import patching and with `ready` semantics staying blocked-edge aware
65. `vida task tree <task-id>` now renders the full dependency tree directly from the authoritative TaskFlow runtime store, preserving edge types and dependency statuses across recursive parent-child and blocking chains without export/import patching or donor-side graph inspection
66. `vida task ready --scope <task-id>` now computes the ready set inside one authoritative parent-child subtree from the TaskFlow runtime store, so operators can query ready work under a chosen epic/task scope while preserving the same blocked-edge gating as the global ready surface
67. `vida task validate-graph` now validates authoritative TaskFlow dependency edges in-process, returning a structured issue set and non-zero exit for broken graphs such as missing dependency targets, self-dependencies, multiple parent-child parents, or parent-child cycles, while succeeding cleanly for valid runtime graphs
68. `vida task dep add <task-id> <depends-on-id> <edge-type>` and `vida task dep remove <task-id> <depends-on-id> <edge-type>` now mutate authoritative TaskFlow dependency edges in-process with preflight graph validation, normalized `task_dependency` table sync, and natural `--json` CLI rendering on the concrete add/remove subcommands
69. the flat canonical prompt-template source paths `vida/config/instructions/prompt-templates.cheap-worker-prompt-pack.md` and `vida/config/instructions/prompt-templates.worker-packet-templates.md` are restored as proof/runtime-discoverable artifacts again, while the clustered prompt-template documents remain as maintained companion guides under distinct artifact ids so `active-canon` validation and strict proofcheck no longer fail on missing or duplicate prompt-home artifacts
70. `vida task critical-path` now computes a deterministic longest unresolved `blocks` chain directly from the authoritative TaskFlow runtime store, returning a bounded planning path from root blocker to terminal downstream task and failing closed if the dependency graph must be repaired first
71. `vida doctor` now also verifies launcher/runtime path resolution and native dependency graph health in addition to storage, spine, task-store, run-graph, compatibility, migration, and effective-bundle checks, so operators can see the active `vida` executable path, resolved project root, resolved TaskFlow runtime path, and zero-issue graph health in one fail-closed diagnosis surface
72. the unified user-facing `vida` CLI now exposes explicit sibling runtime-family command homes at the root surface (`vida taskflow`, `vida docflow`) with bounded runtime-family help, root command-family discovery, and fail-closed routing semantics instead of hidden launcher ambiguity
73. the TaskFlow golden corpus now also covers native dependency-planning outputs for `critical-path` and broken-edge `validate-graph` failure output under `tests/golden/taskflow/**`, and the binary task-smoke suite proves those fixtures stay stable alongside the package-local JSONL and TOON golden tests
74. the TaskFlow binary smoke suite now also proves fail-closed dependency-mutation behavior by rejecting a second `parent-child` edge for the same task, preserving the original authoritative dependency set after the rejected mutation and thereby extending runtime failure-mode coverage for the native dependency graph commands
75. the TaskFlow test foundation now includes a donor-parity semantic fixture for `taskflow-v0 task ready --json` under `tests/golden/taskflow/donor_ready_semantic.json`, and the binary `vida` smoke suite proves that both donor and Rust runtime surfaces agree on the ready-set semantic core for the bounded sample graph even though their raw JSON wire shapes remain intentionally different
76. the TaskFlow test foundation now also includes a donor-parity semantic fixture for `taskflow-v0 task show <task-id> --json` under `tests/golden/taskflow/donor_show_semantic.json`, and the binary `vida` smoke suite proves that both donor and Rust runtime surfaces agree on the bounded operator-relevant semantic core for task identity, status, priority, issue type, and dependency edges even though their raw JSON wire shapes remain intentionally different
77. the TaskFlow test foundation now also includes a donor-parity semantic fixture for the default `taskflow-v0 task list --json` surface under `tests/golden/taskflow/donor_list_semantic.json`, and the binary `vida` smoke suite proves that both donor and Rust runtime surfaces agree on the visible operator list semantic core for open and in-progress tasks, ordering, priority, issue type, and dependency edges while continuing to tolerate intentional raw JSON shape differences
78. the TaskFlow binary smoke suite now also proves fail-closed recovery for `vida task show <task-id> --json` on an initialized store when the requested task id is missing, returning a non-zero exit and an explicit `task is missing` operator error instead of silently rendering empty output
79. `taskflow-state-fs` now also proves bounded cross-adapter persistence semantics by materializing a deterministic snapshot directly from the `taskflow-state` store contract and restoring that snapshot back into `InMemoryTaskStore`, while `taskflow-state` and `taskflow-state-surreal` remain green for the same package-local state kernel and canonical Surreal target/defaults contracts
80. `taskflow-state-fs` now also proves file-backed end-to-end store export/import semantics via `write_store_snapshot` and `read_snapshot_into_memory`, so a bounded `taskflow-state` store can be serialized to disk and restored through the filesystem adapter without losing task rows or dependency edges
81. `taskflow-state-surreal` now also proves deterministic product-store target layout semantics by exposing the canonical embedded backend label `kv-surrealkv` plus namespace-qualified and database-qualified on-disk roots, so the Surreal adapter owns an explicit runtime-home contract instead of only validating non-empty target/default inputs
82. `taskflow-state-surreal` now also proves a canonical storage metadata contract through `SurrealStorageMeta`, exposing the engine/backend/namespace/database tuple that the product store must report, so future runtime/operator metadata surfaces can reuse one adapter-owned source of truth for embedded Surreal identity
83. `taskflow-state-surreal` now also fixes the canonical schema-version payload in its adapter-owned storage metadata contract, so the embedded product-store identity tuple carries the same `state_schema_version` and `instruction_schema_version` values that the runtime bootstraps into `storage_meta`
84. `taskflow-state-surreal` now also exposes one deterministic bootstrap payload object containing both the validated store target and the canonical storage metadata tuple, so future embedded Surreal bootstrap/wiring paths can consume one adapter-owned contract instead of recomposing target and metadata pieces ad hoc
85. `taskflow-state-surreal` now also renders the canonical `UPSERT storage_meta:primary` statement from its adapter-owned bootstrap contract, so embedded Surreal bootstrap paths can reuse one deterministic storage-metadata write payload that matches the runtime’s `storage_meta` row semantics
86. `taskflow-state-surreal` now also renders a deterministic bootstrap schema bundle for the bounded state-family tables plus the canonical `storage_meta` upsert, so the embedded product-store bootstrap contract is available as one adapter-owned statement set instead of being implied only by runtime-local literals
87. `taskflow-state-surreal` now also renders that bounded bootstrap schema as one deterministic document string, so embedded Surreal bootstrap/wiring paths can consume one adapter-owned schema payload instead of joining statement fragments ad hoc
88. `vida` runtime state bootstrap now reuses the adapter-owned `taskflow-state-surreal` bootstrap schema document for the bounded state-family tables, and the `vida` unit test suite proves the active runtime schema still contains that canonical Surreal adapter bootstrap without local drift
89. `vida` state-spine manifest defaults now also reuse the adapter-owned `taskflow-state-surreal` state spine contract, and the `vida` unit test suite proves the active runtime defaults for manifest id, schema version, authoritative mutation root, and entity surfaces stay aligned with the canonical Surreal adapter contract without local drift
90. `vida` backend summary now validates the persisted `storage_meta` row against the canonical `taskflow-state-surreal` storage contract and fails closed on backend or schema-version drift instead of treating any present metadata row as acceptable
91. `vida` state spine summary now validates the persisted `state_spine_manifest` row against the canonical `taskflow-state-surreal` state spine contract and fails closed on manifest drift instead of only checking for missing or trivially empty fields
92. `vida` storage metadata validation now covers the full canonical `taskflow-state-surreal` contract tuple for `engine`, `backend`, `namespace`, `database`, and schema versions, and fails closed on namespace/database drift in addition to backend/version drift
93. `vida status --json` and `vida doctor --json` now expose launcher-owned typed runtime summaries for canonical storage metadata, state spine, compatibility/preflight state, and bundle/runtime path surfaces; the binary-level proof is green through `cargo test -p vida`, `cargo run -q -p vida -- status --state-dir <temp-state> --json`, and `cargo run -q -p vida -- doctor --state-dir <temp-state> --json`
94. `vida` boot compatibility and migration preflight now fail closed on canonical Surreal state-contract drift instead of collapsing those failures into generic missing-state reasons; the package proof is green through `cargo test -p vida`, including explicit tests for storage metadata drift in boot compatibility and state spine contract drift in migration preflight
95. `vida` authoritative state store can now export its task graph into the canonical `taskflow-state-fs` snapshot contract and round-trip that snapshot into `taskflow-state` in-memory runtime form; the bounded proof is green through `cargo test -p vida`, including one Surreal-backed export round-trip test and one fail-closed test for unsupported `issue_type` mapping during canonical snapshot export
96. `vida` authoritative state store can now write the canonical `taskflow-state-fs` snapshot artifact to disk and restore it back through the canonical snapshot reader into `taskflow-state` in-memory form; the bounded proof is green through `cargo test -p vida`, including a real file-backed export/import round-trip from the Surreal-backed authoritative store
97. `vida` authoritative state store can now import the canonical `taskflow-state-fs` snapshot contract back into Surreal-backed authoritative state, both from in-memory snapshot objects and from file-backed snapshot artifacts; the bounded proof is green through `cargo test -p vida`, including canonical snapshot import tests that rehydrate `show`, `ready`, and dependency surfaces from imported taskflow-state records
98. `vida` canonical snapshot restore semantics now support full authoritative replacement instead of add/update-only import: replacement mode validates the whole imported task graph before mutation and then removes stale authoritative tasks/dependencies that are absent from the incoming snapshot; the bounded proof is green through `cargo test -p vida`, including tests for pre-mutation fail-closed invalid-graph rejection and stale-task cleanup during replacement restore
99. `vida` canonical snapshot import now retains minimal authoritative provenance for imported rows instead of leaving closure-relevant fields empty: imported tasks record `source_repo=taskflow-state-fs`, closed canonical tasks derive `closed_at` from the canonical update timestamp and set a deterministic import close reason, and imported dependency rows record deterministic snapshot provenance; the bounded proof is green through `cargo test -p vida`
100. `vida` task snapshot bridge now records persisted reconciliation receipts in `task_reconciliation_summary` for file-backed export and replacement restore operations, including operation kind, source kind/path, task/dependency counts, and stale-removal counts; the bounded proof is green through `cargo test -p vida`
101. `vida status --json` and `vida doctor --json` now surface the latest authoritative task reconciliation receipt directly from `task_reconciliation_summary`, so snapshot-bridge evidence is not only persisted but also launcher-readable; the bounded proof is green through `cargo test -p vida`, plus real booted runtime checks for `cargo run -q -p vida -- status --json` and `cargo run -q -p vida -- doctor --json`
102. `vida status --json` and `vida doctor --json` now also expose a bounded task-reconciliation rollup with total receipt count, per-operation counts, per-source-kind counts, and latest recorded timestamp, so canonical snapshot-bridge evidence is visible as cumulative runtime shape in addition to the latest receipt row; the bounded proof is green through `cargo test -p vida`, plus real booted runtime checks for `cargo run -q -p vida -- status --json` and `cargo run -q -p vida -- doctor --json`
103. `vida` canonical snapshot bridge now records reconciliation evidence for in-memory export as well as file-backed export/import/replace: `export_taskflow_in_memory_store()` persists an `export_snapshot` receipt with `source_kind=canonical_snapshot_memory`, and the bounded proof is green through `cargo test -p vida`, including receipt/rollup assertions on the in-memory export path
104. `vida` canonical snapshot bridge now records authoritative reconciliation evidence for all three export shapes: raw snapshot-object export, in-memory export, and file-backed export each persist `export_snapshot` receipts with distinct `source_kind` values (`canonical_snapshot_object`, `canonical_snapshot_memory`, `canonical_snapshot_file`); the bounded proof is green through `cargo test -p vida`, including receipt/rollup assertions on the object and memory export paths
105. `vida` canonical snapshot bridge proof parity now covers import and replace receipt semantics across memory/file variants as well: memory import, file import, memory replace, and file replace all have bounded receipt/rollup assertions over `task_reconciliation_summary`, including `source_kind`, counts, stale-removal behavior, and source path where applicable; the bounded proof is green through `cargo test -p vida`
106. `vida status --json` and `vida doctor --json` now expose a semantic `taskflow_snapshot_bridge` runtime summary in addition to raw reconciliation receipts, including export/import/replace counts, object/memory/file bucket counts, and latest bridge operation/source kind; the bounded proof is green through `cargo test -p vida`, plus real booted runtime checks for `cargo run -q -p vida -- status --json` and `cargo run -q -p vida -- doctor --json`
107. the canonical snapshot bridge now has a bounded cross-store round-trip proof between authoritative Surreal-backed stores: export from one authoritative store, replace into a second authoritative store, then re-export preserves canonical snapshot task/dependency semantics and authoritative readiness behavior; the bounded proof is green through `cargo test -p vida`
108. `vida status --json` and `vida doctor --json` now expose deeper aggregate snapshot-bridge persistence semantics: both `task_reconciliation_rollup` and `taskflow_snapshot_bridge` carry total task rows, dependency rows, stale-removal totals, and latest source path in addition to receipt counts and latest operation/source-kind buckets; the bounded proof is green through `cargo test -p vida`, plus real booted runtime checks for `cargo run -q -p vida -- status --json` and `cargo run -q -p vida -- doctor --json`
109. the canonical snapshot bridge now also has a bounded file-backed cross-store round-trip proof between authoritative Surreal-backed stores: file export from one store, file-backed replace into a second store, and re-export preserves canonical task/dependency snapshot semantics and authoritative readiness behavior; the bounded proof is green through `cargo test -p vida`, and the SurrealKV reopen/idempotency proof is hardened with bounded retry to stay stable under local lock timing
110. `vida status --json` and `vida doctor --json` now expose refined `taskflow_snapshot_bridge` buckets split by exact operation/source combinations instead of one mixed memory/file count: `memory_export_receipts`, `memory_import_receipts`, `memory_replace_receipts`, `file_export_receipts`, `file_import_receipts`, and `file_replace_receipts` are all present alongside the existing aggregate counters; the bounded proof is green through `cargo test -p vida`, plus real booted runtime checks for `cargo run -q -p vida -- status --json` and `cargo run -q -p vida -- doctor --json`
111. canonical snapshot replacement now has an explicit guardrail proving stale dependency rows are removed for kept tasks when the incoming snapshot shrinks their dependency set: replacing a task that previously depended on a blocker now leaves only the surviving canonical dependency edge and an empty graph-validation result; the bounded proof is green through `cargo test -p vida`
112. canonical snapshot import now also has an additive-update guardrail proving stale dependency rows are replaced for updated tasks without deleting unrelated authoritative tasks: importing a snapshot that narrows one task's dependency set rewrites that task to the canonical incoming edge set while preserving other authoritative tasks outside the import payload; the bounded proof is green through `cargo test -p vida`
113. canonical snapshot additive import now validates against the merged authoritative graph instead of only the incoming payload, so imported tasks may depend on already existing authoritative tasks outside the snapshot payload while still failing closed on post-merge graph violations; the bounded proof is green through `cargo test -p vida`
114. canonical snapshot additive import now also has an explicit fail-closed post-merge conflict guardrail: if an incoming payload would create a merged-graph violation such as `multiple_parent_edges` against the existing authoritative graph, the import is rejected before mutation, leaves the authoritative task/dependency rows unchanged, and emits no reconciliation receipt; the bounded proof is green through `cargo test -p vida`
115. file-backed canonical snapshot additive import now has the same merged-graph positive parity as the in-memory path: `import_taskflow_snapshot_file` accepts dependencies on already existing authoritative tasks outside the snapshot payload, preserves a valid merged graph, and records the canonical file-backed import receipt; the bounded proof is green through `cargo test -p vida`
116. file-backed canonical snapshot additive import now also has the same fail-closed post-merge conflict guardrail as the in-memory path: if a snapshot file would create a merged-graph violation such as `multiple_parent_edges`, the import is rejected before mutation, leaves authoritative task/dependency rows unchanged, and emits no reconciliation receipt; the bounded proof is green through `cargo test -p vida`
117. additive canonical snapshot import now also has explicit reconciliation-summary parity for both accepted and rejected memory/file paths: accepted imports increment the corresponding `taskflow_snapshot_bridge` import buckets (`memory_import_receipts` or `file_import_receipts`) with the expected latest operation/source data, while rejected post-merge imports leave the bridge summary empty and emit no reconciliation receipt; the bounded proof is green through `cargo test -p vida`
118. additive canonical snapshot imports now also have cumulative mixed-path rollup parity: one accepted memory import plus one accepted file-backed import in the same authoritative store accumulate the expected `import_snapshot` operation counts, memory/file source-kind buckets, total task/dependency row counts, and latest file source path in both `task_reconciliation_rollup` and `taskflow_snapshot_bridge`; the bounded proof is green through `cargo test -p vida`
119. canonical snapshot reconciliation now also has cross-operation aggregate parity: one accepted memory additive import plus one accepted file-backed replacement in the same authoritative store accumulate the expected `import_snapshot` and `replace_snapshot` counts, `memory_import_receipts` and `file_replace_receipts` buckets, cumulative task/dependency totals, and `stale_removed` totals while preserving the replaced authoritative end state; the bounded proof is green through `cargo test -p vida`
120. canonical snapshot reconciliation receipts and summaries now also have reopen durability proof on the authoritative Surreal-backed store: after an additive import plus file-backed replacement, reopening the same state root preserves the latest reconciliation receipt, cumulative rollup counters, bridge summary buckets, `stale_removed` totals, latest file source path, and the replaced authoritative task state; the bounded proof is green through `cargo test -p vida`
121. migration preflight runtime state and receipt summaries now also have reopen durability proof on the authoritative Surreal-backed store: after a compatible preflight evaluation, reopening the same state root preserves the persisted migration preflight summary, the canonical source-version tuple, the `normal_boot_allowed` next step, and the migration receipt counters exposed by `migration_receipt_summary`; the bounded proof is green through `cargo test -p vida`
122. boot compatibility runtime state now also has reopen durability proof on the authoritative Surreal-backed store: after a compatible compatibility evaluation, reopening the same state root preserves the persisted boot compatibility classification, the empty-reasons success state, and the `normal_boot_allowed` next step exposed by `latest_boot_compatibility_summary`; the bounded proof is green through `cargo test -p vida`
123. the first Rust-native TaskFlow flow-kernel state contract now exists in the authoritative Surreal-backed store and is launcher-visible on empty-state runtime summaries: `RunGraphStatus` records and reloads typed execution-plan, routed-run, governance, and resumability state through `record_run_graph_status`, `run_graph_status`, and `latest_run_graph_status`, while `vida status` and `vida doctor` now expose the latest run-graph status in both JSON and text surfaces; the bounded proof is green through `cargo test -p vida`, including round-trip and reopen persistence for one routed run plus booted empty-state launcher coverage
124. the first donor-aligned recovery projection now exists on top of the Rust TaskFlow flow-kernel state: `latest_run_graph_recovery_summary` derives `resume_node`, `resume_status`, `checkpoint_kind`, `resume_target`, `policy_gate`, `handoff_state`, and `recovery_ready` from the latest authoritative `RunGraphStatus`, and `vida status` / `vida doctor` now expose that recovery summary in both JSON and text surfaces; the bounded proof is green through `cargo test -p vida`, including round-trip and reopen persistence assertions for one routed run plus booted empty-state launcher coverage
125. `vida taskflow recovery status <run-id> [--json]` now exists as a launcher-owned read-only inspection surface over the authoritative Rust flow state: it reads one run’s donor-aligned recovery summary directly from `run_graph_recovery_summary`, returns JSON or text without delegating to the donor runtime, and fails closed on missing run ids; the bounded proof is green through `cargo test -p vida`, including help-surface coverage and fail-closed runtime inspection on a booted empty state
126. `vida taskflow run-graph status <run-id> [--json]` now also exists as a launcher-owned read-only inspection surface over the authoritative Rust flow state: it reads one run’s full `RunGraphStatus` directly from the Surreal-backed state store, returns JSON or text without delegating to the donor runtime, and fails closed on invalid or unavailable state; the bounded proof is green through `cargo test -p vida`, including fail-closed boot-smoke coverage on an initialized empty state
127. `vida taskflow recovery latest [--json]` now exists as a launcher-owned read-only inspection surface over the authoritative Rust flow state: it returns the latest donor-aligned recovery summary when present, returns `none`/`null` on a booted state with no routed runs, and shares a bounded open-existing retry guardrail with the local `taskflow` recovery/run-graph inspection paths so short SurrealKV post-command lock windows do not break read-only inspection; the bounded proof is green through `cargo test -p vida`
128. `vida taskflow run-graph latest [--json]` now exists as the symmetric launcher-owned read-only inspection surface over the authoritative Rust flow state: it returns the latest `RunGraphStatus` when present, returns `none`/`null` on a booted state with no routed runs, and is routed locally by the launcher instead of falling through to the donor runtime; the bounded proof is green through `cargo test -p vida`, including lock-safe post-boot smoke coverage for both `run-graph latest` and `recovery latest`
129. `vida taskflow recovery checkpoint <run-id> [--json]` and `vida taskflow recovery checkpoint-latest [--json]` now expose a bounded checkpoint projection over the authoritative Rust flow state: they read `checkpoint_kind`, `resume_target`, and `recovery_ready` directly from the canonical run-graph status contract, return `none`/`null` for the latest checkpoint on a booted state with no routed runs, and fail closed on unknown run ids; the bounded proof is green through `cargo test -p vida`
130. `vida taskflow recovery gate <run-id> [--json]` and `vida taskflow recovery gate-latest [--json]` now expose a bounded gate/handoff projection over the authoritative Rust flow state: they read `policy_gate`, `handoff_state`, and `context_state` directly from the canonical run-graph status contract, return `none`/`null` for the latest gate on a booted state with no routed runs, and fail closed on unknown run ids; the bounded proof is green through `cargo test -p vida`
131. `vida status --json` and `vida doctor --json` now expose `latest_run_graph_checkpoint` and `latest_run_graph_gate` beside the existing latest run-graph status/recovery summaries, so the launcher-owned runtime summaries cover all currently promoted Rust flow projections in one place; the same slice also extends the persisted flow proof to direct and latest checkpoint/gate summaries across reopen in `StateStore`, and the bounded proof is green through `cargo test -p vida`, `cargo run -q -p vida -- status --json`, and `cargo run -q -p vida -- doctor --json`
132. `vida taskflow query` now routes operator questions about `gate` and `latest` recovery state to the promoted launcher-owned flow inspection surfaces instead of falling back to the generic task inspection answer: gate-focused questions map to `vida taskflow recovery gate <run-id> --json`, and latest resumability questions map to `vida taskflow recovery latest --json`; the bounded proof is green through `cargo test -p vida`
133. donor-backed `vida taskflow run-graph init/update` now bridge into the authoritative Rust flow state instead of leaving launcher-owned summaries blind to donor run events: after a successful donor mutation, `vida` reads the donor run-graph artifact, projects it into the canonical `RunGraphStatus`, records it in the Surreal-backed state store selected by `VIDA_STATE_DIR`, and the launcher-owned `run-graph latest`, `recovery latest`, `checkpoint-latest`, `gate-latest`, `vida status --json`, and `vida doctor --json` surfaces all return the same non-empty routed-run state; the same fix also hardens all local flow inspection surfaces to honor `VIDA_STATE_DIR` instead of a hardcoded default state root, and the bounded proof is green through `cargo test -p vida`
134. plain `vida status` and plain `vida doctor` now also have end-to-end non-empty proof over the same donor-backed routed run: after `vida taskflow run-graph init/update`, both text-mode operator surfaces show the bridged latest run-graph status, recovery, checkpoint, and gate projections for the authoritative Rust flow state instead of only the empty-state path; the bounded proof is green through `cargo test -p vida`
135. the direct per-run launcher-owned flow inspection surfaces now also have end-to-end non-empty proof over the same bridged donor run: `vida taskflow run-graph status <run-id> --json`, `vida taskflow recovery status <run-id> --json`, `vida taskflow recovery checkpoint <run-id> --json`, and `vida taskflow recovery gate <run-id> --json` all return the expected canonical Rust flow projections after donor-backed `run-graph init/update`; the bounded proof is green through `cargo test -p vida`
136. `vida taskflow run-graph init` and `vida taskflow run-graph update` are now launcher-owned in-process Rust paths instead of donor-backed bridge calls: they write canonical `RunGraphStatus` rows directly into the authoritative state store selected by `VIDA_STATE_DIR`, preserve the promoted read-side semantics for `run-graph` and `recovery` inspection surfaces, accept donor-shaped `meta_json` for bounded compatibility, and no longer require the donor run-graph artifact path for the launcher-visible flow kernel; the bounded proof is green through `cargo test -p vida`
137. the `vida taskflow help run-graph` contract is now aligned with the actual runtime after the in-process cutover: it describes `run-graph` as a launcher-owned in-process mutation and inspection surface and no longer claims delegated-runtime failure semantics for `init/update`; the bounded proof is green through `cargo test -p vida`
138. `vida taskflow consume bundle [--json]` and `vida taskflow consume bundle check [--json]` now exist as launcher-owned in-process runtime-consumption surfaces over the authoritative Rust state: they render the active effective instruction bundle, launcher/runtime path receipt, boot compatibility, migration preflight, task-store summary, and run-graph summary directly from the booted state store, while `consume final` remains intentionally delegated until the wider closure-loop parity slice lands; the bounded proof is green through `cargo test -p vida` plus real booted runtime checks for `vida taskflow consume bundle --json` and `vida taskflow consume bundle check --json`
139. the new launcher-owned `vida taskflow consume bundle` surfaces now also persist runtime-owned snapshot artifacts under `<state-root>/runtime-consumption/*.json` instead of emitting only transient stdout: both `bundle` and `bundle check` write a JSON snapshot path alongside their operator output, so the direct runtime-consumption branch now leaves durable evidence that can be reused by the later `consume final` cut; the bounded proof is green through `cargo test -p vida`
140. `vida taskflow consume final <request_text> [--json]` is now launcher-owned and in-process over the authoritative Rust state and bounded Rust `DocFlow` branch: it consumes the already-proven runtime bundle/check surfaces, activates `vida docflow check --profile active-canon`, `vida docflow readiness-check --profile active-canon`, and `vida docflow proofcheck --profile active-canon`, emits one runtime-owned `taskflow_direct_runtime_consumption` payload with explicit `docflow_activation` evidence and `direct_consumption_ready`, and persists the final closure snapshot under `<state-root>/runtime-consumption/final-*.json`; the bounded proof is green through `cargo test -p vida` plus a real booted runtime check for `vida taskflow consume final "probe closure" --json`
141. the launcher operator contract is now aligned with the in-process `consume final` cutover: `vida taskflow help consume` and `vida taskflow query` no longer imply a delegated final closure loop, and instead describe the real fail-closed posture where final runtime consumption blocks on runtime-bundle readiness or bounded `DocFlow` evidence failures; the bounded proof remains green through `cargo test -p vida`
142. `vida status --json` and `vida doctor --json` now expose runtime-consumption visibility from the authoritative state root: after a launcher-owned `vida taskflow consume final`, both operator surfaces report `runtime_consumption` totals, `final_snapshots`, `latest_kind`, and `latest_snapshot_path`, so the new direct runtime-consumption evidence is visible through the same bounded diagnostics family as the rest of the runtime state; the bounded proof is green through targeted `cargo test -p vida taskflow_consume_final_renders_direct_runtime_consumption_snapshot` plus real booted `status --json` and `doctor --json` checks after `consume final`
143. `vida taskflow consume final <request_text> [--json]` now emits donor-aligned overlay-driven `role_selection` data instead of the old placeholder lane-selection stub: fallback requests such as `probe closure` stay on the configured fallback role with `auto_no_keyword_match`, while scope-shaped requests such as `clarify spec scope` resolve to `scope_discussion`, `business_analyst`, `spec-pack`, and the matched-term evidence derived from `vida.config.yaml` plus the project agent-extension registries; the bounded proof is green through `cargo test -p vida`, targeted smoke tests for both fallback and keyword-match paths, and real booted runtime checks for both `vida taskflow consume final "probe closure" --json` and `vida taskflow consume final "clarify spec scope" --json`
144. `taskflow-v0/src/vida task create <task_id> <title> --parent-id <parent_id> --auto-display-from <parent_display_id> --description <description> [--labels <label>]... --json` and `taskflow-v0/src/vida task update <task_id> --description <description> --json` now have live DB-backed proof for epic backlog mutation against the authoritative runtime store: under `VIDA_ROOT` plus `VIDA_V0_TURSO_PYTHON`, they created `vida-rf1-taskflow-agent-system` as `vida-rf1.1.9` beneath `vida-rf1-taskflow` and corrected its stored description in place, proving parent-child creation, auto display-id allocation, and post-create task repair without relying on `.beads/issues.jsonl`
145. `vida taskflow doctor [--json]` is now launcher-owned and in-process over the same bounded Rust doctor surface as root `vida doctor`: the TaskFlow runtime-family path no longer delegates doctor execution to the donor binary, still renders the canonical launcher/runtime integrity summary, and proves the local routing cut even when `VIDA_TASKFLOW_BIN` points at an executable sentinel that would fail if invoked; the bounded proof is green through targeted `cargo test -p vida taskflow_doctor_routes_in_process_without_taskflow_binary -- --nocapture`, regression coverage for `doctor_surface_supports_json_summary`, and a full `cargo test -p vida`
146. `vida taskflow help task` and `vida taskflow query` now surface the donor-backed backlog mutation/operator contract for `task create`, `task next-display-id`, and `task export-jsonl` without changing backlog authority away from `taskflow-v0` + DB: task-topic help lists the canonical create/display-id/export command shapes, query routing now answers create/display-id/export questions with those exact runtime commands, and the bounded proof is green through targeted `cargo test -p vida taskflow_proxy_ -- --nocapture`, targeted `cargo test -p vida taskflow_query_ -- --nocapture`, and a full `cargo test -p vida`
147. `vida taskflow help doctor` and health-shaped `vida taskflow query ...` prompts now point to the local `vida taskflow doctor` path instead of the older root-only `vida doctor` wording: the TaskFlow runtime-family guidance now references the same in-process doctor command it actually executes, and the bounded proof is green through targeted `cargo test -p vida taskflow_proxy_help_supports_doctor_topic -- --nocapture`, `cargo test -p vida taskflow_query_recommends_doctor_for_health_questions -- --nocapture`, re-validation of `taskflow_doctor_routes_in_process_without_taskflow_binary`, and a full `cargo test -p vida`
148. `vida taskflow task ...` now routes `task` subcommands through the launcher-owned donor-backed DB bridge before any generic external proxy fallback: `ready`, `create`, `import-jsonl`, and nested-root resolution all execute against `taskflow-v0/helpers/turso_task_store.py` with the resolved `VIDA_ROOT`, while truly unhandled `taskflow` commands still fall through to the resolved runtime binary; the bounded proof is green through targeted bridge smoke tests for unhandled fallback, local `task ready`, local `task create` with auto display-id allocation, nested project-root resolution, installed-binary resolution, the full `cargo test -p vida taskflow_ -- --nocapture`, and a full `cargo test -p vida`
149. `vida taskflow consume final <request_text> --json` now emits a broader compiled project agent-system proof bundle instead of only role-selection registries: alongside the already-proven lane-selection data, the runtime payload now includes the configured native `agent_system` section from `vida.config.yaml` and a project-local Codex multi-agent summary derived from `.codex/config.toml` plus the role-specific `.codex/agents/*.toml` files, so the launcher-visible closure surface proves `mode=native`, `internal_subagents` routing posture, `max_parallel_agents=4`, and the current implementer/coach/verifier/escalation execution topology without hardcoded legacy provider assumptions; the bounded proof is green through targeted `cargo test -p vida taskflow_consume_final_renders_direct_runtime_consumption_snapshot -- --nocapture`, `cargo test -p vida taskflow_consume_final_selects_scope_discussion_role_for_spec_queries -- --nocapture`, and a full `cargo test -p vida`
150. `vida taskflow run-graph seed <task_id> <request_text> [--json]` now materializes the initial native project agent-system dispatch state directly into the authoritative Rust run-graph instead of leaving `execution_plan` as a `consume final`-only proof artifact: scope-shaped requests such as `clarify spec scope` seed a `scope_discussion` conversation cursor that routes to `business_analyst` / `spec-pack`, while implementation-shaped requests such as `continue development` seed the default implementation dispatch with `analysis` as the next lane, `validation_report_required` as the policy gate, and the overlay-driven `internal_subagents` backend; the bounded proof is green through `cargo test -p vida --test boot_smoke taskflow_run_graph_seed_builds_scope_discussion_state_from_configured_agent_system -- --exact --nocapture` and `cargo test -p vida --test boot_smoke taskflow_run_graph_seed_builds_implementation_dispatch_state_for_default_route -- --exact --nocapture`
151. `vida taskflow run-graph advance <task_id> [--json]` now provides the first launcher-owned post-seed transition surface for the configured native implementation route: for a run seeded from `continue development`, the authoritative Rust run-graph advances from `planning` to `analysis`, then derives the next handoff from the overlay route contract without manual `meta_json`, producing the `coach` next node, `analysis_lane`, `targeted_verification` policy gate, and `dispatch.coach_lane` resume target; the supporting root-cause fix is that YAML-backed booleans in `vida.config.yaml` now accept string forms such as `yes/no`, which the routing config already uses for `coach_required`; the bounded proof is green through targeted `cargo test -p vida --test boot_smoke taskflow_run_graph_advance_builds_coach_handoff_for_seeded_implementation -- --exact --nocapture`, `cargo test -p vida --test boot_smoke taskflow_run_graph_advance_updates_status_and_recovery_for_seeded_implementation -- --exact --nocapture`, and a full `cargo test -p vida`
152. `vida taskflow run-graph advance <task_id> [--json]` now also covers the seeded `scope_discussion` conversational path, so launcher-owned derived transitions are no longer implementation-only: for a run seeded from `clarify spec scope`, `advance` moves the authoritative Rust run-graph from `planning` to `business_analyst`, derives the next node as `spec-pack`, keeps `business_analyst_lane` and `single_task_scope_required`, preserves the `conversation_cursor`, and sets `resume_target=dispatch.spec-pack` without manual `meta_json`; the bounded proof is green through targeted `cargo test -p vida --test boot_smoke taskflow_run_graph_advance_builds_spec_pack_handoff_for_seeded_scope_discussion -- --exact --nocapture`, `cargo test -p vida --test boot_smoke taskflow_run_graph_advance_updates_status_and_recovery_for_seeded_scope_discussion -- --exact --nocapture`, and a full `cargo test -p vida`
153. launcher-owned native conversational dispatch parity now covers both currently configured standard modes from `vida.config.yaml`: `vida taskflow consume final "prioritize backlog work pool" --json` proves `pbi_discussion -> pm -> work-pool-pack`, `vida taskflow run-graph seed <task_id> ... --json` seeds the corresponding `pbi_discussion` conversation cursor with `pm_lane`, and `vida taskflow run-graph advance <task_id> [--json]` advances that seeded run to `pm` with the next handoff derived as `work-pool-pack`, `resume_target=dispatch.work-pool-pack`, and updated recovery/status state, all without manual `meta_json`; the bounded proof is green through targeted `cargo test -p vida --test boot_smoke taskflow_consume_final_selects_pbi_discussion_role_for_backlog_queries -- --exact --nocapture`, `cargo test -p vida --test boot_smoke taskflow_run_graph_seed_builds_pbi_discussion_state_from_configured_agent_system -- --exact --nocapture`, `cargo test -p vida --test boot_smoke taskflow_run_graph_advance_builds_work_pool_pack_handoff_for_seeded_pbi_discussion -- --exact --nocapture`, `cargo test -p vida --test boot_smoke taskflow_run_graph_advance_updates_status_and_recovery_for_seeded_pbi_discussion -- --exact --nocapture`, and a full `cargo test -p vida`
154. launcher-owned native implementation dispatch now also covers the second post-seed handoff beyond `coach`: after the existing implementation path advance from `planning` to `analysis`, a second `vida taskflow run-graph advance <task_id> [--json]` moves the authoritative Rust run-graph to `coach`, derives the next handoff from overlay-native verification routing as `review_ensemble`, sets `policy_gate=review_findings`, `handoff_state=awaiting_review_ensemble`, and `resume_target=dispatch.review_ensemble`, and persists matching recovery/status summaries without manual `meta_json`; the bounded proof is green through targeted `cargo test -p vida --test boot_smoke taskflow_run_graph_advance_builds_review_ensemble_handoff_after_coach_for_implementation -- --exact --nocapture`, `cargo test -p vida --test boot_smoke taskflow_run_graph_second_advance_updates_status_and_recovery_for_implementation -- --exact --nocapture`, and a full `cargo test -p vida`
155. launcher-owned TaskFlow bridge parity now explicitly covers the remaining in-process `task` commands that were already implemented but previously under-proven: a bounded installed/project-aware smoke slice seeds a tiny backlog, then proves `vida taskflow task list --all --json`, `show`, `next-display-id`, `update`, `close`, and `export-jsonl` all succeed through the local DB bridge without requiring the delegated runtime binary, in both nested project-root resolution and installed-mode helper resolution paths; the bounded proof is green through targeted `cargo test -p vida --test boot_smoke taskflow_task_bridge_keeps_missing_in_process_commands_off_delegated_runtime_in_project_and_installed_modes -- --exact --nocapture` and a full `cargo test -p vida`
156. the first launcher/install migration slice now explicitly bridges the `docflow` runtime family to the existing installed donor payload without renaming archive layout or `codex-v0`: installer wrappers expose `vida docflow` as the canonical top-level launcher contract, the release manifest records that installed entrypoint explicitly, and a bounded regression proves `vida docflow ...` resolves to the installed `codex-v0/codex.py` path through the bundled Python runtime in a temp install root; the bounded proof is green through `python3 -m unittest tests/test_install_docflow_bridge.py`, `bash -n install/install.sh`, and `bash -n scripts/build-release.sh`
157. the active `v0.2.2` TaskFlow protocol-binding bridge is now script-era-primary in `taskflow-v0` and DB-first over the authoritative runtime spine instead of detached file logs: `taskflow-v0 protocol-binding build [--json]` materializes a deterministic compiled payload under `taskflow-v0/generated/protocol_binding.compiled.json`, `taskflow-v0 protocol-binding sync [--json]` imports the bounded Wave-1 runtime-bearing subset (`bridge.instruction-activation-protocol`, `work.taskflow-protocol`, `runtime.task-state-telemetry-protocol`, `work.execution-health-check-protocol`, `work.task-state-reconciliation-protocol`) into persisted `protocol_binding_state` and `protocol_binding_receipt` rows inside `.vida/state/taskflow-state.db`, `status`/`check` query that same runtime truth, and runtime work now fails closed until that DB import exists; the bounded proof is green through `nimble test` in `taskflow-v0`, `python3 -m unittest tests/test_install_docflow_bridge.py`, a real repo-local CLI cycle for `taskflow-v0 protocol-binding build|sync|check`, and a temp-install archive bootstrap path where installer `doctor` stays green after the import
157. the second launcher/install migration slice now hardens installed `vida docflow` from a raw donor passthrough into an explicit bounded compatibility wrapper while keeping the same bundled donor payload: the installed wrapper serves `vida docflow help` locally, allows only the canonical supported `vida docflow overview ...` path to delegate to bundled `codex-v0/codex.py`, and fails closed for unsupported installed-mode commands instead of passing raw donor arguments through; the bounded proof is green through `python3 -m unittest tests.test_install_docflow_bridge`, `bash -n install/install.sh`, and `bash -n scripts/build-release.sh`
158. the third launcher/install migration slice now makes the installed `vida docflow` compatibility boundary explicit across launcher metadata and release-facing docs: the installer wrapper help text, release manifest metadata, canonical `DocFlow` runtime-family map, and `v0.2.1` release notes all encode installed `vida docflow` as `help|overview only`, and the targeted installer regression now asserts that literal boundary in installed help/error output plus the release-build manifest; the bounded proof is green through `python3 -m unittest tests.test_install_docflow_bridge`, `bash -n install/install.sh`, and `bash -n scripts/build-release.sh`
159. the next launcher/install migration slice now hardens installed TaskFlow helper closure: installer `doctor` fails closed if `current/taskflow-v0/helpers/turso_task_store.py` or `current/taskflow-v0/helpers/toon_render.py` is missing from the active installed layout, and installer-facing smoke proofs now cover both the negative missing-helper path and the positive install path where both helper files are materialized into `current/taskflow-v0/helpers/`; the bounded proof is green through `cargo fmt --package vida -- crates/vida/tests/boot_smoke.rs`, `cargo test -p vida installer_doctor_fails_closed_when_installed_helpers_are_missing -- --exact --nocapture`, and `cargo test -p vida installer_install_populates_both_taskflow_helpers_in_current_layout -- --exact --nocapture`
160. the next launcher/install migration slice now makes `vida docflow` explicitly mode-scoped across repo/dev and installed execution: `vida docflow help` in the repo binary declares that repo/dev mode keeps the full in-process Rust `DocFlow` shell while installed mode is a bounded compatibility wrapper with `help|overview only`, `run_docflow_proxy` fails closed for non-`overview` commands in installed compatibility mode, and smoke coverage now proves both repo/dev in-process behavior and installed-layout `overview`-only enforcement; the bounded proof is green through `cargo test -p vida --test boot_smoke docflow_proxy_help_is_runtime_specific -- --exact`, `cargo test -p vida --test boot_smoke docflow_proxy_runs_check_in_process_when_profile_is_supported -- --exact`, `cargo test -p vida --test boot_smoke docflow_proxy_can_use_rust_cli_shell -- --exact`, `cargo test -p vida --test boot_smoke installed_docflow_compatibility_mode_supports_overview_only -- --exact`, and `cargo test -p vida --test boot_smoke installed_docflow_compatibility_mode_rejects_non_overview_commands -- --exact`
161. the next launcher/install migration slice now demotes installed `codex-v0` into migration-only compatibility instead of a public installed launcher: `codex-v0 help` now prints migration guidance to use `vida docflow`, non-help installed `codex-v0` routes through the same installed `vida docflow` compatibility boundary, `vida docflow overview` bridges directly to bundled `codex-v0/codex.py` to avoid wrapper recursion, and release metadata no longer lists `codex-v0` as a public installed entrypoint while still recording its compatibility role; the bounded proof is green through `bash -n install/install.sh` and `python3 -m unittest -v tests.test_install_docflow_bridge`
93. `vida` now exposes a typed `storage_metadata_summary()` runtime read surface over the canonical `taskflow-state-surreal` metadata contract and reuses that validated struct for backend summary rendering, so storage identity is no longer only a string-formatted check path

### Nim Transitional Runtime

The transitional `taskflow-v0` runtime remains buildable and usable as the current donor/runtime substrate.

Proven commands:

1. `nim c -r taskflow-v0/tests/test_runtime_bundle.nim`
2. `VIDA_ROOT=<repo-root> taskflow-v0/src/vida --help`

Current launch conditions for `taskflow-v0`:

1. run from the project repository root or set `VIDA_ROOT=<repo-root>` explicitly
2. local binary path during active development is `taskflow-v0/src/vida`
3. DB-backed task commands additionally require `VIDA_V0_TURSO_PYTHON=<repo-root>/.venv/bin/python3`

Proven task-surface command shape:

1. `VIDA_ROOT=<repo-root> VIDA_V0_TURSO_PYTHON=<repo-root>/.venv/bin/python3 taskflow-v0/src/vida task ready --json`

### DocFlow Donor Runtime

The current documentation/runtime donor remains the Python `codex-v0` surface behind the user-facing `DocFlow` naming.

Current launch conditions for `codex-v0`:

1. run from the project repository root
2. use `python3 codex-v0/codex.py <command> ...` during local development
3. for installer-managed runs, `codex-v0` delegates to `<install-root>/current/.venv/bin/python3 <install-root>/current/codex-v0/codex.py`

Proven command shapes:

1. `python3 codex-v0/codex.py check --profile active-canon`
2. `python3 codex-v0/codex.py fastcheck --profile active-canon`
3. `python3 codex-v0/codex.py proofcheck --profile active-canon-strict`

### Release Build

The release build path is proven for the current tree.

Proven command:

1. `bash scripts/build-release.sh`

Proven release outputs:

1. `dist/vida-stack-v0.2.1.tar.gz`
2. `dist/vida-stack-v0.2.1.zip`
3. `dist/vida-install.sh`
4. `dist/vida-stack-v0.2.1.manifest.json`

Current release-manifest contract:

1. installed entrypoints are `vida`, `taskflow-v0`, `codex-v0`
2. bundled binary is `bin/taskflow-v0`

### Installer

The local installer path is proven end-to-end against a locally built archive.

Proven command shape:

1. `bash dist/vida-install.sh install --archive dist/vida-stack-v0.2.2.tar.gz --root <tmp-root> --bin-dir <tmp-bin> --force`

Current installer guarantees already proven:

1. creates an installer-managed Python runtime under `<install-root>/releases/<version>/.venv`
2. installs user-facing launchers into the configured `bin` directory
3. makes `vida`, `taskflow-v0`, and `codex-v0` executable from `PATH`
4. packages `.codex/` into the active release root
5. scaffolds `vida.config.yaml` from `install/assets/vida.config.yaml.template` when the installed release root does not already contain one
6. materializes `taskflow-v0/generated/protocol_binding.compiled.json` and imports it into `.vida/state/taskflow-state.db`
7. passes `bash install/install.sh doctor --root <tmp-root> --bin-dir <tmp-bin>`

### Unified Launcher Surface

The current user-facing launcher surface is:

1. `vida taskflow <args...>`
2. `vida docflow <args...>`
3. `vida doctor`
4. `vida root`

The current Rust `vida` binary also exposes proven runtime-family help routing.

Proven commands:

1. `cargo run -p vida -- taskflow help`
2. `cargo run -p vida -- docflow help`
3. `cargo test -p vida`
4. `cargo test -p vida` now includes proxy execution smoke for resolved `taskflow` and `docflow` runtimes
5. `cargo test -p vida docflow_proxy_can_use_rust_cli_shell -- --nocapture`
6. `cargo test -p vida docflow_proxy_can_run_rust_validation_surface -- --nocapture`
7. `cargo test -p vida docflow_proxy_can_run_rust_readiness_surface -- --nocapture`
8. `cargo test -p vida docflow_proxy_can_run_rust_check_file_surface -- --nocapture`
9. `cargo test -p vida docflow_proxy_can_run_rust_readiness_file_surface -- --nocapture`
10. `cargo test -p vida docflow_proxy_can_run_rust_registry_scan_surface -- --nocapture`
11. `cargo test -p vida docflow_proxy_can_run_rust_overview_scan_surface -- --nocapture`
12. `cargo test -p vida docflow_proxy_can_run_rust_validate_tree_surface -- --nocapture`
13. `cargo test -p vida docflow_proxy_can_run_rust_readiness_tree_surface -- --nocapture`
14. `cargo test -p vida docflow_proxy_can_run_rust_relations_scan_surface -- --nocapture`
15. `cargo test -p vida docflow_proxy_can_run_rust_registry_write_surface -- --nocapture`
16. `cargo test -p vida docflow_proxy_can_run_rust_readiness_write_surface -- --nocapture`
17. `cargo test -p vida docflow_proxy_can_run_rust_registry_surface -- --nocapture`
18. `cargo test -p vida docflow_proxy_can_run_rust_readiness_check_surface -- --nocapture`
19. `cargo test -p vida docflow_proxy_can_run_rust_layer_status_surface -- --nocapture`
20. `cargo test -p vida docflow_proxy_can_run_rust_summary_surface -- --nocapture`
21. `cargo test -p vida docflow_proxy_can_run_rust_scan_surface -- --nocapture`
22. `cargo test -p vida docflow_proxy_can_run_rust_fastcheck_surface -- --nocapture`
23. `cargo test -p vida docflow_proxy_can_run_rust_doctor_surface -- --nocapture`
24. `cargo test -p vida docflow_proxy_can_run_rust_activation_check_surface -- --nocapture`
25. `cargo test -p vida docflow_proxy_can_run_rust_protocol_coverage_check_surface -- --nocapture`
26. `cargo test -p vida docflow_proxy_can_run_rust_proofcheck_surface -- --nocapture`
27. `cargo test -p vida docflow_proxy_can_run_rust_registry_write_canonical_surface -- --nocapture`
28. `cargo test -p vida docflow_proxy_can_run_rust_readiness_write_canonical_surface -- --nocapture`
29. `cargo test -p vida docflow_proxy_runs_readiness_check_in_process_when_profile_is_supported -- --nocapture`
30. `cargo test -p vida docflow_proxy_runs_proofcheck_in_process_when_profile_is_supported -- --nocapture`
31. `cargo test -p vida docflow_proxy_runs_finalize_edit_in_process_when_supported -- --nocapture`
32. `cargo test -p vida docflow_proxy_runs_touch_in_process_when_supported -- --nocapture`
33. `cargo test -p vida docflow_proxy_runs_rename_artifact_in_process_when_supported -- --nocapture`
34. `cargo test -p vida docflow_proxy_runs_init_in_process_when_supported -- --nocapture`
35. `cargo test -p vida docflow_proxy_runs_move_in_process_when_supported -- --nocapture`

Current launcher semantics:

1. `vida taskflow` is now mixed-mode during the bridge: launcher-owned Rust paths already cover help/query, task dependency diagnostics, doctor-aligned flow inspection, `run-graph` mutation/inspection, and the full `consume` family, while remaining unsupported or not-yet-promoted taskflow paths still delegate to the transitional `taskflow-v0` runtime
2. `vida docflow` now routes the active Rust-native `DocFlow` command map in-process and fails closed for unsupported commands
3. canonical user-facing naming is `DocFlow`; the Python donor remains a parity oracle and bounded validation tool, not a launcher fallback
4. for the Rust `vida` binary, canonical runtime-family help path is currently `vida taskflow help` and `vida docflow help`
5. Rust binary proxy execution is proven through env-overridable runtime resolution for both runtime families
6. root `vida --help` is currently Clap-owned command help, while runtime-family-specific guidance is carried by `vida taskflow help` and `vida docflow help`
7. `vida docflow` now owns the currently implemented Rust-native `DocFlow` surfaces in-process inside the `vida` binary
8. the current in-process Rust-owned `vida docflow` surface is proven for `overview`, `summary`, `layer-status`, `scan`, `fastcheck`, `doctor`, `activation-check`, `protocol-coverage-check`, `readiness-check --profile active-canon`, `proofcheck --profile active-canon-strict`, `finalize-edit`, `touch`, `rename-artifact`, `init`, `move`, `changelog`, `changelog-task`, `task-summary`, `migrate-links`, `proofcheck`, `validate-footer`, `readiness`, `check-file`, `readiness-file`, `registry`, `registry-scan`, `registry-write`, `overview-scan`, `validate-tree`, `readiness-tree`, `readiness-check`, `readiness-write`, and `relations-scan`; both `registry-write` and `readiness-write` are also proven on their canonical shared artifact paths via `--canonical`
9. the transitional installed `docflow-v0` wrapper now forwards into the installed `vida docflow` Rust surface instead of invoking the Python donor runtime directly
10. `bash scripts/build-release.sh` currently produces release artifacts that include `bin/taskflow-v0`, `.codex/`, `AGENTS.sidecar.md`, the packaged runtime-config template under `install/assets/vida.config.yaml.template`, the launcher-owned task-bridge helpers under `taskflow-v0/helpers/turso_task_store.py` and `taskflow-v0/helpers/toon_render.py`, and the protocol-binding seed/compiled JSON pair under `taskflow-v0/config/` plus `taskflow-v0/generated/`; this is proven by inspecting the built `dist/vida-stack-v0.2.2.tar.gz` contents and `dist/vida-stack-v0.2.2.manifest.json`

### Project Python And Internal Agent Environment

The current project-side environment rules are:

1. the canonical backend declaration lives in `vida.config.yaml` under `agent_system.subagents.internal_subagents`
2. the canonical project Python path is `<repo-root>/.venv/bin/python3`
3. installer-managed Python dependencies are declared in `install/requirements-python.txt`
4. the current YAML stack is `ruamel.yaml`, not `PyYAML`
5. run project-side Python and runtime entrypoints from the repository root so the active working tree stays aligned with the current project surface
6. donor `taskflow-v0` DB-backed commands additionally require `VIDA_V0_TURSO_PYTHON=<repo-root>/.venv/bin/python3`
7. operator-local credentials, sessions, or cache remain local runtime state and are not part of project canon

## Documentation Update Rule

After each successful bounded milestone for `VIDA 1`, update this file with:

1. the exact successful command or command shape
2. the resulting artifact or runtime surface that now works
3. any explicit boundary or limitation that still remains transitional

Do not record aspirational steps here.

Only record proven working conditions.

## Current Known Transitional Limits

1. `codex-v0/codex.py` still remains the donor/parity oracle and canonical external proof surface during the bridge wave
2. some project/operator documentation still describes older donor-backed launcher semantics and must be realigned as follow-up documentation cleanup
3. `taskflow-v0` remains the donor/runtime substrate until `taskflow-rs` becomes the primary execution runtime

-----
artifact_path: process/vida1-development-conditions
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/process/vida1-development-conditions.md
created_at: '2026-03-11T09:00:00+02:00'
updated_at: '2026-03-12T19:00:00+02:00'
changelog_ref: vida1-development-conditions.changelog.jsonl
