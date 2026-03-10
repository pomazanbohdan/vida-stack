# Project Operations Pointer

Purpose: keep framework-owned instruction canon free of project-specific runbook bodies while preserving a stable pointer to the extracted `vida-mobile` operations surface.

Rule:
1. Project-specific operations content no longer lives here as active body content.
2. The extracted project runbook now lives at `projects/vida-mobile/docs/process/project-operations.md`.
3. Framework protocols should refer generically to the host-project operations doc resolved by the active overlay, not embed app-specific command catalogs in framework canon.
4. Do not reintroduce `mobile-odoo` or other project-specific runbook bodies into `vida/config/instructions/**`.

-----
artifact_path: config/command-instructions/project-operations
artifact_type: command_instruction
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: pointer_only
source_path: vida/config/instructions/command-instructions.project-operations.md
created_at: 2026-03-09T12:00:46+02:00
updated_at: 2026-03-10T08:10:00+02:00
changelog_ref: command-instructions.project-operations.changelog.jsonl
