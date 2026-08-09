# mutest-rs continuous audit adapter

`scripts/vida-mutest-audit.ps1` adapts the useful parts of the VIDA Mobile Dart mutation harness to the Rust workspace.

## Concurrency contract

- `MaxWorkers` defaults to **5** requested workers.
- The effective worker count is `min(requested, logical CPU cap, free-memory cap)`; the manifest records both values.
- A worker owns a unique `target-dir` and metadata root. Cargo target directories are never shared.
- The scheduler fills the initial slots, polls all active processes, and starts the next queued **file** immediately after every terminal worker (`queue_refill` event). A run is a wave; there is no barrier between files.
- Each worker owns exactly one repo-relative production file and passes `--filter-mutations file:<path>`; this prevents a large package from blocking the whole wave.
- File and global deadlines kill the complete process tree. A timeout is reported, never silently treated as a pass.
- A batch/panic-shaped mutest failure schedules that file once more with `--mutant-batch-size 1`; the retry is logged as `batch_retry_scheduled` and does not double-count the final file score.

## Audit command

Each package receives the exact command:

```text
cargo +nightly-2026-07-18 mutest run --package <pkg> --all-targets --locked
  --target-dir <isolated-target> --metadata-out-root-dir <isolated-metadata>
  --depth 3 --safe --mutation-operators all --parallel-mutants
  --mutant-batch-algorithm greedy --mutant-batch-size 1 --timings
  --filter-mutations file:crates/<pkg>/src/<file>.rs
```

The script records one command manifest entry per queued production file and a SHA-256 command hash. `-PlanOnly` persists a `manifest.json`, a `parallel-report.json` with `status=planned`, and a human-readable `parallel-report.md` under the run evidence root; it does not mutate the canonical registry, create workers, checkpoints, or event streams. `-Json` is accepted as an explicit machine-readable-output switch; JSON is the stable output format.

On Windows, the script auto-discovers Git, Cargo, rustup, and `cargo-mutest.exe`.
Git is resolved from PATH or standard Git installation paths; Cargo and rustup are
resolved from PATH or the user Cargo bin directory. `cargo-mutest.exe` is resolved from the project
`.vida` path, the sibling `mutest-rs\target\release`/`debug` paths, or the user
Cargo bin directory. `-MutestCargoPath <cargo-mutest.exe>` remains an explicit
override. The launcher runs `rustup run <nightly> <cargo-mutest.exe> ...` and
records the selected path and source (`auto`/`explicit`/`cargo-subcommand`) in
the manifest/config hash. `-MutestNativeLibPath <directory>` remains an optional
override; otherwise the script locates the MSVC `windows.lib` directory. Every
worker receives an isolated writable `TMP`/`TEMP` directory, so MSVC linker
temporary files never fall back to `C:\WINDOWS`. For `src/lib.rs` and `src/bin/*`
files the worker also selects `--lib`/`--bin` automatically; other targets retain
`--all-targets`. Resume becomes incompatible when the launcher, native library,
target selector, or environment configuration changes.

The stats adapter understands the native mutest-rs metadata shape: `mutations.json`
contributes `stats.total_mutations_count` to `generated`, while the latest
`evaluation.json` `mutation_runs[*].all_mutations_detection_stats` maps total,
detected, undetected, timed-out, and crashed counts to `evaluated`, `killed`,
`survived`, `timeout`, and `compile_error`. Unknown JSON retains the generic
recursive fallback parser; a zero killed/survived denominator remains a failed
coverage result rather than a green score.

## Provenance and resume

The manifest captures `HEAD`, `HEAD` tree, index tree, `Cargo.lock` SHA-256, nightly/rustc/cargo versions, mutest source commit (when `ToolRoot` is supplied), package set, and command hash. By default a dirty worktree is rejected; `-IncludeWorkingTree` is an explicit opt-in.

`-Resume` loads the newest checkpoint and rejects commit/tree/index/tool/nightly/package/command drift. Only completed files are skipped; failed, timed-out, blocked, and unknown files remain queued.

## Evidence and score

Per-worker file reports and the aggregate report contain `generated`, `evaluated`, `killed`, `survived`, `no_coverage`, `compile_error`, `timeout`, `flaky`, `equivalent`, `unknown`, exact commands, timings, stdout/stderr, and survivor references. Aggregate scores are split into `production` and `test_support` (`vida-test-support`). Mutation score is only `killed / (killed + survived)`; compile, infrastructure, equivalent, and no-evidence cases are excluded from the denominator.

`#[ignore]` Rust tests are scanned and listed separately. Unit, bin, and integration tests execute unchanged; production package results and `vida-test-support` can be selected/reported independently through `-Packages`.

## Controlled file registry and diff scan

- Production mutation scope is `crates/<package>/src/**/*.rs`; generated/freezed/mock paths and files are excluded.
- The default mode is a diff scan. Candidate files are hashed with SHA-256 and compared with `.vida/evidence/mutest-audit/file-registry.json`.
- The registry is schema v3 with `index_role=mutation_wave_orchestrator`: exactly one row per normalized repo-relative production path, plus top-level wave summaries. It is the only authoritative per-file index.
- Each row carries `last_wave_id`, `wave_status`, `wave_updated_at`, `wave_count`, and the current score/follow-up flags. A wave updates existing rows in place; it never appends duplicate file rows.
- `manifest.json` and `parallel-report.json` contain aggregate data and registry/wave references only. Worker reports remain evidence and are not a second status index.
- A completed record is compatible only when its content hash, configuration hash, status, and follow-up flags match. New/changed/pending records enter the queue; deleted snapshot files remain recorded as `deleted_from_snapshot`.
- `-FullRescan` is an explicit invalidation mode and queues every candidate file. It never restores or reconciles TaskFlow state.
- Registry rows carry `status`, `mutation_score`, `killed`, `survived`, `timeout`, `no_coverage`, `needs_tests`, `needs_rerun`, `needs_rescan`, `last_scan_hash`, defect ids, recommendations, and wave fields. Mutest metadata is filtered to the exact source file, so no package aggregate is copied across multiple file rows.
- The score gate is strict: `mutation_score_percent > 90` by default (the configured threshold must stay at least 90); exactly 90, lower, a zero denominator, or no coverage produces `needs_tests` and a defect entry. With `-AutoUpdateTests -TestUpdateCommand '<command with {file} and {package}>'`, a successful update sets `needs_rescan` and starts one bounded package rescan; a still-low result returns to `needs_tests`.
- The run writes `defects.jsonl` plus `defect-remediation.json`, linking each issue to `docs/process/project-error-search-runtime-diagnostics-protocol.md` and test changes to `docs/process/zombie-d-test-writing-protocol.md`.

## Recommended sequence

1. Run the repository baseline gate and stop if it is not green.
2. Run `-PlanOnly -IncludeWorkingTree -Json` and review the file diff queue and registry.
3. Run the audit with the default requested five workers (or a lower explicit cap if resources require it); workers consume only queued files for changed/pending rows.
4. Apply focused ZOMBIE-D test updates through the explicit hook, then let the recorded file rescan close the loop.
5. Process `defect-remediation.json`/`defects.jsonl` using the runtime defect protocol; preserve manifests, registry, JSONL events, checkpoints, timings, and ignored-test list.

Explicit full audit:

```powershell
pwsh -NoProfile -File scripts/vida-mutest-audit.ps1 -PlanOnly -IncludeWorkingTree -FullRescan -Json
```

Focused script contract proof:

```powershell
pwsh -NoProfile -File scripts/tests/vida-mutest-audit.tests.ps1
```
