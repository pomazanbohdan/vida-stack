# Bootstrap Carriers And Project Activator Model

Status: active product law

Purpose: define the canonical split between bootstrap carriers, orchestrator-versus-agent runtime initialization, and the staged `project activator` flow that prepares one project for lawful VIDA execution without forcing the human operator to wire the system manually.

## 1. Core Requirement

`AGENTS.md` is read by all lanes at initialization time.

Therefore it must support two different runtime-entry outcomes:

1. orchestrator initialization,
2. agent initialization.

At the same time it must preserve the bootstrap-carrier split:

1. `AGENTS.md`
   - framework bootstrap carrier
2. `AGENTS.sidecar.md`
   - project docs bootstrap carrier

Rule:

1. `AGENTS.md` must route,
2. `AGENTS.sidecar.md` must orient project docs,
3. deeper project onboarding and activation must not be hidden inside ad hoc chat behavior.

## 2. Runtime Initialization Split

The runtime target model has three bounded initialization surfaces:

1. `vida orchestrator-init`
2. `vida agent-init`
3. `vida project-activator`

Interpretation rule:

1. `orchestrator-init` is for the main orchestrator lane,
2. `agent-init` is for non-orchestrator lanes receiving bounded work,
3. `project-activator` is a project onboarding/configuration path rather than a normal execution lane.

## 3. `AGENTS.md` Routing Rule

`AGENTS.md` must expose two explicit bootstrap flows:

1. orchestrator flow
2. agent flow

The file must remain bootstrap-oriented and must not absorb the full downstream playbooks.

Minimum routing content in `AGENTS.md`:

1. read `AGENTS.md`,
2. read `AGENTS.sidecar.md`,
3. read `vida/root-map.md`,
4. resolve lane,
5. route to `vida orchestrator-init` or `vida agent-init` when those runtime surfaces are available,
6. if project activation is pending, route to `vida project-activator` before ordinary project work.

Source-mode bridge rule:

1. until all runtime init commands are implemented everywhere, source-mode bootstrap may still continue through the current canonical map and entry-contract read path,
2. but the target runtime contract remains the command split above.

## 4. Orchestrator Initialization Contract

`vida orchestrator-init` must output the minimum bounded startup package needed by the main orchestrator lane.

Minimum output classes:

1. project identity and current project shape,
2. environment/runtime posture,
3. minimum VIDA command set for safe start,
4. mandatory framework maps and protocols,
5. current initialization/readiness state,
6. whether project activation is pending,
7. bounded remediation if readiness is not green.

Minimum command-oriented content:

1. the current project/runtime snapshot,
2. the minimum commands needed to inspect and continue safely,
3. the required maps/protocols the orchestrator must treat as active startup inputs,
4. explicit handoff to `vida project-activator` when onboarding is incomplete.

Rule:

1. orchestrator init should reduce broad repo scanning,
2. it should not dump the whole canon blindly,
3. it must still expose the mandatory startup law.

## 5. Agent Initialization Contract

`vida agent-init` must output the bounded startup package for a non-orchestrator agent lane.

It must include:

1. confirmation that the lane is an agent/worker lane,
2. the bounded task goal or prompt from the orchestrator,
3. only the protocol subset needed for that lane,
4. the minimum command set needed for `TaskFlow`, `DocFlow`, or adjacent bounded execution,
5. the runtime/environment posture relevant to the assigned lane,
6. explicit refusal to inherit the whole orchestrator bootstrap law.

Agent-load rule:

1. agent lanes must not load full orchestrator coordination law by default,
2. they should receive worker-lane contracts, worker-thinking subset, and only the runtime/domain surfaces needed for the assigned work,
3. if documentation-shaped, the bounded `DocFlow` activation subset must be included,
4. if tracked execution/task work is active, the bounded `TaskFlow` execution subset must be included.

## 6. Protocol Load Split

### 6.1 Orchestrator-Loaded Startup Set

The orchestrator startup set should include:

1. framework bootstrap carriers,
2. framework root-map surfaces,
3. project docs map surfaces,
4. orchestrator entry contract,
5. always-on thinking and continuity protocols,
6. activation-routing law,
7. bounded runtime-init and readiness/status surfaces,
8. `project-activator` trigger status when project onboarding is incomplete.

### 6.2 Agent-Loaded Startup Set

The agent startup set should include only:

1. worker entry contract,
2. worker thinking subset,
3. bounded runtime-family surface needed by the lane,
4. the task/goal packet from the orchestrator,
5. the minimum relevant maps/protocols for the active domain,
6. the minimum command set to execute and report back lawfully.

Forbidden pattern:

1. an agent lane loading all orchestrator-only protocols merely because it also read `AGENTS.md` first.

## 7. Project Activator Purpose

`vida project-activator` exists to turn one project from "runtime installed" into "runtime actually configured for useful work".

It is not the same as:

1. package install,
2. lane bootstrap,
3. task execution.

It owns staged project onboarding and activation preparation.

## 8. Project Activator Trigger

`vida project-activator` should run when at least one is true:

1. the project has not completed initial VIDA onboarding,
2. project activation/config state is materially incomplete,
3. the sidecar/project-doc structure is missing or too thin for lawful routing,
4. runtime cannot determine the project execution posture safely.

Bootstrap-carrier rule:

1. during this pending state, `AGENTS.md` may carry an explicit instruction to run `vida project-activator`,
2. once activation is complete, that temporary instruction should be removed from the generated project bootstrap carrier so it does not remain as stale onboarding noise.

## 9. Project Activator Pipeline

The canonical activator pipeline is:

1. inspect the current project and determine whether it is empty, partial, or already structured,
2. record the current project structure into the sidecar/project-doc layer,
3. build or refresh the project documentation map when enough structure exists,
4. inspect the current environment and runtime posture,
5. determine which runtime/development environment is already configured,
6. create the high-value bootstrap/project documents required by protocol,
7. ask the user for current and upcoming task context,
8. build the import-ready project/task initialization payload and import it into VIDA,
9. walk through core project configuration settings,
10. walk through automation and external-agent settings,
11. choose and initialize the current host LLM-tool environment template,
12. tell the user to restart the tool after agent-environment initialization when required,
13. continue later into roles/skills/profiles/flows and methodology shaping as those slices become available.

## 10. Sidecar And Project-Map Work

The activator must:

1. inspect the current repository/project structure,
2. persist project-structure understanding into `AGENTS.sidecar.md`,
3. create or refresh the project documentation map when enough project docs exist,
4. make that sidecar/project-map path an explicit required read path for future project work.

Rule:

1. project structure must not remain only as transient chat understanding,
2. it must become durable project-doc routing state.

## 11. Environment And Runtime Survey

The activator must determine:

1. what runtime/development environment already exists,
2. what is already configured,
3. what is missing,
4. which runtime family is expected for the project,
5. which bootstrap documents need to be created to make the environment understandable.

Examples of high-value documents include:

1. environment/setup notes,
2. runtime environment summary,
3. project structure map,
4. onboarding checklists,
5. import/bootstrap receipts visible to later operators.

## 12. Task And Roadmap Intake

The activator must ask the operator whether there is current known information about:

1. completed tasks,
2. active tasks,
3. upcoming tasks,
4. known epics, PBIs, or project scope.

That information should then:

1. be shaped into the canonical import payload,
2. be imported into VIDA,
3. become queryable runtime truth rather than remaining only as chat memory.

## 13. Core Project Configuration Walkthrough

The activator must guide the operator through the core project configuration, including:

1. automation posture,
2. development posture,
3. model/backend posture,
4. external agent inventory,
5. known integration/runtime constraints.

Rule:

1. these settings should become explicit project activation/config state,
2. not remain implied or scattered.

## 14. Host LLM Environment Template

The activator must support initialization of the current host LLM-tool environment from available templates.

Operator step:

1. ask which host environment/template is in use from the currently available supported list,
2. initialize that host template in system/runtime state,
3. record which host environment is active.

Current known template:

1. `Codex`

Post-init rule:

1. when full host-agent initialization requires a fresh session, the activator must tell the user to exit and re-run the tool.

## 15. Future Activation Interview

After base onboarding is complete, later activator slices should ask the user what working posture the project needs, for example:

1. pure development,
2. review-heavy work,
3. documentation-first work,
4. code-vs-spec conformance checking,
5. preferred development frameworks,
6. development methodologies,
7. expected review/verification posture.

These answers should later activate:

1. role posture,
2. skill posture,
3. profile posture,
4. flow posture,
5. methodology-specific overlays,
6. review/documentation ownership expectations.

## 16. Empty-Project Rule

If the project is effectively empty, the activator must not wait for non-existent source structure.

Instead it should:

1. scaffold the minimum project-routing/bootstrap surfaces,
2. initialize the project docs/sidecar structure,
3. create the minimum onboarding/config/environment documents,
4. ask the operator for the intended project shape and working posture,
5. seed VIDA with that starting state.

## 17. Completion Proof

This model is closed enough when:

1. `AGENTS.md` can route both orchestrator and agent startup paths,
2. the runtime has distinct `orchestrator-init` and `agent-init` surfaces,
3. project-activation pending state can route into `project-activator`,
4. the activator can enrich `AGENTS.sidecar.md` and project-doc routing,
5. the activator can build and import the initial project/task payload,
6. the host environment template can be selected and initialized,
7. later lanes no longer depend on transient onboarding chat alone.

-----
artifact_path: product/spec/bootstrap-carriers-and-project-activator-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/bootstrap-carriers-and-project-activator-model.md
created_at: '2026-03-12T22:20:00+02:00'
updated_at: '2026-03-12T22:20:00+02:00'
changelog_ref: bootstrap-carriers-and-project-activator-model.changelog.jsonl
