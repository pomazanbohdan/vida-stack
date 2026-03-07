# Vida Stack Release 1 Scope

Purpose: define the fullest currently known functional scope for `Vida Stack` Release 1 as a framework product target.

This document is not a current-state snapshot.
It defines the maximum intended Release 1 capability envelope that is already justified by the framework direction and active protocols.

Rule:

1. This file describes `what Release 1 should provide`.
2. It does not claim that every capability is already fully implemented.
3. Current-state implementation status must stay in runtime docs, task state, logs, and release work tracking.

## Product Goal

`Vida Stack` Release 1 should deliver a real agentic engineering control plane for product development.

It must be able to:

1. take ambiguous engineering work,
2. normalize it into protocol-governed execution,
3. orchestrate bounded single-agent and multi-agent flows,
4. enforce verification and quality gates,
5. preserve durable execution context,
6. keep documentation and runtime contracts aligned,
7. expose telemetry and scorecards for improvement,
8. operate as a serious framework layer rather than a prompt bundle.

Release 1 should be strong enough to prove the model on real engineering work and stable enough to justify extraction into a standalone framework repository.

## Release 1 Principles

Release 1 is expected to preserve these principles:

1. single authoritative workflow state
2. protocol-driven execution
3. verification-first delivery
4. bounded orchestration over uncontrolled autonomy
5. external-first read-heavy fanout when justified
6. orchestrator-owned synthesis and final gate
7. legacy-zero evolution
8. lean-default execution with explicit richer modes
9. framework/project boundary discipline
10. real-project validation before platform reimplementation

## Release 1 Outcome

At Release 1, Vida Stack should function as:

1. a framework bootloader,
2. a runtime orchestration system,
3. a protocol execution engine,
4. a task/TODO governance layer,
5. a subagent routing and dispatch layer,
6. a verification and review fabric,
7. a telemetry and evaluation layer,
8. a documentation synchronization framework,
9. a migration-ready precursor to the future Rust control plane.

## Architecture Baseline For Release 1

Release 1 should already embody the modern AI-agent architecture baseline.

The end-to-end architecture should be understood as:

1. User Request
2. Goal Interpreter
3. Planner Agent
4. Agent Control Loop
5. Tool Router
6. Execution Environment
7. Observation Layer
8. Memory System
9. Telemetry and Evaluation

Core principle:

Release 1 should already operate on an `observe -> plan -> act -> verify -> reflect` model, even if later releases deepen the implementation.

## Release 1 Control Loop

Release 1 should support a real adaptive control loop:

1. observe environment and task state
2. produce or refine plan
3. execute next bounded action
4. collect feedback and evidence
5. update runtime state
6. continue until completion, escalation, or safe stop

This should work for both:

1. single-agent execution
2. bounded multi-agent orchestration

## Functional Scope By Subsystem

### 1. Boot and Runtime Activation

Release 1 should provide:

1. deterministic boot profiles:
   - `micro`
   - `lean`
   - `standard`
   - `full`
   - `swarm`
2. post-compaction boot recovery
3. context hydration gates
4. algorithm selection and thinking-mode routing
5. explicit activation of framework protocol bundles through root overlay
6. mandatory read/restore of top-level framework policy
7. bootstrap validation before non-trivial execution
8. fail-fast behavior when critical runtime prerequisites are missing
9. bootstrap routing between orchestrator and worker lane entry contracts

Implementation audit:

- [x] Boot profiles and receipts exist.
- [x] Context hydration gates exist.
- [x] Fail-fast behavior exists in boot/runtime scripts.
- [x] `AGENTS.md` now routes between split orchestrator and worker entry contracts instead of carrying one monolithic lane contract.
- [ ] `micro` and `swarm` are not yet established as fully proven runtime profiles at the same maturity as `lean|standard|full`.

### 2. Problem Framing and Routing

Release 1 should provide:

1. problem framing contract before work starts
2. request classification by intent and execution type
   - `answer_only`
   - `artifact_flow`
   - `execution_flow`
   - `mixed`
3. pack routing for:
   - research
   - spec
   - work-pool
   - dev
   - bug-fix
   - reflection
4. orchestration lens selection
5. execution mode selection:
   - autonomous
   - decision_required
6. delivery-cut and scope-boundary handling
7. dependency strategy selection
8. risk-policy selection
9. user-decision escalation when assumptions are unsafe

Implementation audit:

- [x] Pack routing exists.
- [x] Execution mode routing exists.
- [x] Reflection-path routing exists.
- [x] Request-intent classification now exists before `br`, TODO, or pack-heavy execution is engaged.
- [ ] Some higher-level decision-card style framing remains protocol-led rather than fully machine-shaped.

### 3. Task and Execution State

Release 1 should provide:

1. a single authoritative task state path
2. deterministic execution visibility through TODO blocks
3. pack lifecycle support
4. block lifecycle support:
   - `block-plan`
   - `block-start`
   - `block-end`
   - `block-finish`
   - `reflect`
   - `verify`
5. compact and delta task views
6. next-step chaining
7. task queue intake from authoritative state
8. context capsules for restore after compaction
9. quality gates before close or handoff
10. explicit separation between lifecycle state and execution telemetry

Implementation audit:

- [x] Single authoritative task state exists.
- [x] TODO lifecycle commands and validation exist.
- [x] Context capsules and quality gates exist.
- [ ] Not every lifecycle guarantee is yet proven as an always-on invariant for every execution path.

### 4. Use-Case Packs

Release 1 should provide complete pack execution for:

1. `research-pack`
2. `spec-pack`
3. `work-pool-pack`
4. `dev-pack`
5. `bug-pool-pack`
6. `reflection-pack`

Each pack should support:

1. canonical initialization
2. scaffolded TODO decomposition
3. pack-specific step maps
4. bounded execution rules
5. handoff/close semantics
6. change-impact absorption where required

Implementation audit:

- [x] Canonical pack catalog exists.
- [x] Pack helper and scaffold flow exist.
- [x] Reflection-pack change-impact handling exists.
- [ ] Full end-to-end proof for every pack remains incomplete.

### 4.1 Planning and Reasoning Methods

Release 1 should support structured planning behavior rather than unconstrained prompt execution.

Target planning families:

1. reasoning plus acting flows
2. plan-and-execute flows
3. branching decision exploration for complex reasoning
4. dependency-aware workflow graph execution where appropriate

Release 1 does not need every planning family implemented as an isolated named engine, but the runtime should already support their practical equivalents through protocols, orchestration, and bounded decomposition.

Implementation audit:

- [x] Structured reasoning protocols exist.
- [x] Planning decomposition and bounded execution flows exist.
- [ ] Dependency-aware workflow-graph execution is not yet a fully explicit standalone runtime engine.

### 5. Subagent System

Release 1 should provide:

1. provider detection and runtime availability snapshot
2. hybrid mode with explicit provider classes
3. routing by task class
4. routing by provider score state
5. external-first orchestration for eligible read-heavy work
6. deterministic bridge fallback before internal escalation
7. bounded internal senior lane for arbitration and mutation-heavy work
8. fanout metadata:
   - providers
   - minimum result threshold
   - merge policy
9. route-level execution hints:
   - write scope
   - verification gate
   - runtime timeout
   - output-quality threshold
10. dynamic scorecards by:
   - global provider behavior
   - task class
   - inferred domain
11. strategy snapshots generated from observed runs
12. bounded ensemble lease artifacts with conflict visibility for overlapping orchestration lanes
13. subagent-first analysis and review behavior in supported `native|hybrid|disabled` runtime modes while keeping final synthesis under the orchestrator

Implementation audit:

- [x] Provider detection and runtime snapshots exist.
- [x] Hybrid mode and external-first routing exist.
- [x] Dynamic scorecards and strategy snapshots exist.
- [x] Live runtime/config refresh now updates route decisions instead of relying only on stale init snapshots.
- [x] Analysis routing now suppresses task-class-demoted CLI subagents from core fanout while keeping bridge/internal lanes available.
- [x] Ensemble lease acquisition, release, and conflict blocking now exist with operator-visible lease history.
- [x] Subagent-first analysis/review behavior is now codified as the default supported-mode fabric while final user-facing synthesis stays orchestrator-owned.
- [ ] Task/block/file-scope ownership beyond ensemble dispatch leases is still incomplete.

### 5.1 Multi-Agent Role Architecture

Release 1 should provide a role-based multi-agent model.

Core roles:

1. planner
2. researcher
3. executor
4. critic or reviewer
5. integrator
6. supervisor

Role split should be reflected in:

1. dispatch expectations
2. verification boundary
3. integration ownership
4. escalation behavior

Release 1 should also provide explicit ownership mechanics for bounded parallel work:

1. task lease or block lease for active agent runs
2. optional file or worktree scope for mutation-heavy execution
3. ownership release or expiration rules
4. reduced write-conflict risk through scope isolation

Implementation audit:

- [x] Worker vs orchestrator separation exists.
- [x] Role-oriented lane separation exists conceptually in routing/docs.
- [x] Ensemble lane lease acquisition/release now reduces overlapping orchestration collisions on the same task-class resource.
- [ ] Full explicit role runtime for planner/researcher/executor/critic/integrator/supervisor is not yet materialized as separate first-class runtime agents.
- [ ] Task/block/file/worktree ownership contracts and broader expiration semantics are not yet fully implemented as hard stateful runtime contracts.

### 6. Subagent Dispatch

Release 1 should provide:

1. canonical prompt rendering
2. protocol-scoped dispatch units
3. read-only ensemble fanout
4. single-provider dispatch
5. bounded arbitration lane
6. route-aware fallback handling
7. provider runtime limits
8. output-size and merge-readiness checks
9. machine-readable provider run artifacts
10. dispatch manifests containing:
    - route snapshot
    - provider results
    - merge summary
    - arbitration summary
    - provider exhaustion state
    - risk class
    - review state
11. graceful degradation instead of runtime crashes on unsupported paths
12. explicit separation between orchestrator-entry and worker-entry prompt contracts
13. progress-aware dispatch state such as useful-progress tracking and visible run phases during fanout, fallback, merge, and arbitration
14. phase-aware timeout parity between ensemble fanout and single-run dispatch lanes
15. question-driven worker packets with machine-readable answer contracts

Implementation audit:

- [x] Canonical prompt rendering exists.
- [x] Fanout, fallback, and arbitration exist.
- [x] Machine-readable run artifacts and manifests exist.
- [x] Progress-aware dispatch state exists.
- [x] Phase-aware timeout controls now exist for startup, no-output, progress-idle, and bounded runtime extension behavior.
- [x] Single-provider dispatch now has phase-aware timeout parity with ensemble execution instead of one coarse wall-clock timeout.
- [x] Live ensemble manifests expose `active_subagents`, `active_count`, and timeout-policy metadata.
- [x] Worker prompts now carry explicit question/answer/evidence/next-action return fields instead of a looser partial summary contract.
- [ ] Some merge/readiness heuristics are still heuristic rather than final Release 1-stable policy.

### 7. Review and Verification Fabric

Release 1 should provide review and verification as runtime behavior, not convention.

Required capabilities:

1. verification gates by route and task class
2. merge-readiness distinction from raw command success
3. review-state progression for agent results
4. targeted verification for implementation paths
5. review-first behavior for risky or incomplete outputs
6. policy-aware close and handoff gates
7. pre-close health verification
8. regression verification for bug-fix flows
9. source-backed verification for research/review flows
10. required evidence capture in logs and artifacts

Target review-state vocabulary for Release 1:

1. `review_pending`
2. `review_failed`
3. `policy_check_pending`
4. `senior_review_pending`
5. `requires_human`
6. `promotion_ready`

Implementation audit:

- [x] Review state is machine-visible.
- [x] Route and eval artifacts now expose target per-run and manifest review-state intent before dispatch.
- [x] Health and verification scripts exist.
- [x] Merge-readiness is distinguished from raw command success.
- [ ] The full target review vocabulary is not yet fully implemented end to end.
- [ ] Policy-aware close/handoff semantics are not yet complete across all execution classes.

### 8. Risk and Governance

Release 1 should provide explicit risk-aware runtime behavior.

Required capabilities:

1. risk classification for routed work
2. risk-aware review behavior
3. bounded approval escalation
4. write-scope-based risk distinction
5. human escalation for unsafe or ambiguous high-impact paths
6. protocol-critical gate failures with explicit fallback evidence
7. machine-visible risk state in runtime artifacts

Target minimal risk classes for Release 1:

1. `R0` — read-only low-risk execution
2. `R1` — review-sensitive low-write or architecture-sensitive execution
3. `R2` — bounded write with senior review requirement
4. `R3` — high-impact or orchestrator-native write requiring stronger approval
5. `R4` — reserved for future destructive or externally privileged execution

Implementation audit:

- [x] Risk class is machine-visible.
- [x] Risk-aware degraded/suppressed subagent states exist.
- [x] Recovery and routing suppression exist for broken lanes.
- [ ] The full `R0`-`R4` model is not yet fully exercised.
- [ ] Human escalation boundaries remain incomplete.

### 9. Quality, Health, and Runtime Gates

Release 1 should provide:

1. one-command health checks
2. quick, strict-dev, and full health modes
3. execution-log verification
4. pack coverage checks
5. overlay schema validation
6. skill availability validation
7. TODO plan validation
8. boot-profile validation
9. WVP evidence checks when external assumptions are involved
10. finish-gate blocking on contradictions

Implementation audit:

- [x] One-command health check exists.
- [x] Quick, strict-dev, and full modes exist.
- [x] TODO validation and boot-profile validation exist.
- [x] Finish-gate contradiction checks exist.

### 9.1 Safety and Governance

Release 1 should already include the baseline safety/governance model for an autonomous engineering runtime.

Required governance capabilities:

1. policy-aware tool access
2. prompt-injection-aware execution boundaries
3. tool misuse prevention through explicit permissions and contracts
4. human checkpoint support
5. audit trails for critical execution
6. escalation paths when autonomy should stop

Implementation audit:

- [x] Explicit permissions/contracts exist in framework policy.
- [x] Audit trails and health surfaces exist.
- [ ] Prompt-injection-aware and human-checkpoint governance remain only partially materialized as runtime mechanics.

### 10. Web and Reality Validation

Release 1 should provide:

1. a canonical protocol for internet validation
2. live validation for server/API assumptions
3. source hierarchy rules
4. confidence downgrade rules when evidence is weak
5. concise evidence capture in runtime logs
6. integration of validation into:
   - research
   - spec
   - bug-fix
   - implementation decisions when external facts matter

Implementation audit:

- [x] Canonical web-validation protocol exists.
- [x] Source hierarchy and evidence capture rules exist.
- [ ] Full proof of consistent runtime enforcement across every eligible flow remains incomplete.

### 11. Execution Environment

Release 1 should operate inside a real interactive environment, not prompt-only context.

Required environment surfaces:

1. terminal execution
2. filesystem access
3. browser or web-access surface
4. code/runtime execution
5. external API integration surface
6. normalized tool result capture

Implementation audit:

- [x] Terminal, filesystem, web, and API surfaces exist.
- [x] Tool result capture exists in runtime artifacts.

### 12. Documentation and Contract Sync

Release 1 should provide framework-level support for:

1. canonical source mapping
2. framework vs project documentation boundary enforcement
3. synchronized updates to related canonical docs in the same scope
4. reflection-pack for doc/protocol reconciliation
5. change-impact absorption when current docs and task pool diverge
6. runtime policy documentation ownership
7. future-ready structure for:
   - current contract docs
   - proposed change docs
   - decision docs
   - generated references

Release 1 should also define the baseline document-governance model:

1. a document state progression for proposed-to-current updates
2. verifier rules for current-vs-target separation
3. stale-reference detection for superseded docs
4. freshness binding between runtime/policy changes and document re-verification

Implementation audit:

- [x] Canonical source mapping exists.
- [x] Framework/project boundary documentation exists.
- [x] Reflection-pack reconciliation exists.
- [ ] Full document lifecycle and freshness verification remain incomplete.

### 13. Telemetry and Evaluation

Release 1 should provide:

1. provider run logs
2. execution logs
3. scorecard updates from observed behavior
4. eval-pack generation for task slices
5. strategy snapshots for provider ordering and domain fitness
6. failure and timeout visibility
7. merge-readiness visibility
8. review-state visibility
9. risk-state visibility
10. progress visibility across dispatch phases
11. a baseline for future drift detection
12. operator-visible recovery, timeout-instability, and lease-conflict summaries

Implementation audit:

- [x] Provider run logs exist.
- [x] Eval-pack generation exists.
- [x] Scorecards and strategy snapshots exist.
- [x] Review/risk/progress visibility exist.
- [x] Operator status exposes preferred or eligible task-class fit and recovery history visibility.
- [x] Operator status now exposes timeout-instability classes and recent lease-conflict summaries.
- [ ] Full drift visibility remains incomplete.

### 13.1 Drift Detection

Release 1 should provide first-class awareness of behavior drift.

Target drift sources:

1. model changes
2. prompt changes
3. tool changes
4. policy changes
5. data drift
6. documentation drift

Required Release 1 drift capabilities:

1. anomaly visibility in telemetry
2. degradation reflected in scorecards
3. explicit framework self-analysis path
4. evidence for re-routing, demotion, or stricter review

Implementation audit:

- [x] Demotion and re-routing evidence exist.
- [x] Framework self-analysis path exists.
- [ ] Anomaly and drift handling are not yet complete at the intended Release 1 level.

### 13.2 Evaluation Framework

Release 1 should provide continuous evaluation foundations.

Required evaluation capabilities:

1. task-completion quality measurement
2. reasoning quality proxy metrics
3. cost-per-task visibility
4. human-correction visibility
5. benchmark-ready evaluation artifacts
6. task-scoped eval-pack generation

Implementation audit:

- [x] Task-scoped eval-pack generation exists.
- [x] Reasoning/routing proxy metrics exist.
- [ ] Cost-per-task and human-correction visibility are not yet complete as a polished operator-facing model.

### 14. Learning and Improvement Loop

Release 1 should provide early but real improvement mechanics:

1. self-reflection entries
2. evaluation packs
3. provider score learning
4. protocol/runtime self-analysis
5. bounded framework diagnosis flows
6. evidence-backed improvement of routing and fallback behavior
7. a base for future prompt/policy revision loops

Implementation audit:

- [x] Self-reflection protocol exists.
- [x] Evaluation packs exist.
- [x] Provider score learning exists.
- [x] Framework self-analysis path exists.
- [ ] Distilled lesson and memory update flow remain incomplete.

### 14.1 Continuous Learning Pipeline

Release 1 should establish the base shape of:

1. tasks
2. logs
3. evaluation artifacts
4. distilled lessons
5. memory updates
6. future dataset/improvement inputs

It does not need full automated fine-tuning or continuous training in Release 1, but the runtime should preserve the artifacts needed for that future path.

Implementation audit:

- [x] Tasks, logs, and evaluation artifacts exist.
- [ ] Distilled lessons and memory updates are not yet complete as first-class runtime outputs.

### 15. Skills and Capability Routing

Release 1 should provide:

1. dynamic skill checks
2. required-skill health validation
3. domain-triggered skill routing
4. explicit fallback behavior when a tool or skill is unavailable
5. capability evidence logging when fallback is used

Implementation audit:

- [x] Skill validation and discovery utilities exist.
- [x] Fallback evidence logging exists in protocol/scripts.
- [ ] Full dynamic capability-routing maturity still depends on runtime usage breadth.

### 16. Project Overlay and Portability

Release 1 should provide:

1. root overlay activation
2. schema validation for overlay data
3. provider and route configuration in project-owned overlay
4. portable framework defaults when overlay is missing
5. project bootstrap contract for seeding required project artifacts
6. provider-level runtime budget and dispatch environment settings where orchestration realism requires them
7. project-overlay-resolved operations and environment entrypoints instead of hardcoded project paths in framework policy

Implementation audit:

- [x] Overlay activation and schema validation exist.
- [x] Provider/route configuration lives in overlay data.
- [x] Runtime budget and dispatch environment settings exist in the template/runtime.
- [x] Phase-aware timeout knobs now exist in config schema and overlay template, including longer Gemini-oriented runtime profiles.
- [x] Project bootstrap contract exists.
- [x] Framework runtime now relies on overlay-resolved operations/environment entrypoints instead of embedding project-specific runbook paths in `AGENTS.md`.
- [ ] Full standalone portability is not yet complete.

### 17. Cost and Efficiency Model

Release 1 should include the core optimization strategies already justified by framework goals.

Required efficiency capabilities:

1. model routing by task type
2. external-first cheap read-only fanout where justified
3. compact context hydration
4. prompt and artifact reuse
5. context pruning
6. lower-reread runtime artifacts as a path toward compiled policy

Implementation audit:

- [x] External-first low-cost fanout exists.
- [x] Compact hydration exists.
- [ ] Cost-per-task visibility is not yet complete.
- [ ] Compiled-policy-level optimization remains incomplete.

### 17.1 Compiled Policy Direction

Release 1 should establish the first practical step toward compiled protocol/runtime artifacts:

1. machine-readable policy packets where stable rules justify them
2. compact boot or handoff payloads
3. evidence schemas that can be checked by runtime gates
4. derived manifests that reduce markdown-only enforcement

Implementation audit:

- [x] Boot packets and derived manifests exist as early machine-visible artifacts.
- [ ] A broader compiled-policy layer is not yet complete.

### 18. Execution Surface

Release 1 should provide a compact but complete execution surface:

1. command-layer protocol map
2. pack helper commands
3. status commands
4. TODO and board commands
5. health and verification commands
6. provider routing commands
7. provider dispatch commands
8. eval commands
9. project bootstrap commands
10. bootstrap/session-entry commands and helpers for lighter framework startup

Implementation audit:

- [x] Command-layer protocol map exists.
- [x] Pack helper, routing, status, verification, eval, and bootstrap commands exist.
- [x] Bootstrap/session-start helper surfaces now exist for the lighter entry topology.


## Current Implementation Status (Codebase Audit)

- [x] **Done: Boot and Runtime Activation**
  - [x] `boot-profile.sh` implements runtime boot execution and receipt verification.
  - [x] Context capsule write/read/hydrate flow exists.
  - [x] Post-compaction restore flow exists through `beads-compact.sh`.
  - [x] Boot packet artifacts now exist and are linked from protocol docs.
  - [x] `AGENTS.md` now acts as a bootstrap router over split orchestrator and worker entry contracts.
- [x] **Done: Problem Framing and Routing**
  - [x] Pack detection exists through `vida-pack-router.sh` and `vida-pack-helper.sh detect`.
  - [x] Non-dev pack initialization exists through `nondev-pack-init.sh`.
  - [x] Execution mode routing exists through `task-execution-mode.sh`.
  - [x] Reflection-pack routing exists for drift and framework self-analysis flows.
  - [x] Request-intent classification now exists before engaging heavy execution machinery.
- [x] **Done: Task and Execution State**
  - [x] `br` remains the authoritative task-state path.
  - [x] TODO block lifecycle scripts and validation utilities exist.
  - [x] Compact and delta runtime views exist.
  - [x] Quality gates and context checkpoints are wired into the task flow.
- [ ] **Partial: Use-Case Packs**
  - [x] Canonical pack routing exists for research, spec, work-pool, dev, bug-fix, and reflection flows.
  - [x] Pack helpers and TODO scaffolding are implemented.
  - [x] Reflection-pack is wired into documentation and framework self-analysis flows.
  - [ ] Full end-to-end completion proof for every pack is not yet established inside this repository.
- [ ] **Partial: Subagent System and Dispatch**
  - [x] Provider detection, requested/effective mode calculation, and route snapshots exist.
  - [x] External-first fanout, deterministic fallback, and bounded arbitration are implemented.
  - [x] Worker-entry and worker-thinking contracts are separated from orchestrator governance.
  - [x] Supported-mode subagent-first analysis/review behavior is now codified while synthesis and mutation ownership remain orchestrator-owned.
  - [x] Recovery helpers, subagent suppression, active-subagent visibility, and richer scorecards now exist.
  - [x] Phase-aware timeout controls and live route refresh now exist.
  - [x] Ensemble lease acquisition, release, conflict blocking, and lease-history diagnostics now exist.
  - [x] Worker packets now use explicit machine-readable return fields for question-driven execution.
  - [ ] Broader task/block/file-scope ownership contracts are not yet fully materialized as runtime-enforced stateful contracts.
- [ ] **Partial: Review and Verification Fabric**
  - [x] Route artifacts now expose `review_state`.
  - [x] Route and eval artifacts now expose `target_review_state` and `target_manifest_review_state`.
  - [x] Verification and health-check scripts are implemented.
  - [x] `quality-health-check.sh` reads canonical subagent run logs.
  - [ ] The full target review-state vocabulary is not yet demonstrated end to end.
  - [ ] Policy-aware close and handoff behavior is not yet fully proven across all task classes.
- [ ] **Partial: Risk and Governance**
  - [x] Route artifacts now expose `risk_class`.
  - [x] Degraded, cooldown, auth-invalid, and interactive-blocked subagent states are modeled.
  - [x] Bounded recovery and routing suppression semantics are implemented for broken CLI subagents.
  - [ ] The full `R0` to `R4` risk model is not yet fully exercised in runtime behavior.
  - [ ] Human approval boundaries are not yet fully materialized as a complete Release 1 runtime surface.
- [x] **Done: Execution Environment**
  - [x] Terminal execution is present.
  - [x] Filesystem execution is present.
  - [x] Web access and browser-capable runtime surfaces are present.
  - [x] External API and tool result capture surfaces are present.
- [ ] **Partial: Documentation and Contract Sync**
  - [x] Protocol index, framework map, orchestrator/worker contracts, and changelog are in place.
  - [x] Framework/project boundary documentation is established.
  - [x] Release-target documents are synchronized with the current framework shape.
  - [x] Bootstrap split and bounded log-read policy are now reflected in framework docs.
  - [ ] Document lifecycle and freshness enforcement are not yet complete at the full Release 1 level.
- [ ] **Partial: Telemetry and Evaluation**
  - [x] Eval-pack and subagent evaluation scripts exist.
  - [x] Scorecards now track useful-progress and time-to-first-useful-output metrics.
  - [x] Operator status exposes task-class fit, recovery history, degraded subagent visibility, and timeout-instability summaries.
  - [x] Lease conflict and recent recovery summaries are now visible in operator surfaces.
  - [ ] Drift and anomaly visibility are not yet at the full target Release 1 maturity level.
- [ ] **Partial: Learning and Improvement Loop**
  - [x] Reflection, eval-pack, and scorecard-driven routing adaptation exist.
  - [x] Provider promotion, demotion, cooldown, and recovery flows are implemented.
  - [ ] Distilled lesson/memory update flow is not yet a full first-class runtime subsystem.
- [ ] **Partial: Project Overlay and Portability**
  - [x] Validated overlay template exists.
  - [x] The provider template now mirrors the canonical VIDA subagent stack instead of abstract placeholders.
  - [x] Phase-aware timeout profiles now exist in the overlay template and config schema.
  - [x] Bash installer and framework-only release packaging exist.
  - [x] Overlay-resolved operations/environment entrypoints now replace hardcoded project runbook references in framework bootstrap policy.
  - [ ] Production-ready standalone extraction discipline is not yet fully complete.
- [ ] **Not Done: Cost and Efficiency Model**
  - [ ] Compiled-policy level optimization artifacts are not yet implemented.
  - [ ] Cost-per-task visibility is not yet surfaced as a complete Release 1 operator model.
- [ ] **Not Done: Release 1 Exit Criteria**
  - [ ] Stable full-pack execution proof is still incomplete.
  - [ ] Standalone-framework readiness is still incomplete.
  - [ ] Full verification/review gate completeness is still incomplete.

## Release 1 Capability Matrix

Release 1 should be able to support the following classes of work end to end:

1. framework self-analysis
2. framework documentation synchronization
3. project research with external validation
4. specification creation and update
5. task pool formation and dependency routing
6. implementation execution against queued work
7. bug investigation and bug-fix delivery
8. bounded multi-agent analysis and review
9. verification and closure for completed slices

## Release 1 Minimal Production Stack

Release 1 should already stand on a serious minimum stack:

1. LLM/runtime model layer
2. framework control plane
3. persistent state and context artifacts
4. telemetry and observability layer
5. evaluation layer
6. verification and review layer
7. human escalation layer

## Release 1 User and Operator Surfaces

Release 1 should serve these surfaces:

1. top-level orchestrator agent
2. external free read-only providers
3. bridge fallback provider
4. internal senior agents
5. local shell/runtime operators
6. future standalone framework contributors

## Release 1 Delivery Guarantees

Release 1 should guarantee:

1. canonical ownership of task and execution state
2. deterministic protocol routing
3. auditable runtime artifacts
4. bounded subagent execution
5. enforced quality gates before closure
6. explicit degradation paths when runtime capability is insufficient
7. framework/project boundary consistency
8. documented and navigable protocol ownership

## Release 1 Non-Goals

The following are not required to be complete in Release 1:

1. full Rust reimplementation
2. full SurrealDB-native control plane
3. full daemonized standalone runtime
4. complete memory graph subsystem
5. complete DocSync daemon automation
6. complete protocol compiler with all rules machine-compiled
7. unrestricted multi-writer swarm execution
8. destructive external-privilege automation

Those belong to the later Vida Stack evolution path after Release 1 proves the mechanics on real delivery work.

## Release 1 Exit Criteria

Release 1 should be considered complete only when the framework can demonstrate:

1. stable real-project execution across the canonical pack set
2. durable and understandable runtime behavior
3. reliable subagent orchestration with controlled fallback paths
4. enforced verification and review gating
5. usable scorecards and telemetry for runtime decisions
6. framework docs coherent enough for standalone extraction
7. a credible migration path toward the Rust-based full control plane

## Relationship To The Future System

Release 1 is the proving ground.

It is expected to finish the logic, contracts, artifacts, and operational lessons that the future Rust implementation will formalize as a fuller system.

That means Release 1 is not a throwaway prototype.
It is the last major framework stage where the system is validated in real project conditions before the complete control-plane implementation is rebuilt on a stronger runtime foundation.
