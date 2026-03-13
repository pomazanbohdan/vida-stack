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

1. `vida/config/instructions/system-maps/framework.core-protocols-map.md`

Canonical owner artifacts:

1. `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
2. `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`
3. `vida/config/instructions/instruction-contracts/core.skill-activation-protocol.md`
4. `vida/config/instructions/instruction-contracts/core.packet-decomposition-protocol.md`
5. `vida/config/instructions/instruction-contracts/core.agent-prompt-stack-protocol.md`
6. `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md`
7. `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`
8. `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md`

### 2. Orchestration Shell And Lane Coordination

Purpose:

1. orchestrator/worker entry,
2. boot routing,
3. lane selection,
4. worker dispatch and handoff,
5. verification-lane posture around `core`,
6. shell-level role contracts that remain derived from framework canon rather than `core` ownership.

Canonical owner artifacts:

1. `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`
2. `vida/config/instructions/agent-definitions/entry.worker-entry.md`
3. `vida/config/instructions/instruction-contracts/role.orchestrator-contract.md`
4. `vida/config/instructions/instruction-contracts/role.worker-contract.md`
5. `vida/config/instructions/instruction-contracts/lane.worker-dispatch-protocol.md`
6. `vida/config/instructions/runtime-instructions/lane.agent-handoff-context-protocol.md`
7. `vida/config/instructions/runtime-instructions/work.agent-lane-selection-protocol.md`
8. `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md`
9. `vida/config/instructions/system-maps/bootstrap.orchestrator-boot-flow.md`
10. `vida/config/instructions/system-maps/bootstrap.worker-boot-flow.md`

Companion surface with cross-domain thinking ownership:

1. `vida/config/instructions/instruction-contracts/role.worker-thinking.md`

### 3. Runtime Execution, State, And Recovery

Purpose:

1. tracked execution,
2. task-state telemetry,
3. pack/task progression,
4. recovery, replay, readiness, and direct runtime consumption.

Canonical owner artifacts:

1. `vida/config/instructions/system-maps/runtime-family.taskflow-map.md`
2. `vida/config/instructions/system-maps/runtime-family.docflow-map.md`
3. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
4. `vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md`
5. `vida/config/instructions/runtime-instructions/work.task-state-reconciliation-protocol.md`
6. `vida/config/instructions/runtime-instructions/work.execution-priority-protocol.md`
7. `vida/config/instructions/runtime-instructions/work.pack-handoff-protocol.md`
8. `vida/config/instructions/runtime-instructions/work.pack-completion-gate-protocol.md`
9. `vida/config/instructions/runtime-instructions/work.verification-merge-protocol.md`
10. `vida/config/instructions/runtime-instructions/recovery.checkpoint-replay-recovery-protocol.md`
11. `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`
12. `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
13. `vida/config/instructions/runtime-instructions/work.execution-health-check-protocol.md`

### 4. Thinking, Reasoning, And Cognitive Control

Purpose:

1. step-scoped reasoning algorithms,
2. cross-step continuity,
3. worker-safe thinking subset,
4. reasoning-time external validation triggers.

Canonical owner artifacts:

1. `vida/config/instructions/instruction-contracts/overlay.step-thinking-protocol.md`
2. `vida/config/instructions/instruction-contracts/overlay.session-context-continuity-protocol.md`
3. `vida/config/instructions/instruction-contracts/role.worker-thinking.md`
4. `vida/config/instructions/runtime-instructions/work.web-validation-protocol.md`

### 5. Bootstrap, Activation, And Overlay Binding

Purpose:

1. instruction activation,
2. project overlay binding,
3. boot packet shaping,
4. autonomous continuation posture.

Canonical owner artifacts:

1. `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md`
2. `vida/config/instructions/runtime-instructions/bridge.project-overlay-protocol.md`
3. `vida/config/instructions/runtime-instructions/model.boot-packet-protocol.md`
4. `vida/config/instructions/instruction-contracts/overlay.autonomous-execution-protocol.md`

### 6. Documentation And Canon Operations

Purpose:

1. documentation mutation,
2. documentation validation and proof,
3. documentation-layer migration,
4. post-development evidence synchronization into project docs.

Canonical owner artifacts:

1. `vida/config/instructions/instruction-contracts/work.documentation-operation-protocol.md`
2. `vida/config/instructions/instruction-contracts/work.documentation-layer7-migration-protocol.md`
3. `vida/config/instructions/runtime-instructions/work.development-evidence-sync-protocol.md`

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

1. `vida/config/instructions/diagnostic-instructions/analysis.framework-self-analysis-protocol.md`
2. `vida/config/instructions/diagnostic-instructions/analysis.silent-framework-diagnosis-protocol.md`
3. `vida/config/instructions/diagnostic-instructions/analysis.protocol-self-diagnosis-protocol.md`
4. `vida/config/instructions/diagnostic-instructions/analysis.protocol-consistency-audit-protocol.md`
5. `vida/config/instructions/diagnostic-instructions/analysis.self-reflection-protocol.md`
6. `vida/config/instructions/diagnostic-instructions/escalation.debug-escalation-protocol.md`
7. `vida/config/instructions/diagnostic-instructions/evaluation.library-evaluation-protocol.md`
8. `vida/config/instructions/diagnostic-instructions/evaluation.product-proving-pack-scaffold-contract.md`

### 8. Artifact Modeling, Naming, And Extension Governance

Purpose:

1. instruction artifact shape,
2. role/profile identity,
3. naming grammar,
4. project extension admission,
5. new-protocol authoring/update discipline.

Canonical owner artifacts:

1. `vida/config/instructions/agent-definitions/model.agent-definitions-contract.md`
2. `vida/config/instructions/agent-definitions/role.role-profile-contract.md`
3. `vida/config/instructions/instruction-contracts/meta.protocol-naming-grammar-protocol.md`
4. `vida/config/instructions/instruction-contracts/meta.core-protocol-standard-protocol.md`
5. `vida/config/instructions/instruction-contracts/work.agent-system-new-protocol-development-and-update-protocol.md`
6. `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`

## Reading Rule

Use the three routing axes together:

1. use this file for domain-family routing,
2. use `vida/config/instructions/system-maps/framework.protocol-layers-map.md` for owner-layer placement,
3. use `vida/config/instructions/instruction-contracts/bridge.instruction-activation-protocol.md` for activation class and trigger,
4. use `vida/config/instructions/system-maps/protocol.index.md` when the exact canonical artifact must be opened.

## Boundary Rule

1. This map classifies protocol domains; it does not redefine ownership or activation law.
2. `core orchestration` remains a bounded subset, not a synonym for all framework protocols.
3. Non-orchestration protocol families such as thinking, documentation, diagnostics, naming, and artifact governance remain first-class framework protocol domains.

-----
artifact_path: config/system-maps/framework.protocol-domains-map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/framework.protocol-domains-map.md
created_at: '2026-03-12T10:59:09+02:00'
updated_at: '2026-03-12T11:55:06+02:00'
changelog_ref: framework.protocol-domains-map.changelog.jsonl
