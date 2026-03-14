# Framework Protocol Domains Map

Purpose: provide one thin framework-owned routing map for protocol-domain families so agents can distinguish orchestration-core from adjacent non-orchestration protocol domains without turning `protocol.index` into a second law-bearing taxonomy.

## Scope

This map covers domain-family routing for active framework protocol-bearing artifacts under `vida/config/instructions/**`.

It is for:

1. domain-level orientation,
2. distinguishing orchestration architecture from adjacent protocol families,
3. routing to the correct canonical owner when a protocol topic is known but the exact file is not.

This map is routing-only.

It does not replace:

1. ownership-layer routing in `framework.protocol-layers-map.md`,
2. activation-class routing in `bridge.instruction-activation-protocol.md`,
3. bounded `core cluster` routing in `framework.core-protocols-map.md`,
4. the canonical per-artifact registry in `protocol.index.md`.

## Activation Triggers

Read this map when:

1. the task asks which protocol family a topic belongs to,
2. the task asks which protocols are not part of agent-orchestration architecture,
3. the task needs one-pass domain routing before opening the exact canonical owner,
4. protocol inventory exists but is too flat to answer "what kind of protocol is this?"

## Domain Families

### 1. Core Orchestration Cluster

Purpose:

1. top-level orchestration posture,
2. worker-system routing and admissibility,
3. governed context usage,
4. routed-run continuity.

Canonical owner map:

1. `system-maps/framework.core-protocols-map`

Canonical owner artifacts:

1. `instruction-contracts/core.orchestration-protocol`
2. `instruction-contracts/core.agent-system-protocol`
3. `instruction-contracts/core.skill-activation-protocol`
4. `instruction-contracts/core.packet-decomposition-protocol`
5. `instruction-contracts/core.agent-prompt-stack-protocol`
6. `runtime-instructions/core.capability-registry-protocol`
7. `runtime-instructions/core.context-governance-protocol`
8. `runtime-instructions/core.run-graph-protocol`

### 2. Orchestration Shell And Lane Coordination

Purpose:

1. orchestrator/worker entry,
2. boot routing,
3. lane selection,
4. worker dispatch and handoff,
5. verification-lane posture around `core`,
6. shell-level role contracts that remain derived from framework canon rather than `core` ownership.

Canonical owner artifacts:

1. `agent-definitions/entry.orchestrator-entry`
2. `agent-definitions/entry.worker-entry`
3. `instruction-contracts/role.orchestrator-contract`
4. `instruction-contracts/role.worker-contract`
5. `instruction-contracts/lane.worker-dispatch-protocol`
6. `runtime-instructions/lane.agent-handoff-context-protocol`
7. `runtime-instructions/work.agent-lane-selection-protocol`
8. `runtime-instructions/work.verification-lane-protocol`
9. `system-maps/bootstrap.orchestrator-boot-flow`
10. `system-maps/bootstrap.worker-boot-flow`

Companion surface with cross-domain thinking ownership:

1. `instruction-contracts/role.worker-thinking`

### 3. Runtime Execution, State, And Recovery

Purpose:

1. tracked execution,
2. task-state telemetry,
3. pack/task progression,
4. recovery, replay, readiness, and direct runtime consumption.

Canonical owner artifacts:

1. `system-maps/runtime-family.taskflow-map`
2. `system-maps/runtime-family.docflow-map`
3. `runtime-instructions/work.taskflow-protocol`
4. `runtime-instructions/runtime.task-state-telemetry-protocol`
5. `runtime-instructions/work.task-state-reconciliation-protocol`
6. `runtime-instructions/work.execution-priority-protocol`
7. `runtime-instructions/work.pack-handoff-protocol`
8. `runtime-instructions/work.pack-completion-gate-protocol`
9. `runtime-instructions/work.verification-merge-protocol`
10. `runtime-instructions/recovery.checkpoint-replay-recovery-protocol`
11. `runtime-instructions/runtime.runtime-kernel-bundle-protocol`
12. `runtime-instructions/runtime.direct-runtime-consumption-protocol`
13. `runtime-instructions/work.execution-health-check-protocol`

### 4. Thinking, Reasoning, And Cognitive Control

Purpose:

1. step-scoped reasoning algorithms,
2. cross-step continuity,
3. worker-safe thinking subset,
4. reasoning-time external validation triggers.

Canonical owner artifacts:

1. `instruction-contracts/overlay.step-thinking-protocol`
2. `instruction-contracts/overlay.session-context-continuity-protocol`
3. `instruction-contracts/role.worker-thinking`
4. `runtime-instructions/work.web-validation-protocol`

### 5. Bootstrap, Activation, And Overlay Binding

Purpose:

1. instruction activation,
2. project overlay binding,
3. boot packet shaping,
4. autonomous continuation posture.

Canonical owner artifacts:

1. `instruction-contracts/bridge.instruction-activation-protocol`
2. `runtime-instructions/bridge.project-overlay-protocol`
3. `runtime-instructions/model.boot-packet-protocol`
4. `instruction-contracts/overlay.autonomous-execution-protocol`

### 6. Documentation And Canon Operations

Purpose:

1. documentation mutation,
2. documentation validation and proof,
3. documentation-layer migration,
4. post-development evidence synchronization into project docs.

Canonical owner artifacts:

1. `instruction-contracts/work.documentation-operation-protocol`
2. `instruction-contracts/work.documentation-layer7-migration-protocol`
3. `runtime-instructions/work.development-evidence-sync-protocol`

Project-side operator map:

1. `docs/process/documentation-tooling-map.md`

### 7. Diagnostics, Evaluation, And Reflection

Purpose:

1. framework self-analysis,
2. silent diagnosis,
3. protocol drift diagnosis,
4. escalation after repeated technical failure,
5. evaluation and proving-pack support.

Canonical owner artifacts:

1. `diagnostic-instructions/analysis.framework-self-analysis-protocol`
2. `diagnostic-instructions/analysis.silent-framework-diagnosis-protocol`
3. `diagnostic-instructions/analysis.protocol-self-diagnosis-protocol`
4. `diagnostic-instructions/analysis.protocol-consistency-audit-protocol`
5. `diagnostic-instructions/analysis.self-reflection-protocol`
6. `diagnostic-instructions/escalation.debug-escalation-protocol`
7. `diagnostic-instructions/evaluation.library-evaluation-protocol`
8. `diagnostic-instructions/evaluation.product-proving-pack-scaffold-contract`

### 8. Artifact Modeling, Naming, And Extension Governance

Purpose:

1. instruction artifact shape,
2. role/profile identity,
3. naming grammar,
4. project extension admission,
5. host CLI agent-template selection/materialization during project activation,
6. new-protocol authoring/update discipline and safe token-optimization rollout.

Canonical owner artifacts:

1. `agent-definitions/model.agent-definitions-contract`
2. `agent-definitions/role.role-profile-contract`
3. `instruction-contracts/meta.protocol-naming-grammar-protocol`
4. `instruction-contracts/meta.core-protocol-standard-protocol`
5. `runtime-instructions/work.host-cli-agent-setup-protocol`
6. `instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol`
7. `runtime-instructions/work.project-agent-extension-protocol`

## Reading Rule

Use the three routing axes together:

1. use this file for domain-family routing,
2. use `system-maps/framework.protocol-layers-map` for owner-layer placement,
3. use `instruction-contracts/bridge.instruction-activation-protocol` for activation class and trigger,
4. use `system-maps/protocol.index` when the exact canonical artifact must be opened.

## Boundary Rule

1. This map classifies protocol domains; it does not redefine ownership or activation law.
2. `core orchestration` remains a bounded subset, not a synonym for all framework protocols.
3. Non-orchestration protocol families such as thinking, documentation, diagnostics, naming, and artifact governance remain first-class framework protocol domains.

-----
artifact_path: config/system-maps/framework.protocol-domains-map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/framework.protocol-domains-map.md
created_at: '2026-03-12T10:59:09+02:00'
updated_at: '2026-03-13T23:35:00+02:00'
changelog_ref: framework.protocol-domains-map.changelog.jsonl
