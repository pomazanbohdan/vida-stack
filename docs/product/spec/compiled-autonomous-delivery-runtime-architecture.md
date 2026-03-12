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

## 2.2 Operator-Facing Product Surfaces

The user-facing product should remain one visible `VIDA` system while exposing several bounded operational surfaces behind that single product identity.

Minimum user-facing surfaces:

1. `entry surface`
   - one primary CLI entry in Release 1
   - CLI plus UI in Release 2
2. `intake surface`
   - business conversations, requirements, tasks, and external artifacts enter here
3. `project configuration surface`
   - roles, skills, profiles, flows, teams, model/backend policy, and project activation settings
4. `planning surface`
   - specifications, PBIs, task graphs, and delivery scope
5. `execution surface`
   - active runtime work, lane activity, progress, blockers, and recovery state
6. `artifact surface`
   - produced documentation, code artifacts, build outputs, and delivery outputs
7. `approval surface`
   - review, correction, acceptance, rejection, and governance checkpoints
8. `verification surface`
   - proof, readiness, validation, and closure evidence
9. `release/devops surface`
   - build, publish, deployment, environment, and release receipts
10. `observability surface`
    - traces, run history, sync activity, routing decisions, and auditability

Discussion status rule:

1. `entry`, `intake`, `project configuration`, `planning`, `execution`, `artifact`, and `approval` surfaces are currently in active concretization,
2. `verification`, `release/devops`, and `observability` surfaces remain `next discussion` surfaces for now,
3. the following observability-oriented facets are also explicitly deferred to later discussion before deeper specification:
   - `run history`
   - `routing decisions`
   - `cost usage`
   - `failure reasons`
   - `compliance/audit trail`
4. these deferred surfaces may remain listed as target product surfaces, but they should not yet be treated as fully elaborated operator-surface contracts.

Release-1 project-configuration minimum:

1. Release 1 must already cover the full minimal configuration pool needed for a real project runtime, not a reduced preview mode,
2. that minimum pool includes:
   - roles
   - skills
   - profiles
   - flow sets
   - agents
   - teams
   - model classes
   - backend classes
   - policy surfaces
   - project protocols
3. if this pool is materially incomplete, the system should be treated as pre-release or `0.x`, not as a full Release 1 surface.
4. dedicated system-owned root output templates are not required for Release 1,
5. project-facing document or output templates may live inside skills or adjacent project extension surfaces until a later explicit template registry is introduced.

Framework role-catalog rule:

1. the framework must ship a baseline catalog of canonical delivery role classes,
2. that catalog should cover the common delivery spectrum such as business analysis, product/project coordination, implementation, testing, verification, and operations-oriented work,
3. each framework role class must carry at least:
   - a role identity
   - a baseline description
   - baseline behavioral instructions
   - baseline compatible skills
4. project configuration may enable, disable, replace, or extend those role classes within the allowed extensibility rules.

Project-configurator mode rule:

1. project configuration must support both explicit selection mode and automatic selection mode,
2. in explicit mode the project/operator declares which roles, skills, profiles, flows, and adjacent activation surfaces are active,
3. in automatic mode the runtime may select and activate the needed role classes dynamically from the enabled catalog based on the current request and active project policy,
4. automatic mode must still respect explicit project disablement, replacement, and policy constraints,
5. automatic mode must not silently activate a role or project-owned surface that the current configuration has explicitly disabled.

Planning/scope rule for Release 1:

1. the planning/scope surface must already be inspectable through filesystem artifacts and CLI query paths,
2. the operator must be able to retrieve planning state, scope state, specification state, and task-graph state without requiring a GUI,
3. the operator may ask in natural language for the current planning/scope picture,
4. the runtime must resolve that request against the database truth, assemble the required planning data, and render it through a lawful output protocol rather than through ad hoc freeform summarization,
5. each planning/scope view must have bounded commands that can retrieve the full required state directly.

Execution-surface rule:

1. in Release 1 the execution surface must already be retrievable through bounded runtime/query commands and renderable back to the operator in chat or shell form,
2. the rendered execution view must be derived from runtime state rather than improvised narrative,
3. filesystem projection may mirror this execution state where the current implementation can materialize it safely,
4. in Release 2 the same execution state may be elevated into more aggregated operator views such as epics, tasks, blockers, and escalations without creating a second truth model.

Status-family rule:

1. `status` is not one global monolithic output,
2. each major runtime family or subsystem must expose its own bounded status family,
3. examples include:
   - flow/runtime status
   - memory/index status
   - synchronization status
   - active task/execution status
4. model-facing status responses must be built from those bounded status paths rather than from one ambiguous catch-all status command.

Release-1 memory-status boundary:

1. Release 1 memory status is limited to command-time snapshots and bounded retrieval/reporting over the current available memory state,
2. continuous daemon-driven indexing, freshness tracking, long-running graph maintenance, and always-on memory-runtime metrics belong to a later reactive/runtime stage,
3. Release 1 must not assume a permanently running memory daemon in order to expose a lawful memory status surface.

Artifact-surface rule:

1. the artifact surface is where all outputs that can already be materially realized from the current discussion/program state must appear,
2. that includes documentation, specifications, planning artifacts, task graphs, code changes, build outputs, release outputs, proofs, and other realizable delivery artifacts,
3. the runtime should not leave realizable outputs only as chat conclusions when they can already be materialized as lawful artifacts,
4. artifact visibility in Release 1 must already be available through filesystem materialization and CLI retrieval paths,
5. the artifact surface must remain tied to operational DB truth and synchronized filesystem projection rather than becoming a disconnected reporting layer.

Next-discussion carry-forward rule:

1. when a requested or inferred output cannot yet be materially realized within the current implementation/program state, it must not disappear into chat history,
2. such unresolved outputs must be recorded explicitly as `next discussion`, `next implementation`, `blocked`, or equivalent forward state,
3. the system must distinguish between:
   - `implemented now`
   - `planned next`
   - `blocked / unresolved`
4. artifact and planning surfaces should preserve this distinction so the operator can see what is already real versus what still requires later design or implementation closure.

Approval-surface rule:

1. the approval surface governs all user/runtime interaction where the system must ask, clarify, confirm, or pause for a decision,
2. in Release 1 this surface must already work through CLI/shell interaction tools,
3. in Release 2 the same approval model must remain available through UI without removing CLI capability,
4. approval interactions must be treated as operational events, not as disposable chat fragments.

Minimum approval interaction classes:

1. `inform`
   - show the user a bounded status or decision context
2. `confirm`
   - ask for yes/no or approve/reject
3. `single-choice`
   - ask the user to select one bounded option
4. `multi-choice`
   - ask the user to select from several bounded options when the shell or UI supports it
5. `sequenced questions`
   - ask several questions in order when one answer is not enough
6. `freeform answer`
   - allow the user to provide custom input when predefined options are insufficient
7. `correction request`
   - allow the user to modify or redirect a proposed runtime action

Approval-surface operational rule:

1. approval prompts must be generated through explicit project/framework interaction protocols rather than ad hoc wording,
2. every approval interaction must carry enough context for the user to make the decision safely,
3. every user response must be captured as durable operational evidence in the database,
4. approval outcomes must remain queryable through CLI in Release 1 and through both CLI and UI in Release 2,
5. approval events must be eligible for filesystem projection and Git-backed lineage when they belong to project-visible artifacts or audit trails.

Approval next-discussion items:

1. exact approval prompt DSL and schema,
2. timeout/escalation behavior when the user does not answer,
3. default option policy and whether defaults are allowed,
4. approval batching versus one-question-at-a-time behavior,
5. which approval classes may be delegated to model-assisted recommendation before user confirmation.

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

Release-1 configuration and migration rule:

1. Release 1 must already provide a DB-first project-configuration surface managed through CLI,
2. framework-owned protocol state loaded into the operational database must update only through an explicit migration/init path rather than ad hoc direct mutation,
3. project-owned protocols, skills, roles, agents, teams, and adjacent project activation elements may be created and updated inside the project layer,
4. those project-owned elements must still pass the same controlled migration/init and validation path before they become active operational state,
5. persistence and restoration of this activation/configuration state are required in Release 1, not deferred to a later release.

Release-1 lifecycle rule:

1. project-facing configuration elements must support a consistent control lifecycle through runtime tools,
2. at minimum the system must support import, activation, update, replacement, disablement, and restoration flows across the main project-owned element classes,
3. exact per-element permission differences remain a `next discussion` item where the architecture has not yet drawn a stricter boundary.

Release-1 state model:

1. operational truth is database-first,
2. the primary operational database is `SurrealDB`,
3. filesystem artifacts remain editable and importable/exportable,
4. bidirectional synchronization between DB and filesystem is required,
5. `Git` acts as backup and historical lineage for the filesystem projection rather than as runtime truth,
6. CLI remains the full-capability management surface for import, export, sync, reconcile, status, and migration operations.

Release-1 queryability rule:

1. every major user-facing surface must have corresponding CLI retrieval commands,
2. those commands must be able to read the required operational state from the database and return a bounded complete result,
3. model-assisted answers may be built on top of those query paths, but the underlying retrieval path must remain explicit and inspectable,
4. the operator should be able to get the same state either by direct command or by asking the model through the same runtime surface.

Release-1 bootstrap command rule:

1. the host-shell integration must be able to bootstrap VIDA through runtime commands rather than by depending only on raw repository bootstrap files,
2. the minimum root bootstrap family must include one orchestrator-init path and one general agent-init path,
3. the orchestrator-init path must output the minimum command/help/protocol set that the orchestrator model must see at session start,
4. the agent-init path must output the bounded command/help/protocol set needed by non-orchestrator lanes,
5. exact command naming and argument syntax remain a `next discussion` item, but the boot behavior itself is mandatory in Release 1.

Release-1 installer/init behavior:

1. the init/installer flow must inspect the current project structure,
2. if the project configuration file is missing, it must be created from a canonical template,
3. the init flow must discover and record key project paths such as documentation roots and synchronized file surfaces,
4. the init flow must export a usable project structure view for later synchronization and runtime routing,
5. the init flow must detect whether `AGENTS.md` already exists,
6. if `AGENTS.md` exists, project-specific rules must be moved or normalized into the sidecar/project layer rather than left to compete with framework bootstrap law,
7. sidecar and project instruction surfaces must be checked so that project-specific behavior does not conflict with framework-owned bootstrap and system instructions,
8. the installer/init protocol must be expressed in a human-readable form suitable for the orchestrating model to execute through runtime tools.

Release-1 approval rule:

1. CLI must already support bounded question/answer interaction for approvals and clarifications,
2. the runtime must be able to pause, ask, receive, and persist user decisions through the shell interaction layer,
3. approval packets and user answers must be rendered through explicit interaction protocols and, where the project provides them, project-owned skill or sidecar templates rather than improvised chat phrasing alone.

### Release 2: Host-Project Integration

Release 2 embeds VIDA into another project or product environment as the autonomous delivery runtime for that host project.

Release-2 rule:

1. VIDA is integrated into the host project rather than only into the host shell,
2. project roles, skills, profiles, flow sets, model/backend policy, documentation surfaces, and delivery/runtime behavior become integrated into that host project environment,
3. VIDA acts as an embedded delivery platform for the host project,
4. this release optimizes for project-level integration rather than only shell-level entry.

Release-2 extension rule:

1. Release 2 adds a UI control surface without removing or weakening full CLI capability,
2. UI and CLI must operate over the same DB-first operational model,
3. Release 2 adds reactive synchronization rather than relying only on operator-triggered synchronization,
4. file watchers, hooks, decision protocols, and apply tools become part of the live runtime,
5. synchronization may call an LLM decision protocol for ambiguous cases, but controlled tools remain the execution path for actual mutation and DB update.

Release-2 approval extension:

1. the same approval model must become available through UI interaction surfaces,
2. UI may provide richer interaction modes, but it must not become the only approval path,
3. CLI and UI approvals must write into the same operational evidence model.

Boundary rule:

1. Release 1 is shell integration,
2. Release 2 is project integration,
3. the two releases should not be collapsed into one ambiguous entry model.

## 3. Architectural Problem Statement

VIDA already has:

1. canonical framework law in `vida/config/instructions/**`,
2. active product law in `docs/product/spec/**`,
3. project activation data in `vida.config.yaml`,
4. project extension registries in `docs/process/agent-extensions/**`,
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

Primary homes:

1. `vida.config.yaml`
2. `docs/process/agent-extensions/**`

Operational-state rule:

1. the project activation plane must have a durable operational representation in the database,
2. framework-owned protocol state projected into that database is protected and updated only through migration/init rules,
3. project-owned activation elements may evolve more freely, but only within the lawful project layer and under controlled migration/validation,
4. runtime must distinguish sealed framework state from mutable project activation state.

Project-activation discussion note:

1. `team` is treated as a coordinated runtime composition of roles, profiles, agents, and flows under one shared objective and policy boundary rather than only as a static list of members,
2. the compiled framework owns the coordination semantics of that composition,
3. project configuration owns which teams exist, which roles/profiles/agents they enable, and whether they are active,
4. exact storage, activation, and lifecycle semantics of `team` remain a `next discussion` item.

Role-class rule:

1. a `role class` is the canonical role identity/type recognized by the framework, such as business analysis, project coordination, implementation, testing, verification, or operations,
2. a role class is not the same as a profile, agent instance, or team,
3. a `profile` is a configured operating variant of a role class,
4. an `agent` is a runtime executor or lane instance that acts using a selected role class and profile,
5. a `team` is a coordinated composition that may contain multiple role classes, profiles, and agents.

### 5.3 Compiled Control Plane

Owns:

1. machine-readable compiled orchestration bundle,
2. intent classes,
3. compiled lane graph,
4. compiled role/skill/profile/flow activation,
5. model/backend selection policy,
6. packet schemas,
7. gate chain,
8. route constraints,
9. activation triggers and runtime-family branches.

This plane is the central missing target of the current program.

Output-rendering rule:

1. the compiled control plane must also know which output protocol and, when one exists, which project-owned or future system-owned template family applies to each root user-facing response class,
2. retrieval alone is not enough; runtime must be able to render stable operator-facing outputs from retrieved state,
3. root outputs must not depend on ad hoc prompt phrasing alone.

### 5.4 Execution Plane

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

### 5.5 State And Evidence Plane

Owns:

1. task truth,
2. tracked execution telemetry,
3. run-graph state,
4. governed context,
5. receipts and proofs,
6. readiness verdicts,
7. closure evidence.

### 5.6 Retrieval And Memory Plane

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

### 5.7 Operational State And Synchronization Model

The target runtime uses three coordinated state representations:

1. `operational state`
   - the live DB-first runtime truth
2. `filesystem projection`
   - synchronized editable markdown/YAML and adjacent file artifacts
3. `Git lineage`
   - historical backup and review surface for the filesystem projection

State rule:

1. the runtime executes against database truth,
2. the filesystem is a synchronized editable projection rather than an equal second truth source,
3. changes may flow in both directions,
4. final operational convergence must still pass through the database.

Synchronization rule:

1. synchronization is bidirectional,
2. authority remains DB-first,
3. conflicts must fail closed,
4. reconciliation must be explicit and auditable,
5. silent destructive merges are forbidden.

Conflict-resolution rule:

1. DB/filesystem conflicts must not be resolved authoritatively by freeform LLM judgment alone,
2. conflict resolution must occur through explicit point mutations using available tools in one direction or the other,
3. the model may help classify, explain, or recommend a resolution path,
4. the authoritative resolution path must remain tool-mediated, bounded, and auditable.

### 5.8 Reactive Synchronization And Domain Routing

Release 2 adds a reactive synchronization engine that must distinguish at least two event domains:

1. `engine-owned domain`
   - VIDA engine files, documentation, config, and synchronized internal projections
2. `host-project domain`
   - external codebase and project artifacts integrated by VIDA

For the engine-owned domain:

1. watcher events trigger sync/reconcile flow,
2. the runtime decides whether to import, export, refresh, block, or escalate,
3. controlled tools apply the accepted change into the DB-first state model.

For the host-project domain:

1. watcher events trigger indexing/memory flow rather than internal engine sync,
2. the runtime may update semantic search, graph state, code index, and memory artifacts,
3. these updates support orchestration, search, and context resolution for the host project.

Reactive-flow rule:

1. watcher detects,
2. classifier interprets,
3. decision layer resolves,
4. apply layer mutates,
5. reconciliation verifies,
6. receipt layer records the result.

Reactive-event discussion note:

1. the exact canonical event taxonomy for Release 2 remains a `next discussion` item,
2. that discussion must define what an event is, why it exists, which domains emit it, and which classes are required in the first reactive engine,
3. until then, the minimal architectural meaning of the event model is:
   - detect a change
   - classify the change
   - decide whether sync, indexing, ignore, block, or escalation is required
   - execute the bounded mutation path
   - persist receipts for audit and later recovery.

## 6. Runtime Compilation Rule

The runtime must not treat protocols, overlays, and extension registries as unrelated prompt fragments.

Instead, the system must compile one runtime control bundle from:

1. framework `core` law,
2. orchestration-shell law,
3. runtime-family capability law,
4. project activation data from `vida.config.yaml`,
5. project role/skill/profile/flow registries,
6. enabled shared skills,
7. route and gate constraints,
8. model/backend policy,
9. runtime-family discovery maps,
10. current evidence/gate requirements.

Compact rule:

1. human-readable canon in,
2. compact executable control bundle out.

Compilation output rule:

1. the compiled control plane must know where the engine workspace is,
2. it must know where synchronized project/config/document surfaces are located,
3. it must know where the host project codebase is located,
4. it must route changes and retrieval requests differently depending on that domain classification,
5. it must know which response protocol and, when defined, which output template family applies to each major operator query/result class.

Project-protocol compilation rule:

1. the project may register many project-owned protocols,
2. runtime must distinguish between:
   - `known project protocols`
   - `compiled executable project protocols`
3. a known project protocol may be discoverable, triggerable, or readable by the model without becoming part of compiled execution control,
4. only project protocols that are explicitly activated, mapped, or compiled into runtime control become executable protocol surfaces,
5. team-management behavior belongs by default to project configuration and compiled framework control rather than to a separate compiled project-execution protocol unless a later canonical rule narrows that boundary.

Project-protocol discussion note:

1. the exact promotion path by which a project-owned protocol becomes a compiled executable protocol remains a `next discussion` item,
2. that discussion must define how maps, triggers, bindings, and validation rules admit a project protocol into compiled execution control,
3. until then, the runtime must keep the distinction between merely known project protocols and compiled executable project protocols explicit.

## 7. Compiled Control Bundle

The compiled orchestration bundle must contain at least:

1. `intent_classes`
   - business intake, research, spec formation, implementation, verification, release/devops, documentation maintenance
2. `role_registry`
   - base framework roles, enabled project roles, authority boundaries
3. `skill_registry`
   - compatible skills, capability payloads, enablement rules
4. `profile_registry`
   - resolved role + skill set + operating posture + optional model/backend preference
5. `flow_set_registry`
   - enabled standard and project-specific multi-lane flow definitions
6. `activation_graph`
   - what activates from which request shape or runtime state
7. `routing_policy`
   - which profile/backend/model posture is lawful for which task class
8. `gate_chain`
   - review, verification, approval, readiness, and closure requirements
9. `packet_contracts`
   - handoff packet schemas and allowed context/evidence payloads
10. `runtime_family_branches`
    - `TaskFlow` branch, `DocFlow` branch, and the lawful seams between them
11. `cost_quality_policy`
    - explicit tradeoffs for cheap routing, deep research, synthesis, verification, and deploy/release work
12. `fail_closed_rules`
    - what must block rather than degrade silently

## 8. Canonical Runtime Roles Of TaskFlow And DocFlow

### 8.1 TaskFlow

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

### 8.2 DocFlow

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

## 9. Human-To-Runtime Operating Flow

The target operator journey is:

1. a human provides business conversation, request, or artifact input,
2. the runtime classifies the intent,
3. the runtime activates the lawful role/profile/flow composition,
4. the runtime routes the work into the proper lane chain,
5. the proper runtime family is activated to create or validate the needed artifact,
6. a human may review or correct the result,
7. approved output becomes the next runtime input without manual internal rewiring,
8. delivery continues until runtime closure produces both the binary outcome and the required evidence surfaces.

Example target:

1. business conversation arrives,
2. intake resolves to requirement/spec formation,
3. a `business_analyst` lane with compatible documentation/spec skill payload is activated,
4. documentation/spec artifacts are created and validated through lawful documentation surfaces,
5. PM/governance review is requested,
6. accepted specification activates implementation flow,
7. the runtime compiles one execution graph and dispatches specialized lanes,
8. coding, research, verification, and devops proceed under one compiled route,
9. final output includes working binaries, documentation, and proof/readiness evidence.

## 10. Role, Skill, Profile, And Flow Composition

The current role/skill/profile/flow model already defines the semantic pieces.

This architecture adds one requirement:

1. these pieces must become runtime-compiled identity rather than late prompt heuristics.

Required compiled identity shape:

1. `base_role`
2. `validated_project_role`
3. `validated_project_skills`
4. `enabled_shared_skills`
5. `validated_profile`
6. `selected_flow_set`
7. `route_constraints`
8. `gate_constraints`
9. `cost_quality_constraints`

The orchestrator must consume this compiled identity instead of rediscovering the same logic ad hoc on every request.

## 10.1 Extensibility Classes

The runtime must treat extension rights as a controlled matrix rather than a universal edit-right.

Three extensibility classes are required:

1. `sealed`
   - protected framework/core/system law
   - not directly replaceable by the project
2. `augmentable`
   - upper system surfaces that may accept sidecar-like extension without replacing the canonical owner
3. `replaceable`
   - project-facing roles, skills, profiles, flows, teams, agents, and adjacent behavior surfaces that may be disabled and replaced by project-owned alternatives

Extensibility rule:

1. the user must not directly rewrite protected core/system canon as a project customization path,
2. sidecar-style extension is the lawful way to deepen augmentable surfaces,
3. full replacement is allowed only where the architecture marks the surface as project-replaceable,
4. activation, validation, migration, and compiled runtime identity must respect these classes.

## 10.1.1 Protocol Versus Template

The runtime must distinguish clearly between protocols and templates.

1. `protocol`
   - an instruction-bearing behavioral or operational rule that governs what must happen
2. `template`
   - a rendering or structural output form used at the moment the protocol requires information, an artifact, or a contract to be produced

Template rule:

1. templates may define document shapes, screen-output shapes, packets, or external interaction contracts,
2. templates do not replace protocols,
3. templates are activated by protocol-governed moments and output classes,
4. project templates are project-replaceable surfaces unless a narrower rule explicitly protects them.

Project replacement rule:

1. project roles, skills, profiles, flow sets, agents, project protocols, and output templates are project-replaceable surfaces,
2. a project-replaceable surface may be disabled in active configuration while still remaining preserved in the database or import state,
3. active configuration must declare which source/path is authoritative for each replaced project surface,
4. sidecars may override or deepen project-replaceable templates and project-owned behavior surfaces,
5. sealed framework/core/system protocols remain non-replaceable,
6. the exact boundary for some upper non-core system protocols remains a `next discussion` item.

## 10.2 Root Output Template System

The runtime requires an explicit output-rendering strategy for root operator-facing outputs.

This strategy must define:

1. which output classes exist,
2. which protocol governs each output class,
3. when a dedicated template exists versus when model rendering is allowed,
4. which data fields are mandatory for each rendered output,
5. which surfaces may produce human-facing narrative output versus structured output.

Minimum target output classes:

1. planning/scope snapshot,
2. specification snapshot,
3. task-graph snapshot,
4. execution status snapshot,
5. artifact surface snapshot,
6. approval/review packet,
7. verification/readiness result,
8. sync/reconcile result,
9. release/deployment result,
10. observability/audit snapshot.

Output-rendering rule:

1. Release 1 does not require a dedicated system-owned root template registry,
2. project-owned templates may live inside skills or adjacent project extension surfaces,
3. the model may compose the final answer dynamically in Release 1 when no canonical template exists,
4. when a project-owned or later system-owned canonical template exists, rendering must follow that template rather than inventing a new output shape,
5. later releases may formalize a stronger shared template registry without making Release 1 invalid.

Format-selection rule:

1. when multiple project-owned output formats or templates are available and no higher-precedence canonical format is active, the model may choose the best-fit format contextually for the current task,
2. format choice under those conditions is a project-level behavior concern rather than a fixed framework-level priority rule,
3. if a higher-precedence canonical format is active for the current output class, the model must follow it instead of making a free contextual choice.

## 11. Model And Backend Policy

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

## 12. Orchestrator Design Rule

The target orchestrator should be cheap in steady state.

That means:

1. it should read compiled control data rather than large protocol corpora,
2. it should classify request intent and activate flow branches efficiently,
3. it should delegate specialized work to bounded lanes with explicit packet contracts,
4. it should remain responsible for top-level coordination, not for doing every expensive subtask itself.

Cheap-orchestrator rule:

1. complexity moves into canon and compilation,
2. not into repeated large-context orchestration prompts.

## 13. Non-Negotiable Architectural Rules

### 13.1 Canon-Law Rule

Framework and product law remain in canonical docs and executable config, not in hidden prompt templates or runtime-only code.

### 13.2 Compilation Rule

The runtime must compile the canon into machine-readable control artifacts before orchestration.

### 13.3 Fail-Closed Rule

Unresolved roles, skills, profiles, flows, packets, gates, or model/backend policies must block activation rather than degrade silently.

### 13.4 One-Owner Rule

Each semantic concern keeps one canonical owner:

1. `core` owns framework runtime law,
2. the shell owns orchestration and dispatch semantics,
3. runtime families own enactment,
4. implementations own concrete machinery.

### 13.5 Runtime-Family Separation Rule

`TaskFlow` and `DocFlow` remain sibling runtime families with one explicit seam, not one merged blob.

### 13.6 Human-Governance Rule

Human approval remains explicit at the places where the active policy requires it; operator convenience must not remove lawful review, verification, or approval boundaries.

### 13.7 Protected-Core Rule

Framework core and protected system protocols remain shielded from direct project rewrite.

Projects may:

1. extend designated augmentable surfaces,
2. activate, deactivate, or replace designated project-facing surfaces,
3. add project-owned protocols and behavior within lawful project extension points.

Projects must not:

1. mutate protected framework state directly in the operational database,
2. bypass migration/init protection for framework-owned protocol state,
3. replace sealed core law through ad hoc file edits or direct DB mutation.

## 14. External Architecture Baseline

This target architecture is aligned to the current official vendor baselines below.

### 14.1 OpenAI

Alignment:

1. behavior, tools, and guardrails sit at agent/runtime boundaries,
2. orchestration owns routing and handoffs,
3. tracing and execution state belong in runtime surfaces.

Official references:

1. `https://openai.github.io/openai-agents-python/handoffs/`
2. `https://openai.github.io/openai-agents-js/guides/guardrails/`
3. `https://openai.github.io/openai-agents-js/guides/running-agents`

### 14.2 Anthropic

Alignment:

1. subagents own scoped prompts and bounded expertise,
2. role wording tends toward behavior contract,
3. upper-layer orchestration should therefore prefer lane/coordination semantics over role-behavior ownership.

Official references:

1. `https://docs.anthropic.com/en/docs/claude-code/sub-agents`
2. `https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices#give-claude-a-role`

### 14.3 Microsoft

Alignment:

1. orchestration is a coordination-pattern layer,
2. execution/runtime machinery stays below orchestration,
3. agent specialization and plugin/tool execution are explicit rather than implicit.

Official references:

1. `https://learn.microsoft.com/en-us/semantic-kernel/frameworks/agent/agent-architecture`
2. `https://learn.microsoft.com/en-us/semantic-kernel/frameworks/agent/agent-orchestration/`

## 15. Target Implementation Program

The target implementation program should proceed in this order:

1. define the compiled orchestration bundle contract,
2. compile canonical role/skill/profile/flow activation into that bundle,
3. compile model/backend/cost/quality policy into that bundle,
4. build cheap intent classification and flow activation over the compiled bundle,
5. connect compiled dispatch into `TaskFlow`,
6. connect documentation/readiness/proof activation into `DocFlow`,
7. close the `TaskFlow -> DocFlow` seam for final evidence-driven runtime trust,
8. replace donor bridges gradually with native runtime-family implementations.

## 16. Completion Proof

This architecture is considered materially realized only when all are true:

1. one canonical spec defines the compiled control-plane target,
2. one machine-readable orchestration bundle can be generated from canon plus project activation data,
3. the orchestrator can classify high-level intent without manual protocol selection,
4. runtime can activate lawful lanes, skills, profiles, and flow sets from compiled data,
5. `TaskFlow` executes the delivery path as primary execution authority,
6. `DocFlow` supplies documentation/readiness/proof authority through one explicit bounded seam,
7. model/backend routing is inspectable, project-activatable, and fail-closed,
8. Release 1 exposes a full CLI-managed DB-first configuration and synchronization surface,
9. filesystem projection and Git-backed lineage operate as a synchronized mirror without displacing DB-first authority,
10. protected framework state and mutable project activation state are clearly separated and migration-controlled,
11. Release 2 adds reactive synchronization, hooks, and watcher-driven routing without removing Release-1 CLI capability,
12. the runtime can distinguish engine-owned sync events from host-project indexing/memory events,
13. retrieval/memory data and operational runtime truth can coexist in one database engine without collapsing into one undifferentiated state model,
14. implementation-facing work closes with working binaries or equivalent runnable build outputs for the scoped deliverable,
15. a business-input-to-binary-output flow can complete without hidden human rewiring of internal protocol mechanics.

## 17. Source Alignment

This architecture is synthesized from the currently active canon and runtime-family target plans, especially:

1. `docs/product/spec/canonical-runtime-layer-matrix.md`
2. `docs/product/spec/root-map-and-runtime-surface-model.md`
3. `docs/product/spec/agent-role-skill-profile-flow-model.md`
4. `docs/product/spec/agent-lane-selection-and-conversation-mode-model.md`
5. `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`
6. `docs/product/spec/docflow-v1-runtime-modernization-plan.md`
7. `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`
8. `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`
9. `vida/config/instructions/runtime-instructions/runtime.direct-runtime-consumption-protocol.md`

## 18. Current Rule

This document is the current target architecture root for the autonomous delivery runtime program.

That means:

1. future TaskFlow, DocFlow, compiler, orchestrator, and overlay work should be checked against this target,
2. changes to runtime-family responsibilities should update this document when they affect the target architecture,
3. this document may deepen over time, but it must not become a second competing owner of lower-level law that already has a canonical home elsewhere.

-----
artifact_path: product/spec/compiled-autonomous-delivery-runtime-architecture
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md
created_at: '2026-03-11T20:05:00+02:00'
updated_at: '2026-03-11T22:34:08+02:00'
changelog_ref: compiled-autonomous-delivery-runtime-architecture.changelog.jsonl
