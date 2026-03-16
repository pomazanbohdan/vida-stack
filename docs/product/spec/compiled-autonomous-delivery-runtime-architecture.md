# Compiled Autonomous Delivery Runtime Architecture

Status: active target product architecture

Purpose: define the canonical target architecture for turning VIDA framework law, project activation data, role/skill/profile/flow composition, and runtime-family execution into one compiled autonomous delivery runtime that can accept high-level business intent and drive lawful end-to-end delivery without requiring the human operator to manage internal protocol wiring by hand.

## 1. Mission

The target is not:

1. a prompt-only orchestration habit,
2. a pile of independent tools with hidden glue,
3. a runtime where every model repeatedly reads large protocol markdown bodies,
4. a system where the human operator must manually choose each internal protocol, lane, or packet.

The target is:

1. one compiled autonomous delivery runtime,
2. where framework/product law remains canonical in docs and config,
3. where project activation is explicit and fail-closed,
4. where orchestration reads a compiled control bundle rather than raw canon on every step,
5. where `TaskFlow` and `DocFlow` act as bounded sibling runtime families,
6. where a human operator can drive the system through business intent, approval, cost/quality posture, and enabled runtime capabilities rather than internal framework mechanics.

Compact rule:

1. canonical law stays human-readable,
2. runtime control becomes machine-readable,
3. orchestration consumes compiled law,
4. delivery proceeds through explicit lanes, gates, packets, and evidence.

## 2. Operator-Facing Goal

The top-level operator should be able to work at the level of:

1. business input,
2. project intent,
3. enabled roles,
4. enabled skills,
5. enabled profiles,
6. enabled flow sets,
7. model/backend policy,
8. quality/cost/speed posture,
9. approval and correction points.

The operator should not need to manage directly:

1. internal protocol routing,
2. activation sequencing,
3. handoff packet construction,
4. runtime-family selection logic,
5. worker dispatch mechanics,
6. checkpoint or verification wiring.

## 2.1 Release Phasing Model

The target architecture uses phased externalization rather than exposing the full runtime stack directly from the beginning.

### Release 1: Host-Shell CLI Integration

Release 1 integrates VIDA into an existing external shell or agentic CLI environment through one user-facing CLI bridge.

Release-1 rule:

1. the operator interacts with VIDA through one host-shell CLI entry bridge,
2. VIDA provides autonomous delivery behavior behind that bridge,
3. internal runtime-family names, protocol mechanics, provider wiring, and backend wiring stay hidden by default,
4. the visible experience is one delivery-oriented assistant/tool surface rather than a family of separate internal runtimes,
5. this release optimizes for usable shell integration before VIDA owns the full surrounding product or project environment.

Boundary note:

1. the practical Release-1 working entrypoint and detailed execution program are owned by `docs/product/spec/release-1-plan.md`,
2. the active seam and closure hardening surface is `docs/product/spec/release-1-seam-map.md`,
3. the version ladder remains owned by `VERSION-PLAN.md`.

### Release 2: Host-Project Integration

Release 2 embeds VIDA into another project or product environment as the autonomous delivery runtime for that host project.

Release-2 rule:

1. VIDA is integrated into the host project rather than only into the host shell,
2. project roles, skills, profiles, flow sets, model/backend policy, documentation surfaces, and delivery/runtime behavior become integrated into that host project environment,
3. VIDA acts as an embedded delivery platform for the host project,
4. this release optimizes for project-level integration rather than only shell-level entry.

Boundary rule:

1. Release 1 is shell integration,
2. Release 2 is project integration,
3. the two releases should not be collapsed into one ambiguous entry model.

## 2.2 Operator-Facing Product Surfaces

The detailed operator-surface inventory and end-to-end operating journey are owned by:

1. `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
2. `docs/product/spec/release-1-plan.md`

Anchor rule:

1. VIDA remains one visible product with bounded operational surfaces,
2. this architecture document defines the top-level product/runtime direction,
3. user-facing surface inventory and operator-flow detail live in their owner docs rather than inside this anchor.

## 3. Architectural Problem Statement

VIDA already has:

1. canonical framework law in `vida/config/instructions/**`,
2. active product law in `docs/product/spec/**`,
3. bridge-era project activation data in `vida.config.yaml`,
4. bridge-era project extension registries in `docs/process/agent-extensions/**`,
5. runtime-family execution surfaces in `TaskFlow`,
6. documentation/readiness/proof surfaces in `DocFlow`.

The missing architectural closure is:

1. one compiled runtime control plane that turns all of the above into a compact executable orchestration bundle,
2. one cheap orchestration runtime that reads that bundle and dispatches work efficiently,
3. one explicit end-to-end path from business intake to delivered binary and supporting proofs.

## 4. Layered Runtime Stack

This architecture keeps the current four-layer stack:

1. `Layer 1: core`
   - bounded framework law for orchestration, routing, admissibility, context governance, and resumability
2. `Layer 2: orchestration shell`
   - entry, lane selection, dispatch, handoff, and verification routing around `core`
3. `Layer 3: runtime-family execution layer`
   - execution/state/telemetry/recovery/readiness/proof enactment through runtime families
4. `Layer 4: implementation layer`
   - concrete binaries, adapters, commands, storage, and runtime machinery

Layer rule:

1. upper layers own law and routing boundaries,
2. lower layers enact those boundaries,
3. no lower layer may redefine upper-layer authority,
4. no upper layer may absorb concrete implementation detail that belongs below it.

## 5. Runtime Planes

The target runtime is best understood as cooperating planes:

### 5.1 Canon Plane

Owns:

1. framework protocols,
2. product specs,
3. runtime layer law,
4. role/skill/profile/flow law,
5. gate and evidence law.

Primary homes:

1. `vida/config/instructions/**`
2. `docs/product/spec/**`

### 5.2 Project Activation Plane

Owns:

1. which framework capabilities are enabled for this project,
2. project-specific protocols, roles, skills, profiles, flows, agents, and teams,
3. model/backend policy,
4. project-specific runtime preferences.

Owner:

1. `docs/product/spec/project-activation-and-configurator-model.md`

### 5.3 Compiled Control Plane

Owns:

1. machine-readable compiled orchestration bundle,
2. intent classes,
3. compiled lane graph,
4. compiled role/skill/profile/flow activation,
5. model/backend selection policy,
6. cache-stable control partitions derived from compiled law.

Owner:

1. `docs/product/spec/compiled-runtime-bundle-contract.md`

### 5.4 Derived Serving Cache Plane

Owns:

1. fast CLI/runtime query views,
2. cache-stable prompt-prefix bundle partitions,
3. derived activation/control snapshots,
4. protocol-binding query snapshots,
5. cache manifests and invalidation tuples.

Primary home:

1. `.vida/cache/**`

Owner:

1. `docs/product/spec/runtime-paths-and-derived-cache-model.md`

### 5.5 Execution Plane

Owns:

1. orchestrator runtime behavior,
2. lane selection and dispatch,
3. tracked execution,
4. documentation/readiness/proof operations,
5. verifier and approval execution,
6. runtime-family consumption and closure.

Current bounded runtime families:

1. `TaskFlow`
2. `DocFlow`

### 5.6 State And Evidence Plane

Owns:

1. task truth,
2. tracked execution telemetry,
3. run-graph state,
4. governed context,
5. receipts and proofs,
6. readiness verdicts,
7. closure evidence.

### 5.7 Retrieval And Memory Plane

Owns:

1. semantic retrieval,
2. vector search,
3. graph relations,
4. code indexing,
5. symbol and dependency retrieval,
6. searchable long-term memory for project and runtime use,
7. retrieval support for skills, protocols, project artifacts, and host-project code intelligence.

Boundary rule:

1. this plane may share the same database engine as operational truth,
2. but retrieval/memory data must remain logically separate from operational runtime truth,
3. retrieval memory supports orchestration and project understanding, but it does not replace operational truth, receipts, or gate state.

### 5.8 Operational State And Synchronization Model

The detailed state split, synchronization law, conflict handling, and Release-2 reactive domain routing are owned by:

1. `docs/product/spec/operational-state-and-synchronization-model.md`
2. `docs/product/spec/embedded-runtime-and-editable-projection-model.md`
3. `docs/product/spec/runtime-paths-and-derived-cache-model.md`

Anchor rule:

1. runtime truth remains DB-first,
2. filesystem artifacts remain synchronized projections,
3. Git remains lineage rather than live runtime truth.

## 6. Canonical Runtime Roles Of TaskFlow And DocFlow

### 6.1 TaskFlow

`TaskFlow` is the primary execution substrate.

It owns:

1. tracked execution,
2. execution planning and block lifecycle,
3. task-state enactment and telemetry,
4. lane assignment enactment,
5. handoff/runtime packet execution,
6. verification/merge execution,
7. checkpoint/replay/recovery execution,
8. final runtime-consumption authority.

`TaskFlow` must not own:

1. framework law,
2. documentation law,
3. project documentation canon,
4. project-specific role law beyond compiled activation and validated overlay input.

### 6.2 DocFlow

`DocFlow` is the bounded documentation/readiness/proof runtime family.

It owns:

1. canonical documentation mutation/finalization,
2. inventory,
3. relation analysis,
4. validation,
5. readiness,
6. proof surfaces,
7. documentation-oriented operator tooling,
8. final documentation/readiness evidence used by `TaskFlow` before trust/closure.

`DocFlow` must not own:

1. execution authority,
2. top-level closure authority,
3. framework truth itself,
4. project-specific workflow law beyond canonical documentation/runtime operation rules.

## 7. Role, Skill, Profile, Flow, And Extensibility Ownership

The detailed owner docs for runtime identity and extension rights are:

1. `docs/product/spec/agent-role-skill-profile-flow-model.md`
2. `docs/product/spec/execution-preparation-and-developer-handoff-model.md`
3. `docs/product/spec/extensibility-and-output-template-model.md`

Anchor rule:

1. runtime identity must be compiled rather than rediscovered ad hoc,
2. execution preparation remains a first-class pre-implementation layer,
3. extensibility remains controlled by explicit sealed/augmentable/replaceable classes.

## 8. Model And Backend Policy

The system must support explicit runtime policy for:

1. cheap coding models,
2. deeper research models,
3. stronger synthesis/orchestration models,
4. verification or proving models,
5. deployment/devops-oriented execution lanes.

Policy rule:

1. model/backend selection is part of compiled runtime control,
2. it must be explicit and inspectable,
3. it must be project-activatable,
4. it must remain bounded by framework safety, handoff, verification, and closure law.

The operator may set:

1. speed bias,
2. cost bias,
3. quality bias,
4. verification strictness,
5. allowed backend/model classes,
6. preferred profiles by task class.

## 9. Orchestrator Design Rule

The target orchestrator should be cheap in steady state.

That means:

1. it should read compiled control data rather than large protocol corpora,
2. it should classify request intent and activate flow branches efficiently,
3. it should delegate specialized work to bounded lanes with explicit packet contracts,
4. it should remain responsible for top-level coordination, not for doing every expensive subtask itself.

Cheap-orchestrator rule:

1. complexity moves into canon and compilation,
2. not into repeated large-context orchestration prompts.

## 10. Non-Negotiable Architectural Rules

### 10.1 Canon-Law Rule

Framework and product law remain in canonical docs and executable config, not in hidden prompt templates or runtime-only code.

### 10.2 Compilation Rule

The runtime must compile the canon into machine-readable control artifacts before orchestration.

### 10.3 Fail-Closed Rule

Unresolved roles, skills, profiles, flows, packets, gates, or model/backend policies must block activation rather than degrade silently.

### 10.4 One-Owner Rule

Each semantic concern keeps one canonical owner:

1. `core` owns framework runtime law,
2. the shell owns orchestration and dispatch semantics,
3. runtime families own enactment,
4. implementations own concrete machinery.

### 10.5 Runtime-Family Separation Rule

`TaskFlow` and `DocFlow` remain sibling runtime families with one explicit seam, not one merged blob.

### 10.6 Human-Governance Rule

Human approval remains explicit at the places where the active policy requires it; operator convenience must not remove lawful review, verification, or approval boundaries.

### 10.7 Protected-Core Rule

Framework core and protected system protocols remain shielded from direct project rewrite.

Projects may:

1. extend designated augmentable surfaces,
2. activate, deactivate, or replace designated project-facing surfaces,
3. add project-owned protocols and behavior within lawful project extension points.

Projects must not:

1. mutate protected framework state directly in the operational database,
2. bypass migration/init protection for framework-owned protocol state,
3. replace sealed core law through ad hoc file edits or direct DB mutation.

## 11. Target Implementation Program

The target implementation program should proceed in this order:

1. define the compiled orchestration bundle contract,
2. compile canonical role/skill/profile/flow activation into that bundle,
3. compile model/backend/cost/quality policy into that bundle,
4. build cheap intent classification and flow activation over the compiled bundle,
5. connect compiled dispatch into `TaskFlow`,
6. connect documentation/readiness/proof activation into `DocFlow`,
7. close the `TaskFlow -> DocFlow` seam for final evidence-driven runtime trust,
8. replace donor bridges gradually with native runtime-family implementations.

## 12. Completion Proof

This architecture is considered materially realized only when all are true:

1. one canonical spec defines the compiled control-plane target,
2. one machine-readable orchestration bundle can be generated from canon plus project activation data,
3. the orchestrator can classify high-level intent without manual protocol selection,
4. the runtime can activate lawful role/profile/flow combinations from compiled control,
5. `TaskFlow` handles tracked execution and `DocFlow` handles final documentation/readiness/proof at the explicit seam,
6. runtime state, receipts, and approval/proof surfaces remain queryable through bounded operator paths,
7. the same system can be driven by business intent without the human operator managing internal routing by hand.

## 13. Source Alignment

This architecture aligns with:

1. `docs/product/spec/external-architecture-baseline.md`
2. `docs/product/spec/current-spec-map.md`
3. the active Release-1 program surfaces mapped in `docs/product/spec/release-1-plan.md`

Interpretation rule:

1. external baselines inform this architecture but do not replace VIDA-owned product law,
2. the current spec map remains the full registry,
3. `release-1-plan.md` remains the bounded Release-1 working entrypoint for the current release line.

## 14. Current Rule

1. this document is the top-level architecture anchor for the compiled autonomous delivery runtime,
2. detailed owner law lives in adjacent owner specs rather than accumulating here,
3. `TaskFlow` and `DocFlow` remain separate runtime-family owners inside one product runtime,
4. Release 1 and Release 2 remain distinct integration phases,
5. compiled control plus DB-first operational truth remains the non-negotiable target direction.

-----
artifact_path: product/spec/compiled-autonomous-delivery-runtime-architecture
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md
created_at: '2026-03-11T17:26:46+02:00'
updated_at: 2026-03-16T08:48:58.585356384Z
changelog_ref: compiled-autonomous-delivery-runtime-architecture.changelog.jsonl
