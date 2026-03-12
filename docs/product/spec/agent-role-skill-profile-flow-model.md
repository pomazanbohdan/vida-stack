# VIDA Agent Role, Skill, Profile, And Flow Model

Status: active product law

Purpose: define the canonical distinction between `role`, `skill`, `profile`, and `flow set`, and define how projects may extend them dynamically through `vida.config.yaml` without redefining framework runtime law.

## 1. Canonical Runtime Objects

VIDA recognizes four different composition objects for runtime agent behavior:

1. `role`
2. `skill`
3. `profile`
4. `flow set`

These objects are related, but they are not interchangeable.

## 2. Role

`role` answers: who is this runtime lane in the system?

It owns:

1. mission and responsibility boundary,
2. allowed actions,
3. forbidden actions,
4. proof and receipt obligations,
5. authority boundary,
6. relation to handoff, review, verification, approval, and closure.

Current first-class framework roles include:

1. `orchestrator`
2. `worker`
3. `business_analyst`
4. `pm`
5. `solution_architect`
6. `coach`
7. `verifier`
8. `prover`

Role note:

1. `business_analyst` and `pm` are first-class framework roles for scope formation, requirement shaping, task formation, and delivery framing before execution lanes begin.
2. `solution_architect` is the first-class pre-execution architecture-preparation role:
   - reads the bounded task or PBI,
   - studies governing specs and active project/runtime constraints,
   - inspects the relevant codebase and dependency surface,
   - produces one architecture-preparation report plus developer handoff packet,
   - defines what can be changed, what must not be changed, reuse points, dependency impact, and expected implementation boundaries before developer execution begins.
3. `solution_architect` does not replace `business_analyst`, `pm`, `coach`, `verifier`, or the developer/worker lane:
   - `business_analyst` shapes scope and requirements,
   - `pm` shapes delivery/task cut and launch readiness,
   - `solution_architect` prepares implementation architecture and constraints,
   - `worker` executes implementation,
   - `coach` and `verifier` remain downstream quality gates.
4. `coach` must remain a separate role and must not collapse into `worker`, `verifier`, or `approver`.

## 3. Skill

`skill` answers: what bounded capability payload may be attached?

It owns:

1. focused capability payload,
2. attach/select semantics,
3. compatible roles,
4. enablement policy.

Rules:

1. a skill is not a role,
2. a skill must not silently grant stronger authority than the resolved base role already allows,
3. a skill may deepen competence, but it must not rewrite runtime safety law.

## 4. Profile

`profile` answers: how is one role configured for one bounded use posture?

It owns:

1. resolved role reference,
2. attached skill set,
3. stable stance or operating posture,
4. optional runtime/provider rendering preference,
5. project-specific composition metadata.

Rules:

1. a profile is not a replacement for role law,
2. a profile may compose skills and stance, but it must not hide approval authority, escalation law, or fallback law.

## 5. Flow Set

`flow set` answers: how do lanes interact for one class of work?

It owns:

1. role-chain or lane-chain shape,
2. standard or custom flow identity,
3. bounded gate expectations,
4. route-level composition posture.

Rules:

1. a flow set is a runtime composition artifact, not a fifth instruction-artifact class,
2. a flow set must remain separate from role identity, skill payload, and profile stance,
3. a flow set may specialize sequencing, but it must remain compatible with framework review, verification, approval, and closure boundaries.

## 6. Framework Base And Project Extensions

Framework-owned base:

1. framework roles,
2. framework role law,
3. framework role-profile law,
4. standard flow sets,
5. runtime safety invariants.

Project-owned extension layer:

1. enabled subset of framework roles,
2. enabled subset of standard flow sets,
3. custom project roles,
4. custom project skills,
5. enabled shared skills,
6. custom project profiles,
7. custom project flow sets.

## 7. Dynamic Activation Through Overlay

The project chooses its active role/skill/profile/flow posture through:

1. `vida.config.yaml`
2. `agent_extensions`
3. project-owned registries referenced by that overlay section

The current canonical project-owned registry family is:

1. `docs/process/agent-extensions/roles.yaml`
2. `docs/process/agent-extensions/skills.yaml`
3. `docs/process/agent-extensions/profiles.yaml`
4. `docs/process/agent-extensions/flows.yaml`

Shared-skill rule:

1. shared skills are not required to live in the project-owned registries above,
2. project activation of shared skills must still be explicit in `vida.config.yaml`,
3. profile references to shared skills should use the `shared:<skill_id>` form so runtime validation can distinguish them from project-owned skills.

## 8. Runtime Compilation Rule

`taskflow` must not treat project roles, skills, profiles, or flow sets as unrelated ad hoc flags.

Instead it must compile one runtime identity from:

1. resolved framework base role,
2. optional project role derived from that base,
3. validated project skill attachments,
4. enabled shared skill attachments,
5. validated project profile,
6. selected standard or custom flow set,
7. route/gate constraints from framework runtime law.

## 9. Validation Rule

Project extensions are valid only when all enabled references resolve and remain compatible.

Minimum validation obligations:

1. registry files exist,
2. ids are unique,
3. project roles resolve to known framework base roles,
4. project profiles resolve to known roles,
5. project profile skill refs resolve to known project skills or lawful shared skill refs,
6. profile skill compatibility matches the resolved base role,
7. enabled project flow sets resolve,
8. flow role-chains resolve to known framework or project roles,
9. `default_flow_set` resolves to one enabled standard or project flow set,
10. invalid project extensions fail closed.

## 10. Standard Flow Ladder

The canonical standard framework flow ladder is:

1. `minimal`
2. `reviewed`
3. `verified`
4. `governed`
5. `durable`

Meaning:

1. projects may enable a subset,
2. projects may also define custom flows,
3. custom flows may specialize sequencing, but must not weaken framework safety gates.

## 11. Layered Development Order

Build this model in layers:

1. enable framework roles and standard flow sets,
2. add project role registry,
3. add project skill registry,
4. add project profile registry,
5. add project flow-set registry,
6. compile these into `taskflow` runtime assignment,
7. validate and fail closed on unresolved references.

Each layer must deepen the previous one without depending on a later unfinished layer.

## 12. Placement Rule

Ownership placement must remain clean:

1. framework role and runtime law belong in `vida/config/instructions/**`,
2. project extension maps and project registries belong in project-owned surfaces under `docs/process/**`,
3. activation data belongs in root `vida.config.yaml`,
4. runtime implementation belongs in `taskflow-v0/**`.

## 13. Completion Proof

This model is considered wired when:

1. one framework protocol defines project extension activation and validation,
2. one project-owned map exposes project extension registries,
3. root `vida.config.yaml` can enable framework roles and standard flows and reference project registries,
4. runtime validation can fail closed on invalid extension wiring.

-----
artifact_path: product/spec/agent-role-skill-profile-flow-model
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/agent-role-skill-profile-flow-model.md
created_at: '2026-03-10T15:45:00+02:00'
updated_at: '2026-03-12T23:59:59+02:00'
changelog_ref: agent-role-skill-profile-flow-model.changelog.jsonl
