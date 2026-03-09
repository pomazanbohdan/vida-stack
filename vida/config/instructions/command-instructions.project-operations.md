# Project Operations Runbook

This file is the canonical project-level command map for `mobile-odoo`.

Framework/runtime policy stays in `vida/config/instructions/*`. App-specific operational commands stay here and in `scripts/`.

## Command Discipline

1. Use only canonical project commands documented in this file; do not invent ad hoc build, run, audit, or packaging commands that bypass project scripts.
2. Follow the project-specific preflight and execution order documented here; keep project sequencing rules out of `legacy helper surfaces`.
3. Run stateful project commands sequentially. This includes dependency resolution, analysis, tests, builds, and scripts that mutate local runtime or cache state.

## Build And Packaging

1. Full build entrypoint:

```bash
bash scripts/build.sh [apk|appbundle|ios] [debug|profile|release]
```

2. Fast debug APK:

```bash
bash scripts/build-debug-apk.sh
```

Policy:

1. `apk debug` is the manual-testing cycle entrypoint.
2. Debug/manual builds run the GlitchTip gate in `manual-cycle` mode: current unresolved issues are automatically marked `resolved` before the build so the next manual pass starts from a clean baseline.
3. Non-debug builds keep the strict GlitchTip gate unless explicitly documented otherwise.

3. Debug APK optimization details live in `docs/process/debug-apk-build.md`.

## Analysis And Test Preconditions

1. Run `flutter pub get` before `flutter analyze`, tests, or builds.
2. Run stateful project commands sequentially:
   - dependency resolution,
   - `flutter analyze`,
   - tests,
   - builds,
   - scripts that mutate local cache/runtime state.
3. Prefer existing hook wrappers from `scripts/hooks/` when they match the requested check.
4. Do not bypass project scripts with ad hoc Docker/build invocations unless the runbook is updated in the same change.
5. For module-navigation changes, preserve the explicit `/module/:xmlId?menu_id=<id>` contract whenever real menu intent exists; if menu intent is unavailable, route through the safe-fallback path instead of section guessing.

Recommended project preflight order:

```bash
cd src
flutter pub get
flutter analyze
flutter test <targeted-scope>
cd ..
bash scripts/build.sh apk debug
```

Generated Flutter artifact recovery:

```bash
bash scripts/flutter-generated-sanitize.sh
bash scripts/flutter-generated-sanitize.sh --apply
```

Use this when `flutter pub get`, codegen, or other Flutter commands fail because generated platform directories are root-owned or otherwise unwritable. The script safely moves problematic generated directories into `_temp/` so Flutter can regenerate them. `scripts/hooks/codegen.sh` invokes the sanitize step automatically before regeneration.

## Observability And GlitchTip

1. Local observability coverage audit for changed Dart files:

```bash
bash scripts/observability-audit.sh
```

2. GlitchTip issue scan / `br` sync:

```bash
bash scripts/glitchtip-audit.sh scan
bash scripts/glitchtip-audit.sh sync-br
bash scripts/glitchtip-audit.sh resolve-unresolved --all
```

3. Release/build-time quality gate:

```bash
bash scripts/hooks/glitchtip-quality-gate.sh
```

Rule:

1. New or changed recoverable error paths should emit GlitchTip-visible logging or explicit local logging primitives before fallback UX is shown.
2. `GLITCHTIP_GATE_POLICY=manual-cycle` resets the local unresolved baseline and must be used only for local manual-test cycles, never as a CI/release bypass.
3. `GLITCHTIP_GATE_POLICY=strict` is the default for release-oriented builds and CI.

## Live Odoo Validation

Use project scripts for server reality checks before concluding on auth/menu/API behavior:

```bash
bash scripts/odoo-auth.sh
bash scripts/odoo-menus-live.sh
```

Environment-specific credentials and domains live in `docs/environments.md`.

## Local App Lifecycle

```bash
bash scripts/start.sh
bash scripts/stop.sh
bash scripts/test-connectivity.sh
```

## Ownership Rule

1. If a command is app-specific, document it here and implement it in `scripts/`.
2. If a command enforces VIDA runtime protocol, keep it in `scripts/` and reference it from `vida/config/instructions/*.md`.

-----
artifact_path: config/command-instructions/project-operations
artifact_type: command_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/command-instructions.project-operations.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: command-instructions.project-operations.changelog.jsonl
