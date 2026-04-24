# VIDA Agent Role, Skill, Profile, And Flow Model

Status: active product law

Purpose: define the canonical distinction between `execution carrier`, `role`, `skill`, `profile`, and `flow set`, and define how projects may extend them dynamically through `vida.config.yaml` without redefining framework runtime law.

## 1. Canonical Runtime Objects

VIDA recognizes five different composition objects for runtime agent behavior:

1. `execution carrier`
2. `role`
3. `skill`
4. `profile`
5. `flow set`

These objects are related, but they are not interchangeable.

## 1.1 Execution Carrier

`execution carrier` answers: which executor/model tier runs the bounded packet?

It owns:

1. model family and reasoning-effort envelope,
2. internal rate (cost units),
3. local effective score and lifecycle state,
4. admissibility metadata (`runtime_roles`, `task_classes`),
5. telemetry-backed adaptation surfaces.

Carrier rule:

1. the carrier is not the runtime role,
2. runtime role remains explicit activation state (`worker`, `coach`, `verifier`, `solution_architect`, and other lawful roles),
3. one carrier may execute multiple runtime roles when admissibility metadata allows it,
4. runtime must select the cheapest admissible carrier that satisfies role/task-class constraints and local score guard from telemetry state.

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

1. `orchestrator` is the routing-and-closure role:
   - owns bootstrap, request classification, packet shaping, lane sequencing, synthesis, and lawful closure,
   - keeps delegated state explicit,
   - must not become the default local writer for normal development.
2. `worker` is the mutation-owning implementation role:
   - executes one bounded packet inside explicit write scope,
   - returns diffs, proof notes, blockers, and residual risks,
   - must not self-approve closure or widen packet scope.
3. `business_analyst` is the scope-and-spec formation role:
   - turns feature or ambiguous asks into bounded requirements, constraints, and design-ready scope,
   - prepares canonical design/spec artifacts before execution lanes begin,
   - does not replace implementation, coaching, or verification.
4. `pm` is the delivery-shaping role:
   - cuts work into lawful tracked units, prioritizes slices, and frames rollout/budget consequences,
   - owns delivery framing and work-pool quality,
   - does not replace technical implementation or proof lanes.
5. `solution_architect` is the first-class pre-execution architecture-preparation role:
   - reads the bounded task or PBI,
   - studies governing specs and active project/runtime constraints,
   - inspects the relevant codebase and dependency surface,
   - produces one architecture-preparation report plus developer handoff packet,
   - defines what can be changed, what must not be changed, reuse points, dependency impact, and expected implementation boundaries before developer execution begins.
6. `solution_architect` does not replace `business_analyst`, `pm`, `coach`, `verifier`, or the developer/worker lane:
   - `business_analyst` shapes scope and requirements,
   - `pm` shapes delivery/task cut and launch readiness,
   - `solution_architect` prepares implementation architecture and constraints,
   - `worker` executes implementation,
   - `coach` and `verifier` remain downstream quality gates.
7. `coach` is the bounded spec-conformance and definition-of-done review role:
   - compares the implemented result against the approved spec, acceptance criteria, and packet `definition_of_done`,
   - returns bounded rework guidance or explicit forward approval,
   - stays formative and packet-local rather than widening into architecture or milestone scope.
8. `verifier` is the independent proof-and-closure-readiness role:
   - validates declared proof targets, verification commands, and closure evidence,
   - fails closed on missing or weak evidence,
   - must remain independent from the implementer and distinct from coach review.
9. `prover` is the evidence-deepening verification specialist:
   - strengthens or reconstructs proof when verification requires deeper evidence than routine review,
   - stays read-only unless a stronger route explicitly authorizes mutation,
   - does not replace delivery shaping or implementation.
10. `coach` must remain a separate role and must not collapse into `worker`, `verifier`, or `approver`.

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

The current bridge/source project-owned registry family is:

1. `docs/process/agent-extensions/roles.yaml`
2. `docs/process/agent-extensions/skills.yaml`
3. `docs/process/agent-extensions/profiles.yaml`
4. `docs/process/agent-extensions/flows.yaml`

Active runtime-owned projection family:

1. `.vida/project/agent-extensions/roles.yaml`
2. `.vida/project/agent-extensions/skills.yaml`
3. `.vida/project/agent-extensions/profiles.yaml`
4. `.vida/project/agent-extensions/flows.yaml`
5. matching `.sidecar.yaml` override files under the same runtime home

Shared-skill rule:

1. shared skills are not required to live in the project-owned registries above,
2. project activation of shared skills must still be explicit in `vida.config.yaml`,
3. profile references to shared skills should use the `shared:<skill_id>` form so runtime validation can distinguish them from project-owned skills.

## 8. Runtime Compilation Rule

`taskflow` must not treat project roles, skills, profiles, or flow sets as unrelated ad hoc flags.

Instead it must compile one runtime identity from:

1. resolved execution carrier candidate set,
2. resolved framework base role,
3. optional project role derived from that base,
4. validated project skill attachments,
5. enabled shared skill attachments,
6. validated project profile,
7. selected standard or custom flow set,
8. route/gate constraints from framework runtime law.

### 8.1 Compiled Runtime Identity Shape

Required compiled identity shape:

1. `selected_execution_carrier`
2. `base_role`
3. `validated_project_role`
4. `validated_project_skills`
5. `enabled_shared_skills`
6. `validated_profile`
7. `selected_flow_set`
8. `route_constraints`
9. `gate_constraints`
10. `cost_quality_constraints`

The orchestrator must consume this compiled identity instead of rediscovering the same logic ad hoc on every request.

`cost_quality_constraints` should expose at least:

1. selected executor tier when the host CLI provides a tier ladder,
2. selected model profile id and model ref when one carrier exposes multiple admissible model profiles,
3. selected reasoning effort / sandbox posture when the executor exposes them explicitly,
4. local effective score for that tier or profile,
5. internal rate or normalized cost units used for that tier/profile,
6. pricing basis used for the bounded task estimate,
7. pricing source paths and freshness posture when price evidence is config-tracked,
8. stale/missing price policy and whether that policy is diagnostic-only or enforced,
9. local score-store path when score refresh is runtime-local,
10. local observability/history store path when host-agent telemetry is runtime-local,
11. local budget rollup surface when spend tracking is runtime-local.

Carrier-selection proof fields should expose at least:

1. selected carrier id/tier,
2. selected model profile id and selected model ref,
3. activation runtime role,
4. admissibility evidence (`supports_runtime_role`, `supports_task_class`, `write_scope`, `readiness`),
5. effective score and lifecycle state from telemetry stores,
6. selected quality/speed posture when the executor catalog exposes them,
7. selection rule path (`role/task admissibility -> readiness -> score guard -> cost/quality`),
8. rejected candidate diagnostics when admissible competitors were filtered out.

## 9. Validation Rule

Project extensions are valid only when all enabled references resolve and remain compatible.

Minimum validation obligations:

1. active runtime projection files exist,
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
2. bridge/source registries may remain in project-owned surfaces under `docs/process/**`,
3. active runtime-owned projections belong under `.vida/project/**`,
4. activation data belongs in root `vida.config.yaml` only as a bridge overlay until `.vida/config/**` fully replaces it,
5. runtime implementation belongs in the active TaskFlow runtime-family implementation surfaces.

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
updated_at: 2026-04-22T15:34:41.118728061Z
changelog_ref: agent-role-skill-profile-flow-model.changelog.jsonl
