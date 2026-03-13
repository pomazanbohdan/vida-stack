# Framework Canon Index

Purpose: thin instruction-home entrypoint inside the top-level framework root map.

Primary entrypoints:

1. [vida/root-map.md](/home/unnamed/project/vida-stack/vida/root-map.md)
   - top-level framework root map for `vida/`
2. [framework-map-protocol.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/framework.map.md)
   - canonical topology/layer/promotion map inside the framework root map
3. [framework-core-protocols-map.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/framework.core-protocols-map.md)
   - bounded discovery and stitching map for the framework `core cluster`
4. [framework-protocol-layers-map.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/framework.protocol-layers-map.md)
   - thin routing map for placing protocol-bearing artifacts into the correct owner layer
5. [framework-protocol-domains-map.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/framework.protocol-domains-map.md)
   - thin routing map for classifying protocol-bearing artifacts by domain family
6. [protocol-index.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/protocol.index.md)
   - canonical registry of domain protocols
7. [meta-core-protocol-standard-protocol.md](/home/unnamed/project/vida-stack/vida/config/instructions/instruction-contracts/meta.core-protocol-standard-protocol.md)
   - framework-level standard for what a `core` protocol must contain, own, and avoid absorbing
8. [agent-system-new-protocol-development-and-update-protocol.md](/home/unnamed/project/vida-stack/vida/config/instructions/instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol.md)
   - canonical owner for protocol authoring/update discipline and safe optimization rollout of protocol-bearing surfaces
9. [runtime-family-index.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/runtime-family.index.md)
   - bounded runtime-family discovery surface for `codex`, `taskflow`, and future runtimes
10. [template-map.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/template.map.md)
   - canonical template-family discovery surface
11. [governance-map.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/governance.map.md)
   - canonical governance/policy-gate discovery surface
12. [observability-map.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/observability.map.md)
   - canonical runtime observability/trace/proving discovery surface

Bootstrap discovery rule:

1. Start framework-owned downstream discovery at [vida/root-map.md](/home/unnamed/project/vida-stack/vida/root-map.md).
2. Use this file as the instruction-home index once framework discovery has entered `vida/config/instructions/**`.
3. If repository/documentation ownership, canonical maps, or downstream documentation surfaces must be resolved, continue next to [framework-map-protocol.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/framework.map.md).
4. If the task is specifically about the bounded `core cluster` of framework protocols, continue to [framework-core-protocols-map.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/framework.core-protocols-map.md).
5. If the task is specifically about protocol-bearing layer placement, continue to [framework-protocol-layers-map.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/framework.protocol-layers-map.md).
6. If the task is specifically about protocol-domain classification beyond owner layers, continue to [framework-protocol-domains-map.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/framework.protocol-domains-map.md).
7. If the task is specifically about what a `core` protocol must contain or must not absorb, continue to [meta-core-protocol-standard-protocol.md](/home/unnamed/project/vida-stack/vida/config/instructions/instruction-contracts/meta.core-protocol-standard-protocol.md).
8. If the task is specifically about framework protocol authoring, update discipline, or safe token-optimization rollout, continue to [agent-system-new-protocol-development-and-update-protocol.md](/home/unnamed/project/vida-stack/vida/config/instructions/instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol.md).
9. If the resolved target is project/product documentation, hand off discovery to the project-context surface rather than embedding project-document pointers here.

Bootstrap cluster:

1. [AGENTS.md](/home/unnamed/project/vida-stack/AGENTS.md)
2. [ORCHESTRATOR-ENTRY.MD](/home/unnamed/project/vida-stack/vida/config/instructions/agent-definitions/entry.orchestrator-entry.md)
3. [WORKER-ENTRY.MD](/home/unnamed/project/vida-stack/vida/config/instructions/agent-definitions/entry.worker-entry.md)
4. [WORKER-THINKING.MD](/home/unnamed/project/vida-stack/vida/config/instructions/instruction-contracts/role.worker-thinking.md)
5. [instruction-activation-protocol.md](/home/unnamed/project/vida-stack/vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md)
6. [orchestrator-boot-flow.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/bootstrap.orchestrator-boot-flow.md)
7. [worker-boot-flow.md](/home/unnamed/project/vida-stack/vida/config/instructions/system-maps/bootstrap.worker-boot-flow.md)

Split rule:

1. `vida/config/instructions/**` and `docs/product/spec/**` carry the active promoted framework/product canon.
2. `vida/config/**` is the executable law home.
3. sidecar changelogs and Git history are evidence/history by default.
4. deleted framework-formation plans/research lineage is preserved only in [framework-source-lineage-index.md](/home/unnamed/project/vida-stack/docs/process/framework-source-lineage-index.md).

-----
artifact_path: config/system-maps/framework.index
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/framework.index.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: '2026-03-13T23:35:00+02:00'
changelog_ref: framework.index.changelog.jsonl
