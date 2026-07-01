# Project Artifact Cleanup Protocol

Purpose: define the project-local cleanup boundary for removing generated repository, runtime, worktree, and filesystem artifacts produced by `vida-stack` work.

This protocol is operational cleanup guidance. It does not redefine TaskFlow, DocFlow, release, or runtime-state ownership law.

## Scope

1. Project root: `C:\project\vida-stack`.
2. Project-adjacent root: `C:\project`.
3. Project-created system temp roots on `C:\`.
4. Codex automation entrypoint: `scripts\cleanup-project-artifacts.ps1`.

## Default Safety Rules

1. Default mode is dry-run.
2. Destructive cleanup requires `-Apply`.
3. Locked paths are skipped, not forced.
4. Cleanup must never recursively delete a computed path unless the resolved absolute path matches an explicit allowlist or allowlisted prefix.
5. Cleanup must never delete live runtime state:
   - `C:\project\vida-stack\.vida\data\state`
   - descendants of `C:\project\vida-stack\.vida\data\state`
6. Cleanup must not touch active source or product-owned directories:
   - `crates`, `docs`, `vida`, `scripts`, `tests`, `benches`, `spikes` except nested generated `target` directories
   - `C:\project\vida_mobile`
   - `C:\_temp\ComfyUI`
   - `C:\temp\gemma4-vllm`
   - `C:\Users`, `C:\Windows`, `C:\Program Files`, `C:\Program Files (x86)`, `C:\ProgramData`
7. Before deleting build caches, check active build/runtime processes: `cargo`, `rustc`, `rustdoc`, `cl`, `link`, and `vida`.
8. If an active build/runtime process exists, either skip build-cache cleanup or run with explicit operator intent to skip locked paths.

## Repository Cleanup Allowlist

The following `C:\project\vida-stack` paths are disposable generated artifacts:

1. `target`
2. `.vida\cargo-target*`
3. `.vida\release-target-*`
4. `.vida\tmp`
5. `.vida\build-temp`
6. `dist`
7. `benches\ldrk-qualification\target`
8. `spikes\vida-runtime-restate\target`
9. `spikes\local-durable-runtime\target`
10. `tests\model\target`
11. `tmp\*` except intentional retained references when a current task names them
12. `tmp-task-notes` after the notes are no longer referenced by active TaskFlow work

## Historical Runtime-State Cleanup

The following are historical copies and may be removed after the live state path is confirmed present:

1. `.vida\data\state.backup-*`
2. `.vida\data\state.archive.*`
3. `.vida\data\state.archived-*`
4. `.vida\data\state-backups`

The live path `.vida\data\state` is excluded even when it contains logs, projections, WAL, manifest, or lock files.

## Project-Adjacent Cleanup Allowlist

The following `C:\project` paths are disposable when they are not registered git worktrees:

1. `C:\project\vida-stack-test-temp`
2. `C:\project\vida-stack-vh*`
3. empty `C:\project\vida-stack\.vida\worktrees`
4. empty `C:\project\vida-stack\.vida\cache`

Before deleting project-adjacent worktree-like directories, check:

```powershell
git -C C:\project\vida-stack worktree list --porcelain
```

Do not delete any directory that appears in that worktree registry.

## C Drive Cleanup Allowlist

The following root-level artifacts are project cleanup candidates:

1. `C:\tmp\vida-*`
2. `C:\tmp\runtime-path-policy-rooted-*`
3. `C:\vida-tgt-*`
4. `C:\vida-tmp-*`
5. `C:\temp\vida-*`
6. `C:\temp\vida_cl_probe.c`
7. `C:\temp\vida_cl_probe.obj`
8. `C:\temp\probe_root_tmp.c`
9. `C:\temp\diff_vida.txt`
10. `C:\t`
11. `C:\tc`
12. `C:\vt`
13. `C:\manifest`
14. `C:\sstables`
15. `C:\wal`
16. `C:\vlog`
17. `C:\c` when it only contains the stale copied `Users\pomaz\.cargo\bin\lean-ctx.exe` tree
18. `C:\Dumps\SystemSettings`
19. `C:\WRPEDC6.tmp` if not locked

## Required Procedure

1. Run dry-run:

```powershell
.\scripts\cleanup-project-artifacts.ps1
```

2. Review planned paths and exclusions.
3. Run active process check:

```powershell
Get-Process | Where-Object { $_.ProcessName -match '^(cargo|rustc|rustdoc|vida|link|cl)$' }
```

4. Apply cleanup:

```powershell
.\scripts\cleanup-project-artifacts.ps1 -Apply -SkipLocked
```

5. Verify:

```powershell
.\scripts\cleanup-project-artifacts.ps1
git -C C:\project\vida-stack status --short
```

6. If the script reports `partial_or_locked`, leave it unless the operator explicitly authorizes closing the owning process.

## Automation Rule

Codex automation may run this cleanup every three days with `-Apply -SkipLocked`.

The automation must:

1. Use `C:\project\vida-stack` as cwd.
2. Run the protocol script, not handwritten deletes.
3. Report deleted count, planned MiB, remaining allowlisted paths, and locked skips.
4. Not expand the allowlist without a project doc update.

-----
artifact_path: process/project-artifact-cleanup-protocol
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-07-01'
schema_version: '1'
status: canonical
source_path: docs/process/project-artifact-cleanup-protocol.md
created_at: '2026-07-01T21:20:00+03:00'
updated_at: 2026-07-01T21:20:00+03:00
changelog_ref: project-artifact-cleanup-protocol.changelog.jsonl
