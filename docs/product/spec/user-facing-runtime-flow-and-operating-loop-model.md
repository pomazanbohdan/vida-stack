# User-Facing Runtime Flow And Operating Loop Model

Status: active product law

Purpose: define the canonical operator-facing runtime journey for VIDA as one lawful product loop, starting with `install / init / bootstrap`, so Release 1 runtime internals map to an explicit user-visible operating path rather than remaining only as internal architecture fragments.

## 1. Problem

The current canon already defines:

1. compiled runtime direction,
2. DB-first activation and protocol import,
3. `.vida/` runtime placement,
4. fail-closed runtime execution,
5. status-family and approval surfaces.

What remains under-concretized is the first operator journey:

1. how a user gets from no runtime to a healthy initialized project,
2. what belongs to `install`,
3. what belongs to `init`,
4. what belongs to per-session `bootstrap`,
5. what the runtime must refuse to do before readiness is achieved.

Without that split:

1. installer behavior, runtime init, and session bootstrap can blur together,
2. the operator cannot tell which remediation command to run,
3. architecture remains correct but product operation stays underspecified.

## 2. Goal

The operator-facing loop must distinguish:

1. `install`
   - get the VIDA runtime onto the machine
2. `init`
   - attach and prepare one project-local `.vida/` runtime home
3. `bootstrap`
   - start one lawful runtime session against already initialized project state

Compact rule:

1. install the runtime once,
2. initialize the project once per project lifecycle or after structural drift,
3. bootstrap every working session,
4. refuse non-bootstrap work before readiness.

## 3. Stage-1 Scope

This document currently closes these operator stages:

1. `install / init / bootstrap`
2. `project activation / config`
3. `intake / planning`
4. `execution / approval / interrupt-resume`
5. `artifact materialization`
6. `status / doctor / remediation`
7. `export / edit / import`
8. `closure / reopen`

## 3.1 Minimum Product Surfaces

The user-facing product should remain one visible `VIDA` system while exposing several bounded operational surfaces behind that single product identity.

Minimum user-facing surfaces:

1. `entry surface`
   - one primary CLI entry in Release 1
   - CLI plus UI in Release 2
2. `intake surface`
   - business conversations, requirements, tasks, and external artifacts enter here
   - current launcher-owned intake/runtime seam is `vida taskflow consume final|continue|advance`
3. `project configuration surface`
   - roles, skills, profiles, flows, teams, model/backend policy, and project activation settings
4. `planning surface`
   - specifications, PBIs, task graphs, and delivery scope
5. `execution surface`
   - active runtime work, lane activity, progress, blockers, recovery state, and persisted dispatch packets
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

## 3.2 Cross-Stage Operator Journey

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
6. accepted specification activates execution-preparation flow,
7. a `solution_architect` lane prepares one bounded architecture-preparation report and developer handoff packet,
8. the runtime compiles one execution graph and dispatches specialized lanes from that prepared handoff,
9. coding, research, verification, and devops proceed under one compiled route,
10. final output includes working binaries, documentation, and proof/readiness evidence.

Design-first intake rule:

1. when one user request already includes research/specification/planning plus later implementation or code delivery, the intake surface must not jump directly into developer-lane execution,
2. the runtime must first route that request into a design-first path,
3. the preferred one-shot launcher surface for that path is `vida taskflow bootstrap-spec "<request>" --json`,
4. the intake/runtime path may first be materialized through `vida taskflow consume final "<request>" --json`, which must emit the routed receipt, dispatch packet, and the bounded next lawful command sequence,
5. once a lawful persisted packet exists, `vida taskflow consume continue` and `vida taskflow consume advance` become the canonical launcher-owned progression surfaces instead of manually reconstructing the handoff path,
6. `vida taskflow bootstrap-spec "<request>" --json` remains the preferred tracked launcher action when the returned route is design-first,
7. that surface must materialize the feature epic, the spec-pack task, and the canonical design-doc scaffold before normal development-team execution begins,
8. when the runtime launches a bounded `business_analyst` or `pm` lane from that design-first path, that launch is an active wait state, not completion proof,
9. `work-pool-pack` shaping must remain blocked until specification/planning evidence returns and the design artifact is finalized and validated through `vida docflow`,
10. only after that evidence plus spec-pack closure may normal `TaskFlow` delivery orchestration continue.

## 4. Stage 1: Install / Init / Bootstrap

### 4.1 Entry Cases

Release 1 must support these bounded entry cases:

1. fresh machine install,
2. upgrade of an existing installed runtime,
3. first-time initialization of one project,
4. re-initialization after migration or broken runtime state,
5. normal session bootstrap in an already initialized project.

Rule:

1. each case must resolve through a bounded runtime path,
2. the operator must not need to repair protocol/bootstrap state by manual file surgery.

### 4.2 `install`

`install` is machine-level runtime acquisition.

It owns:

1. downloading or placing the bounded release payload,
2. installing the primary `vida` entry surface,
3. installing the bounded runtime-family donor surfaces required by Release 1,
4. installing the installer-management surface,
5. ensuring the active installer-management script can be refreshed on forced upgrade,
6. scaffolding the minimal packaged runtime assets required for first project initialization.

`install` does not own:

1. project activation,
2. project-specific protocol import,
3. project-local readiness,
4. session-level bootstrap.

Install success rule:

1. after install, the operator has a usable `vida` surface,
2. but the runtime may still be uninitialized for the current project.

### 4.3 `init`

`init` is project-level runtime materialization.

It owns:

1. locating or creating the project-local `.vida/` runtime home,
2. materializing the minimum model-visible framework/bootstrap surfaces required for the current project,
3. creating `.vida/config/**`, `.vida/db/**`, `.vida/cache/**`, `.vida/framework/**`, `.vida/project/**`, and adjacent required runtime directories,
4. scaffolding runtime configuration from canonical templates when absent,
5. enforcing the bootstrap-carrier split around `AGENTS.md` and `AGENTS.sidecar.md`,
6. importing required machine-readable framework/protocol payloads into the authoritative project-local DB,
7. writing import and migration receipts,
8. establishing the first valid project-local readiness state,
9. reporting precise remediation when any required import or scaffold step fails.

Bootstrap-carrier rule during `init`:

1. `init` must materialize framework-owned `AGENTS.md` into the project root when it is absent,
2. `init` must ensure `AGENTS.sidecar.md` exists as the project-doc bootstrap carrier,
3. if a pre-existing root `AGENTS.md` mixes framework and project rules, `init` must preserve project-owned content by moving or normalizing it into `AGENTS.sidecar.md`,
4. after normalization, root `AGENTS.md` must remain framework-owned bootstrap only,
5. `init` must not leave two competing bootstrap carriers in root scope,
6. if `AGENTS.sidecar.md` is absent, `init` must create it rather than keeping project-routing rules embedded in `AGENTS.md`,
7. if safe normalization cannot be determined, `init` must fail closed with an explicit bounded remediation path rather than silently guessing how to rewrite the bootstrap carriers.

`init` does not own:

1. normal task execution,
2. project-delivery work,
3. interactive planning or approval flow beyond bounded initialization questions.

Initialization success rule:

1. after `init`, the project has one lawful `.vida/` home,
2. required framework/protocol state exists in DB truth,
3. non-bootstrap execution is now eligible, subject to later gates.

### 4.4 `bootstrap`

`bootstrap` is session-level runtime entry.

It owns:

1. opening a bounded runtime session against the initialized project,
2. checking whether required imported state is present and valid,
3. determining the active init/bundle/runtime posture,
4. loading the bounded orchestrator or general-agent bootstrap payload,
5. exposing the minimum help/status/remediation surfaces needed at session start,
6. refusing to continue into non-bootstrap execution when required state is missing or invalid.

Bootstrap rule:

1. bootstrap happens every working session,
2. bootstrap must prefer DB truth, embedded artifacts, and derived cache over raw source rereads,
3. bootstrap must not silently downgrade into a broad source-tree fallback.

### 4.5 Canonical Stage-1 Sequence

The lawful Stage-1 sequence is:

1. `install`
2. `init`
3. `bootstrap`
4. only then `non-bootstrap execution`

Interpretation rule:

1. install alone is not enough for project work,
2. init alone is not enough for a live session,
3. bootstrap alone must not try to compensate for missing initialization by guessing.

### 4.6 Runtime Truth During Stage 1

Stage 1 must keep this ownership split explicit:

1. packaged or embedded framework artifacts provide sealed framework input,
2. `.vida/db/**` is the authoritative project-local runtime truth,
3. `.vida/cache/**` is derived only,
4. `.vida/config/**` and `.vida/project/**` are runtime-owned project surfaces,
5. root project files are bridge or projection surfaces, not final runtime authority.

### 4.7 Fail-Closed Rule

During Stage 1 the runtime must fail closed when any of the following is true:

1. required runtime config scaffold is missing and cannot be materialized,
2. required protocol-binding or adjacent required framework payloads are missing,
3. required DB import is missing, invalid, or revision-incompatible,
4. runtime cannot determine the authoritative initialized state safely.

Fail-closed behavior:

1. allow only bounded bootstrap/remediation/query commands,
2. block normal execution commands,
3. render the exact missing prerequisite and the exact bounded remediation path.

### 4.8 Allowed Surfaces Before Readiness

Before Stage-1 readiness is green, the runtime may expose only bounded surfaces such as:

1. `init`
2. bounded installer-management commands,
3. `status`,
4. `doctor`,
5. `vida taskflow protocol-binding sync|status|check`,
6. bounded `vida taskflow consume bundle|bundle check|final|continue|advance` query/progression surfaces when they remain launcher-owned and fail closed,
7. help/recipe surfaces needed to recover to healthy state.

Rule:

1. this allowlist exists so the operator can recover,
2. it must not silently widen into normal runtime execution.

### 4.9 Stage-1 Status Questions

After bootstrap, the runtime must be able to answer at least:

1. is VIDA installed here,
2. is this project initialized,
3. is the `.vida/` runtime home present,
4. is required framework/protocol state imported,
5. which init/bundle posture is active,
6. what remediation is required before work can continue.

These questions must resolve through bounded query/status families rather than ad hoc narration.

### 4.10 Upgrade And Re-Init

`upgrade` and `re-init` remain Stage-1 variants rather than separate product stages.

Upgrade rule:

1. runtime upgrade may refresh the installed payload and installer-management surface,
2. it must then preserve or lawfully migrate project-local `.vida/` state,
3. it must not leave the operator in an ambiguous half-upgraded state.

Re-init rule:

1. re-init is lawful when migration, broken state, or missing scaffold requires re-materialization,
2. re-init must remain bounded and receipt-bearing,
3. re-init must not silently discard authoritative project-local runtime truth.

## 5. Completion Proof For Stage 1

Stage 1 is closed enough when all are true:

1. the operator can install VIDA and obtain one working entry surface,
2. the operator can initialize one project into a healthy `.vida/` runtime home,
3. required machine-readable framework/protocol state imports into DB truth through the init path,
4. bootstrap can tell whether the project is healthy without broad manual repo traversal,
5. non-bootstrap execution refuses to proceed when Stage-1 readiness is missing,
6. the runtime exposes bounded status/doctor/remediation surfaces that explain how to recover.

## 6. Relationship To Other Specs

This model refines and connects:

1. `compiled-autonomous-delivery-runtime-architecture.md`
   - top-level runtime/product direction
2. `release-1-plan.md`
   - active Release-1 execution sequencing
3. `embedded-runtime-and-editable-projection-model.md`
   - embedded-versus-projection runtime split
4. `project-activation-and-configurator-model.md`
   - project-level DB-first activation after Stage 1
5. `runtime-paths-and-derived-cache-model.md`
   - `.vida/` placement and cache boundaries
6. `status-families-and-query-surface-model.md`
   - bounded operator query surfaces
7. `bootstrap-carriers-and-project-activator-model.md`
   - bootstrap-carrier split, orchestrator-init, agent-init, and project-activator routing
8. `docs/product/research/execution-approval-and-interrupt-resume-survey.md`
   - external runtime evidence for Stage-4 pause/resume, approval interruption, and resumable continuation

## 7. Stage 2: Project Activation / Config

### 7.1 Purpose

After Stage 1 has produced a healthy initialized project, the next operator task is to define what runtime posture is active for that project.

Stage 2 owns:

1. project activation state,
2. project-owned runtime entities,
3. explicit versus automatic activation mode,
4. import/export/sync/reconcile of activation surfaces,
5. bounded status and inspection of effective activation posture.

Compact rule:

1. framework law stays sealed,
2. project runtime posture is configured on top of it,
3. DB truth stays authoritative,
4. execution uses only validated active composition.

### 7.2 What The Operator Is Configuring

Release 1 project activation must cover at least:

1. roles,
2. skills,
3. profiles,
4. flow sets,
5. agents,
6. teams,
7. model classes,
8. backend classes,
9. policy surfaces,
10. project protocols.

Operator rule:

1. these are project-owned runtime inputs,
2. they do not replace sealed framework safety or core orchestration law.

### 7.3 Where Activation Lives

The known target placement is:

1. `.vida/config/**`
2. `.vida/project/**`
3. `.vida/db/**`

Interpretation rule:

1. DB truth is authoritative,
2. `.vida/config/**` and `.vida/project/**` are runtime-owned project surfaces,
3. root-tree config and registry files are bridge or projection surfaces only.

### 7.4 Canonical Operator Flow

The currently known lawful Stage-2 flow is:

1. inspect current activation/config status,
2. choose activation mode:
   - `explicit`
   - `automatic`
3. provide or import the required activation entities,
4. validate references and compatibility,
5. activate the selected runtime posture,
6. inspect effective activation status,
7. reconcile/export when projection or Git-backed backup is needed,
8. only then allow later bundle compilation and execution to consume the project posture.

Rule:

1. Stage 2 is not "edit some files and hope runtime picks them up",
2. Stage 2 is "change runtime posture through the configurator lifecycle and DB-first admission path".

### 7.5 Activation Modes

#### 7.5.1 Explicit Mode

In explicit mode the operator declares the active runtime composition directly.

Known operator expectations:

1. choose the enabled roles,
2. choose the enabled skills,
3. choose the active profiles,
4. choose the enabled flow sets,
5. choose teams and adjacent policy,
6. runtime respects that selected set exactly unless validation fails.

#### 7.5.2 Automatic Mode

In automatic mode the runtime may select from the enabled project activation pool.

Known operator expectations:

1. the operator still defines the allowed pool,
2. runtime may choose dynamically only inside that allowed pool,
3. runtime must not auto-enable a project-owned surface that was disabled,
4. automatic mode is bounded selection, not autonomous policy redefinition.

### 7.6 Lifecycle Operations

The known configurator lifecycle is:

1. `import`
2. `activate`
3. `update`
4. `replace`
5. `disable`
6. `restore`

Lifecycle rule:

1. every project-owned activation class must fit this lifecycle coherently,
2. exact per-entity permissions may differ later, but the lifecycle model is already fixed.

### 7.7 Validation And Fail-Closed Behavior

Activation is lawful only when:

1. required entities resolve,
2. ids remain unique where required,
3. roles resolve to lawful framework bases,
4. profiles resolve to known roles,
5. skill attachments are compatible,
6. selected flows resolve,
7. project protocols are distinguished between `known` and `compiled/promoted`,
8. no project-owned surface bypasses sealed framework law.

Fail-closed rule:

1. invalid activation must block later bundle compilation and execution,
2. runtime must not silently drop invalid references and continue with a weaker hidden posture,
3. runtime must surface which activation input is invalid and which bounded remediation step is needed.

### 7.8 Project Protocol Posture

Stage 2 must keep this distinction explicit:

1. `known project protocols`
2. `compiled executable project protocols`

Operator rule:

1. presence alone is not activation,
2. project protocols become executable only after lawful promotion, binding, validation, and compilation,
3. until then they remain visible but non-executable.

### 7.9 Query And Status Surfaces

After Stage 2, the operator must be able to retrieve at least:

1. active roles,
2. active skills,
3. active profiles,
4. active flows,
5. active teams,
6. activation mode,
7. model/backend posture,
8. project protocol registration and promotion state,
9. sync/reconcile posture.

These views belong to bounded `config status` and `sync status` families rather than freeform narrative.

### 7.10 Projection, Sync, And Git

Stage 2 keeps the DB-first sync model explicit:

1. DB changes may project outward,
2. projection edits may be imported back,
3. runtime must detect and reconcile drift explicitly,
4. Git preserves the filesystem projection as history or backup lineage,
5. projected files never outrank DB truth automatically.

### 7.11 Completion Proof For Stage 2

Stage 2 is closed enough when all are true:

1. the operator can inspect project activation posture,
2. explicit mode works,
3. automatic mode works inside a bounded allowed pool,
4. lifecycle operations are coherent,
5. invalid activation wiring fails closed,
6. runtime distinguishes known versus executable project protocols,
7. status and sync surfaces can show the effective activation state.

## 8. Stage 3: Intake / Planning

### 8.1 Purpose

After project activation is healthy, the next operator stage is to turn raw requests, research, and scope discussion into lawful planning state.

Stage 3 owns:

1. intake normalization,
2. bounded scope discussion,
3. bounded PBI discussion,
4. specification and contract formation,
5. planning/scope/spec/task-graph state,
6. lawful handoff from planning into tracked work.

Compact rule:

1. do not route raw conversation directly into execution,
2. normalize intake first,
3. form bounded planning artifacts,
4. hand off to tracked work only after the planning contract is lawful.

### 8.2 Entry Conditions

Stage 3 starts when at least one is true:

1. the operator brings a new request or problem statement,
2. research findings materially affect scope or solution shape,
3. existing scope or task assumptions need clarification,
4. a candidate PBI/task must be formed,
5. planning/scope state must be refreshed after drift.

### 8.3 Conversational Planning Modes

The currently known planning conversation modes are:

1. `scope_discussion`
   - default lane class: `business_analyst`
   - target outcome: bounded scope, clarified constraints, acceptance direction
   - lawful tracked handoff: `spec-pack`
   - required first tracked artifacts: one feature epic, one spec-pack task, one bounded design document
2. `pbi_discussion`
   - default lane class: `pm`
   - target outcome: one bounded task/PBI candidate, delivery cut, ordering, launch readiness
   - lawful tracked handoff: `work-pool-pack`
3. `execution_preparation`
   - default lane class: `solution_architect`
   - target outcome: architecture-preparation report, dependency map, allowed/prohibited change boundaries, reuse guidance, and developer handoff packet
   - lawful tracked handoff: `execution-plan`

Rule:

1. these are bounded conversational stages,
2. they do not silently become execution lanes,
3. they must hand off into canonical tracked planning/task-formation paths when artifact or execution work begins.

### 8.4 Single-Task Planning Rule

Planning conversation remains bounded.

Rules:

1. one active bounded scope or one active candidate task/PBI at a time,
2. additional candidates must be parked or forked explicitly,
3. no silent explosion from one discussion into many tracked tasks,
4. broader planning requires explicit operator broadening rather than inertia.

### 8.5 Canonical Intake Flow

The known Stage-3 flow is:

1. capture the raw request, research findings, or clarification need,
2. determine whether the flow is `scope_discussion` or `pbi_discussion`,
3. normalize the input into a compact intake artifact,
4. determine intake status:
   - `ready_for_scp`
   - `ready_for_icp`
   - `needs_user_negotiation`
   - `needs_spec_delta`
5. if needed, stay in bounded clarification rather than widening scope silently,
6. form or refresh the specification/contract layer,
7. for feature-delivery requests that combine research/specification/planning with code, open one feature epic and one spec-pack task before implementation routing,
8. keep the bounded design artifact canonical through `vida docflow` and close the spec-pack task only after the design artifact is finalized and validated,
9. when the target work is code-shaped or architecture-sensitive, run `execution_preparation` before developer execution begins,
10. materialize planning state, execution-preparation state, and task-graph state through lawful tracked paths,
11. hand off:
   - `research-pack -> spec-pack`
   - `spec-pack -> work-pool-pack`
   - `work-pool-pack -> execution-plan`
12. expose planning/scope state through bounded query surfaces.

### 8.6 Intake Artifact Rule

Stage 3 must not treat raw chat alone as sufficient runtime truth.

The intake layer must capture at least:

1. source request or research signal,
2. clarified assumptions,
3. proposed scope in,
4. proposed scope out,
5. constraints,
6. acceptance direction,
7. uncertainty or open decisions,
8. readiness status for deeper spec/task formation.

Rule:

1. if research findings materially change scope, requirements, routing, or design, the intake/spec layer must be refreshed,
2. planning must not continue on stale intake.

### 8.7 Specification And Planning Outputs

The planning stage must be able to produce or refresh:

1. planning/scope snapshot,
2. specification snapshot,
3. PBI/task candidate,
4. architecture-preparation report,
5. developer handoff packet,
6. task-graph or work-pool formation inputs,
7. explicit distinction between:
   - implemented now
   - planned next
   - blocked / unresolved

### 8.8 Queryability Rule

The operator must be able to retrieve, through bounded query paths:

1. current planning state,
2. current scope state,
3. current specification state,
4. current task-graph state.

Conversational answers about planning must be built on those bounded retrieval paths rather than ad hoc summarization.

### 8.9 Drift And Reconciliation

If approved scope, assumptions, acceptance criteria, dependencies, or task-shape drift materially:

1. planning must not quietly continue on stale state,
2. the runtime must re-baseline through the lawful intake/spec-delta/spec-review path,
3. task-pool or work-pool formation must rebuild when the executable pool is no longer lawful.

### 8.10 Completion Proof For Stage 3

Stage 3 is closed enough when all are true:

1. new scope enters through bounded intake rather than raw execution,
2. `scope_discussion` and `pbi_discussion` remain bounded and lawful,
3. intake can normalize ambiguity before deeper planning,
4. planning/scope/spec/task-graph state is queryable,
5. planning can hand off lawfully into work-pool/execution formation,
6. drift can trigger explicit re-baselining instead of silent continuation.

## 9. Stage 4: Execution / Approval / Interrupt-Resume

### 9.1 Purpose

After planning has produced a lawful tracked scope, the runtime enters execution.

Stage 4 owns:

1. tracked execution progression,
2. lane dispatch and route progression,
3. coach and verification handoffs,
4. human approval gates,
5. interrupt/resume behavior,
6. checkpoint/replay-safe pause and continuation boundaries.

Compact rule:

1. execution proceeds through lawful route and plan state,
2. approval is blocking when required,
3. interrupts and resumes are explicit runtime states, not ad hoc chat pauses,
4. continuation must remain replay-safe and fail-closed.

### 9.2 Entry Conditions

Stage 4 starts only when all are true:

1. Stage 1 readiness is green,
2. project activation is lawful,
3. intake/planning produced a lawful tracked target,
4. required execution-plan and route authorization exist,
5. if the target task class requires execution preparation, a lawful architecture-preparation report and developer handoff packet exist.

### 9.3 Execution Core

The current execution core is governed by:

1. `task_lifecycle`
2. `execution_plan`
3. `route_progression`
4. `coach_lifecycle`
5. `verification_lifecycle`
6. `approval_lifecycle`

Operator interpretation:

1. task state alone does not explain execution,
2. route, review, verification, and approval are separate control layers,
3. closure depends on downstream gates rather than on “work seems finished”.

### 9.4 Canonical Execution Flow

The known Stage-4 flow is:

1. start the tracked execution target,
2. advance through execution-plan steps and route stages,
3. dispatch bounded lanes as required,
4. collect coach and verification results where required,
5. if approval is required, stop at the governance gate,
6. resume only through explicit approval/rejection/manual continuation signals,
7. continue to synthesis/closure only when route, verification, and approval state are all lawful.

### 9.5 Approval Gate

Approval is not advisory.

Current rule:

1. `policy_gate_required`
2. `senior_review_required`
3. `human_gate_required`

are blocking governance states.

Execution rule:

1. closure-ready state is invalid until a matching approval receipt exists,
2. approval is route-bound,
3. stale approval becomes invalid after route drift,
4. rejection is first-class and blocks continuation until rework or escalation resolves it.

### 9.6 Interrupt And Resume

The accepted runtime direction is explicit interrupt/resume semantics.

Current runtime meaning:

1. manual intervention and approval waits are explicit resumable runtime states,
2. resume should target the correct continuation point deterministically,
3. broad-scan resume fallback is not the target lawful path,
4. handles/checkpoints remain runtime-owned rather than root product-law state.

Research grounding rule:

1. Stage-4 pause/resume semantics should stay aligned with `docs/product/research/execution-approval-and-interrupt-resume-survey.md`,
2. approval pauses are runtime waits rather than chat-level conventions,
3. resume must remain bounded, durable, and inspectable.

Practical rule:

1. execution pauses must map back to route, approval, verification, or execution-plan semantics,
2. resumes must be inspectable and bounded,
3. resume must not silently invent a new continuation target.

### 9.7 Between-Task Approval Mode

The optional task-approval loop remains a separate mode.

Current rule:

1. it is inactive by default,
2. when active, it inserts a user approval gate between tasks,
3. it does not disable autonomous execution inside an already approved task,
4. when suspended, execution may continue continuously while preserving internal boundary analysis.

### 9.8 Replay-Safe Continuation

Execution-stage pause/resume must remain compatible with checkpoint and replay law.

Rules:

1. checkpoints are derived resumability artifacts, not canonical state,
2. replay rebuilds derived/runtime surfaces and must not rewrite canonical history,
3. duplicate delivery must be tolerated where delayed checkpoint writes are possible,
4. side effects must remain replay-safe or explicitly bounded.

### 9.9 Queryability Rule

During Stage 4 the operator must be able to query at least:

1. active task execution posture,
2. active route/stage posture,
3. coach/verification state,
4. approval state,
5. blocked or waiting gateway posture,
6. whether execution is waiting for resume/approval/manual intervention.

These answers must come from bounded runtime/query surfaces rather than freeform chat inference.

### 9.10 Completion Proof For Stage 4

Stage 4 is closed enough when all are true:

1. execution is driven by plan and route state rather than ad hoc continuation,
2. approval gates block correctly when required,
3. interrupt/resume semantics are explicit,
4. between-task approval remains optional rather than hidden default behavior,
5. replay/checkpoint direction remains consistent with explicit continuation boundaries,
6. operators can inspect what execution is waiting on before continuation.

## 10. Stage 5: Artifact Materialization

### 10.1 Purpose

After Stage 4 has produced a lawful execution result, the runtime must materialize required user-facing artifacts instead of leaving them only in chat memory.

Stage 5 owns:

1. artifact-output class selection,
2. template or format selection,
3. bounded artifact rendering from runtime truth,
4. filesystem or project-visible materialization,
5. artifact receipts and queryability.

Compact rule:

1. artifacts are rendered from lawful state,
2. a template deepens rendering but does not replace the governing protocol,
3. realizable artifacts must become durable outputs rather than chat-only summaries.

### 10.2 Entry Conditions

Stage 5 starts when at least one is true:

1. the execution route requires a document, packet, report, code patch, or other durable output,
2. approval, review, handoff, or verification requires a materialized artifact,
3. the user explicitly requested a durable artifact rather than a conversational answer.

### 10.3 Canonical Artifact Flow

The lawful Stage-5 flow is:

1. determine the required output class from route, protocol, or user request,
2. gather the bounded runtime truth needed for that output,
3. choose the active template/format:
   - higher-precedence canonical template when it exists,
   - project-owned template when lawfully activated,
   - contextual model rendering only when no stronger template exists,
4. render the artifact,
5. materialize it to the bounded project/runtime surface,
6. emit the matching receipt or status update.

### 10.4 Template And Queryability Rule

Stage 5 must preserve:

1. explicit linkage between artifact class and governing protocol,
2. inspectable source state for the materialized result,
3. visibility of where the artifact was written,
4. the ability to distinguish durable artifacts from chat-only commentary.

### 10.5 Completion Proof For Stage 5

Stage 5 is closed enough when all are true:

1. realizable outputs become durable artifacts,
2. artifact rendering uses lawful protocol/template precedence,
3. artifact status can be queried after materialization,
4. operators can tell which runtime truth produced the artifact.

## 11. Stage 6: Status / Doctor / Remediation

### 11.1 Purpose

After artifacts and execution outputs exist, the operator must be able to inspect bounded runtime posture and recover from non-green states without broad manual digging.

Stage 6 owns:

1. bounded status-family retrieval,
2. bounded doctor diagnostics,
3. explicit remediation routing,
4. renderable recovery guidance grounded in runtime truth.

### 11.2 Required Families

Release 1 Stage 6 must expose at least:

1. `orchestrator status`
2. `doctor`
3. `config status`
4. `flow status`
5. `tasks status`
6. `sync status`
7. `memory status`

### 11.3 Canonical Operator Flow

The lawful Stage-6 flow is:

1. determine which status family the operator actually needs,
2. query the bounded truth surface for that family,
3. render the result,
4. if the result is non-green, route into bounded remediation instructions,
5. re-query after remediation rather than trusting freeform narration.

### 11.4 Doctor And Remediation Rule

`doctor` is the integrity and failure-localization surface for Release 1.

Rules:

1. doctor must name the failing subsystem or readiness class,
2. remediation must point to a bounded next action rather than vague advice,
3. remediation must not depend on a hidden always-on daemon,
4. status families and doctor outputs must remain distinguishable.

### 11.5 Completion Proof For Stage 6

Stage 6 is closed enough when all are true:

1. each required status family has a bounded query path,
2. doctor can localize non-green runtime posture,
3. remediation is explicit and query-grounded,
4. the operator can recover without broad manual repo traversal.

## 12. Stage 7: Export / Edit / Import

### 12.1 Purpose

Release 1 keeps DB truth authoritative, but it must still support lawful human editing through bounded export/import loops.

Stage 7 owns:

1. export of editable projections,
2. bounded manual or tool-driven editing outside DB truth,
3. validation before re-import,
4. import and reconcile back into authoritative runtime state.

### 12.2 Canonical Export / Edit / Import Flow

The lawful Stage-7 flow is:

1. export the bounded editable surface from runtime truth,
2. edit only the exported surface or designated source-mode artifact,
3. validate the edited result against the owning law,
4. import it back into DB truth,
5. run reconcile or drift checks when required,
6. surface updated status after the import result becomes authoritative.

### 12.3 Boundary Rule

Stage 7 must preserve all of:

1. exported files are editable projections rather than primary truth,
2. imports are explicit and receipt-bearing,
3. invalid edits fail closed and must not silently weaken active posture,
4. root-tree bridge files may remain source-mode helpers but must not re-become final runtime truth.

### 12.4 Completion Proof For Stage 7

Stage 7 is closed enough when all are true:

1. a bounded export path exists for editable project/runtime surfaces,
2. edited exports can be validated before import,
3. imports update authoritative DB truth explicitly,
4. drift and reconcile posture remain queryable after import/export activity.

## 13. Stage 8: Closure / Reopen

### 13.1 Purpose

The Release-1 loop needs an explicit ending state and an explicit way back into active work when new evidence or drift appears.

Stage 8 owns:

1. closure readiness,
2. durable closure evidence,
3. bounded reopen triggers,
4. non-silent transition from closed back to actionable state.

### 13.2 Closure Rule

Closure is lawful only when all required downstream gates are green for the scoped target:

1. execution route is complete,
2. required artifacts are materialized,
3. required approval and verification receipts exist,
4. status/doctor posture does not show an unresolved blocking condition,
5. the resulting deliverable can be reported as closed from runtime truth.

### 13.3 Reopen Rule

Reopen is lawful when at least one is true:

1. new blocking evidence appears,
2. the closed result drifts from active canonical state,
3. a required receipt or artifact proves missing or invalid,
4. the user explicitly requests renewed work on the same target.

Reopen behavior:

1. the prior closure evidence must remain preserved,
2. reopen must identify the new active blocker or delta,
3. runtime must not pretend the target was never closed before.

### 13.4 Completion Proof For Stage 8

Stage 8 is closed enough when all are true:

1. closure uses explicit runtime evidence rather than ad hoc chat judgment,
2. operators can see why a target is closed or not yet closed,
3. reopen is explicit, bounded, and receipt-bearing,
4. the runtime can transition from closure back to actionable work without silent state loss.

## 14. Deferred Beyond Release 1

The following remain outside this Release-1 operator loop:

1. UI-native control surfaces,
2. always-on memory freshness daemons,
3. full observability/audit histories beyond bounded status and doctor families,
4. host-project embedding and reactive watcher-driven control.

-----
artifact_path: product/spec/user-facing-runtime-flow-and-operating-loop-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md
created_at: '2026-03-12T21:25:00+02:00'
updated_at: 2026-03-14T12:41:58.831722767Z
changelog_ref: user-facing-runtime-flow-and-operating-loop-model.changelog.jsonl
