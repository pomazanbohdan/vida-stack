# Worker Packet Templates

Purpose: framework-facing pointer to the canonical worker-packet template home.

Canonical prompt home for `vida 0.2.0` and `vida 1.0`:

1. `vida/config/instructions/prompt-templates.worker-packet-templates.md`
2. `vida/config/instructions/prompt-templates.cheap-worker-prompt-pack.md`

Rule:

1. Human-readable worker packet bodies belong in `vida/config/instructions/`.
2. This framework document remains a consumer guide and protocol pointer, not the canonical prompt-body store.
3. Worker packets bind to `vida/config/instructions/agent-definitions.worker-entry.md` and `vida/config/instructions/instruction-contracts.worker-thinking.md`.

Consumer note:

1. Keep worker-lane confirmation explicit.
2. Keep one blocking question per packet.
3. Keep packet law stronger than reusable prompt scaffolding.

-----
artifact_path: config/instructions/prompt-templates/worker.packet-templates
artifact_type: prompt_template_configuration
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/prompt-templates.worker-packet-templates.md
created_at: 2026-03-09T22:51:59+02:00
updated_at: 2026-03-10T00:55:00+02:00
changelog_ref: prompt-templates.worker-packet-templates.changelog.jsonl
