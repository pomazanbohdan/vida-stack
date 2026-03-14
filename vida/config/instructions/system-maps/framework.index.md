# Framework Canon Index

Purpose: thin instruction-home entrypoint for active framework discovery during development bootstrap.

Primary entrypoints:

1. `system-maps/framework.map`
   - canonical topology/layer/promotion map inside the framework root map
2. `system-maps/framework.core-protocols-map`
   - bounded discovery and stitching map for the framework `core cluster`
3. `system-maps/framework.protocol-layers-map`
   - thin routing map for placing protocol-bearing artifacts into the correct owner layer
4. `system-maps/framework.protocol-domains-map`
   - thin routing map for classifying protocol-bearing artifacts by domain family
5. `system-maps/protocol.index`
   - canonical registry of domain protocols
6. `instruction-contracts/meta.core-protocol-standard-protocol`
   - framework-level standard for what a `core` protocol must contain, own, and avoid absorbing
7. `instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol`
   - canonical owner for protocol authoring/update discipline and safe optimization rollout of protocol-bearing surfaces
8. `system-maps/runtime-family.index`
   - bounded runtime-family discovery surface for `codex`, `taskflow`, and future runtimes
9. `system-maps/template.map`
   - canonical template-family discovery surface
10. `system-maps/governance.map`
   - canonical governance/policy-gate discovery surface
11. `system-maps/observability.map`
   - canonical runtime observability/trace/proving discovery surface

Bootstrap discovery rule:

1. Use this file as the framework instruction-home index when bounded framework discovery is needed during development bootstrap.
2. If repository/documentation ownership, canonical maps, or downstream documentation surfaces must be resolved, continue next to `system-maps/framework.map`.
3. If the task is specifically about the bounded `core cluster` of framework protocols, continue to `system-maps/framework.core-protocols-map`.
4. If the task is specifically about protocol-bearing layer placement, continue to `system-maps/framework.protocol-layers-map`.
5. If the task is specifically about protocol-domain classification beyond owner layers, continue to `system-maps/framework.protocol-domains-map`.
6. If the task is specifically about what a `core` protocol must contain or must not absorb, continue to `instruction-contracts/meta.core-protocol-standard-protocol`.
7. If the task is specifically about framework protocol authoring, update discipline, or safe token-optimization rollout, continue to `instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol`.
8. If the resolved target is project/product documentation, hand off discovery to the project-context surface rather than embedding project-document pointers here.

Bootstrap cluster:

1. [AGENTS.md](/home/unnamed/project/vida-stack/AGENTS.md)
2. `system-maps/bootstrap.router-guide`
3. `agent-definitions/entry.orchestrator-entry`
4. `agent-definitions/entry.worker-entry`
5. `instruction-contracts/role.worker-thinking`
6. `instruction-contracts/bridge.instruction-activation-protocol`
7. `system-maps/bootstrap.orchestrator-boot-flow`
8. `system-maps/bootstrap.worker-boot-flow`

Split rule:

1. `vida/config/instructions/**` and `docs/product/spec/**` carry the active promoted framework/product canon.
2. `vida/config/**` is the executable law home.
3. sidecar changelogs and Git history are evidence/history by default.
4. deleted framework-formation plans/research lineage is preserved only in [framework-source-lineage-index.md](/home/unnamed/project/vida-stack/docs/process/framework-source-lineage-index.md).

-----
artifact_path: config/system-maps/framework.index
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/framework.index.md
created_at: '2026-03-09T20:28:59+02:00'
updated_at: '2026-03-13T23:20:00+02:00'
changelog_ref: framework.index.changelog.jsonl
