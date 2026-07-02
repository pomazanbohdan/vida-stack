# Command Timing And Gate Optimization Protocol

Purpose: define the project-owned operating protocol for recording timings on significant agent, shell, script, test, CI, build, release, browser, simulator, and runtime operations, then using those timings to optimize development throughput without weakening required proof.

This document is a process protocol. It does not replace product/runtime law, release admission, TaskFlow ownership, DocFlow proof law, or CI branch protection. It defines how operators and agents must collect timing evidence, diagnose slow work, and decide whether a gate should stay blocking, become faster, move to a later admission point, or become diagnostic-only for PR iteration.

## Scope

This protocol applies to:

1. `vida` and TaskFlow/DocFlow commands,
2. shell commands,
3. Git and GitHub CLI calls,
4. package managers and tool installers,
5. scripts under `scripts/**`,
6. local tests and focused regression tests,
7. workspace-wide tests,
8. build, release, install, packaging, and smoke gates,
9. browser, simulator, emulator, and mobile/web validation,
10. CI runs, CI jobs, and individual CI steps,
11. delegated agent lanes, advisory agents, and fallback manual emulation steps.

Read-only commands are not exempt when they materially affect orchestration time. A slow read-only command is still operator-friction evidence.

## Timing Envelope

Every significant operation report must include this minimum envelope:

```text
operation_id: stable short id or command family
command_or_surface: redacted exact command, script, CI step, agent lane, or UI validation surface
cwd_or_context: repo/worktree/project/CI job/lane context
started_at: ISO-8601 timestamp when available
duration_ms: wall-clock duration in milliseconds
exit_status: pass | fail | blocked | cancelled | timed_out | unknown
blocking_scope: none | local_iteration | pr_acceptance | main_admission | release_admission | runtime_continuation
artifact_refs: log paths, CI URLs, receipt paths, screenshots, or command output paths
classification: fast | watch | slow | hard_defect | long_gate_expected
next_decision: keep_blocking | make_fast_proof | diagnostic_only_for_pr | move_to_main_or_release | remove_or_replace_stale_check | create_runtime_defect | none
target_dir_policy: caller_provided | repo_local_worktree_shared | repo_local_default | not_applicable
effective_cargo_target_dir: absolute Cargo target directory when the operation can invoke Cargo or a Cargo-built binary
```

If a tool cannot emit this envelope directly, the orchestrator must record it in the TaskFlow note, PR task, diagnostic note, or linked process artifact.

### Secret Redaction Requirement

1. Never persist raw secrets in `command_or_surface`, diagnostic notes, TaskFlow notes, PR tasks, logs, receipts, or linked artifacts.
2. Before persistence, sanitize commands by replacing secret-bearing values with stable placeholders (for example `<REDACTED_TOKEN>`, `<REDACTED_PASSWORD>`, `<REDACTED_AUTH_HEADER>`, `<REDACTED_URL_CREDENTIAL>`).
3. Treat as secret-bearing by default:
   - authorization headers, bearer/basic tokens, API keys, passwords, cookies, session ids, private keys, and one-time codes;
   - credential-bearing URLs (`https://user:pass@host/...`);
   - CLI flags or args commonly carrying credentials (for example `--token`, `--password`, `--apikey`, `--auth`, `-H 'Authorization: ...'`).
4. If exact reproduction is required, store only a redacted command plus a non-secret command family identifier in project artifacts; keep real secret material only in the operator's transient local environment or approved secret manager.
5. When uncertainty exists, fail closed: redact first, then record.

## Thresholds

1. Normal inspection, planning, routing, status, continuation, task mutation, lightweight diagnostic, and operator-query commands target `<= 2000 ms`.
2. Any normal operator command over `5000 ms` is an architectural or operator-surface defect unless the command is explicitly documented as a long-running proof gate.
3. Any local proof gate over `120000 ms` that blocks ordinary development requires a gate-optimization diagnostic.
4. Long-running commands are allowed only when their admission role is explicit: workspace proof, CI proof, build proof, release proof, install proof, simulator/browser proof, or external-provider probe.
5. A repeated slow command is stronger evidence than a single slow command. Three repeated observations in one active case require TaskFlow actualization unless the task already exists.
6. GitHub Actions jobs that can block PR, main, release, or installer admission must set an explicit `timeout-minutes` bound. An unbounded CI job that remains running without logs is a gate defect; add the timeout first, then rerun or repair the underlying failing step from bounded evidence.

## Command Execution Rules

1. Prefer one bounded command per proof step when timing matters.
2. Avoid long command chains that hide which segment was slow or failed.
3. When a sequence is repeated twice, create or update a reusable script or command surface.
4. Scripts that guard PRs, release, install, packaging, runtime smoke, or diagnostics must support `--help` when practical.
5. Scripts should print a concise summary, deterministic exit code, and artifact paths for verbose logs.
6. Log-backed proof scripts should capture timed command stdout and stderr separately as UTF-8 text artifacts, normally `.out.txt` and `.err.txt`, so lean-ctx and default shell readers can inspect them without binary-log fallbacks.
7. On failure, default human output should include the stdout/stderr artifact paths and a compact tail snippet from nonempty streams before exiting with the underlying command code.
8. Scripts should expose JSON or structured status when their output is consumed by agents, runtime diagnostics, CI, or TaskFlow notes; timed JSON records should include the same stdout/stderr artifact paths in `artifact_refs`.
9. If a command is expected to run longer than two minutes, state that expectation before running it and identify what smaller proof has already passed.
10. Do not repeatedly rerun a long gate to discover hidden failure details; repair output/artifact capture first.
11. If a CI run is superseded by a newer pushed commit, cancel the stale run once the newer run is queued or running so runner capacity and status surfaces reflect the current head.
12. Windows local proof scripts that invoke Cargo must use deterministic target-dir behavior and report it in timing records. If `CARGO_TARGET_DIR` is already set by the caller, respect that value and record `target_dir_policy=caller_provided`. If the repository is a linked worktree under `.vida/worktrees`, use the owner repository's `.vida/cargo-target` cache and record `target_dir_policy=repo_local_worktree_shared`. Otherwise use the current repository's `.vida/cargo-target` cache and record `target_dir_policy=repo_local_default`.
13. Debug-runtime smoke commands must resolve the debug binary from `effective_cargo_target_dir`, not from a hardcoded `target/debug` path. This keeps worktree output visible under `.vida/cargo-target` while preventing each linked worktree from cold-building an isolated `target` tree.
14. Command timing JSON for Cargo-like proof commands must include a `cargo` object with `target_dir_policy`, `effective_cargo_target_dir`, `artifact_lock_wait_ms`, `compile_ms`, and `wait_classification`. When Cargo phase data is unavailable, keep wait fields null and classify the slow command as `cargo_wait_unclassified_without_cargo_phase_data` instead of guessing whether the delay was artifact lock wait or compilation.
15. Local build/test scripts must be current, reusable proof surfaces. Remove stale scripts when they hardcode carrier names, ambient models, legacy provider lists, or heavy pre-commit builds that are no longer part of the current gate ladder. Replace them with config-derived runtime/status checks, Rust contract tests, or `scripts/vida-dev-gate.ps1` modes.
16. Do not keep a separate CI smoke step that only reruns package tests already covered by the workspace `cargo nextest` matrix. Keep separate smoke jobs only when they validate a different contract, such as doc tests, runtime boot/status behavior, docflow validation, packaging, installer behavior, or an external artifact path that nextest cannot exercise.
17. Do not make installer smoke depend on debug build jobs as if those jobs produced installer-ready binaries. GitHub Actions job dependencies do not transfer runner files unless an explicit artifact is uploaded and downloaded, and debug `cargo build` output is not a release package input. The CI graph must distinguish debug compile gates from release artifact producer gates: installer smoke consumes only release artifacts emitted by the package/release stage, while debug cross-platform build gates may run in parallel as compile sanity checks.
18. When root-session token pressure is active, prefer compact runtime surfaces, refreshed TaskFlow snapshots, and targeted `vida task show/children` calls before broad `status --json`, `orchestrator-init --full --json`, or large log reads.
19. For bounded read-only diagnosis, report triage, duplicate-risk review, and second-opinion review, use internal read-only/medium lanes by default; dispatch `jcode_nim_cli` with `mistralai/mistral-medium-3.5-128b` only when an independent NIM-backed advisory pass can reduce root-session analysis calls. Treat advisory results as evidence until the orchestrator validates them.
20. During task research, evaluate both external delegation modes before root implementation when useful: `external_readonly_complete` for analysis/spec/review/proof-diagnosis reports, and `external_patch_proposal` for patch plans/diffs that root will apply and verify. Record which mode ran, which mode was skipped, and why; high-ambiguity or high-risk tasks may run both in parallel before root synthesis.
21. For write-producing development, try internal low first when the packet owns one clear scope and proof command; move to internal medium for test authoring, ambiguous bounded implementation, or coach gating; reserve high/xhigh for architecture, safety, release, or repeated low/medium failure evidence.
22. After each coherent work pool, record whether root-token usage could have been reduced by earlier advisory prefetch, compact command output, task snapshot refresh, or batching TaskFlow mutations; create or update a TaskFlow optimization item when the answer is yes.

## Gate Decision Model

When a gate is slow or repeatedly blocks iteration, classify it with exactly one decision:

| Decision | Use when | Required follow-up |
| --- | --- | --- |
| `keep_blocking` | The gate directly protects current product/runtime behavior under change. | Keep it in the current proof matrix and improve output if needed. |
| `make_fast_proof` | The same defect can be caught by a focused test, smoke script, syntax check, or targeted command. | Add or update the fast proof and run the long gate only at batch proof time. |
| `diagnostic_only_for_pr` | The gate checks release/install/package behavior outside the PR's bounded product change and focused proof is green. | Keep the signal visible in PR CI but do not block PR closure solely on this gate. |
| `move_to_main_or_release` | The gate is valid but belongs to mainline, nightly, release, or installer admission. | Move or scope the gate and create a TaskFlow note explaining the admission boundary. |
| `remove_or_replace_stale_check` | The gate asserts obsolete text, legacy paths, hidden output, or deprecated behavior. | Replace it with the current contract and prove the new assertion locally. |
| `create_runtime_defect` | The slow operation is a runtime/operator-surface defect. | Create or update the defect under the relevant runtime/operator-efficiency epic. |

## Build Profile Decision Ladder

Use this ladder before starting a Rust proof, runtime smoke, release install, or script gate:

1. `script_or_doc_proof`: default for process, docs, and script-only edits that do not need Cargo. Use `scripts/vida-dev-gate.ps1 -Mode script-check -Json` on Windows, or equivalent `git diff --check` plus script parser checks on Unix-like hosts.
2. `debug_source_proof`: default for active Rust repair loops. Use `scripts/vida-dev-gate.ps1 -Mode quick -Json` on Windows, or the equivalent direct changed-file rustfmt check, `cargo check --locked -p vida`, and `git diff --check` on Unix-like hosts. This is the normal cheap compile-aware proof class for code correctness while a batch is still being assembled. Use `scripts/vida-dev-gate.ps1 -Mode focused-nextest -Package <crate> -TestFilter <filter> -Json` or direct `cargo nextest run --locked -p <crate> --profile default <filter>` only when the bounded slice needs a regression test proof.
3. `debug_runtime_smoke`: use `target/debug/vida ...` only after the debug binary proves it can open the current project state with an authoritative read such as `target/debug/vida status --json`. If the debug binary cannot open the state store, classify that as `debug_runtime_incompatible` and do not use it for runtime closure.
4. `installed_runtime_validation`: use the environment-resolved `vida ...` when the acceptance target is specifically the operator-facing launcher, installed binary path, state compatibility, command timing, or downstream project behavior through the user's normal PATH.
5. `release_install_gate`: run `vida release install --json` only for installed-runtime acceptance, release admission, packaging/installer proof, explicit user order, closure of a TaskFlow wave parent, repeated runtime defects that block active work, or when debug runtime smoke is invalid and the current closure must validate the installed launcher. It is not a per-micro-edit proof.
6. `release_packaging_gate`: run full release/installer/package smoke after the coherent batch is complete, not while more related code edits are still expected.

When a release install is considered, first record why `debug_source_proof` and, when applicable, `debug_runtime_smoke` are insufficient. A closed wave parent is sufficient reason because the operator-facing system binary must advance to the closed wave state. If the reason is only "the code changed" inside an unfinished task or micro-edit, use the debug proof class instead.

## System Binary Update Policy

1. Batch system binary updates after several unrelated closed tasks when no remaining unblocked runtime-defect task is expected to change the same installed binary.
2. Update the system binary immediately after a validated fix when the defect blocks the current session, blocks an external downstream project, recurs repeatedly in the session, corrupts state/receipts, or makes the lawful runtime flow impossible.
3. Defer `vida release install --json` when another unblocked, related runtime-defect task is ready and likely to touch the same runtime command family; use debug source proof and debug runtime smoke until the coherent fix pool is complete.
4. Before a system binary update, complete focused tests, a medium validator or equivalent independent review, PR protocol, and scoped git status review.
5. After a system binary update, record `vida --version`, installed path/fingerprint evidence when available, `vida status --json` or a narrower authoritative smoke, and any downstream project proof that motivated the update.
6. If all remaining ready tasks are blocked by the old runtime binary or by PR/review state, prefer one system binary update for the already validated batch so downstream work can resume.

## Diagnostic Update Format

Every runtime self-diagnostic, post-push diagnostic, PR CI diagnostic, long-gate diagnostic, or operator-friction audit must append or update a timing section using this format:

```text
Timing diagnostics:
- observed_operations:
  - operation_id:
    command_or_surface:
    duration_ms:
    exit_status:
    blocking_scope:
    artifact_refs:
    classification:
- slowest_operations:
  - operation_id:
    duration_ms:
    suspected_cause:
    proposed_decision:
- gate_decisions:
  - gate:
    decision:
    reason:
    taskflow_item:
    next_proof:
- optimization_backlog:
  - task_id:
    owner_scope:
    expected_gain:
```

If the diagnostic finds no slow operations, record `observed_operations: []` and `gate_decisions: []` so the absence is explicit.

## Bootstrap And TaskFlow Requirements

1. This protocol is part of the active project bootstrap read path through `AGENTS.sidecar.md`, `docs/project-root-map.md`, and `docs/process/index.md`.
2. Any bounded work item that runs commands must record timing evidence for commands that influence task selection, proof acceptance, PR closure, runtime continuation, or release admission.
3. Timing evidence belongs in the active TaskFlow task notes or linked artifact before closure.
4. A timing optimization that changes CI, scripts, command output, command options, diagnostics, or release gating must be its own TaskFlow item unless it is the direct bounded work item already in progress.
5. Timing diagnostics must optimize both wall-clock time and operator/agent iteration count. Reducing a 20-second command to 3 seconds is useful; reducing three separate reads to one structured output is also useful.
6. After every session/environment self-diagnostic or command-timing audit, update this protocol or the more specific mapped owner document in the same bounded batch when a new reusable optimization factor is discovered. Examples include lost output from long gates, repeated slow read surfaces, missing artifact paths, a better fast recovery command, a new sharding pattern, or a command option that would avoid follow-up reads. Do not keep these findings only in chat.

## Post-Pool Continuous Improvement Checklist

After every coherent work pool is proven, committed, pushed, released, or otherwise closed, run and record this checklist before selecting unrelated follow-up work:

1. command/operation timing diagnostics for the pool, including non-VIDA shell commands, tests, builds, CI, GitHub calls, runtime reads, and delegated/advisory lanes,
2. VIDA runtime diagnostic status for normal orchestration surfaces, including status, init, next-lawful, recovery, run-graph, lane, dispatch, and task inspection where relevant,
3. slow-surface classification against the two-second target and five-second hard-defect ceiling,
4. token and output-volume reduction opportunities, including repeated JSON reads, broad logs, missing compact fields, and hidden failure details,
5. stage-ordering and parallelism review for the next pool, including whether advisory prefetch should have started earlier,
6. script/gate decision for every slow or failed gate: keep blocking, make fast proof, diagnostic-only for PR, move to main/release, remove/replace stale check, or create runtime defect,
7. command-surface follow-up: missing options, help text, JSON fields, artifact paths, or recovery commands,
8. documentation sync: whether this protocol, `project-error-search-runtime-diagnostics-protocol.md`, a project spec, or sidecar rule was updated for any new reusable optimization factor.
9. pack-finalize friction: whether proof attachment, task close, graph reconciliation, status re-entry, temp cleanup, or git publication required repeated manual commands that should become a pack-level runtime command.
10. output-loss friction: whether a failed command produced only an exit code or an overlarge log without the failure set, and whether a log-backed summary artifact or command-output fix is required.
11. environment drift: whether different shell surfaces resolved different `vida` binaries or PATH state. If so, prefer the environment-resolved `vida` surface consistently or record the absolute installed binary path in the command evidence.

The checklist is required even when build, release, commit, push, or CI proof is already green. Green gates prove the bounded change; they do not prove the session workflow is optimized.

## Recommended Local Patterns

1. For local shell timing, use a wrapper that prints duration, exit code, and command id.
2. For PowerShell, prefer `Measure-Command` or a small reusable project script when the same measurement is repeated.
3. For Bash scripts, prefer `SECONDS`, `date +%s%3N`, or a shared helper that prints a final timing line.
4. For CI, prefer step-level timing from GitHub Actions plus script-level summaries inside long steps.
5. For agent lanes, record role, resolved carrier/profile when available, duration, result, rework count, and proof outcome.
6. For browser/simulator/emulator validation, record launch/setup time separately from user-flow validation time.
7. For long local gates that may exceed the host-tool timeout, redirect stdout/stderr to a deterministic log file and print that path before starting the gate. A timed-out host tool call without a log artifact is itself an optimization defect because it forces reruns.
8. For Rust workspace proof during active repair, prefer focused filters and package shards first, then run the workspace-wide gate once the coherent batch is assembled. If the workspace gate exceeds the local tool timeout, rerun it through a log-backed script or background job rather than repeating foreground calls that can lose output.
9. Nextest accepts focused filters, but each proof record should still have one intentional filter expression or one broader module/package scope. Do not hide unrelated focused proofs inside opaque command chains; use the dev-gate JSON output or separate timed commands so each filter has an auditable duration and result.
10. For VIDA runtime recovery diagnostics, prefer the fastest authoritative inspection surface that exposes the needed evidence. If a timeout/recovery path only needs task metadata or current owned scope, use `vida task show <task-id> --json` before heavier lane or run-graph projections.
11. If a long test shard or runtime command is killed because it exceeds the local tool timeout, immediately record the command, duration, missing artifact gap, and replacement proof strategy in the post-pool checklist.
12. Prefer `scripts/vida-dev-gate.ps1` for local Windows proof loops that need consistent timing records and deterministic Cargo cache behavior. Use `-Mode script-check` for no-Cargo diff/script proof, `-Mode quick` for cheap source proof through diff check, changed-file Rust formatting proof, and `cargo check`, `-Mode scoped-format -FormatFile <path>` for bounded Rust formatting that must not rewrite unrelated dirty files, `-Mode focused-nextest -Package <crate> -TestFilter <filter>` for bounded regression proof, `-Mode package-nextest` for the full `vida` package nextest proof, `-Mode workspace-nextest` for a log-backed local workspace test gate, `-Mode doc-test` for Rust doc tests, `-Mode build-debug` for debug runtime entrypoint builds, `-Mode runtime-smoke` for debug-runtime state compatibility, `-Mode release-package -SkipBuild -Windows -ReleaseBinDir <dir>` for native PowerShell release archive packaging from existing Windows release binaries, and `-Mode release-install` only when installed runtime validation is the bounded acceptance target. Use `-Mode target-dir-policy -Json` as the cheap policy probe before running Cargo from a new linked worktree. Pass `-Jobs <n>` only when the local machine or task class needs an explicit nextest concurrency cap. With `-Json`, every operation record must include `target_dir_policy` and `effective_cargo_target_dir`.
13. For bounded packets, prefer `-Mode scoped-format` before broad `cargo fmt`. Pass every intended Rust file with repeated `-FormatFile`; pass repeated `-AllowDirtyFile` only for intentional non-Rust artifacts that belong to the same packet. The mode fails before formatting when unrelated dirty files already exist, rejects symlink/reparse-point paths that could route rustfmt outside the repository, and fails after formatting if rustfmt creates out-of-scope churn, so it is safe to run before staging scoped commits.
14. For broad Rust package gates, use or create a summary artifact that reports passed count, failed count, failure names, delta from the previous run when available, failures touching current changed files, and a full-log path. Do not make operators read thousands of lines to decide whether focused proof is clean and residual failures are outside scope.
15. If tests create repo-root tool runtimes, caches, or generated directories such as `Python/`, classify that as temp-root leakage. Remove verified untracked leakage before staging, then create or update the runtime defect unless an existing task already owns that command family.
16. When a shell surface cannot resolve `vida` but another project-approved shell can, do not retry mixed commands blindly. Use the resolved installed binary path from `Get-Command vida` or stay on the project runtime shell, and create a command-surface follow-up if this causes repeated operations.

## Prohibited Patterns

1. Do not hide slow operations inside opaque command chains.
2. Do not make every PR wait for full release/install proof when a focused PR proof and diagnostic release signal are enough for the bounded change.
3. Do not classify a command as acceptable merely because it eventually succeeds.
4. Do not increase timeouts as the primary fix for an operator command that should be fast.
5. Do not keep stale assertions in CI because they are "only smoke"; stale smoke is still false evidence.
6. Do not leave a repeated timing finding only in chat; create or update the TaskFlow item.
7. Do not keep hardcoded external-carrier smoke scripts in the local build/test surface when carrier readiness is owned by `vida.config.yaml`, runtime assignment, and status/preflight projections.

## Current Known Timing Evidence

As of this protocol slice, the following observations are known from the active session and must feed follow-up optimization work:

1. `vida orchestrator-init --json` observed around `22000 ms` through the local command wrapper during re-entry.
2. `vida task next-lawful --json` observed around `17000 ms`.
3. `vida task create` and `vida task update` observed around `14000 ms`.
4. PR `validate` CI remained blocked for multiple minutes inside `cargo test --workspace --locked -- --test-threads=1`.
5. Local `cargo test -p vida runtime_dispatch_state --locked -- --test-threads=1` exceeded the 120-second host-tool timeout during active runtime repair without a preserved foreground result; future runs of this shard should be log-backed or split further before becoming a blocking local gate.
6. `vida release install --json` took about 84 seconds in the 2026-06-01 bridge-adapter repair loop. The release install was useful only for installed-runtime validation; subsequent debug runtime smoke showed `target/debug/vida status --json` can be a faster validation step when the debug binary is state-store compatible.
7. The 2026-06-01 CI migration proved the fastest reliable test shape is `cargo nextest archive` plus four `slice:m/n` shards, but archived nextest runs do not carry workspace support binaries automatically. Test shards that depend on `CARGO_BIN_EXE_*` helpers must restore a support-binary artifact before execution.
8. In the same CI window, Linux runtime entrypoint build completed in under one minute, macOS in about six minutes, and Windows in about seven minutes after the gate was narrowed to deliverable runtime entrypoints. Cross-platform build remains a downstream proof gate, not the first defect-discovery gate.
9. A cold local `scripts/vida-dev-gate.ps1 -Mode quick -Json` run spent about `79758 ms` in `cargo check --locked -p vida`; this is acceptable only as compile-aware source proof. Docs/script-only edits must use `-Mode script-check` first so local proof does not pay a Cargo compile cost unnecessarily.
10. The 2026-06-02 local script audit retired `scripts/external-cli-carrier-smoke.sh` because it hardcoded a fixed carrier/provider list instead of deriving readiness from project config/runtime truth. The same audit retired `.githooks/pre-commit` and `scripts/precommit-build-json.sh` because the old heavy pre-commit JSON build hook was not the current local proof ladder and could reintroduce slow hidden build work.
11. The replacement bounded adapter proof completed locally in about `7438 ms` with 20 tests passing. Treat package-scoped adapter tests as scoped Cargo proof, not as a normal two-second operator surface; prefer `cargo nextest run --locked -p vida-pi-agent --profile default` for current local batch proof when adapter behavior is in scope.
12. The 2026-06-02 post-push CI smoke run exposed a Linux race in `vida-pi-agent` adapter tests: a fast provider process can write `agent_end` and exit before parent-side process polling observes the queued terminal record. Adapter gates must drain terminal stdout events before classifying process exit as missing execution evidence; do not hide this class behind serial test execution or stale hardcoded smoke scripts.
13. The 2026-06-02 local build/test cleanup removed the duplicate `validate-smoke` package-test step for `vida-pi-agent`; those adapter tests are now covered by the workspace nextest shards, while `validate-smoke` remains reserved for doc tests, docflow validation, and runtime smoke.
14. The 2026-06-02 local surface cleanup made `scripts/vida-dev-gate.ps1` the current local build/test entrypoint for Makefile targets. The script now has an explicit help surface, changed-file Bash script syntax checks in `script-check`, package/workspace nextest modes, doc-test, debug build, debug runtime smoke, native PowerShell release packaging, and release install modes. `scripts/build-release.ps1` is the Windows-native release package path for `-SkipBuild -Windows -ReleaseBinDir <dir>` proofs, while `scripts/build-release.sh` remains the Unix-compatible packager and must respect `CARGO_TARGET_DIR` so release packaging works under the same deterministic target-dir policy as the local gate. Treat direct local Cargo invocations as Unix fallback or focused ad hoc proof, not as the canonical reusable local script contract.
15. On this Windows host, `bash -n scripts/build-release.sh` took about `15-20s` during local script proof. Do not make full Bash syntax checks unconditional for docs-only proof; run them only when a Bash script changed, and keep the timing artifact so future shell startup regressions are visible.
16. The 2026-06-02 CI/CD graph audit found that `package-linux` and `package-windows` were previously treated as release packaging stages but rebuilt their own release binaries from a fresh checkout instead of consuming a release-binary producer artifact. The corrected graph separates debug sanity build from release delivery: `build-runtime` remains `Build debug runtime`, `build-release-linux` and `build-release-windows` build installer-ready release binaries once per OS and upload them as artifacts, `package-linux` and `package-windows` download those binaries and run `scripts/build-release.sh` with `VIDA_RELEASE_SKIP_BUILD=1` plus `VIDA_RELEASE_BIN_DIR`, and installer smoke downloads only the final package artifacts from the package jobs.
17. `scripts/build-release.sh` must keep `VIDA_RELEASE_BIN_DIR` as the release-binary handoff contract. When `VIDA_RELEASE_SKIP_BUILD=1` is set, the script may not require Cargo merely to package already-built binaries; it may infer version from source metadata and platform suffix from explicit environment or host shell, then copy and verify the supplied binaries before producing ZIP/TAR, manifest, installer scripts, and checksums.
18. The 2026-06-02 recovery-surface repair loop showed that broad local `cargo test -p vida taskflow_run_graph -- --nocapture --test-threads=1` exceeded the 120-second host-tool timeout and left test child processes alive. Do not use the whole `taskflow_run_graph` filter as the default local proof for a narrow recovery/operator-surface fix. Prefer a named focused filter such as `recovery_surface_contract`, then let workspace nextest/CI cover the broad shard unless the broad run is launched log-backed with an explicit timeout and cleanup plan.
19. The 2026-06-26 active run repair summary proof observed repeated `Blocking waiting for file lock on artifact directory` delay during focused `cargo test -p vida ...` commands. Treat repeated waits as `runtime-optimization-cargo-artifact-lock-proof-latency-20260625` evidence. The command timing hook now emits Cargo target-dir policy metadata for Cargo-like timed commands and recommends grouped focused proof, `scripts/vida-dev-gate.ps1 -Mode focused-nextest`, serialized Cargo shards, or isolated `CARGO_TARGET_DIR` instead of rerunning many tiny proof commands that contend on the same artifact directory.
20. The 2026-07-02 team-flow E2E repair established the normal operator command set as `vida taskflow team continue <task-id>`, `vida taskflow team status <task-id>`, and `vida taskflow team diagnose <task-id>`. Any E2E blocker diagnosis that requires `vida taskflow run-graph`, `vida lane show`, `vida agent-init --dispatch-packet`, `vida agent host-bridge`, staged JSON result authoring, or host subagent spawn/wait/close is manual bridge glue and must be classified as a runtime/operator-surface defect instead of accepted as the happy-path flow. Focused source proof and debug-runtime smoke are acceptable while the operator explicitly defers installed-runtime updates; installed-runtime validation belongs to the batch/release-install gate when that boundary is lifted.

These observations do not prove one root cause. They prove that timing diagnostics must cover both local runtime commands and CI/test gates.

-----
artifact_path: process/command-timing-and-gate-optimization-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: 2026-06-02
schema_version: '1'
status: canonical
source_path: docs/process/command-timing-and-gate-optimization-protocol.md
created_at: 2026-05-26T00:00:00+03:00
updated_at: 2026-06-02T04:22:00+03:00
changelog_ref: command-timing-and-gate-optimization-protocol.changelog.jsonl
