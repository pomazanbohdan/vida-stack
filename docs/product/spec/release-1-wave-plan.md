# Release 1 Wave Plan

Status: active product execution program

Purpose: define the canonical autonomous wave plan for `Release 1` of the compiled autonomous delivery runtime so each wave is independently valuable, does not depend on future unfinished waves, and leaves the system stronger enough to build the next wave through the already-shipped `VIDA` shell itself.

## 1. Program Rule

Every Release-1 wave must satisfy all of:

1. it is autonomous and can close without future-wave implementation,
2. it delivers immediate operator/runtime value,
3. it strengthens the same `VIDA` shell that later waves will use,
4. it fails closed on missing prerequisites rather than pretending the later wave already exists,
5. it defers non-basis improvements when they do not materially increase Release-1 closure.

Compact rule:

1. basis first,
2. compilers before decoration,
3. one working shell before broad optional capability growth.

## 2. Prioritization Matrix

Feature selection for Release 1 must be judged by:

1. `need`
   - how necessary the feature is for a usable autonomous CLI runtime,
2. `operator value`
   - how much direct user/runtime value appears after the wave closes,
3. `framework leverage`
   - whether the feature helps build later waves using the same shell,
4. `implementation effort`
   - how much concrete implementation complexity is required,
5. `criticality`
   - whether missing the feature blocks Release-1 viability.

Decision rule:

1. high-need, high-leverage, medium-effort features enter early waves,
2. low-basis alternatives or late optimizations stay deferred even when attractive,
3. a feature that unlocks the next wave is preferred over a feature that only beautifies the current one.

## 3. Current Donor And Proof Base

Release 1 is not starting from zero.

Current bounded donors and proof sources:

1. `docs/product/spec/taskflow-v1-runtime-modernization-plan.md`
2. `docs/product/spec/docflow-v1-runtime-modernization-plan.md`
3. `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`
4. `vida/config/instructions/runtime-instructions/runtime.runtime-kernel-bundle-protocol.md`
5. `vida/config/instructions/runtime-instructions/work.project-agent-extension-protocol.md`
6. `taskflow-v0/**`
7. `codex-v0/**`
8. current Rust `vida` operator shell under `crates/vida/**`

Donor rule:

1. donor runtime behavior may accelerate Release 1,
2. donor runtime behavior must not silently redefine product law,
3. Release-1 wave closure still depends on current canonical specs and bounded proof surfaces.

## 4. Wave 1: Operational Spine

### 4.1 Goal

Ship the first usable `VIDA` shell that can manage and inspect its own runtime state through a DB-first operational spine.

### 4.2 Mandatory Scope

1. stable `boot/init` path,
2. DB-first state spine on embedded `SurrealDB`,
3. framework bundle bootstrap for orchestrator and agent initialization,
4. bounded `status` family roots,
5. `doctor` diagnostics,
6. baseline task/taskflow surfaces:
   - list
   - ready
   - show
   - update
   - close
7. snapshot, export, import, and restore for runtime task state,
8. bounded help/operator recipes sufficient to operate the shell safely.
9. a bounded `0.2.2` TaskFlow protocol-binding bridge slice that keeps protocol-binding authority on the DB-first taskflow state spine rather than detached file-log truth.
10. deterministic compiled protocol-binding JSON materialization plus installer bootstrap into the same authoritative state spine.

### 4.3 Why This Wave Exists

1. it is the smallest wave that already produces a real operational shell,
2. it uses already-proven donor/runtime work,
3. later waves can be executed through this shell instead of ad hoc tooling.

### 4.4 Out Of Scope

1. daemon memory,
2. reactive file watchers,
3. UI,
4. host-project embedding,
5. broad project protocol promotion.

### 4.5 Completion Proof

Wave 1 closes only when:

1. the operator can initialize and inspect the runtime from CLI,
2. task/taskflow state is queryable and restorable through bounded commands,
3. boot/status/doctor proof surfaces pass for the scoped implementation,
4. the resulting shell is usable for driving the next wave.
5. the first protocol-binding path is queryable from the same authoritative taskflow/runtime state spine rather than only from detached file exports.
6. installed runtime bootstrap can materialize required config/template state and protocol-binding DB state without ad hoc manual repair.

## 5. Wave 2: Project Activation Surface

### 5.1 Goal

Turn the operational shell into a project-aware runtime that can load and manage project-owned activation state lawfully.

### 5.2 Mandatory Scope

1. DB-first configurator,
2. project-owned:
   - roles
   - skills
   - profiles
   - flow sets
   - agents
   - teams
   - model classes
   - backend classes
   - policy surfaces
   - known project protocols
3. explicit selection mode,
4. automatic selection mode,
5. import/export/sync between DB truth and filesystem projection,
6. status families for configuration and activation.

### 5.3 Why This Wave Exists

1. Wave 1 gives a shell,
2. Wave 2 gives that shell project identity and activation posture,
3. later compilation waves need validated project activation state as input.

### 5.4 Out Of Scope

1. full reactive hooks engine,
2. always-on memory runtime,
3. broad project protocol auto-compilation beyond the bounded Release-1 rule.

### 5.5 Completion Proof

Wave 2 closes only when:

1. project activation state is persisted in DB,
2. filesystem projection remains synchronized under DB-first authority,
3. explicit and auto configuration modes are both validatable,
4. invalid activation wiring fails closed.

## 6. Wave 3: Compiled Runtime Bundles

### 6.1 Goal

Compile the active framework/project runtime posture into machine-readable bundles for the orchestrator and bounded agent execution.

### 6.2 Mandatory Scope

1. framework protocol compilation always-on,
2. role-class selection,
3. profile selection,
4. enabled skills,
5. gate rules,
6. packet rules,
7. model/backend policy,
8. `orchestrator-init`,
9. `agent-init`,
10. bundle inspection/query surfaces,
11. explicit separation between:
    - known project protocols,
    - compiled executable project protocols.

### 6.3 Why This Wave Exists

1. Release 1 must not depend on large raw protocol rereads per step,
2. bundle compilation is the bridge between human-readable canon and cheap orchestration runtime,
3. later execution/artifact waves depend on compiled runtime identity.

### 6.4 Out Of Scope

1. universal project protocol auto-compilation,
2. host-project semantic indexing,
3. background bundle refresh daemons.

### 6.5 Completion Proof

Wave 3 closes only when:

1. orchestrator and agent bundles can be built from active law/config state,
2. bundle contents are inspectable,
3. invalid inputs block compilation,
4. runtime can initialize from bundles rather than broad manual protocol traversal.

## 7. Wave 4: Planning, Execution, Artifact, And Approval Loop

### 7.1 Goal

Close the first end-to-end operator loop for planning, execution reporting, artifact materialization, and user approval.

### 7.2 Mandatory Scope

1. planning queries,
2. scope queries,
3. execution status families,
4. artifact materialization,
5. approval interactions through CLI,
6. project-facing templates through skills where needed,
7. task-graph formation for bounded execution,
8. rendered outputs built from lawful query/state paths rather than freeform narration.

### 7.3 Why This Wave Exists

1. this is where Release 1 becomes useful beyond framework operators,
2. it closes the operator path:
   - intent
   - plan
   - execution
   - artifact
   - approval
3. it still stays CLI-first and avoids premature UI complexity.

### 7.4 Out Of Scope

1. UI dashboards,
2. broad release/devops automation beyond bounded already-active protocols,
3. advanced observability families that remain next discussion.

### 7.5 Completion Proof

Wave 4 closes only when:

1. planning/scope state is queryable,
2. execution state is renderable from runtime truth,
3. realizable artifacts are materialized rather than left only in chat,
4. approval prompts and replies become durable operational evidence.

## 8. Wave 5: Release 1 Closure And Hardening

### 8.1 Goal

Close Release 1 as a coherent CLI-first runtime rather than a partial prototype.

### 8.2 Mandatory Scope

1. restore and reconcile flows,
2. DB/filesystem conflict discipline,
3. bounded `taskflow -> docflow` readiness/proof seam,
4. closure proof surfaces,
5. working binary or equivalent runnable output proof for scoped delivery work,
6. operator diagnostics completeness,
7. install/packaging path sufficient for Release-1 use.

### 8.3 Why This Wave Exists

1. Release 1 should close as a usable shell, not as an unfinished program board,
2. this wave hardens the earlier waves without forcing Release 2 concerns into Release 1.

### 8.4 Out Of Scope

1. full reactive synchronization engine,
2. always-on memory/index daemon,
3. UI,
4. host-project embedding and foreign codebase runtime control.

### 8.5 Completion Proof

Wave 5 closes only when:

1. Release 1 can recover from bounded drift or state restore scenarios,
2. proof and readiness surfaces remain lawful,
3. the CLI-first runtime can support further framework work through itself,
4. the system is stable enough to become the basis for Release 2 embedding work.

## 9. Deferred Beyond Release 1

The following remain intentionally outside Release 1:

1. reactive watcher/hook engine,
2. always-on memory daemon and freshness maintenance,
3. UI control surface,
4. host-project embedding,
5. advanced semantic indexing as permanent background runtime,
6. broad autonomous project protocol promotion without bounded admission rules,
7. Release-2 event-driven synchronization architecture.

## 10. Program Closure Rule

The Release-1 program is complete only when:

1. all five waves are closed,
2. each closed wave remains independently useful,
3. the delivered shell can be used to drive later framework work,
4. deferred Release-2 capability is clearly separated rather than half-implemented inside Release 1.

-----
artifact_path: product/spec/release-1-wave-plan
artifact_type: product_spec
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/spec/release-1-wave-plan.md
created_at: '2026-03-11T23:01:49+02:00'
updated_at: '2026-03-12T19:00:00+02:00'
changelog_ref: release-1-wave-plan.changelog.jsonl
