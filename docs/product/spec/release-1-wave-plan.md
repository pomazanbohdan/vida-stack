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
9. `docs/product/spec/taskflow-protocol-runtime-binding-model.md`
10. `docs/product/research/instruction-packing-and-caching-survey.md`
11. `docs/product/spec/user-facing-runtime-flow-and-operating-loop-model.md`
12. `docs/product/spec/bootstrap-carriers-and-project-activator-model.md`
13. `docs/product/research/runtime-framework-open-questions-and-external-patterns-survey.md`
14. `docs/product/research/runtime-home-and-surface-migration-research.md`
15. `docs/product/research/derived-cache-delivery-and-invalidation-research.md`
16. `docs/product/research/embedded-runtime-bootstrap-and-projection-research.md`
17. `docs/product/spec/execution-preparation-and-developer-handoff-model.md`
18. `docs/product/research/execution-preparation-and-developer-handoff-survey.md`

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
11. installer force-refresh semantics:
    - `--force` must not only replace the installed release payload,
    - it must also re-download and refresh the installer-management script so the active management surface cannot stay stale while the runtime payload is replaced.
12. explicit import of the compiled protocol-description JSON payload into authoritative runtime state rather than treating the generated file as terminal truth.
13. fail-closed runtime execution gating:
    - non-bootstrap work must refuse to run when required protocol-binding state is missing or invalid,
    - the runtime must expose bounded remediation/status surfaces instead of a generic crash.
14. installer/init bootstrap for the current project:
    - materialize the minimum model-visible framework bootstrap/config surfaces into the active project root,
    - enforce the root bootstrap split so `AGENTS.md` stays framework-owned and `AGENTS.sidecar.md` carries project-doc routing,
    - if a project-specific root `AGENTS.md` already exists, normalize its project-owned content into `AGENTS.sidecar.md`,
    - if `AGENTS.sidecar.md` is absent, create it as part of init,
    - then import the runtime-bearing protocol state into the same DB-first taskflow spine.
15. converge runtime-owned placement under `.vida/`:
    - active runtime config belongs under `.vida/config/**`,
    - authoritative DB state belongs under `.vida/db/**`,
    - runtime-owned project activation belongs under `.vida/project/**`,
    - bridge-era root files remain migration-only rather than long-term active runtime truth.
16. Release-1 bootstrap/onboarding split:
    - expose separate orchestrator-init and agent-init startup surfaces,
    - route pending project onboarding into `project-activator`,
    - enrich the project sidecar and project-doc map during activation instead of leaving project structure as transient chat knowledge.
17. runtime-home migration contract:
    - classify bridge-era root/runtime surfaces before cutover,
    - preserve root `AGENTS.md` and `AGENTS.sidecar.md` as bootstrap carriers,
    - move active runtime config into `.vida/config/**`,
    - move runtime-owned activation into `.vida/project/**`,
    - treat root `vida.config.yaml` and source-mode registries as bridge or projection surfaces after lawful import.

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
7. installer force-refresh can replace both the release payload and the installer-management script in one bounded operation.
8. the compiled protocol-description JSON payload can be imported into the authoritative runtime state spine and queried back through status/check surfaces.
9. runtime execution fails closed on missing or invalid protocol-binding state while still exposing bounded remediation instructions and allowlisted bootstrap commands.
10. project-local init/bootstrap can materialize the required model-visible framework surfaces and converge the matching protocol import without manual patching.
11. project-local init/bootstrap can normalize the root bootstrap carriers so framework bootstrap remains in `AGENTS.md` and project routing remains in `AGENTS.sidecar.md` without leaving competing root bootstrap law behind.
12. Release-1 startup can distinguish orchestrator-init, agent-init, and project-activator routing without collapsing them into one ambiguous bootstrap path.

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
6. status families for configuration and activation,
7. `solution_architect` as a first-class activation/runtime role,
8. `execution_preparation` as the canonical pre-execution stage for code-shaped or architecture-sensitive work,
9. architecture-preparation and developer-handoff artifacts as bounded execution inputs before worker implementation.

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
5. tasks that require execution preparation can distinguish planning output from developer-ready handoff and fail closed when the required preparation artifacts are missing.

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
12. a bounded cache-system slice for compiled instruction delivery:
    - stable cache-friendly bundle prefixes,
    - deterministic cache-key inputs,
    - explicit boundary between always-on bundles, activated bundles, and dynamic task context.
13. compiled bundles must be built from already imported DB-backed protocol/activation state rather than from broad raw markdown traversal at every execution step.
14. the first cache-friendly bundle contract must keep explicit partitions for:
    - `always_on_core`
    - `lane_bundle`
    - `triggered_domain_bundle`
    - `task_specific_dynamic_context`
15. provider-cache compatibility must stay explicit:
    - stable prefixes are cacheable,
    - dynamic evidence, receipts, and task deltas are excluded from the cache prefix,
    - retrieval remains an optional adjunct path rather than a replacement for mandatory invariants.
16. the first Release-1 compiled control bundle schema must be explicit and strict:
    - one canonical JSON schema,
    - one explicit root metadata block,
    - one explicit split between `control_core`, `activation_bundle`, `protocol_binding_registry`, and `cache_delivery_contract`.
17. the first derived serving-cache slice must remain subordinate to DB truth:
    - cache artifacts live under `.vida/cache/**`,
    - they accelerate CLI/runtime/model-serving hot paths,
    - they rebuild from authoritative revision tuples rather than becoming a second truth model.

### 6.3 Why This Wave Exists

1. Release 1 must not depend on large raw protocol rereads per step,
2. bundle compilation is the bridge between human-readable canon and cheap orchestration runtime,
3. later execution/artifact waves depend on compiled runtime identity,
4. cache-friendly bundle delivery is the first practical token-efficiency layer once compilation exists.

### 6.4 Out Of Scope

1. universal project protocol auto-compilation,
2. host-project semantic indexing,
3. background bundle refresh daemons,
4. fine-tuning or prompt-compression-first optimization before cache-safe compiled bundles exist.

### 6.5 Completion Proof

Wave 3 closes only when:

1. orchestrator and agent bundles can be built from active law/config state,
2. bundle contents are inspectable,
3. invalid inputs block compilation,
4. runtime can initialize from bundles rather than broad manual protocol traversal,
5. the first cache-system slice has a bounded contract for stable prefixes, cache-key inputs, and dynamic-context exclusion.
6. bundle compilation consumes the already-imported DB-backed protocol/activation state rather than re-deriving execution truth from raw markdown at runtime.
7. the runtime can show which bundle segments are cache-stable versus task-dynamic.
8. the first cache-system slice proves explicit boundaries between cacheable prefixes, retrieval-only optional context, and non-cacheable runtime evidence.
9. the Release-1 bundle compiler produces one strict schema-valid control bundle rather than several unrelated machine-readable payloads.

### 6.6 Research Link And Discussion Task

Research reference:

1. [instruction-packing-and-caching-survey.md](/home/unnamed/project/vida-stack/docs/product/research/instruction-packing-and-caching-survey.md)

Bounded implementation discussion task:

1. before Rust-native cache/runtime implementation begins, define the cache-system contract for Release 1:
   - compiled bundle format for cache-friendly delivery,
   - provider-cache compatibility assumptions,
   - cache-key derivation inputs,
   - retrieval versus always-on bundle boundary,
   - proof metrics for real token savings without protocol drift.
2. before policy hardening is treated as closed, define the Release-1 governance-policy contract using:
   - approval risk bands,
   - execution-boundary classes,
   - verification evidence/risk gates,
   - closure-only blocker semantics,
   - the vendor-aligned research captured in `docs/product/research/agent-governance-and-policy-hardening-survey.md`.

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
updated_at: '2026-03-13T00:10:00+02:00'
changelog_ref: release-1-wave-plan.changelog.jsonl
