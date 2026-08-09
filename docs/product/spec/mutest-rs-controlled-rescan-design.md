# Controlled mutest-rs Diff Scan And Rescan Design

Status: `implemented`

## Summary
- Feature / change: file-level mutation-audit registry with diff scan, partial resume, test-update loop, and defect handoff.
- Owner layer: `project`
- Runtime surface: `launcher`
- Status: implemented in `scripts/vida-mutest-audit.ps1`; proof remains evidence-driven.

## Current Context
- `scripts/vida-mutest-audit.ps1` runs mutest-rs by isolated one-file workers with a dynamic queue, resource caps, timeouts, checkpoints, and JSONL events.
- The canonical index orchestrates the wave: it selects queued rows, records `running`, and applies each terminal worker result to the same unique file row.
- The repository can be dirty during an explicitly requested working-tree audit; the audit must still identify a deterministic snapshot and content hash.

## Goal
- Maintain one durable registry entry for every in-scope production Rust file under `crates/*/src/**/*.rs`.
- Default to a Git/index diff scan: queue new, changed, or pending files and resume only compatible completed records.
- Make `-FullRescan` an explicit invalidation mode that queues every candidate file.
- Require a strict mutation score `> 90%` by default; a score equal to 90%, lower, zero denominator, or no-coverage is `needs_tests`; a successful controlled test-update hook changes the file to `needs_rescan` and requeues the same file for a bounded rescan.
- Persist mutation evidence, bug records, recommendations, and remediation protocol handoff without restoring or reconciling TaskFlow state.
- Out of scope: generated/freezed/mock Rust, registry dependencies, arbitrary source edits, hidden test mutation, and TaskFlow recovery.

## Requirements

### Functional Requirements
- Production scope filter: `crates/<package>/src/**/*.rs`, excluding path/name segments containing `generated`, `freezed`, or `mock`.
- Registry fields: `path`, `package`, `hash`/`content_hash_sha256`, `status`, `mutation_score`, `killed`, `survived`, `timeout`, `no_coverage`, `needs_tests`, `needs_rerun`, `needs_rescan`, `last_scan_hash`, `last_scan_run_id`, `last_wave_id`, `wave_status`, `wave_updated_at`, `wave_count`, `defects`, `recommendations`, `test_update_status`, and provenance/config hashes.
- Diff queue reasons: `new_file`, `content_hash_changed`, `needs_tests`, `needs_rescan`, `needs_rerun`, `incompatible_config`, or `full_rescan`.
- Compatible resume requires identical SHA-256 content hash, mutation configuration hash, completed status, and all follow-up flags false.
- Every worker keeps stdout/stderr, metadata, isolated target, per-file report, events, and checkpoint evidence.
- Final artifacts include `file-registry.json`, `events.jsonl`, `parallel-report.json`, `parallel-report.md`, `defects.jsonl`, and `defect-remediation.json`.

### Non-Functional Requirements
- Preserve the existing default `MaxWorkers=5`; CPU and free-memory caps remain authoritative.
- Queue refill is immediate after a terminal worker; timeout kills the complete process tree.
- Registry writes are atomic; a partial run remains resumable after interruption.
- Test updates are opt-in and controlled by `-AutoUpdateTests -TestUpdateCommand`; no arbitrary source mutation is silently performed.

## Ownership And Canonical Surfaces
- Project docs / specs affected: this document and `docs/process/mutest-rs-audit-script-adaptation.md`.
- Framework protocols affected: none; TaskFlow runtime state is intentionally not restored or reconciled.
- Runtime families affected: mutest launcher, Cargo worker isolation, evidence/defect handoff.
- Config / receipts / runtime surfaces affected: only run evidence and the durable file registry below `.vida/evidence/mutest-audit`.

## Design Decisions

### 1. Registry is the wave orchestrator and source of file-level continuity
Will implement / choose:
- Store one schema-v3 JSON document at `.vida/evidence/mutest-audit/file-registry.json` with `index_role=mutation_wave_orchestrator`, exactly one row per normalized path, and top-level wave summaries without file arrays.
- Update the same row atomically on queue, worker start, worker completion, test update, and rescan; append only compact wave summary metadata.
- Why: package reports alone cannot support hash-based partial resume, dynamic file waves, or file-level follow-up state.
- Trade-offs: worker reports are diagnostic evidence, while the registry remains the only status authority.
- Alternatives considered: ephemeral manifests (rejected because interruption loses continuity) and TaskFlow state restoration (forbidden by the operator request).

### 2. Diff scan and FullRescan are separate modes
Will implement / choose:
- Default candidate set is the Git/index snapshot plus explicitly included working-tree Rust files; `-FullRescan` bypasses all compatible records.
- Why: normal runs stay bounded while a deliberate audit can invalidate every file.
- Trade-offs: an untracked working-tree file is eligible only with `-IncludeWorkingTree` and is hashed from its current bytes.
- Alternatives considered: always scanning all packages (rejected due to unnecessary mutation cost).

### 3. Test-update/rescan loop is fail-closed
Will implement / choose:
- The score gate is strict: `mutation_score_percent > threshold_percent`, with default threshold `90`; equal-to-threshold, lower, zero denominator, or no-coverage sets `needs_tests=true` and emits a defect/recommendation.
- If the configured update hook exits zero, the record becomes `needs_rescan=true` and a bounded same-file rescan runs; a still-low result returns to `needs_tests`.
- Why: tests must be updated before claiming adequate mutation evidence.
- Trade-offs: without an explicit hook, the registry remains actionable instead of mutating source unexpectedly.
- Alternatives considered: automatic generated tests (rejected; violates project test-writing protocol).

### 4. Toolchain launch and mutest-rs metadata are automatic and explicit in evidence
Will implement / choose:
- Auto-discover Git, Cargo, rustup, and `cargo-mutest.exe` from PATH or standard
  Windows/user-tool locations. `cargo-mutest.exe` is resolved from the project `.vida` cache, sibling
  `mutest-rs\target\release`/`debug`, or the user Cargo bin directory; retain
  `-MutestCargoPath` as an override and record the source in the config hash.
- Auto-discover the MSVC `windows.lib` directory when no native path override is
  supplied. Each worker receives an isolated writable `TMP`/`TEMP` directory and
  the required native `RUSTFLAGS`, preventing linker fallback to `C:\WINDOWS`.
- Select `--lib` or `--bin <name>` automatically from the production source path;
  retain `--all-targets` only for non-standard target paths.
- Emit one command-manifest entry per queued file with its exact
  `--filter-mutations file:<repo-relative-path>` selector; the canonical registry
  remains one deduplicated row per production file.
- Accept comma-separated file/package selectors at the CLI boundary so batch waves
  remain shell-safe across PowerShell child-process launches.
- Parse the native mutest-rs `mutations.json`/`evaluation.json` schema before the
  generic recursive fallback so generated/evaluated/killed/survived/timeout and
  compile-error totals are not silently reported as zero.
- Why: the first full scan exposed a Windows temp-path linker failure and a
  mutest-driver target-path panic; both must be represented in the command/config
  hash and evidence.
- Trade-offs: auto-discovery can select a stale local tool binary, so the absolute
  path, source, tool commit, nightly, and native path remain in provenance; changed
  values queue files as `incompatible_config`.
- Alternatives considered: modifying the external mutest-rs checkout (rejected;
  the workaround stays in an ignored project temp copy) and treating unknown
  metadata as zero (rejected; that masks mutation evidence).

## Technical Design

### Core Components
- `Get-ProductionFilesForSnapshot`: production Rust scope and diff candidate discovery.
- `Get-FileContentHash`: SHA-256 bytes for each candidate.
- `Get-FileRegistryPlan`: compatibility, queue reason, and deleted-file classification.
- File-wave scheduler: one isolated Cargo target/metadata pair per file, repo-relative `--filter-mutations file:<path>`, timeout, process-tree cleanup, checkpoint, and refill events.
- `Invoke-TestUpdateHook` + `Invoke-SynchronousRescan`: controlled test update and one bounded same-file rescan cycle.
- `Write-DefectProtocolPlan`: defect handoff to the canonical runtime diagnostics and ZOMBIE-D protocols.

### Data / State Model
- Registry top-level: `schema_version=3`, `index_role`, `registry_revision`, `run_id`, `last_wave_id`, `snapshot_mode`, `snapshot_index_tree`, `config_hash`, `threshold_percent`, `full_rescan`, `diff_scan`, `waves`, `needs_*`, and unique `files`.
- File lifecycle: `queued -> running -> completed | needs_tests | needs_rescan | blocked | timeout` with wave metadata on the same row.
- A compatible completed record is retained with `resume_source=compatible_registry`; changed/new records carry a queue reason.
- Deleted snapshot files are retained as `deleted_from_snapshot` with all rerun flags cleared.

### Integration Points
- mutest-rs command remains `cargo +nightly-2026-07-18 mutest run` with isolated `--target-dir` and `--metadata-out-root-dir`; custom Windows launches use the explicit `rustup run <nightly> <cargo-mutest> ...` form and native-lib path.
- Stats parsing consumes the native `mutations.json`/`evaluation.json` detection counters before generic fallback aggregation.
- Test-update hook receives `{file}` and `{package}` placeholders and writes result evidence under the run directory.
- Defects link to `docs/process/project-error-search-runtime-diagnostics-protocol.md`; test changes follow `docs/process/zombie-d-test-writing-protocol.md`.

### Bounded File Set
- `scripts/vida-mutest-audit.ps1`
- `scripts/tests/vida-mutest-audit.tests.ps1`
- `docs/process/mutest-rs-audit-script-adaptation.md`
- `docs/product/spec/mutest-rs-controlled-rescan-design.md`

## Fail-Closed Constraints
- Do not restore, reconcile, or synthesize TaskFlow runtime state.
- Do not scan generated/freezed/mock files or registry dependencies.
- Reject dirty worktrees unless `-IncludeWorkingTree` is explicit.
- Reject resume on provenance, command/config hash, package, or file-content drift.
- Keep worker stdout/stderr and process-tree cleanup in `finally` paths.
- Do not claim a green file when score is below threshold, denominator is zero, no-coverage exists, or a worker is blocked/timed out.

## Implementation Plan

### Phase 1
- Add production scope, SHA-256 registry, diff/full-rescan planning, and PlanOnly artifacts.
- First proof target: `pwsh -NoProfile -File scripts/vida-mutest-audit.ps1 -PlanOnly -IncludeWorkingTree -FullRescan -Json`.

### Phase 2
- Connect worker reports to per-file follow-up flags, defect JSONL, update hook, and bounded rescan.
- Second proof target: `pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/tests/vida-mutest-audit.tests.ps1`.

### Phase 3
- Run the selected diff scan, preserve partial evidence, and process the defect handoff after terminal workers.
- Final proof target: script-check gate, `git diff --check`, registry/report inspection, and targeted mutation evidence.

## Validation / Proof
- Unit tests: PowerShell parser and registry/queue contract tests in `scripts/tests/vida-mutest-audit.tests.ps1`.
- Integration tests: PlanOnly diff and FullRescan manifests with persisted registry.
- Runtime checks: worker timeout/process-tree cleanup, dynamic refill, report/checkpoint/events artifacts.
- Canonical checks:
  - `activation-check`: not used to restore TaskFlow state.
  - `protocol-coverage-check`: use project gate when available.
  - `check`: script parser and contract tests.
  - `doctor`: classify runtime/worker blockers through the defect protocol.

## Observability
- `events.jsonl`: run, worker, refill, test-update, rescan, drift, and defect-protocol events.
- `parallel-report.json`: aggregate, worker evidence, and registry/wave references; no per-file status array.
- `file-registry.json`: the single durable lifecycle, hash, and wave state index.
- Every file row also carries `loc` (non-empty source lines), `loc_total` (physical lines), and `loc_hash`; `-RefreshIndex` backfills these metrics without launching mutest workers so small files can be selected first.
- `defects.jsonl` and `defect-remediation.json`: confirmed mutation gaps and protocol actions.

## Rollout Strategy
- Start with PlanOnly and inspect the diff queue.
- Run the bounded diff scan with `MaxWorkers=5`; use `-Resume` only for compatible run checkpoints/registry records.
- Apply test updates through the explicit hook, then allow the recorded same-file rescan to close each row.
- Use `-FullRescan` only as a deliberate audit mode.

## Future Considerations
- Add native mutest metadata source-file attribution to remove package-aggregate fallback.
- Add a project-owned test-update adapter once the canonical write protocol exposes a stable command.
- Add a machine-readable defect owner/family classifier after the first completed diff scan.

## References
- `scripts/vida-mutest-audit.ps1`
- `docs/process/mutest-rs-audit-script-adaptation.md`
- `docs/process/zombie-d-test-writing-protocol.md`
- `docs/process/project-error-search-runtime-diagnostics-protocol.md`

-----
artifact_path: product/spec/mutest-rs-controlled-rescan-design
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-08-09'
schema_version: '1'
status: implemented
source_path: docs/product/spec/mutest-rs-controlled-rescan-design.md
created_at: '2026-08-08T00:00:00+03:00'
updated_at: '2026-08-08T00:00:00+03:00'
changelog_ref: mutest-rs-controlled-rescan-design.changelog.jsonl
