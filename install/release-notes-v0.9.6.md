# Vida Stack v0.9.6

This patch release fixes cross-platform VIDA installer and init diagnostics after the `v0.9.5` launcher canonicalization wave.

## Highlights

- Added explicit install layout evidence to `vida release install`, including install root, active `current` root, runtime `bin`, generated env file, and platform.
- Centralized install-root resolution in Rust runtime diagnostics: `VIDA_HOME` first, then OS defaults (`%LOCALAPPDATA%\vida-stack` on Windows and `~/.local/share/vida-stack` on Unix-like systems).
- Hardened empty environment handling so empty `VIDA_HOME`, `LOCALAPPDATA`, `HOME`, `USERPROFILE`, `HOMEDRIVE`, or `HOMEPATH` values do not become accidental install roots.
- Added launcher PATH resolution diagnostics to `vida status --json` and `vida orchestrator-init --json`.
- Updated Windows installer env/PATH handling to be case-insensitive and to refresh the current installer process PATH.
- Updated Unix installer env bootstrap to clear shell command lookup cache after prepending the runtime `bin` directory.
- Hardened `scripts/build-release.sh` so packaged runtime binaries must report the requested release version; Windows cross-environment packaging can use native `.exe.version` stamps generated after the Windows release build.
- Documented the Windows Codex App environment-propagation boundary without applying Windows-only PATH recovery to Linux or macOS.

## Validation

Observed local validation for the 2026-05-04 release wave:

1. `cargo test -p vida release_install -- --test-threads=1`
2. `cargo test -p vida doctor_launcher_summary -- --test-threads=1`
3. `cargo test -p vida launcher_path_helpers -- --test-threads=1`
4. `cargo fmt -p vida -- --check`
5. `cargo run -p vida -- status --summary --json`
6. `cargo run -p vida -- orchestrator-init --json`
7. `cargo run -p vida -- docflow check-file --path docs/process/codex-agent-configuration-guide.md`
8. `bash -n install/install.sh`
9. PowerShell parser validation for `install/install.ps1`
10. `git diff --check`
11. Windows native release build produced `target\release\vida.exe`, `taskflow.exe`, and `docflow.exe` reporting `0.9.6`.
12. Generated version stamps beside the Windows release binaries, then ran `VIDA_RELEASE_SUFFIX=windows-x86_64 bash scripts/build-release.sh v0.9.6`.
13. Verified `dist/package/vida-stack-v0.9.6-windows-x86_64/bin/vida.exe --version` reports `vida 0.9.6` and no `.version` stamp files are shipped in the package.
14. Installed from `dist\vida-stack-v0.9.6-windows-x86_64.zip` with `dist\vida-install.ps1 install -Archive ... -Force`.
15. Installed runtime smoke confirmed `vida 0.9.6`, `taskflow 0.9.6`, and `docflow 0.9.6`; installed `vida status --summary --json` and `vida orchestrator-init --json` expose `install_layout` and `path_resolution`.

## Operator Notes

1. Windows Codex App shells can still miss environment updates inherited by ordinary terminals; when this happens, source `%LOCALAPPDATA%\vida-stack\env.ps1` or restart the host shell.
2. Linux and macOS installs continue to use the normal Unix profile/PATH flow and should not inherit Windows `%LOCALAPPDATA%` fallback behavior.
3. For explicit custom installs, set `VIDA_HOME` or pass the installer root option; do not embed machine-specific user paths in agent instructions.
4. When building a Windows release archive from WSL or another shell that cannot execute Windows `.exe` files, first run the Windows release binaries from PowerShell and write matching `target\release\<binary>.exe.version` stamp files before invoking `scripts/build-release.sh`.

-----
artifact_path: install/release-notes/v0.9.6
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-05-04'
schema_version: '1'
status: canonical
source_path: install/release-notes-v0.9.6.md
created_at: '2026-05-04T00:00:00Z'
updated_at: 2026-05-04T12:41:46.5617402Z
changelog_ref: release-notes-v0.9.6.changelog.jsonl
