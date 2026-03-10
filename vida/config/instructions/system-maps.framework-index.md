# Framework Canon Index

Purpose: thin instruction-home entrypoint inside the top-level framework root map.

Primary entrypoints:

1. [vida/root-map.md](/home/unnamed/project/vida-stack/vida/root-map.md)
   - top-level framework root map for `vida/`
2. [framework-map-protocol.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps.framework-map-protocol.md)
   - canonical topology/layer/promotion map inside the framework root map
3. [protocol-index.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps.protocol-index.md)
   - canonical registry of domain protocols
4. [runtime-family-index.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps.runtime-family-index.md)
   - bounded runtime-family discovery surface for `codex`, `taskflow`, and future runtimes
5. [template-map.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps.template-map.md)
   - canonical template-family discovery surface
6. [governance-map.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps.governance-map.md)
   - canonical governance/policy-gate discovery surface

Bootstrap discovery rule:

1. Start framework-owned downstream discovery at [vida/root-map.md](/home/unnamed/project/vida-stack/vida/root-map.md).
2. Use this file as the instruction-home index once framework discovery has entered `vida/config/instructions/**`.
3. If repository/documentation ownership, canonical maps, or downstream documentation surfaces must be resolved, continue next to [framework-map-protocol.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps.framework-map-protocol.md).
4. If the resolved target is project/product documentation, hand off discovery to the project-context surface rather than embedding project-document pointers here.

Bootstrap cluster:

1. [AGENTS.md](/home/unnamed/project/vida-stack/AGENTS.md)
2. [ORCHESTRATOR-ENTRY.MD](/home/unnamed/project/vida-stack/vida/config/instructions/agent-definitions.orchestrator-entry.md)
3. [WORKER-ENTRY.MD](/home/unnamed/project/vida-stack/vida/config/instructions/agent-definitions.worker-entry.md)
4. [WORKER-THINKING.MD](/home/unnamed/project/vida-stack/vida/config/instructions/instruction-contracts.worker-thinking.md)
5. [instruction-activation-protocol.md](/home/unnamed/project/vida-stack/vida/config/instructions/instruction-contracts.instruction-activation-protocol.md)

Split rule:

1. `docs/framework/plans/**` are active strategic and execution-spec artifacts by default.
2. sidecar changelogs and Git history are evidence/history by default.
3. `docs/product/spec/**` are promoted stable product canon.
4. `vida/config/**` is the executable law home.

-----
artifact_path: config/system-maps/framework.index
artifact_type: system_map
artifact_version: 1
artifact_revision: 2026-03-09
schema_version: 1
status: canonical
source_path: vida/config/instructions/system-maps.framework-index.md
created_at: 2026-03-09T20:28:59+02:00
updated_at: 2026-03-10T09:50:00+02:00
changelog_ref: system-maps.framework-index.changelog.jsonl
