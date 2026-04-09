# Environments

Initial environment assumptions:

- local project root: `/home/unnamed/project/vida-stack`
- VIDA runtime directories are managed under `.vida/`
- default long-lived authoritative local state root: `.vida/data/state/`
- repeatable proof/audit runs may bind a disposable temp root via `VIDA_STATE_DIR=<temp-dir>`
- generated files under `.vida/data/state/**` are operational runtime artifacts, not canonical product-doc inputs
- host CLI agent template is selected through `vida project-activator`
- current host CLI system is selected through `vida.config.yaml -> host_environment.cli_system` and materialized by `vida project-activator`
- the canonical agent registry is `vida.config.yaml -> agent_system.subagents`
- route policy should use explicit executor backend fields and treat legacy `subagents` hints as compatibility-only
- host posture is the primary runtime environment, but it must not hard-block a policy-selected external backend when the runtime is running in hybrid mode
- hybrid mode means both internal and external executors remain admissible when policy selects them
- external CLI subagents are enabled via `vida.config.yaml -> agent_system.subagents`
- internal backends remain internal-only even when hybrid mode is active
- if sandbox is active and network is unavailable, `vida status --json` should report `host_agents.external_cli_preflight.status=blocked` with next actions

-----
artifact_path: process/environments
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-04-08'
schema_version: '1'
status: canonical
source_path: docs/process/environments.md
created_at: '2026-04-04T20:24:09+03:00'
updated_at: 2026-04-09T05:43:45.437700509Z
changelog_ref: environments.changelog.jsonl
