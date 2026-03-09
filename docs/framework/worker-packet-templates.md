# Worker Packet Templates

Purpose: framework-facing pointer to the canonical worker-packet template home.

Canonical prompt home for `vida 0.2.0` and `vida 1.0`:

1. `vida/config/instructions/prompt_templates/worker-packet-templates.md`
2. `vida/config/instructions/prompt_templates/cheap-worker-prompt-pack.md`

Rule:

1. Human-readable worker packet bodies belong in `vida/config/instructions/prompt_templates/`.
2. This framework document remains a consumer guide and protocol pointer, not the canonical prompt-body store.
3. Worker packets bind to `docs/framework/WORKER-ENTRY.MD` and `docs/framework/WORKER-THINKING.MD`.

Consumer note:

1. Keep worker-lane confirmation explicit.
2. Keep one blocking question per packet.
3. Keep packet law stronger than reusable prompt scaffolding.
