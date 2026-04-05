# VIDA Project Codex Agent Configuration Guide

Status: active project process doc

Purpose: describe how the active repository should configure project-local OpenAI Codex multi-agent roles and map them into VIDA project activation surfaces without turning Codex config into framework law.

## Scope

This guide defines only the project-facing Codex agent configuration surface for the active repository:

1. which OpenAI Codex multi-agent schema elements the project should use,
2. where project-local Codex configuration files should live,
3. how Codex role configs should map into VIDA roles, profiles, flows, and teams,
4. which development-team topology the project should target first.

This guide does not define:

1. framework bootstrap routing,
2. framework-owned role or lane law,
3. runtime bundle compilation law,
4. the full product-law meaning of team coordination,
5. framework bootstrap routing.
6. the packet-level team operating protocol, which is owned separately.

## External Baseline

The project configuration in this guide is grounded in the official OpenAI Codex multi-agent schema.

Confirmed schema baseline from the official page:

1. multi-agent is enabled through `multi_agent = true` under `[features]`,
2. role declarations live under `[agents]` in Codex configuration,
3. agent-runtime caps include:
   - `agents.max_threads`
   - `agents.max_depth`
4. each project-defined role lives under `[agents.<name>]`,
5. each role may carry:
   - `description`
   - `config_file`
6. role-specific config files may override at least:
   - `model`
   - `model_reasoning_effort`
   - `sandbox_mode`
   - `developer_instructions`

Official source:

1. `https://developers.openai.com/codex/multi-agent`

## Boundary Rule

1. Codex role configs are executor/runtime settings, not framework protocol owners.
2. Framework-owned bootstrap and safety law remain in `AGENTS.md`, `vida/config/instructions/**`, and root `vida.config.yaml`.
3. Framework-owned selection/materialization of the Codex host template belongs to `runtime-instructions/work.host-cli-agent-setup-protocol`; this project guide starts only after the framework activation slice selected `codex`.
4. Project-owned roles, skills, profiles, flows, and teams remain owned by VIDA project surfaces such as:
   - `.vida/project/agent-extensions/**`
   - `docs/process/agent-extensions/**` as bridge/export surfaces
   - `vida.config.yaml`
   - `docs/product/spec/**`
5. `.codex/**` must not become a second owner of framework or product law.
6. `.codex/**` should only carry Codex runtime configuration that helps the shell execute the already-defined project/runtime posture.

## Canonical Project File Layout

Project-local Codex configuration should live under:

1. `.codex/config.toml`
   - project-local multi-agent root config
2. `.codex/agents/junior.toml`
3. `.codex/agents/middle.toml`
4. `.codex/agents/senior.toml`
5. `.codex/agents/architect.toml`
6. `vida.config.yaml -> host_environment.codex.agents`
   - canonical project-owned source of truth for tier metadata, rates, runtime-role fit, and task-class fit

Layout rule:

1. the active root Codex session is the orchestrator and must remain outside the delegated agent list,
2. `vida.config.yaml -> host_environment.codex.agents` owns carrier-tier/rate/runtime-role/task-class metadata,
3. `vida.config.yaml -> agent_extensions.registries.dispatch_aliases` is the canonical internal alias registry for executor-local overlays and is not the primary project-visible agent model,
4. `.codex/config.toml` is the rendered delegated carrier-tier registration surface, including thread/depth caps and per-role config-file mapping,
5. `.codex/agents/*.toml` are rendered host-executor surfaces and must not become the owner of tier or dispatch-alias policy,
6. project activation should render `.codex/**` from the overlay catalog while preserving the framework-owned tier instruction bodies from the template source,
7. project-visible agent activation should target the carrier tiers `junior`, `middle`, `senior`, and `architect`; runtime role selection is carried separately in packet/runtime state instead of replacing the carrier identity,
8. VIDA role/skill/profile/team meaning still comes from the project activation layer, not from Codex TOML alone.
9. Role/profile/flow catalogs should be sourced from the agent-extension YAML registries; `vida.config.yaml` may narrow them, but runtime should not require duplicated id lists when the registries already define the active set.
10. the root session is a bootstrap and coordination owner, not a separate long-lived local implementer role.

## Development Team Target

The first project Codex team should be a bounded development team aligned with the current Release-1 direction.

Flow posture:

1. the primary development posture is `fast with verification`,
2. bounded research and analysis may still use agent lanes,
3. bounded research, specification, planning, implementation, review, and verification work should all be eligible for the Codex development team once a lawful packet exists.

Minimum tier topology:

1. root Codex session
   - manager-led orchestrator that completes lawful bootstrap, decomposes work, delegates lanes, and owns closure routing
2. `junior`
   - rate `1`
   - low-cost bounded implementation lane that owns one explicit write packet and returns concrete delivery evidence
3. `middle`
   - rate `4`
   - specification/planning lane, explicit spec-conformance coach lane, and medium implementation lane while the packet still fits one bounded closure cycle
4. `senior`
   - rate `16`
   - independent verification and high-confidence proof lane
5. `architect`
   - rate `32`
   - architecture-preparation and hard-escalation lane for conflicts the normal delivery cycle cannot close

Internal dispatch aliases:

1. canonical `dispatch_aliases` should live in the registry path declared by `vida.config.yaml -> agent_extensions.registries.dispatch_aliases`,
2. it is not the primary visible agent model of the project,
3. the primary visible agent model is the carrier ladder `junior -> middle -> senior -> architect`,
4. runtime role is activation-time state such as `worker`, `coach`, `verifier`, or `solution_architect`.

Ownership note:

1. optional named aliases are not Rust-owned catalogs,
2. they should be treated as internal dispatch projections from the configured dispatch-alias registry, not as the operational team model,
3. carrier tiers remain the primary activated agent ids; alias ids, runtime-role coverage, task-class coverage, and overlay instruction bodies should be changed in overlay/template owner state and then re-materialized through activation.

Packet posture:

1. delegated Codex roles must consume one bounded `delivery_task` or one bounded `execution_block` packet,
2. `.codex/**` should be tuned for packet execution, not for epic- or feature-shaped delegation,
3. packet semantics are owned by `docs/process/team-development-and-orchestration-protocol.md`,
4. the default decomposition leaf is `delivery_task`,
5. `execution_block` is reserved for packets that still fail one-owner bounded closure,
6. normal write-producing work should be delegated once a lawful packet exists,
7. available skills must be inspected and relevant skills activated before bounded work begins,
8. packet rendering should follow the project packet-template protocol rather than free-form delegation text,
9. packet interpretation should follow the project prompt-stack protocol so role prompt, packet, skill, and runtime-state precedence stay explicit.

Coordination pattern:

1. default posture is `manager-led delegation-first` by the active root Codex session,
2. `junior`, `middle`, and `senior` are the normal delegated tiers for eligible work,
3. the root session should stay in orchestrator scope after bootstrap rather than collapsing into a second local implementer,
4. runtime role law still distinguishes `worker`, `coach`, `verifier`, and `solution_architect`; Codex tiers are execution carriers, not replacements for those framework roles,
5. runtime should activate the chosen carrier tier and pass the lawful `runtime_role` explicitly instead of presenting alias ids as the primary project role model,
6. `architect` is not part of the normal steady-state path and should activate only when the first-line tiers cannot close lawfully.
7. a user request to continue development does not reassign the root session into `junior`.

Top-level orchestrator note:

1. if the project wants a cheaper but logical root orchestrator, the upper-lane operating contract is owned by `docs/process/project-orchestrator-operating-protocol.md`,
2. `.codex/**` should stay aligned to that upper-lane protocol rather than compensating for weak top-level routing inside agent-specific TOML.

Normalization rule:

1. `orchestrator-only` is lawful only for bounded bootstrap, direct chat diagnosis, or recorded saturation/exception handling,
2. normal project development posture is agentic: orchestrator-led, delegation-first, and verification-backed,
3. canonical delegated execution still routes through `vida agent-init`; host-tool-specific Codex subagent APIs are executor details and are not the primary project delegation surface,
4. before any local write decision, re-check `vida status --json`, `vida taskflow recovery latest --json`, and `vida taskflow consume continue --json`; an active root-session write guard still means orchestration-only,
5. if delegation temporarily fails because of thread or lane saturation, attempt lawful reuse or recorded saturation recovery before accepting local-only continuation as the active posture.
6. generic implementation intent is not a lane-change receipt and must not by itself authorize root-session coding.
7. finding the patch location or reproducing a runtime defect is still read-only packet shaping evidence, not permission for root-session completion of the same write scope.
8. recorded saturation recovery must explicitly check whether any delegated Codex lanes already completed or were superseded and can now be closed/reclaimed before "agent limits" remains a valid blocker.
9. worker wait timeout or empty poll result does not authorize replacing the packet cycle with one generic internal development lane.
10. under continued-development intent, stay in commentary/progress mode and continue routing; do not emit final closure wording while a next lawful continuation item is already known.
11. do not treat commentary, an intermediate status update, or “I have explained the result” as a lawful pause boundary.
12. if closure-style wording is emitted by mistake, immediately re-enter commentary mode and bind the next lawful continuation item without waiting for more user input.
13. when recording task progress from shell, prefer `vida task update <task-id> --notes-file <path> --json` over inline shell quoting for complex text.

Coach separation rule:

1. the active repository already treats `coach` as a first-class framework role,
2. `coach` must not collapse into `worker`, `verifier`, or `approver`,
3. `coach` is the formative packet-local gate for implemented-result vs approved-spec conformance,
4. `coach` feedback may request rework or raise bounded quality concerns tied to acceptance criteria and `definition_of_done`,
5. `coach` does not replace independent verification.

## Model, Tier, And Pricing Policy

Current project decision for Codex development agents:

1. use the selected `GPT-5.4` family with a four-level reasoning ladder,
2. use four Codex execution tiers:
   - `junior` -> reasoning `low` -> rate `1`
   - `middle` -> reasoning `medium` -> rate `4`
   - `senior` -> reasoning `high` -> rate `16`
   - `architect` -> VIDA reasoning band `xhigh` mapped onto Codex `high` reasoning effort -> rate `32`
3. do not use the highest tier as the normal default,
4. choose the cheapest tier that satisfies:
   - the required task-class minimum,
   - the local score guard from `.vida/state/worker-strategy.json`,
   - the lane/packet role boundary,
5. use local scorecards and strategy state to refresh effective tier score dynamically:
   - `.vida/state/worker-scorecards.json`
   - `.vida/state/worker-strategy.json`
6. record post-task feedback through:
   - `vida agent-feedback --agent-id <tier> --score <0-100> --task-class <task_class> [--outcome <success|failure|neutral>] [--notes "..."]`
7. use the local host-agent observability ledger for automatic feedback history and budget rollup:
   - `.vida/state/host-agent-observability.json`
8. use `vida status --json` as the bounded operator surface for current tier state, recent host-agent events, and total estimated budget units recorded so far,
9. prefer `vida taskflow task close ...` over ad hoc task finalization when the task belongs to the tracked Codex execution path, because close-time telemetry now refreshes the same score/observability loop automatically.

Policy note:

1. this is project policy, not a statement of framework law,
2. if the exact deployable Codex model identifiers differ from the project shorthand, keep the same tier policy and map it to the nearest supported Codex model ids during implementation.

Vendor-basis rule:

1. OpenAI Codex guidance supports explicit multi-agent config, per-agent reasoning tuning, and project-local structured configuration rather than implicit chat heuristics.
2. Anthropic guidance supports structured prompt templates, explicit variable fields, and evaluation-backed iteration rather than free-form prompt drift.
3. Microsoft guidance supports explicit architecture/design artifacts and cost-quality tradeoff recording instead of ad hoc escalation.

## Recommended Codex Runtime Caps

For the first bounded development team, prefer:

1. `agents.max_threads = 4`
2. `agents.max_depth = 2`

Reasoning:

1. one root orchestrator session may need to keep `junior`, `middle`, `senior`, and `architect` tiers available without overexpanding the shell,
2. depth `2` permits bounded escalation without turning nested spawning into an unbounded tree.

## Mapping Into VIDA

Codex role configuration should map into VIDA project activation like this:

1. VIDA project roles define semantic job ownership,
2. VIDA profiles bind those roles to skills and preferred backend/model posture,
3. VIDA flows define lawful role chains,
4. VIDA teams define the coordinated runtime composition,
5. Codex role configs provide the concrete execution settings used when a Codex-backed lane is selected.

For the active repository, the target mapping is:

1. project extension registries:
   - `.vida/project/agent-extensions/roles.yaml`
   - `.vida/project/agent-extensions/skills.yaml`
   - `.vida/project/agent-extensions/profiles.yaml`
   - `.vida/project/agent-extensions/flows.yaml`
   - matching `.vida/project/agent-extensions/*.sidecar.yaml`
   - root `docs/process/agent-extensions/**` remains bridge/export lineage
2. root overlay activation:
   - `vida.config.yaml`
3. compiled runtime bundle surface:
   - `docs/product/spec/compiled-runtime-bundle-contract.md`

Mapping rule:

1. Codex tiers should be introduced only where the project activation layer already knows how to admit the corresponding VIDA role/profile/team posture,
2. Codex TOML must not be used as a bypass around VIDA validation and activation,
3. documentation-only work should stay outside the Codex development team unless a future project rule explicitly promotes it into an agent-backed path.

## Implementation Rule

The implementation order for Codex agents should be:

1. define the development-team posture in project docs and project activation surfaces,
2. add project-local `.codex/config.toml`,
3. add tier-specific `.codex/agents/*.toml`,
4. wire the same roles/profiles/teams into VIDA project activation,
5. expose them through compiled runtime bundles only after validation passes.

## Current Status

At the current repository cut:

1. project roles, skills, profiles, and flow sets already have active registry surfaces,
2. team semantics already exist as product law,
3. project-local Codex multi-agent configuration is materialized under `.codex/config.toml` and `.codex/agents/*.toml`,
4. the first intended Codex-backed project team is the bounded four-tier ladder defined in this guide.

## Routing

1. for project role/skill/profile/flow semantics, read `docs/product/spec/agent-role-skill-profile-flow-model.md`,
2. for project activation and DB-first configurator behavior, read `docs/product/spec/project-activation-and-configurator-model.md`,
3. for team runtime semantics, read `docs/product/spec/team-coordination-model.md`,
4. for compiled runtime bundle expectations, read `docs/product/spec/compiled-runtime-bundle-contract.md`,
5. for framework/runtime validation of project extensions, read `runtime-instructions/work.project-agent-extension-protocol.md`,
6. for canonical coach/verifier separation, read `runtime-instructions/work.verification-lane-protocol.md`,
7. for the project packet-level team operating protocol, read `docs/process/team-development-and-orchestration-protocol.md`,
8. for the project top-level orchestrator operating protocol, read `docs/process/project-orchestrator-operating-protocol.md`,
9. for routine orchestrator startup and Codex-backed session launch, read `docs/process/project-orchestrator-startup-bundle.md` first,
10. expand to `docs/process/project-orchestrator-session-start-protocol.md` or `docs/process/project-orchestrator-reusable-prompt.md` only when the startup bundle does not settle the question,
11. for startup readiness, skill gating, packet rendering, or packet/lane defaults beyond the bundle, expand only the needed compact project runtime capsules:
   - `docs/process/project-start-readiness-runtime-capsule.md`
   - `docs/process/project-packet-rendering-runtime-capsule.md`
   - `docs/process/project-packet-and-lane-runtime-capsule.md`
12. open deeper owner docs for skill initialization, packet templates, prompt-stack law, or boot validation only when those compact project surfaces still leave an edge case unresolved.

## References

1. OpenAI Codex multi-agent:
   - `https://developers.openai.com/codex/multi-agent`
2. Anthropic prompt templates and variables:
   - `https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/prompt-templates-and-variables`
3. Anthropic prompt engineering overview:
   - `https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/overview`
4. Microsoft architecture decision records:
   - `https://learn.microsoft.com/en-us/azure/well-architected/architect-role/architecture-decision-record`

-----
artifact_path: process/codex-agent-configuration-guide
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-14'
schema_version: '1'
status: canonical
source_path: docs/process/codex-agent-configuration-guide.md
created_at: '2026-03-12T08:35:27+02:00'
updated_at: 2026-04-05T06:19:10.134108261Z
changelog_ref: codex-agent-configuration-guide.changelog.jsonl
