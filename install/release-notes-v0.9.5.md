# Vida Stack v0.9.5

This patch release hardens the installed VIDA runtime after the `v0.9.4` Windows bootstrap fix.

## Highlights

- Canonicalized the installed `vida` binary location to one path: `%LOCALAPPDATA%\vida-stack\current\bin\vida.exe` on Windows and `<install-root>/current/bin/vida` on Unix-like systems.
- Changed `vida release install` to install the release binary to `current/bin` by default; legacy `all`, `local`, and `cargo` target names now resolve to that same canonical target instead of creating duplicate binaries.
- Updated Windows and Unix installers to expose direct runtime binaries from `current/bin` and clean legacy launcher or duplicate binary surfaces.
- Fixed clean TaskFlow startup after project initialization: terminal completed historical runs with no active TaskFlow work now report `idle` instead of blocking `status` or `doctor`.
- Added Windows path normalization for persisted `/mnt/c/...` runtime packet paths so old receipts do not break lane/recovery surfaces from native Windows `vida.exe`.
- Relaxed runtime trust and trace checks for clean startup so `vida status --json`, `vida doctor --json`, `vida orchestrator-init --json`, and `vida taskflow consume bundle check --json` agree on a ready idle project.

## Validation

Observed local validation for the 2026-05-04 release wave:

1. `gh release list --repo pomazanbohdan/vida-stack --limit 10` confirmed `v0.9.4` is already the latest published GitHub release, so this patch is `v0.9.5`.
2. `cargo test -p vida continuation_binding_summary -- --nocapture`
3. `cargo test -p vida doctor_surface -- --nocapture`
4. `cargo test -p vida runtime_consumption_state -- --nocapture`
5. `cargo test -p vida release_surface -- --nocapture`
6. `cargo test -p vida runtime_consumption_surface -- --nocapture`
7. `bash -n install/install.sh`
8. `powershell -NoProfile -ExecutionPolicy Bypass -File install\install.ps1 help`
9. `VIDA_RELEASE_SUFFIX=windows-x86_64 bash scripts/build-release.sh v0.9.5` built the Windows release assets in `dist/`.
10. `powershell -NoProfile -ExecutionPolicy Bypass -File dist\vida-install.ps1 install -Archive dist\vida-stack-v0.9.5-windows-x86_64.zip -Force`
11. Clean Windows session resolved only `%LOCALAPPDATA%\vida-stack\current\bin\vida.exe`; `vida --version` reported `vida 0.9.5`; `vida status --json`, `vida doctor --json`, `vida orchestrator-init --json`, and `vida taskflow consume bundle check --json` all passed against `C:\project\vida_mobile` with continuation status `idle`.

## Operator Notes

1. New Windows shells should resolve `vida`, `taskflow`, and `docflow` directly to `.exe` files under `%LOCALAPPDATA%\vida-stack\current\bin`.
2. Existing shells may need to be restarted to pick up the updated User PATH.
3. `~\.local\bin\vida(.exe)` and `~\.cargo\bin\vida(.exe)` are cleanup targets only; they are not canonical install targets.
4. `vida release install --target path` remains available for explicitly updating the first `vida` found on PATH, but the default release install path is `current/bin`.

-----
artifact_path: install/release-notes/v0.9.5
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-05-04'
schema_version: '1'
status: canonical
source_path: install/release-notes-v0.9.5.md
created_at: '2026-05-04T00:00:00Z'
updated_at: '2026-05-04T00:00:00Z'
changelog_ref: release-notes-v0.9.5.changelog.jsonl
