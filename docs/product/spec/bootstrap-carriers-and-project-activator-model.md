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

Carrier ownership rule:

1. one shared framework bootstrap law must govern all bootstrap carriers,
2. root `AGENTS.md` is the stronger live bootstrap carrier for the current repository,
3. `system-maps/bootstrap.router-guide` is the synchronized framework-owned bootstrap-router read surface for runtime/help/discovery,
4. packaged/generated bootstrap carriers are delivery surfaces only and must not become parallel owner layers,
5. when those carriers diverge, the drift must be repaired in the same change rather than tolerated as dual ownership.

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
2. determine the bounded bootstrap route,
3. route to `vida orchestrator-init` or `vida agent-init` when those runtime surfaces are available,
4. if project activation is pending, route to `vida project-activator` before ordinary project work,
5. read `AGENTS.sidecar.md` as the project docs map after bootstrap routing establishes or confirms the project path,
6. use bounded shorthand framework ids only when runtime bootstrap surfaces leave an edge case unresolved.

Source-mode bridge rule:

1. until all runtime init commands are implemented everywhere, source-mode bootstrap may still continue through the current canonical map and entry-contract read path,
2. but the target runtime contract remains the command split above.
3. local Rust launcher now implements `vida orchestrator-init` and `vida agent-init`; `project-activator` and some worker-side source-mode flows may still use bounded fallback routing until the remaining activation path is fully runtime-native.

Reference grammar rule:

1. in framework routing prose, a backticked canonical id such as `instruction-contracts/core.orchestration-protocol` means the bounded framework inspection target for `vida protocol view <canonical_id>`,
2. full command-form `vida protocol view <canonical_id>` should remain only in runnable shell examples, operator help, or explicit command snippets,
3. `.md` suffixes must not appear in ordinary framework routing prose.

## 4. Orchestrator Initialization Contract

`vida orchestrator-init` must output the minimum bounded startup package needed by the main orchestrator lane.

Minimum output classes:

1. project identity and current project shape,
2. environment/runtime posture,
3. minimum VIDA command set for safe start,
4. mandatory framework maps and protocols,
5. active orchestrator thinking bootstrap surface and per-step mode-selection rule,
6. runtime-visible reporting contract for `Thinking mode` and counter prefixes,
7. current initialization/readiness state,
8. whether project activation is pending,
9. bounded remediation if readiness is not green.

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
3. the worker thinking subset and worker-safe allowed thinking modes,
4. the runtime-visible reporting contract for worker-safe `Thinking mode` and counter prefixes,
5. the minimum command set needed for `TaskFlow`, `DocFlow`, or adjacent bounded execution,
6. the runtime/environment posture relevant to the assigned lane,
7. explicit refusal to inherit the whole orchestrator bootstrap law.

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

Current packaging rule:

1. until a separate generated root bootstrap carrier exists, the packaged project bootstrap carrier may be produced from the current root `AGENTS.md`,
2. that packaging shortcut does not make the packaged copy a second owner layer,
3. any shared bootstrap-law edit must keep root `AGENTS.md`, `system-maps/bootstrap.router-guide`, and packaged delivery output synchronized.

## 8.1 Minimum Runtime Surface

`vida project-activator` must be a bounded operator-facing command rather than an implied chat ritual.

Minimum rule:

1. the command must exist on the root `vida` surface,
2. it must appear in root help beside `init`, `orchestrator-init`, and `agent-init`,
3. it must expose one bounded current-project activation view rather than silently mutating project state,
4. it may report `pending` or `ready_enough_for_normal_work`, but it must not claim full activation closure without explicit supporting evidence.
5. while it reports `pending`, it must also make the bounded activation algorithm explicit:
   - collect required interview inputs,
   - apply safe defaults,
   - materialize the minimum docs/config/host-template slice,
   - log the activation receipt,
   - tell the operator whether restart is required.

Minimum output classes:

1. current project root/path under evaluation,
2. project shape classification:
   - `empty`
   - `partial`
   - `structured`
3. bootstrap carrier state:
   - `AGENTS.md`
   - `AGENTS.sidecar.md`
   - project docs/runtime roots when relevant
4. activation posture:
   - `pending`
   - `partial`
   - `ready_enough_for_normal_work`
5. missing activation prerequisites or blockers,
6. bounded next steps,
7. whether restart or later host-template initialization is still required.
8. required interview inputs still missing for lawful activation.
9. whether TaskFlow is forbidden while activation remains pending.
10. which documentation/runtime surface is preferred during activation.
11. selected host CLI system.
12. host template materialization mode and runtime root.
13. current carrier catalog owner and routing posture.

JSON rule:

1. the command should expose a machine-readable `--json` view with the same bounded activation summary,
2. plain-text and JSON views must agree on status, blockers, and next steps.
3. the machine-readable view should expose a one-shot example command whenever the remaining activation inputs are small enough to complete in one bounded call.
4. the machine-readable view must identify whether the selected host/carrier system is fully executable, projection-only, or still blocked on later runtime support.

## 9. Project Activator Pipeline

The canonical activator pipeline is:

1. inspect the current project and determine whether it is empty, partial, or already structured,
2. if activation is pending, collect the minimum bounded interview inputs first:
   - project identity,
   - language policy,
   - supported host CLI system,
   - when applicable, the intended carrier/routing posture for the selected host system,
3. record the current project structure into the sidecar/project-doc layer,
4. build or refresh the minimum project documentation map and docs roots using safe defaults where the framework owns the default,
5. inspect the current environment and runtime posture,
6. choose and initialize the current host LLM-tool environment template,
7. log the activation mutation under `.vida/receipts/`,
8. tell the user to restart the tool after agent-environment initialization when required,
9. continue later into richer project/task/roles/skills/profiles/flows slices as those capabilities become available.

Host-carrier rule:

1. the selected host CLI system must come from the configured supported host-system set,
2. host-system identity and carrier metadata must remain config-driven rather than launcher-hardcoded,
3. bootstrap/materialization must not treat one host system as the canonical default owner for all projects once the configured host-system registry is present.

Pipeline staging rule:

1. the first runtime-native `vida project-activator` surface may summarize and route the canonical pipeline in bounded steps without implementing every later interview/config slice in one command,
2. early runtime-native output must still make the pending-vs-ready activation posture explicit and must not pretend the full pipeline is already automated when it is not,
3. however, when the missing slice is only the minimum onboarding set (`project id`, language policy, supported host CLI selection), the activator should support one bounded one-shot command rather than forcing the agent into broad manual file edits.

Activation boundary rule:

1. pending activation is not TaskFlow execution,
2. `vida taskflow` and any non-canonical external TaskFlow runtime are out of scope while activation remains pending,
3. `vida docflow` is the lawful companion runtime family for documentation/readiness inspection during activation,
4. the activator itself is the bounded mutation surface for activation-owned docs/config changes.

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

Canonical host-system registry:

1. supported host systems are owned by `vida.config.yaml -> host_environment.systems`,
2. per-system carrier metadata is owned by `vida.config.yaml -> host_environment.systems.<system>.carriers`,
3. per-system runtime roots, materialization modes, and execution class are owned by the same host-system registry entry,
4. compatibility carrier sources may exist for one host system, but they must not remain the sole canonical runtime-carrier schema.

Operator step:

1. ask which host environment/template is in use from the currently available supported list,
2. initialize that host template in system/runtime state,
3. record which host environment is active.

Current built-in template families:

1. `Codex`
2. `Qwen`
3. `Kilo`
4. `OpenCode`

Framework ownership rule:

1. host CLI selection/materialization is framework-owned and must route through `runtime-instructions/work.host-cli-agent-setup-protocol`,
2. project-local guides such as `docs/process/agent-system.md` or a selected-host tuning guide may tune the selected tool, but they are not the framework owner for choosing or materializing the template.
3. generated docs, readiness checks, and activation reports must be host-neutral or selected-host-specific; they must not require a Codex-branded guide when another host system is selected.

Materialization rule:

1. the activator must materialize the selected host system generically from the configured host-system registry,
2. non-selected host systems must not be treated as missing activation blockers,
3. `copy_tree_only`, rendered-catalog, and future materialization modes are implementation details of the selected host system, not proof that one vendor owns the bootstrap law.

Carrier-routing rule:

1. activation must surface the selected host system together with the configured carrier/routing posture,
2. `agent_system.subagents` and routing posture are part of activation truth and must be visible to the operator,
3. activation must fail closed if the selected host system is materialized but the configured carrier/runtime posture cannot be interpreted lawfully.

Post-init rule:

1. when full host-agent initialization requires a fresh session, the activator must tell the user to exit and re-run the tool.
2. the restart note must be tied to the selected host system rather than treated as Codex-only owner law.
3. for `Codex`, the activator should explicitly say to close and restart Codex after `.codex/**` is materialized so agents become visible in the runtime execution environment.

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
artifact_revision: '2026-04-03'
schema_version: '1'
status: canonical
source_path: docs/product/spec/bootstrap-carriers-and-project-activator-model.md
created_at: '2026-03-12T22:20:00+02:00'
updated_at: 2026-04-03T19:00:00+03:00
changelog_ref: bootstrap-carriers-and-project-activator-model.changelog.jsonl
