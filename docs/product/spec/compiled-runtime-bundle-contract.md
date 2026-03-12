# Compiled Runtime Bundle Contract

Status: active product law

Purpose: define the canonical machine-readable runtime bundle contract that compiles framework law, project activation, role/profile/skill composition, flow posture, and runtime policy into executable orchestrator and agent payloads for Release 1.

## 1. Why This Contract Exists

The runtime must not require large repeated markdown rereads in order to act lawfully.

Instead:

1. canonical law stays human-readable in specs and protocol surfaces,
2. active runtime control is compiled into one bounded machine-readable bundle,
3. orchestration consumes that bundle directly,
4. invalid or incomplete activation must fail closed before execution begins.

## 2. Bundle Classes

Release 1 recognizes these bundle classes:

1. `framework base bundle`
   - compiled framework-owned runtime law needed by all execution
2. `orchestrator bundle`
   - framework base plus the active orchestration posture
3. `agent bundle`
   - framework base plus one selected role/profile/skill/flow composition
4. `team bundle`
   - optional compiled coordination object when one team-level runtime composition is active

Bundle rule:

1. bundle classes may share fields,
2. but runtime must still know which class it is loading,
3. no bundle may smuggle in authority that its class does not own.

## 3. Source Inputs

### 3.1 Always-Compiled Framework Inputs

These inputs compile into every executable bundle:

1. framework core/system protocols,
2. lane/role coordination law,
3. gate and proof obligations,
4. packet and handoff law,
5. route constraints,
6. runtime-family boundary rules.

### 3.2 Selected Project Inputs

These inputs compile when active and valid:

1. selected role class,
2. selected project role override or extension,
3. selected profile,
4. enabled skills,
5. selected flow set,
6. model/backend policy,
7. approval/escalation posture,
8. project output/render posture,
9. promoted project protocols admitted for execution.

## 4. Minimum Bundle Shape

Every executable bundle must expose at least:

1. `bundle_id`
2. `bundle_class`
3. `framework_revision`
4. `project_activation_revision`
5. `role_class`
6. `resolved_profile`
7. `enabled_skills`
8. `selected_flow_set`
9. `gate_rules`
10. `packet_rules`
11. `handoff_rules`
12. `model_policy`
13. `backend_policy`
14. `output/render posture`
15. `evidence requirements`
16. `activation scope`

Rule:

1. the runtime may add more detail,
2. but it must not omit the minimum shape and still claim lawful execution.

## 5. Bundle Compilation Rule

### 5.1 Framework Compile Rule

Framework-owned law compiles always.

That includes:

1. core law,
2. orchestration shell law,
3. runtime-family execution law needed by the active release slice,
4. safety boundaries that remain sealed from project mutation.

### 5.2 Project Compile Rule

Project-owned inputs compile selectively.

That means:

1. project roles, skills, profiles, and flows may compile when enabled and valid,
2. known project protocols do not automatically become executable,
3. only promoted project protocols admitted by the project protocol promotion rule may enter executable bundles.

## 6. Validation And Failure

Bundle compilation is valid only when:

1. all required framework surfaces resolve,
2. selected project activation state resolves,
3. every enabled reference is valid,
4. gate and packet obligations are complete,
5. promotion/admission rules for project protocols are satisfied.

Fail-closed rule:

1. if required data is missing or inconsistent, compilation must stop,
2. runtime must not silently drop invalid project inputs and continue with a weaker bundle.

## 7. Orchestrator And Agent Initialization

Release 1 must expose at least two initialization paths:

1. `orchestrator-init`
   - builds or loads the active orchestrator bundle
2. `agent-init`
   - builds or loads one bounded agent bundle for the selected execution posture

Initialization rule:

1. init paths must be inspectable,
2. init output must tell the runtime what law/policy was compiled,
3. init output must remain usable by an LLM orchestrator without broad manual repo traversal.

## 8. Inspection Surfaces

Release-1 operator/runtime surfaces must support:

1. bundle summary,
2. bundle validation result,
3. bundle source inputs,
4. effective role/profile/skill/flow composition,
5. effective gate/policy summary.

Inspection rule:

1. bundle inspection exists for proof and debugging,
2. it must not become a second human-owned product-law source.

## 9. Boundary Rule

1. the bundle is executable composition, not the owner of product law,
2. canonical law remains in specs and framework protocols,
3. project activation remains DB-first truth,
4. the bundle is the compact runtime consumption surface built from that truth.

## 10. Completion Proof

This contract is operationally closed enough for Release 1 when:

1. framework law compiles into bounded executable bundles,
2. project activation compiles into orchestrator and agent bundles when valid,
3. invalid inputs fail closed,
4. bundle inspection can show the effective runtime posture,
5. init paths can bootstrap the orchestrator and bounded agents lawfully.

-----
artifact_path: product/spec/compiled-runtime-bundle-contract
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-11'
schema_version: '1'
status: canonical
source_path: docs/product/spec/compiled-runtime-bundle-contract.md
created_at: '2026-03-11T23:01:49+02:00'
updated_at: '2026-03-11T23:01:49+02:00'
changelog_ref: compiled-runtime-bundle-contract.changelog.jsonl
