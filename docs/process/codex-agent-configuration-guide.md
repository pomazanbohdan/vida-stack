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
5. project documentation execution surfaces that remain non-agentic.
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
3. Project-owned roles, skills, profiles, flows, and teams remain owned by VIDA project surfaces such as:
   - `docs/process/agent-extensions/**`
   - `vida.config.yaml`
   - `docs/product/spec/**`
4. `.codex/**` must not become a second owner of framework or product law.
5. `.codex/**` should only carry Codex runtime configuration that helps the shell execute the already-defined project/runtime posture.

## Canonical Project File Layout

Project-local Codex configuration should live under:

1. `.codex/config.toml`
   - project-local multi-agent root config
2. `.codex/agents/development-implementer.toml`
3. `.codex/agents/development-coach.toml`
4. `.codex/agents/development-verifier.toml`
5. `.codex/agents/development-escalation.toml`

Layout rule:

1. the active root Codex session is the orchestrator and must remain outside the delegated agent list,
2. `.codex/config.toml` owns delegated lane registration, thread/depth caps, and per-role config-file mapping,
3. `.codex/agents/*.toml` own role-specific Codex execution posture only,
4. VIDA role/skill/profile/team meaning still comes from the project activation layer, not from Codex TOML alone.
5. the root session is a bootstrap and coordination owner, not a separate long-lived local implementer role.

## Development Team Target

The first project Codex team should be a bounded development team aligned with the current Release-1 direction.

Flow posture:

1. the primary development posture is `fast with verification`,
2. bounded research and analysis may still use agent lanes,
3. project documentation work remains non-agentic and must not be forced through the Codex development team.

Minimum team topology:

1. root Codex session
   - manager-led orchestrator that completes lawful bootstrap, decomposes work, delegates lanes, and owns closure routing
2. `development_implementer`
   - execution-focused role for writing code and making bounded implementation changes
3. `development_coach`
   - formative-review role for bounded critique, rework signals, and quality guidance before independent verification
4. `development_verifier`
   - independent verification role for defect finding, proof checking, and closure readiness
5. `development_escalation`
   - high-cost escalation role for hard architecture, conflict, or blocked situations only

Packet posture:

1. delegated Codex roles must consume one bounded `delivery_task` or one bounded `execution_block` packet,
2. `.codex/**` should be tuned for packet execution, not for epic- or feature-shaped delegation,
3. packet semantics are owned by `docs/process/team-development-and-orchestration-protocol.md`,
4. the default decomposition leaf is `delivery_task`,
5. `execution_block` is reserved for packets that still fail one-owner bounded closure,
6. normal write-producing work should be delegated once a lawful packet exists,
7. available skills must be inspected and relevant skills activated before bounded work begins.

Coordination pattern:

1. default posture is `manager-led delegation-first` by the active root Codex session,
2. implementer, coach, and verifier are the normal delegated lanes for eligible development work,
3. the root session should stay in orchestrator scope after bootstrap rather than collapsing into a second local implementer,
4. `coach` must remain distinct from `verifier`,
5. escalation is not part of the normal steady-state path and should activate only when the first-line development team cannot close lawfully.

Top-level orchestrator note:

1. if the project wants a cheaper but logical root orchestrator, the upper-lane operating contract is owned by `docs/process/project-orchestrator-operating-protocol.md`,
2. `.codex/**` should stay aligned to that upper-lane protocol rather than compensating for weak top-level routing inside agent-specific TOML.

Normalization rule:

1. `orchestrator-only` is lawful only for bounded bootstrap, direct chat diagnosis, or recorded saturation/exception handling,
2. normal project development posture is agentic: orchestrator-led, delegation-first, and verification-backed,
3. if delegation temporarily fails because of thread or lane saturation, attempt lawful reuse or recorded saturation recovery before accepting local-only continuation as the active posture.

Coach separation rule:

1. the active repository already treats `coach` as a first-class framework role,
2. `coach` must not collapse into `worker`, `verifier`, or `approver`,
3. `coach` feedback may request rework or raise bounded quality concerns,
4. `coach` does not replace independent verification.

## Model And Reasoning Policy

Current project decision for Codex development agents:

1. use the selected `GPT-5.4` family with a four-level reasoning ladder,
2. do not use the highest reasoning tier as the normal default,
3. code writing runs at low reasoning,
4. `coach` runs at medium reasoning,
5. independent verification runs at low reasoning,
6. the active root orchestrator may run at medium reasoning,
7. highest-tier reasoning is reserved for escalation only.

Policy note:

1. this is project policy, not a statement of framework law,
2. if the exact deployable Codex model identifiers differ from the project shorthand, keep the same tier policy and map it to the nearest supported Codex model ids during implementation.

## Recommended Codex Runtime Caps

For the first bounded development team, prefer:

1. `agents.max_threads = 4`
2. `agents.max_depth = 2`

Reasoning:

1. one root orchestrator session may need to keep implementer, coach, verifier, and escalation lanes available without overexpanding the shell,
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
   - `docs/process/agent-extensions/roles.yaml`
   - `docs/process/agent-extensions/skills.yaml`
   - `docs/process/agent-extensions/profiles.yaml`
   - `docs/process/agent-extensions/flows.yaml`
   - future/target: `docs/process/agent-extensions/teams.yaml`
2. root overlay activation:
   - `vida.config.yaml`
3. compiled runtime bundle surface:
   - `docs/product/spec/compiled-runtime-bundle-contract.md`

Mapping rule:

1. Codex roles should be introduced only where the project activation layer already knows how to admit the corresponding VIDA role/profile/team posture,
2. Codex TOML must not be used as a bypass around VIDA validation and activation,
3. documentation-only work should stay outside the Codex development team unless a future project rule explicitly promotes it into an agent-backed path.

## Implementation Rule

The implementation order for Codex agents should be:

1. define the development-team posture in project docs and project activation surfaces,
2. add project-local `.codex/config.toml`,
3. add role-specific `.codex/agents/*.toml`,
4. wire the same roles/profiles/teams into VIDA project activation,
5. expose them through compiled runtime bundles only after validation passes.

## Current Status

At the current repository cut:

1. project roles, skills, profiles, and flow sets already have active registry surfaces,
2. team semantics already exist as product law,
3. project-local Codex multi-agent configuration is materialized under `.codex/config.toml` and `.codex/agents/*.toml`,
4. the first intended Codex-backed project team is the bounded development team defined in this guide.

## Routing

1. for project role/skill/profile/flow semantics, read `docs/product/spec/agent-role-skill-profile-flow-model.md`,
2. for project activation and DB-first configurator behavior, read `docs/product/spec/project-activation-and-configurator-model.md`,
3. for team runtime semantics, read `docs/product/spec/team-coordination-model.md`,
4. for compiled runtime bundle expectations, read `docs/product/spec/compiled-runtime-bundle-contract.md`,
5. for framework/runtime validation of project extensions, read `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`,
6. for canonical coach/verifier separation, read `vida/config/instructions/runtime-instructions/work.verification-lane-protocol.md`,
7. for the project packet-level team operating protocol, read `docs/process/team-development-and-orchestration-protocol.md`,
8. for the project top-level orchestrator operating protocol, read `docs/process/project-orchestrator-operating-protocol.md`,
9. for repeatable orchestrator startup and reusable prompt wording, read:
   - `docs/process/project-orchestrator-session-start-protocol.md`
   - `docs/process/project-orchestrator-reusable-prompt.md`
10. for mandatory skill initialization and activation, read `docs/process/project-skill-initialization-and-activation-protocol.md`.

-----
artifact_path: process/codex-agent-configuration-guide
artifact_type: process_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/process/codex-agent-configuration-guide.md
created_at: '2026-03-12T08:35:27+02:00'
updated_at: '2026-03-13T19:11:00+02:00'
changelog_ref: codex-agent-configuration-guide.changelog.jsonl
