# Launcher Decomposition And Code Hygiene Design

Status: approved bounded implementation design

Purpose: record the next safe launcher decomposition slices, validate dead-code and duplication claims against the current workspace, and bind one immediate implementation slice that reduces concentration without widening runtime behavior.

## 1. Scope

This design covers:

1. the first bounded extraction slice from `crates/vida/src/main.rs`,
2. the next safe extraction seam identified in `crates/vida/src/state_store.rs`,
3. validation of current dead-code and duplication claims,
4. proof targets for the first implementation wave.

This design does not cover:

1. a full launcher-to-family carve-out,
2. broader codex-era materialization/proof residue cleanup,
3. complete `lane` / `approval` family-owned implementations,
4. broad crate-merging across TaskFlow and DocFlow families.

## 2. Current Findings

### 2.1 File Concentration

Current file concentration remains high:

1. `crates/vida/src/main.rs` is `7370` lines.
2. `crates/vida/src/state_store.rs` is `11510` lines.

### 2.2 Dead-Code And Duplication Validation

Validated claims:

1. JSONL helpers are byte-identical across:
   - `crates/taskflow-format-jsonl/src/lib.rs`
   - `crates/docflow-format-jsonl/src/lib.rs`
2. Toon render helpers are near-identical across:
   - `crates/taskflow-format-toon/src/lib.rs`
   - `crates/docflow-format-toon/src/lib.rs`
3. launcher `super::*` coupling is real but narrower than the independent audit claimed; current direct occurrences are bounded rather than universal.

Completed deduplication in this wave:

1. JSONL helpers now route through shared `crates/common-format-jsonl`.
2. Toon helpers now route through shared `crates/common-format-toon`.

Not yet validated enough for immediate cleanup:

1. `docflow-format-toon` as a truly dead crate rather than a thin utility surface,
2. broad config-crate deduplication,
3. API-constructor asymmetry as a release-blocking issue.

## 3. Safe Extraction Seams

### 3.1 First Slice: `main.rs`

Safest first extraction target:

1. task-command and feature-request helper cluster:
   - `shell_quote`
   - `build_task_create_command`
   - `build_task_ensure_command`
   - `build_task_show_command`
   - `build_task_close_command`
   - `infer_feature_request_slug`
   - `infer_feature_request_title`

Why this cluster is first:

1. it is pure and deterministic,
2. it has no storage or async ownership,
3. it reduces launcher concentration without changing runtime law,
4. it is directly regression-testable through existing launcher tests.

### 3.2 Second Slice: `state_store.rs`

Next safe extraction target after the first slice:

1. instruction patch utility cluster:
   - `split_lines`
   - `join_lines`
   - `apply_patch_operation`
   - `resolve_operation_target`
   - `validate_patch_conflicts`
   - `validate_patch_bindings`
   - `collect_patch_ids`

Why this is the next honest seam:

1. these helpers are locally cohesive,
2. they are patch-domain logic rather than broad state-store authority,
3. they already have bounded tests,
4. the extraction can stay inside the `state_store` module boundary and avoid public API churn.

## 4. Immediate Implementation Plan

This wave now implements:

1. extract the `main.rs` task-command and feature-request helper cluster into one bounded module,
2. extract the `state_store.rs` instruction patch utility cluster into one bounded submodule,
3. extract the `main.rs` project-root/path helper cluster into one bounded module,
4. keep signatures stable to avoid broad call-site churn,
5. validate no behavior changes through targeted cargo tests.

Completed in this wave:

1. `crates/vida/src/launcher_task_commands.rs`
2. `crates/vida/src/state_store_patching.rs`
3. `crates/vida/src/project_root_paths.rs`
4. `crates/vida/src/state_store_taskflow_snapshot_codec.rs`
5. `crates/vida/src/state_store_source_scan.rs`
6. `crates/vida/src/state_store_run_graph_summary.rs`
7. `crates/common-format-jsonl`
8. `crates/common-format-toon`

## 5. Proof Targets

Proof for this wave must include:

1. targeted launcher tests that exercise root command/help flows,
2. targeted consume/handoff tests that rely on the extracted helpers,
3. a green bounded regression run after the extraction,
4. updated canonical docs if the decomposition status materially changes.

## 6. Next Moves

Next after this wave:

1. continue decomposing the remaining `state_store.rs` authority seams only where contracts stay local and proof coverage already exists,
2. next safe bounded seam is the `project_activator_surface.rs` codex materialization/render cluster, but only if it can be split without changing host-system behavior,
3. continue the carrier-neutral burn-down by removing remaining codex-era proof/materialization residue now that `codex_runtime_assignment` readers are gone,
4. decide whether additional tiny format/helper crates should remain standalone or be folded into broader shared utility crates later.

-----
artifact_path: product/spec/launcher-decomposition-and-code-hygiene-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-04-08'
schema_version: '1'
status: canonical
source_path: docs/product/spec/launcher-decomposition-and-code-hygiene-design.md
created_at: '2026-04-08T14:20:00+03:00'
updated_at: 2026-04-08T09:37:14Z
changelog_ref: launcher-decomposition-and-code-hygiene-design.changelog.jsonl
