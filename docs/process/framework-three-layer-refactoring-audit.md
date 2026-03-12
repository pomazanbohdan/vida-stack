# Framework Three-Layer Refactoring Audit

Purpose: provide one unified report format for the completed refactoring and audit cycle across the first three framework layers: `core`, the `orchestration shell`, and the `runtime-family execution layer`.

Status: completed consolidated audit.

Date: `2026-03-11`

## Consolidated Layer Stack

This report normalizes the completed layer reports into one common structure:

1. Layer 1: `core`
2. Layer 2: `orchestration shell`
3. Layer 3: `runtime-family execution layer`

This document is a consolidation surface, not a second owner of framework law.

Retention rule:

1. this consolidated report is the retained process-facing report for the first three framework layers,
2. the earlier layer-specific process reports were removed after consolidation,
3. current framework law still lives in `vida/config/instructions/**` and `docs/product/spec/**`, not in this report.

## External Validation Baseline

Layer decisions across this program were validated against the same three official external baselines before boundary-sensitive refactor choices were accepted:

1. OpenAI Agents SDK
   - agent-local behavior, tools, and guardrails stay at agent/runtime boundaries, while orchestration owns routing and handoffs
2. Anthropic Claude / Claude Code
   - role wording tends toward behavior contracts, so upper layers should prefer coordination and lane semantics over behavior ownership
3. Microsoft Semantic Kernel
   - orchestration is a coordination-pattern layer above runtime execution primitives and below higher-level product/process overlays

Resulting framework-wide rule:

1. keep `core` responsibility-centric,
2. keep the second layer as coordination/handoff/dispatch shell,
3. keep the third layer as execution/state/telemetry/recovery layer,
4. push anything that does not belong to the active layer outward to its rightful owner.

## Layer 1: Core

### Layer Definition

For this audit, the first layer is the framework `core` cluster:

1. top-level orchestration law,
2. generic agent-system routing and lane law,
3. typed admissibility gating,
4. context governance,
5. routed-run resumability and state continuity.

This layer must not own:

1. concrete runtime command syntax,
2. role-behavior contracts,
3. project/process overlays,
4. backend lifecycle mechanics,
5. implementation-specific execution surfaces.

### Audited Scope

Primary audited artifacts:

1. `vida/config/instructions/instruction-contracts/core.orchestration-protocol.md`
2. `vida/config/instructions/instruction-contracts/core.agent-system-protocol.md`
3. `vida/config/instructions/runtime-instructions/core.capability-registry-protocol.md`
4. `vida/config/instructions/runtime-instructions/core.context-governance-protocol.md`
5. `vida/config/instructions/runtime-instructions/core.run-graph-protocol.md`

Immediate stitching/projection surfaces used by the wave program:

1. `vida/config/instructions/system-maps/framework.core-protocols-map.md`
2. `vida/config/instructions/system-maps/framework.protocol-layers-map.md`
3. `vida/config/instructions/system-maps/protocol.index.md`
4. `docs/product/spec/canonical-runtime-layer-matrix.md`

### Findings Before Refactor

The `core` cluster was already documentation-green, but not yet fully tightened as one explicit package.

Main drifts before the refactor cycle:

1. tool and operator leakage inside some `core` owner surfaces,
2. underlinked package edges between peer `core` protocols,
3. thinner closure semantics in `core.context-governance`,
4. implementation-flavored and role-flavored terminology inside `core`,
5. excess workflow doctrine density in `core.orchestration`.

### Refactor Applied

The `core` cycle was executed wave-by-wave and closed green.

Main corrections:

1. removed tool/runtime/operator leakage from `core` owner law,
2. made package edges explicit across peer `core` protocols,
3. strengthened owner boundaries and closure semantics,
4. normalized `role`/implementation/catalog wording toward coordination-centric and lane-based language,
5. shrank `core.orchestration` toward pure coordination law,
6. pushed non-`core` content into rightful adjacent owners and maps.

### Final Verdict

Verdict: `all green` for the first layer in the bounded audited scope.

What is now correct:

1. the `core` cluster is green both as documentation and as one explicit inter-protocol package,
2. `core` is responsibility-centric rather than role-behavior-centric,
3. package stitching is explicit enough for the current bounded scope,
4. `core` no longer carries the main tooling and implementation drift that originally pressured the cluster.

### Residual Watchpoints

Non-blocking watchpoints:

1. `core.orchestration` remains the densest `core` artifact and the most likely future drift point,
2. `core.context-governance` and `core.run-graph` should continue resisting overlap in continuity/replay language,
3. future changes should keep command catalogs, role-behavior, and runtime procedures out of `core`.

### Validation

Completed proof surfaces for the `core` cycle:

1. bounded `check` -> green
2. bounded `activation-check` -> green
3. `protocol-coverage-check --profile active-canon` -> green
4. `doctor --profile active-canon-strict` -> green
5. bounded `proofcheck --profile active-canon-strict` -> green
6. `readiness-check --profile active-canon` -> green for the original `core` audit scope

### Next Logical Step

The next logical step after `core` was the second layer: the orchestration shell around `core`.

## Layer 2: Orchestration Shell

### Layer Definition

For this audit, the second layer around `core` is the orchestration shell:

1. orchestrator and worker entry/boot,
2. lane selection,
3. worker dispatch,
4. handoff and bounded context shaping,
5. verification-lane routing,
6. shell-level routing and closure posture around `core`.

This layer must not own:

1. `core` law,
2. concrete runtime-family command syntax,
3. implementation-specific runtime procedures,
4. project-specific overlays beyond validated activation points,
5. lower-layer role-behavior contracts.

### Audited Scope

Primary audited artifacts:

1. `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`
2. `vida/config/instructions/system-maps/bootstrap.orchestrator-boot-flow.md`
3. `vida/config/instructions/instruction-contracts/role.orchestrator-contract.md`
4. `vida/config/instructions/instruction-contracts/lane.worker-dispatch-protocol.md`
5. `vida/config/instructions/runtime-instructions/lane.agent-handoff-context-protocol.md`
6. `vida/config/instructions/runtime-instructions/work.agent-lane-selection-protocol.md`
7. `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md`
8. `vida/config/instructions/agent-definitions/entry.worker-entry.md`
9. `vida/config/instructions/instruction-contracts/role.worker-contract.md`
10. `vida/config/instructions/instruction-contracts/role.worker-thinking.md`

Projection touched:

1. `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`

### Findings Before Refactor

The shell was structurally sound but not yet clean.

Main drifts:

1. concrete runtime command leakage in orchestrator and lane-selection surfaces,
2. transition-era implementation naming in shell law,
3. duplicate provenance references in handoff and verification surfaces,
4. worker-contract drift toward orchestrator bootstrap inheritance,
5. product projection still carrying runtime-specific wording instead of owner-neutral shell language.

### Refactor Applied

Main corrections:

1. removed concrete runtime command syntax from shell-law surfaces,
2. normalized shell vocabulary toward owner-neutral tracked-execution and task-state wording,
3. cleaned handoff and verification references,
4. tightened worker boundary so worker surfaces stay packet/entry/dispatch driven,
5. synced the product lane-selection model with the refactored shell language.

### Final Verdict

Verdict: `all green` for the second layer within the audited shell scope.

What is now correct:

1. the orchestration shell is `core`-compatible,
2. it keeps coordination, routing, handoff, dispatch, and verification posture in the shell,
3. it no longer owns concrete runtime command syntax,
4. worker surfaces no longer drift toward orchestrator bootstrap inheritance,
5. shell projections are consistent with the refactored owner-neutral wording.

### Residual Watchpoints

Non-blocking watchpoints:

1. `entry.orchestrator-entry.md` remains the densest shell artifact and the main future drift point,
2. the shell must continue resisting reintroduction of concrete runtime command catalogs,
3. future lane-selection runtime proofs should stay in runtime-family surfaces, not return to shell protocols.

### Validation

Completed validation set for the shell refactor:

1. bounded `check` -> green
2. bounded `activation-check` -> green
3. `protocol-coverage-check --profile active-canon` -> green
4. `doctor --profile active-canon-strict` -> green
5. `proofcheck --profile active-canon-strict` -> green

### Next Logical Step

The next logical step after the shell was the third layer: runtime-family execution surfaces beneath the shell.

## Layer 3: Runtime-Family Execution Layer

### Layer Definition

For this audit, the third layer is the runtime-family execution layer:

1. execution enactment,
2. tracked execution,
3. task-state materialization,
4. execution telemetry,
5. checkpoint, replay, and recovery,
6. runtime proof/readiness/consumption loops,
7. runtime-family discovery for active execution surfaces.

This layer may own:

1. concrete runtime surfaces,
2. concrete runtime commands,
3. execution-state and telemetry contracts,
4. resumability and consumption loops.

This layer must not own:

1. `core` law,
2. orchestration-shell routing law,
3. project/process methodology overlays,
4. framework-diagnosis policy,
5. pack- or methodology-specific reporting schemes as runtime owner-law.

### Audited Scope

Primary audited artifacts:

1. `vida/config/instructions/system-maps/runtime-family.taskflow-map.md`
2. `vida/config/instructions/system-maps/runtime-family.docflow-map.md`
3. `vida/config/instructions/runtime-instructions/work.taskflow-protocol.md`
4. `vida/config/instructions/runtime-instructions/runtime.task-state-telemetry-protocol.md`
5. `vida/config/instructions/runtime-instructions/recovery.checkpoint-replay-recovery-protocol.md`
6. `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`
7. `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`
8. `vida/config/instructions/runtime-instructions/work.verification-merge-protocol.md`

### Findings Before Refactor

The runtime layer was functionally strong but not yet cleanly bounded.

Main drifts:

1. `work.taskflow-protocol.md` carried framework-diagnosis integration policy instead of only execution-side persistence requirements,
2. `work.taskflow-protocol.md` carried SCP/BFP/FTP transparency material that belongs above the execution layer,
3. `runtime.direct-runtime-consumption-protocol.md` still used `role-selection` wording in a lane-based canon,
4. recovery and direct-consumption surfaces still used transition-era phrasing that sounded like provenance/process law rather than runtime owner wording.

### Refactor Applied

Main corrections:

1. re-narrowed `work.taskflow-protocol.md` to execution block lifecycle, telemetry, gates, resumability, and next-step materialization,
2. pushed diagnosis-policy and methodology-transparency content out of the execution owner,
3. normalized runtime consumption wording from `role-selection` to `lane-selection`,
4. normalized recovery provenance wording toward historical lineage instead of current owner-law.

### Final Verdict

Verdict: `all green` for the third layer within the audited runtime scope.

What is now correct:

1. the runtime-family execution layer owns execution, state, telemetry, recovery, and consumption,
2. it no longer pretends to own methodology transparency or framework diagnosis policy,
3. runtime-consumption wording is consistent with the lane-based upper layers,
4. execution-layer boundaries are cleaner against both the orchestration shell above and the implementation surfaces below.

### Residual Watchpoints

Non-blocking watchpoints:

1. `work.taskflow-protocol.md` remains the densest third-layer artifact and the most likely future drift point,
2. transitional helper and wrapper references should keep shrinking over time,
3. direct runtime consumption should continue resisting reintroduction of upper-shell selection law.

### Validation

Completed validation set for the runtime refactor:

1. bounded `check` -> green
2. bounded `activation-check` -> green
3. `protocol-coverage-check --profile active-canon` -> green
4. `doctor --profile active-canon-strict` -> green
5. `proofcheck --profile active-canon-strict` -> green

### Next Logical Step

If refactoring continues outward, the next likely targets are:

1. the implementation layer beneath runtime-family execution surfaces, or
2. cross-layer parity between runtime execution surfaces and executable law/config bundles.

## Consolidated Verdict

Across the first three layers, the refactor cycle is now green in one consistent framing:

1. Layer 1 `core`: `all green`
2. Layer 2 `orchestration shell`: `all green`
3. Layer 3 `runtime-family execution layer`: `all green`

Cross-layer interpretation:

1. `core` now holds bounded coordination, routing, admissibility, context, and resumability law,
2. the second layer now acts as a clean shell around `core` for dispatch, handoff, and verification posture,
3. the third layer now acts as the enactment/state/telemetry/recovery substrate beneath the shell,
4. the main remaining work lies outside these three layers, not inside them.

## Source Reports

This consolidated report retains the normalized outcome of the earlier layer-specific audits and wave reports after their removal from the active process lane.

-----
artifact_path: process/framework-three-layer-refactoring-audit
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/process/framework-three-layer-refactoring-audit.md
created_at: '2026-03-11T19:10:00+02:00'
updated_at: '2026-03-12T07:03:49+02:00'
changelog_ref: framework-three-layer-refactoring-audit.changelog.jsonl
