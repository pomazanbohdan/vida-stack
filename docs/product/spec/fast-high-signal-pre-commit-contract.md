# Fast High Signal Pre Commit Contract

Status: active product contract

Purpose: define a fast, high-signal repository-local pre-commit gate for
`vida-stack` that catches trivial quality defects before commit without
reintroducing hidden Cargo-heavy hooks.

## Context

The borrowed Rig config uses three ideas:

1. generic `pre-commit-hooks` file hygiene,
2. Rust hooks from `doublify/pre-commit-rust`,
3. Commitizen on `commit-msg`.

For `vida-stack`, only the first category is safe as a default commit gate.
The project already owns Rust proof through `scripts/vida-dev-gate.ps1`, and
the command-timing protocol explicitly retired stale heavy pre-commit build
hooks. Default pre-commit must therefore stay fast and predictable.

## Decisions

1. Use `pre-commit/pre-commit-hooks` at `v6.0.0`.
2. Add syntax and repository hygiene checks: YAML, JSON, TOML, merge conflicts,
   case conflicts, illegal Windows names, large added files, EOF, trailing
   whitespace, and private key detection.
3. Preserve Markdown hard line breaks by passing
   `--markdown-linebreak-ext=md` to `trailing-whitespace`.
4. Exclude generated/runtime-heavy surfaces from default hook scans:
   `.vida/`, `target/`, `vendor/`, `tmp/`, `tasks_tmp/`, and
   `*.changelog.jsonl`.
5. Add one local default hook, `vida-script-check`, which runs
   `scripts/vida-dev-gate.ps1 -Mode script-check`.
6. Add `vida-rust-quick` only as a manual hook. It runs
   `scripts/vida-dev-gate.ps1 -Mode quick` but is not part of default
   commit flow.
7. Do not use `doublify/pre-commit-rust` as a default hook source. Its release
   line is old, and direct local project proof commands are clearer.
8. Do not add blocking Commitizen yet. Commit-message policy can be useful, but
   it should be introduced as a separate release/changelog policy decision.
9. Do not add full secret scanning yet. `detect-private-key` is cheap and safe;
   broader tools such as Gitleaks need a baseline/policy task before becoming
   blocking.

## Hook Matrix

| Hook | Stage | Purpose | Runtime Cost |
| --- | --- | --- | --- |
| `trailing-whitespace` | `pre-commit` | remove stray whitespace while preserving Markdown hard breaks | fast |
| `end-of-file-fixer` | `pre-commit` | enforce one terminal newline | fast |
| `check-yaml` | `pre-commit` | parse staged YAML | fast |
| `check-json` | `pre-commit` | parse staged JSON | fast |
| `check-toml` | `pre-commit` | parse staged TOML | fast |
| `check-merge-conflict` | `pre-commit` | block conflict markers | fast |
| `check-case-conflict` | `pre-commit` | prevent case-only path collisions | fast |
| `check-illegal-windows-names` | `pre-commit` | prevent Windows-reserved filenames | fast |
| `check-added-large-files` | `pre-commit` | block accidental large additions over 1024 KiB | fast |
| `detect-private-key` | `pre-commit` | block private key material | fast |
| `vida-script-check` | `pre-commit` | run repo-owned no-Cargo script/diff gate | bounded |
| `vida-rust-quick` | `manual` | opt-in compile-aware Rust source proof | slower/manual |

## Acceptance Criteria

1. `.pre-commit-config.yaml` is present and validates as YAML.
2. Default hooks avoid `cargo check`, `cargo clippy`, tests, nextest, and release
   install/package gates.
3. Rust proof remains available through a manual hook and
   `scripts/vida-dev-gate.ps1`.
4. The config excludes generated/runtime-heavy paths that should not be
   normalized by file-fixer hooks.
5. This contract records the hook matrix and non-goals.

## Proof Targets

1. `pre-commit validate-config`, when `pre-commit` is installed.
2. YAML parse of `.pre-commit-config.yaml` when `pre-commit` is unavailable.
3. `pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode script-check`.
4. `vida docflow check --root . docs/product/spec/fast-high-signal-pre-commit-contract.md`.

## Operator Commands

Install the runner once per workstation:

```powershell
python -m pip install --user pre-commit
pre-commit install
```

Run default hooks manually:

```powershell
pre-commit run --all-files
```

Run the opt-in Rust quick gate:

```powershell
pre-commit run vida-rust-quick --hook-stage manual
```

-----
artifact_path: product/spec/fast-high-signal-pre-commit-contract
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-06-03
schema_version: 1
status: canonical
source_path: docs/product/spec/fast-high-signal-pre-commit-contract.md
created_at: 2026-06-03T12:33:24.3171285Z
updated_at: 2026-06-03T12:47:25.4844746Z
changelog_ref: fast-high-signal-pre-commit-contract.changelog.jsonl
