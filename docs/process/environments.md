# Environments

Initial environment assumptions:

- local project root: `/home/unnamed/project/vida-stack`
- VIDA runtime directories are managed under `.vida/`
- host CLI agent template is selected through `vida project-activator`
- current host CLI system is `qwen` (external CLI)
- external CLI subagents (`qwen_cli`, `kilo_cli`, `opencode_cli`) are enabled via `vida.config.yaml -> agent_system.subagents`
- if sandbox is active and network is unavailable, `vida status --json` should report `host_agents.external_cli_preflight.status=blocked` with next actions

-----
artifact_path: process/environments
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-04-04'
schema_version: '1'
status: canonical
source_path: docs/process/environments.md
created_at: '2026-04-04T20:24:09+03:00'
updated_at: '2026-04-04T20:24:09+03:00'
changelog_ref: environments.changelog.jsonl
