# Release 1 Plan

Status: `approved`

Use this template for one bounded feature/change design before implementation.

Structured-template rule:
1. Keep headings stable.
2. Replace placeholders rather than rewriting the shape.
3. Prefer explicit fields and short bullets over long free-form prose.
4. Link separate ADRs when one or more major decisions need durable decision records.

## Summary
- Feature / change: define the canonical Release-1 refactor plan that fixes the target V1 architecture, mandatory missing capabilities, crate/file decomposition, and bounded migration order
- Owner layer: `project`
- Runtime surface: `docflow | taskflow | project activation`
- Status: `approved`
- Release-1 control rule:
  - this document is the active Release-1 execution owner
  - `release-1-current-state.md`, `release-1-event-state-and-projection-topology-design.md`, `release-1-capability-matrix.md`, `release-1-seam-map.md`, `release-1-closure-contract.md`, `release-1-workflow-classification-and-risk-matrix.md`, `release-1-control-metrics-and-gates.md`, `release-1-canonical-artifact-schemas.md`, `release-1-decision-tables.md`, `release-1-state-machine-specs.md`, `release-1-error-and-exception-taxonomy.md`, `release-1-ownership-to-code-map.md`, `release-1-proof-scenario-catalog.md`, `release-1-schema-versioning-and-compatibility-law.md`, `release-1-runtime-enum-and-code-contracts.md`, `release-1-conformance-matrix.md`, `release-1-operator-surface-contract.md`, `release-1-unsupported-surface-contract.md`, `release-1-fixture-and-golden-data-contract.md`, `release-1-risk-acceptance-register.md`, `taskflow-v1-runtime-modernization-plan.md`, and `docflow-v1-runtime-modernization-plan.md` remain active companion surfaces

## Current Context
- Existing system overview: `vida-stack` already has substantial native Rust implementation value across the `TaskFlow` family, the `DocFlow` family, and the `vida` launcher shell, but execution ownership is split across too many plan documents and too much runtime behavior remains concentrated in `crates/vida/**`.
- Key components and relationships: `TaskFlow` owns execution substrate and closure authority, `DocFlow` owns documentation/readiness/proof, `vida` currently hosts shell, activation, runtime-consumption, route shaping, agent activation, run-graph, and observability glue, and `.vida/**` already acts as the DB-first runtime home with projected filesystem surfaces.
- Current pain point or gap: critical mandatory capabilities are still incomplete or structurally mis-owned:
  - stateful agent-lane completion and exception-path governance
  - explicit execution-preparation enforcement for code-shaped work
  - broader platform-level contracts and registries beyond shell-local runtime behavior
  - launcher concentration in `crates/vida/src/main.rs` and `crates/vida/src/state_store.rs`
  - config-driven multi-system carrier law is documented in `vida.config.yaml`, but runtime execution and bundle truth remain codex-centric through `codex_multi_agent`, `codex_runtime_assignment`, and `vida agent-init`
  - DB-first activation truth is persisted as a launcher-captured snapshot rather than owned by one DB-native configurator service
  - top-level `consume` / `lane` / `approval` / `recovery` contracts and the full shared enum/value layer remain incomplete
  - final `TaskFlow -> DocFlow -> Release 1 closure` seam hardening
  - current seam proof is shell-assembled and replay/checkpoint law is resumability-heavy rather than lineage-complete
  - codex-era fixtures and smoke proofs still pin runtime contracts that must become carrier-neutral
  - first-class evaluation, governance, and registry surfaces aligned with the broader product vision recorded in the Airtable `Vida` base `Spec` table on `2026-03-16`

## Goal
- What this change should achieve: define the canonical Release-1 refactor program in enough detail to guide implementation from the current codebase to the intended V1 end-state architecture.
- What success looks like:
  - Release-1 work can be routed from one owner document
  - every bounded implementation packet maps to one phase of this master plan
  - mandatory missing capabilities are explicit and sequenced
  - target crate/file ownership is fixed before further broad implementation continues
  - the resulting V1 architecture remains vendor-neutral, DB-first, fail-closed, and platform-shaped rather than shell-shaped
- What is explicitly out of scope:
  - Release-2 embedding and reactive host-project synchronization
  - broad UI/product-surface implementation
  - speculative multi-cloud feature work beyond vendor-neutral contracts
  - wholesale rewrite of already-proven `DocFlow` or `TaskFlow` foundations

## Release-1 Routing
- This document is the single active Release-1 owner and entrypoint.
- Use it for:
  - execution sequencing and migration order
  - current Release-1 architecture and owner boundaries
  - routing into the capability matrix, seam map, runtime-track plans, and current-state checkpoint
- Read next by concern:
  - top-level architecture direction: `compiled-autonomous-delivery-runtime-architecture.md`
  - bounded event-state and projection topology: `release-1-event-state-and-projection-topology-design.md`
  - TaskFlow implementation: `taskflow-v1-runtime-modernization-plan.md`
  - DocFlow implementation: `docflow-v1-runtime-modernization-plan.md`
  - cross-track closure: `release-1-capability-matrix.md`
  - seam and hardening blockers: `release-1-seam-map.md`
  - current implementation/readiness posture: `release-1-current-state.md`
  - release admission and definition of done: `release-1-closure-contract.md`
  - supported workflow classes and risk posture: `release-1-workflow-classification-and-risk-matrix.md`
  - control metrics and release gates: `release-1-control-metrics-and-gates.md`
  - minimum machine-readable runtime artifact contracts: `release-1-canonical-artifact-schemas.md`
  - executable control decisions: `release-1-decision-tables.md`
  - transition law for runtime control lifecycles: `release-1-state-machine-specs.md`
  - canonical blocker and failure vocabulary: `release-1-error-and-exception-taxonomy.md`
  - doc-to-code owner boundary map: `release-1-ownership-to-code-map.md`
  - minimum proof scenario set: `release-1-proof-scenario-catalog.md`
  - artifact schema evolution law: `release-1-schema-versioning-and-compatibility-law.md`
  - canonical runtime enum/value set: `release-1-runtime-enum-and-code-contracts.md`
  - doc-to-code-to-proof conformance scoreboard: `release-1-conformance-matrix.md`
  - stable CLI/operator output contract: `release-1-operator-surface-contract.md`
  - explicit unsupported and reserved surfaces: `release-1-unsupported-surface-contract.md`
  - canonical fixture and golden-data contract: `release-1-fixture-and-golden-data-contract.md`
  - explicit open-risk register: `release-1-risk-acceptance-register.md`

## Requirements

### Functional Requirements
- Must-have behavior:
  - Release 1 must keep one architecture anchor, one execution-program owner, two runtime-track owners, one seam owner, and one reality/proof checkpoint model.
  - Release 1 execution routing must be reconstructible from this plan alone without requiring chat-history reconstruction.
  - The runtime must distinguish canon, activation, compiled control, execution, state/evidence, projection, and host-agent carrier surfaces explicitly.
  - Code-shaped execution must fail closed without `execution_preparation` artifacts and a lawful developer handoff packet.
  - Agent execution must become receipt-governed rather than prompt-disciplined:
    - packet activation
    - lane-open state
    - completion/block/supersession receipts
    - exception-path receipts
  - `TaskFlow` must remain final execution and closure authority.
  - `DocFlow` must remain bounded documentation, readiness, validation, and proof authority.
  - Release 1 must expose stable contracts for:
    - request intake
    - tool invocation
    - agent result
    - lane execution receipt
    - readiness/proof verdict
    - closure admission
    - approval decision
    - evaluation run
    - policy decision
  - Host carrier selection and dispatch must be config-driven across internal and external backends:
    - one carrier-neutral runtime contract set
    - no codex-specific canonical field names in closure-facing runtime artifacts
    - configured backend execution must not stop at preflight or metadata-only routing
  - DB-first activation truth must be authoritative:
    - launcher-captured snapshots are evidence and projection material, not the final owner model
    - filesystem and YAML remain import/export surfaces, not equal operational truth
  - Release 1 must expose stable top-level operator contracts for:
    - `status`
    - `doctor`
    - `consume`
    - `lane`
    - `approval`
    - `recovery`
  - The shared enum/value layer must close for:
    - `workflow_class`
    - `risk_tier`
    - `approval_status`
    - `gate_level`
    - `compatibility_class`
    - canonical blocker registry
  - The `TaskFlow -> DocFlow` seam must use explicit handoff and receipt/proof artifacts rather than shell-assembled in-process verdicts.
  - Checkpoint/recovery law must support append-evidence receipts and replay lineage rather than only latest resumability summaries.
  - Proof fixtures and smoke tests must validate carrier-neutral contracts and operator capabilities rather than codex-era bundle names, backend literals, or exact legacy surfaces.
  - The codebase must converge to a crate/module shape where launcher shell concerns are thin and runtime-family concerns are family-owned.
  - The architecture must leave explicit room for first-class:
    - agent registry
    - tool registry
    - ADR/change governance
    - evaluation benchmark registry
    - deployment and runtime policy
  - Release 1 mandatory production-baseline capabilities must be explicit and closure-gated:
    - trace, telemetry, and evidence foundation with root trace and side-effect linkage
    - tool contract normalization with side-effect class, auth mode, idempotency, retry, and policy hook
    - retrieval trust with source registry, ACL propagation, freshness posture, and citation contract
    - identity, delegation, and approval enforcement for all sensitive actions
    - runtime SLO, failure taxonomy, incident evidence bundles, and rollback/fallback control
  - Release 1 mandatory production-control capabilities must also be staged before closure or explicitly risk-accepted:
    - prompt lifecycle and controlled rollout
    - process evaluation and feedback ingestion
    - memory governance operationalization
    - safety, red teaming, and cost-governed optimization
  - The execution lifecycle must expand beyond current shell-local stages and align with the broader platform lifecycle:
    - `RECEIVED`
    - `AUTHORIZED`
    - `CLASSIFIED`
    - `ROUTED`
    - `PLANNED`
    - `EXECUTION_PREPARATION`
    - `EXECUTING`
    - `WAITING_FOR_TOOL`
    - `WAITING_FOR_APPROVAL`
    - `RESUMED`
    - `VALIDATING`
    - `COMPLETED | FAILED | CANCELLED | ESCALATED`
  - Human-in-the-loop, tool policy, memory policy, and guardrail placement must remain explicit Release-1 architecture concerns even where the first implementation is contract-first rather than full-surface-complete.
- Integration points:
  - `vida` CLI shell
  - `TaskFlow` family crates
  - `DocFlow` family crates
  - `.vida/data/state`, `.vida/project/**`, `.vida/cache/**`
  - project agent-extension projections under `.vida/project/agent-extensions/**`
  - Airtable-backed current architecture vision used as product-direction evidence, not runtime truth
- User-visible or operator-visible expectations:
  - one canonical Release-1 execution program entrypoint
  - explicit status/readiness/gate surfaces for open delegated cycles
  - bounded proofs for each migration phase
  - no silent fallback from orchestrator posture into local writer posture

### Non-Functional Requirements
- Performance: control-bundle compilation and derived cache delivery must reduce broad raw-canon rereads and keep runtime hot paths cache-stable.
- Scalability: the architecture must scale from current CLI-first Release 1 into a broader platform shape without forcing a second rewrite of execution, registry, or evidence contracts.
- Observability: every runtime transition, tool call, approval, retry, handoff, evaluation, and closure decision must remain explicit, persisted, and queryable.
- Security: policy-before-action, least privilege by default, tenant/policy boundary preservation, auditable exception paths, and fail-closed gate semantics remain mandatory.
- Reliability:
  - lifecycle state must be replayable
  - side-effecting tool calls must be idempotency-aware
  - retry and compensation posture must remain explicit
- Maintainability:
  - owner boundaries must map cleanly from docs to crates/modules
  - no new monolithic shell concentrations may be introduced while this plan is active

## Ownership And Canonical Surfaces
- Project docs / specs affected:
  - `docs/product/spec/release-1-plan.md`
  - `docs/product/spec/current-spec-map.md`
- Framework protocols affected: none directly, but this plan materially routes work across `runtime.runtime-kernel-bundle-protocol`, `runtime.direct-runtime-consumption-protocol`, `work.taskflow-protocol`, `work.project-agent-extension-protocol`, and verification/handoff protocols already promoted in canon
- Runtime families affected:
  - `TaskFlow`
  - `DocFlow`
  - launcher shell `vida`
- Config / receipts / runtime surfaces affected:
  - `.vida/data/state/**`
  - `.vida/project/**`
  - `.vida/cache/**`
  - `.vida/state/host-agent-observability.json`
  - run-graph status and dispatch receipts
  - future lane-completion and exception-path receipts

## Design Decisions

### 1. Replace the fragmented Release-1 execution program with one master owner plan
Will implement / choose:
- `release-1-plan.md` is the primary Release-1 execution owner.
- Why: one owner document is the cleanest control model for V1 closure because architecture, reality, backlog, and migration target must be routed through one canonical surface.
- Trade-offs: the new plan is larger and must be maintained carefully to avoid becoming a vague umbrella doc.
- Alternatives considered:
  - keep separate active execution owners; rejected because operator routing and implementation prioritization remain fragmented
  - rewrite the workspace first and re-document later; rejected because this would widen uncontrolled drift
- ADR link if this must become a durable decision record: optional follow-up ADR if Release-1 execution ownership ever needs a separate durable decision record

### 2. Treat Release 1 as a platform-shaped architecture, not only a runtime-family modernization effort
Will implement / choose:
- V1 target architecture keeps the current four runtime layers and adopts the broader nine-plane platform view as the end-state shape:
  - Experience
  - Identity & Access
  - Agent Control Plane
  - Model Plane
  - Tool & Integration Plane
  - Memory & Knowledge Plane
  - Runtime Execution Plane
  - Observability & Evaluation Plane
  - Governance & Administration Plane
- Why: local canon already defines a compiled autonomous delivery runtime, and the Airtable `Vida` architecture vision adds mandatory product-scale concerns such as identity, registries, evaluation, governance, and deployment that must not be architecturally blocked by Release-1 refactors.
- Trade-offs: Release 1 cannot implement every plane fully, but it must preserve their boundaries explicitly.
- Alternatives considered:
  - optimize only the current shell/runtime internals; rejected because it would likely hardcode a too-narrow workflow-engine architecture
- ADR link if needed: none

### 3. Prefer controlled carve-out over big-bang rewrite
Will implement / choose:
- Keep proven `DocFlow` family crates, `TaskFlow` core/contracts/state foundations, and already-green operator/readiness surfaces.
- Refactor or extract only the wrong owners:
  - launcher-heavy routing
  - launcher-heavy run-graph and delegation behavior
  - shell-owned runtime-consumption and host-agent state handling
- Why: the implementation-reality pass shows the main problem is concentration and owner misplacement, not absence of real code.
- Trade-offs: migration must temporarily preserve some bridge-shaped seams while ownership is being moved.
- Alternatives considered:
  - full fresh rewrite from architecture intent; rejected because it would discard real assets and proofs
- ADR link if needed: none

### 4. Make agent execution stateful and receipt-governed
Will implement / choose:
- Replace advisory agent-lane behavior with runtime-owned lifecycle contracts for:
  - activation
  - open lane state
  - completion
  - block
  - supersession
  - exception-path takeover
- Why: root-session orchestration must be blocked by open delegated cycles through authoritative state, not through prompt discipline alone.
- Trade-offs: this adds explicit receipt types and more state transitions, but it closes the most dangerous architectural gap now visible in the codebase.
- Alternatives considered:
  - stronger prompts only; rejected because it does not create trustworthy runtime truth
- ADR link if needed: recommended future ADR if the lane-completion protocol becomes a reusable framework-wide contract

### 5. Make `vida` a thin shell and move domain ownership below it
Will implement / choose:
- `crates/vida` becomes composition, routing, and rendering shell only.
- Runtime-family and domain behavior move into bounded modules/crates below the shell.
- Why: current `main.rs` and `state_store.rs` concentration shows the shell still owns too much runtime law.
- Trade-offs: more crates and modules require explicit interfaces and migration discipline.
- Alternatives considered:
  - keep one large launcher and continue adding functions; rejected because it worsens owner drift and proof fragility
- ADR link if needed: none

### 6. Split by files first, then extract stable crate boundaries
Will implement / choose:
- First migration wave is intra-crate modularization.
- Second migration wave is crate extraction only where the owner boundary is stable and independently testable.
- Why: many current boundaries are obvious in concept but still entangled in implementation details inside `crates/vida`.
- Trade-offs: this is slightly slower than immediate crate explosion, but it reduces the risk of freezing the wrong package topology.
- Alternatives considered:
  - immediate many-crate split; rejected because it would likely encode unstable boundaries prematurely
- ADR link if needed: none

### 7. Put TaskFlow ownership back on execution-state domains
Will implement / choose:
- `run graph`, `lane lifecycle`, `closure admission`, and `execution progression` move toward `TaskFlow`-family ownership rather than staying launcher-owned or being re-extracted into generic shell crates.
- Why: `TaskFlow` is the execution substrate and closure authority, so the main runtime state machine should not end Release 1 as shell glue.
- Trade-offs: some current shell helpers will need to be split into shared contracts versus TaskFlow-native behavior.
- Alternatives considered:
  - extract all execution domains into `vida-*` crates; rejected because it would leave execution authority outside the runtime family that canon says must own it
- ADR link if needed: none

### 8. Introduce contract-first registries and evaluation before broad UX
Will implement / choose:
- Release 1 includes explicit contracts for agent/tool/ADR/evaluation registries even if the first runtime surfaces stay operator/CLI-first.
- Why: Airtable architecture vision and local canon both require platform-shaped extensibility and traceability that cannot be retrofitted safely after execution contracts freeze.
- Trade-offs: some early registry surfaces will feel infrastructure-first rather than product-visible.
- Alternatives considered:
  - postpone registries/evaluation to Release 2; rejected because it would force later contract breaks across control plane, governance, and deployment
- ADR link if needed: none

### 9. Treat production safety and control maturity as Release-1 closure law, not later optimization
Will implement / choose:
- Release 1 closure must be measured against explicit P0 and P1 operational tracks reflected from the Airtable `Vida` spec backlog refreshed on `2026-03-16`.
- P0 tracks:
  - trace, telemetry, and evidence foundation
  - tool contract and side-effect control
  - retrieval, freshness, and citation reliability
  - identity, delegation, and approval enforcement
  - runtime SLO, failure recovery, and rollback
- P1 tracks:
  - prompt lifecycle and controlled rollout
  - process evaluation and feedback loop
  - memory governance operationalization
  - safety, red teaming, and FinOps maturity
- Why: the updated product vision no longer treats these as optional future hardening; they are the minimum trustworthy production boundary for Release 1.
- Trade-offs: Release 1 becomes stricter and less willing to declare closure on runtime-refactor progress alone.
- Alternatives considered:
  - keep Release 1 focused only on runtime-family modernization; rejected because it would under-specify production trust boundaries
- ADR link if needed: none

## Technical Design

### Core Components
- Main components:
  - Canon Plane
    - human-readable law in `vida/config/instructions/**` and `docs/product/spec/**`
  - Project Activation Plane
    - DB-first project activation/configuration and runtime-owned projections
  - Compiled Control Plane
    - machine-readable runtime bundle, lane graph, selection policy, gate rules, and cache-delivery contract
  - TaskFlow Execution Plane
    - task lifecycle, run graph, lane routing, execution state, closure admission
  - DocFlow Evidence Plane
    - documentation inventory, validation, readiness, proof, and seam-facing verdicts
  - Host-Agent Carrier Plane
    - Codex carrier activation, lane assignment, local score strategy, and completion feedback
  - Observability And Evaluation Plane
    - traces, metrics, proof receipts, benchmark runs, drift/replay surfaces
  - Governance And Registry Plane
    - agent catalog, tool catalog, ADR/change governance, evaluation catalog
- Key interfaces:
  - `UserRequestContract`
  - `ExecutionPreparationReport`
  - `DeveloperHandoffPacket`
  - `ToolInvocationContract`
  - `AgentResultContract`
  - `LaneExecutionReceipt`
  - `ExceptionPathReceipt`
  - `ReadinessVerdict`
  - `ClosureAdmissionVerdict`
  - `CompiledControlBundle`
- Bounded responsibilities:
  - `TaskFlow` owns execution state, run graph, handoff, closure admission, and final release-facing closure authority
  - `DocFlow` owns documentation/inventory validation, readiness, proof, and seam-facing machine-readable verdicts
  - host-agent carriers execute bounded lanes only; they do not own runtime truth
  - `.vida/**` projections accelerate or expose runtime state; they do not replace DB-first authority

### Data / State Model
- Important entities:
  - `ProjectActivationState`
  - `CompiledControlBundle`
  - `RunGraphStatus`
  - `DispatchReceipt`
  - `LaneExecutionReceipt`
  - `ExceptionPathReceipt`
  - `SupersessionReceipt`
  - `ReadinessVerdict`
  - `ClosureAdmission`
  - `AgentDefinition`
  - `ToolDefinition`
  - `EvaluationBenchmark`
  - `ArchitectureDecision`
- Receipts / runtime state / config fields:
  - `dispatch_status` must stop being the only meaningful delegated-lane marker
  - explicit `lane_status` is required:
    - `activated`
    - `open`
    - `completed`
    - `blocked`
    - `superseded`
  - explicit proof/evidence status is required:
    - `missing`
    - `recorded`
    - `accepted`
    - `rejected`
  - open delegated cycles must map into a runtime-owned takeover gate that blocks local root-session writing without a recorded exception-path receipt
- Migration or compatibility notes:
  - current `packet_ready` semantics are transitional and must be replaced by richer lane lifecycle contracts
  - no additional Release-1 planning doc may become an active owner without superseding this plan

### Canonical Release-1 Execution Lifecycle
- Request lifecycle:
  - `RECEIVED`
  - `AUTHORIZED`
  - `CLASSIFIED`
  - `ROUTED`
  - `PLANNED`
  - `EXECUTION_PREPARATION`
  - `EXECUTING`
  - `WAITING_FOR_TOOL`
  - `WAITING_FOR_APPROVAL`
  - `RESUMED`
  - `VALIDATING`
  - `COMPLETED | FAILED | CANCELLED | ESCALATED`
- Lane lifecycle:
  - `packet_created`
  - `packet_activated`
  - `lane_open`
  - `lane_blocked`
  - `lane_completed`
  - `lane_superseded`
  - `closure_submitted`
  - `closure_admitted | closure_rejected`
- Lifecycle rule:
  - request lifecycle and lane lifecycle are related but not identical
  - request state is business/runtime progress
  - lane state is delegated execution governance
  - both must be explicit, persisted, replayable, auditable, and version-aware

### Mandatory Missing-Capability Ledger
| Capability | Current gap | Required owner by V1 | Required proof |
| --- | --- | --- | --- |
| Stateful lane completion | `agent_lane` stops at `packet_ready` and does not return authoritative completion | `TaskFlow` execution/run-graph domain | contract tests for lane-open, completion, block, supersession |
| Exception-path governance | root takeover relies on prompt discipline instead of receipt-backed authority | `TaskFlow` closure/run-graph domain | fail-closed tests showing takeover blocked without recorded exception receipt |
| `execution_preparation` enforcement | planning can still hand raw work toward implementation in practice | `TaskFlow` planning/execution-preparation domain | tests that code-shaped work is blocked without handoff packet |
| Expanded lifecycle model | current flow underrepresents authorization, approval waiting, resume, validation | shared contracts + `TaskFlow` runtime state | persisted lifecycle transitions and replayability proofs |
| Thin launcher shell | `main.rs` and `state_store.rs` still own too much runtime law | `vida` shell plus extracted domains | line-count and dependency reductions with command/routing tests still green |
| Registry contracts | agent/tool/eval/ADR surfaces are not yet first-class runtime contracts | control plane + activation/governance domains | schema/contract tests and bundle inclusion proofs |
| Human-in-the-loop approval contract | approval exists as a concept but not yet as a fully explicit cross-layer runtime contract | `taskflow-closure` plus shared contracts | approval-object tests, timeout/delegation/rejection flow proofs |
| Tool governance contract | tool execution policy is not yet formalized enough across routing, auth, and side effects | control plane + tool/policy contracts | schema-validation and policy-before-action test coverage |
| Memory governance contract | memory scopes and approval-sensitive writes are not yet carried as first-class runtime contracts | control plane + governance contracts | retention/scope classification tests and guarded-write proofs |
| Guardrail and policy-decision logging | guardrails are present conceptually but not yet modeled as a full plane with logged policy decisions | closure/governance plus observability | policy-decision log coverage and blocked-side-effect traces |
| Observability and evaluation | host telemetry exists, but platform trace/eval model is incomplete | observability/evaluation domain | correlation-id, replay, and benchmark-record proofs |
| Seam closure hardening | `TaskFlow -> DocFlow -> closure` is real but not fully contract-closed | seam owners in `TaskFlow` and `DocFlow` | end-to-end seam tests with machine-readable verdict ingestion |
| Vendor-neutral deployment shape | local runtime works, but deployable-unit and version-pinning contracts are still thin | control plane + governance | bundle/deployment metadata tests and release-surface auditability |

### Integration Points
- APIs:
  - `vida` CLI shell
  - `vida taskflow ...`
  - `vida docflow ...`
  - future registry and lane-completion surfaces
  - model/tool adapters and policy gateways beneath the control plane
- Runtime-family handoffs:
  - `TaskFlow` activates `DocFlow` explicitly at the trust/closure boundary
  - `execution_preparation` must feed implementation lanes with explicit change boundaries and proof targets
  - host-agent completion must flow back into `TaskFlow` through receipts instead of chat-only summaries
- Cross-document / cross-protocol dependencies:
  - architecture anchor: `compiled-autonomous-delivery-runtime-architecture.md`
  - runtime-layer law: `canonical-runtime-layer-matrix.md`
  - activation/config: `project-activation-and-configurator-model.md`
  - execution-preparation: `execution-preparation-and-developer-handoff-model.md`
  - seam closure: `release-1-seam-map.md`
  - reality baseline: `release-1-current-state.md`

### Target Workspace Shape
- Keep and extend:
  - `crates/docflow-*`
  - `crates/taskflow-core`
  - `crates/taskflow-contracts`
  - `crates/taskflow-state`
  - `crates/taskflow-state-fs`
  - `crates/taskflow-state-surreal`
- Required dedicated crates by Release 1 closure:
  - `crates/vida`
    - thin CLI shell only
  - `crates/vida-control-bundle`
    - compiled control-plane bundle schema, compiler, cache-delivery contract, activation projection shaping
  - `crates/vida-activation`
    - project activation import/export, projection sync, project identity, host-cli activation binding
  - `crates/vida-host-agents`
    - carrier catalog, tier selection, rendered executor surfaces, feedback, budget and local observability
  - `crates/taskflow-execution`
    - request intake, planning, execution-preparation gating, dispatch, continue/advance flows, closure submission
  - `crates/taskflow-run-graph`
    - run-graph FSM, lane lifecycle, takeover gate, checkpoint/recovery, exception-path and supersession receipts
  - `crates/taskflow-closure`
    - approval/validation/closure admission, seam-facing verdict ingestion, release-closure gates
- Module-first transition split inside `crates/vida`:
  - `src/main.rs`
    - only CLI entry and command dispatch
  - `src/cli/`
    - argument parsing, render mode, JSON/text output helpers
  - `src/commands/`
    - `boot.rs`
    - `init.rs`
    - `project_activator.rs`
    - `taskflow.rs`
    - `docflow.rs`
    - `status.rs`
    - `doctor.rs`
    - `agent_feedback.rs`
  - `src/runtime_consumption/`
    - `bundle.rs`
    - `lane_selection.rs`
    - `execution_plan.rs`
    - `dispatch.rs`
    - `downstream.rs`
    - `closure_admission.rs`
  - `src/agent_runtime/`
    - `agent_init.rs`
    - `carrier_selection.rs`
    - `lane_completion.rs`
    - `exception_paths.rs`
    - `observability.rs`
  - `src/state/`
    - `mod.rs`
    - `task_store.rs`
    - `run_graph.rs`
    - `dispatch_receipts.rs`
    - `protocol_binding.rs`
    - `activation.rs`
    - `host_agent.rs`
  - `src/activation/`
    - `bundle_loader.rs`
    - `projection_sync.rs`
    - `project_protocols.rs`
- Target file/function topology by owner:
  - `crates/vida/src/main.rs`
    - `main()`
    - top-level command bootstrap only
  - `crates/vida/src/commands/*.rs`
    - parse command args into service calls
    - no business-state transitions
  - `crates/vida-control-bundle/src/lib.rs`
    - public service entrypoints for compile/load/query
  - `crates/vida-control-bundle/src/schema.rs`
    - bundle structs and serde contracts
  - `crates/vida-control-bundle/src/compiler.rs`
    - canon + activation -> compiled bundle
  - `crates/vida-control-bundle/src/registry.rs`
    - registry projections for agent/tool/ADR/evaluation catalogs
  - `crates/vida-control-bundle/src/policy_projection.rs`
    - guardrail, approval, tool, and memory policy shaping into bundle-ready contracts
  - `crates/vida-control-bundle/src/cache_keys.rs`
    - cache partitions and invalidation tuples
  - `crates/vida-control-bundle/src/cache_delivery.rs`
    - stable serving-cache artifacts under `.vida/cache/**`
  - `crates/vida-activation/src/model.rs`
    - activation entities and revision tuples
  - `crates/vida-activation/src/import.rs`
    - filesystem/project inputs -> DB-first activation state
  - `crates/vida-activation/src/export.rs`
    - DB-first activation state -> projections
  - `crates/vida-activation/src/projection_sync.rs`
    - explicit bidirectional sync logic with fail-closed conflict handling
  - `crates/vida-host-agents/src/carrier_selection.rs`
    - cheapest-healthy-capable routing logic
  - `crates/vida-host-agents/src/feedback.rs`
    - manual and automatic score feedback ingestion
  - `crates/vida-host-agents/src/observability.rs`
    - per-carrier activity, budget, and local health ledgers
  - `crates/taskflow-execution/src/intake.rs`
    - request admission and lifecycle initialization
  - `crates/taskflow-execution/src/planning.rs`
    - planning-stage state transitions and packet creation
  - `crates/taskflow-execution/src/execution_preparation.rs`
    - developer-handoff artifacts, freshness checks, and bypass policy
  - `crates/taskflow-execution/src/dispatch.rs`
    - lawful handoff into implementer/coach/verifier lanes
  - `crates/taskflow-execution/src/continue_flow.rs`
    - continuation routing with delegation-gate enforcement
  - `crates/taskflow-execution/src/advance_flow.rs`
    - downstream progression only after receipt-backed completion
  - `crates/taskflow-run-graph/src/model.rs`
    - core lane/run-graph data model
  - `crates/taskflow-run-graph/src/transitions.rs`
    - legal transitions and guard checks
  - `crates/taskflow-run-graph/src/delegation_gate.rs`
    - open-lane takeover blocking
  - `crates/taskflow-run-graph/src/lane_receipts.rs`
    - `LaneOpenReceipt`, `LaneCompletionReceipt`, `LaneBlockReceipt`
  - `crates/taskflow-run-graph/src/exception_path.rs`
    - exception-path recording and admissibility
  - `crates/taskflow-run-graph/src/supersession.rs`
    - re-route and supersession handling
  - `crates/taskflow-closure/src/approvals.rs`
    - approval objects and WAITING_FOR_APPROVAL state
  - `crates/taskflow-closure/src/policy.rs`
    - policy-decision receipts and high-risk action gating
  - `crates/taskflow-closure/src/validation.rs`
    - validation and seam verdict reconciliation
  - `crates/taskflow-closure/src/admission.rs`
    - final closure gating and rejection reasons
  - `crates/taskflow-state/src/*.rs`
    - split current state-store concentration by domain:
      - `task_store.rs`
      - `run_graph_store.rs`
      - `dispatch_receipt_store.rs`
      - `activation_store.rs`
      - `closure_store.rs`
      - `host_agent_store.rs`
      - `registry_store.rs`
  - `crates/docflow-cli/src/lib.rs`
    - must shrink toward command wiring only
  - `crates/docflow-cli/src/commands/*.rs`
    - docflow command entrypoints
  - `crates/docflow-cli/src/mutation/*.rs`
    - init/finalize/move/touch orchestration
  - `crates/docflow-cli/src/operator/*.rs`
    - operator/readiness/report rendering
- Contract-first module surfaces that may stay module-owned through V1 if they remain explicit and testable:
  - registry contracts for agent/tool/ADR/evaluation catalogs
  - observability/evaluation schemas
  - deployment metadata and version pinning contracts
- File/crate ownership rule:
  - new crate only when the interface is independently testable, free from CLI concerns, and already carries a stable owner boundary

### Bounded File Set
- `docs/product/spec/release-1-plan.md`
- `docs/product/spec/current-spec-map.md`

## Fail-Closed Constraints
- Forbidden fallback paths:
  - no big-bang rewrite of the workspace
  - no further widening of `crates/vida/src/main.rs` or `crates/vida/src/state_store.rs` as default implementation posture
  - no prompt-only fix for delegated-cycle governance
  - no silent direct `planning -> worker` path for code-shaped execution that requires `execution_preparation`
  - no shell-local ownership of final readiness/proof truth
- Required receipts / proofs / gates:
  - new phase work must always name:
    - target slice
    - target owner
    - target layer or seam segment
    - target closure class
  - `TaskFlow` must fail closed on open delegated cycles without a recorded exception path
  - `DocFlow` seam verdicts must stay explicit and machine-readable
  - new registry/evaluation/governance surfaces must remain contract-backed, not ad hoc files
- Safety boundaries that must remain true during rollout:
  - DB-first authority remains primary
  - filesystem remains synchronized projection
  - `TaskFlow` remains closure authority
  - `DocFlow` remains readiness/proof authority
  - host agents remain carriers, not truth owners
  - vendor neutrality remains preserved at the core contract level

## Implementation Plan

### Phase 1
- Keep this document as the primary Release-1 execution owner.
- Freeze new uncontrolled architecture drift:
  - no new broad features
  - no new launcher concentration
  - no non-critical speculative runtime work
- Create the first contract backlog for missing mandatory capabilities:
  - lane lifecycle contracts
  - exception-path contracts
  - compiled control bundle contract hardening
  - cross-layer request/tool/result contracts
  - registry and evaluation contracts
- First proof target:
  - project maps route Release-1 execution through this document
  - implementation packets can reference one canonical owner plan
  - supersession of the former execution-plan owners is explicit and queryable

### Phase 2
- Split `crates/vida` into bounded internal modules without changing workspace crate boundaries yet.
- Extract:
  - CLI parsing/rendering
  - runtime consumption
  - agent runtime
  - state access by domain
  - activation/projection handling
- Reduce `main.rs` to entry and command wiring only.
- Second proof target:
  - launcher behavior remains green while internal ownership boundaries become explicit
  - `main.rs` and `state_store.rs` stop growing and begin shrinking
  - command surfaces can be traced to bounded owner modules rather than one monolith

### Phase 3
- Introduce stateful agent-cycle governance:
  - lane-open receipts
  - lane completion/block/supersession receipts
  - exception-path receipts
  - takeover gate enforcement in continuation/advance flows
- Replace transitional `packet_ready` semantics with explicit lane lifecycle and evidence state.
- Enforce `execution_preparation` artifacts for code-shaped work.
- Third proof target:
  - open delegated cycles block root takeover through runtime state rather than prompt discipline
  - `execution_preparation` is enforced for code-shaped work
  - request lifecycle includes approval-wait and validation states

### Phase 4
- Extract stable domains into the target dedicated crates:
  - `vida-control-bundle`
  - `vida-activation`
  - `vida-host-agents`
  - `taskflow-execution`
  - `taskflow-run-graph`
  - `taskflow-closure`
- Keep `TaskFlow` and `DocFlow` family ownership explicit and do not collapse them into a new generic blob crate.
- Fourth proof target:
  - the shell is thin and most runtime law is no longer launcher-owned
  - execution-state ownership sits under `TaskFlow`-family crates rather than in `vida`

### Phase 5
- Harden `TaskFlow -> DocFlow -> Release 1 closure` seam:
  - explicit activation contract
  - explicit readiness/proof return contract
  - explicit closure admission contract
- Align seam outputs with machine-readable verdicts and restore/reconcile discipline.
- Fifth proof target:
  - seam segments 1-3 all move toward green with direct proof coverage
  - machine-readable readiness/proof verdicts are the only closure inputs admitted across the seam

### Phase 6
- Introduce first-class platform governance surfaces required by the broader V1 vision:
  - agent registry contract
  - tool registry contract
  - ADR/change-governance contract
  - evaluation benchmark contract
  - HITL approval contract
  - tool-governance policy contract
  - memory-governance contract
  - guardrail taxonomy and policy-decision log contract
- Keep these Release-1-bounded:
  - contract first
  - runtime surfaces second
  - broad product UX later
- Sixth proof target:
  - V1 architecture no longer blocks the broader platform direction recorded in Airtable and local product canon
  - registries and evaluation artifacts can be versioned, queried, and embedded into compiled control bundles

### Phase 7
- Complete hardening:
  - observability and evaluation
  - deployment and operator readiness
  - release closure evidence
  - final capability-matrix and seam-map alignment
- Final proof target:
  - Release 1 can close as one coherent, compiled, DB-first, state-disciplined autonomous delivery runtime
  - `release-1-capability-matrix.md` and `release-1-seam-map.md` can be read as proof dashboards rather than planning owners

## Validation / Proof
- Unit tests:
  - new contract tests for lane lifecycle, exception paths, and closure admission
  - crate-local tests for extracted domains after modularization
- Integration tests:
  - `TaskFlow -> host agent -> completion receipt -> coach/verifier -> closure`
  - `TaskFlow -> DocFlow -> readiness/proof -> closure admission`
  - project activation and compiled control-bundle hot path
- Runtime checks:
  - `vida orchestrator-init --json`
  - `vida agent-init --json`
  - `vida taskflow consume final|continue|advance`
  - `vida status --json`
  - `vida doctor --json`
- Canonical checks:
  - `vida docflow activation-check --root . docs/product/spec/release-1-plan.md docs/product/spec/current-spec-map.md`
  - `vida docflow protocol-coverage-check --profile active-canon`
  - `vida docflow check --root . docs/product/spec/release-1-plan.md docs/product/spec/current-spec-map.md`
  - `vida docflow doctor --root .`

## Observability
- Logging points:
  - lane activation
  - lane completion/block/supersession
  - exception-path recording
  - seam activation and verdict consumption
  - compiled bundle compilation/invalidation
- Metrics / counters:
  - open delegated cycles
  - blocked takeovers
  - seam pass/block counts
  - bundle compile counts
  - evaluation benchmark runs
  - host-agent success/failure by carrier tier
- Receipts / runtime state written:
  - run-graph status
  - dispatch receipts
  - lane lifecycle receipts
  - exception-path receipts
  - readiness/proof verdicts
  - closure admission verdicts
  - host-agent observability ledger

## Rollout Strategy
- Development rollout:
  - adopt the new master plan first
  - follow with module split
  - then stateful lane governance
  - then crate extraction
  - then seam hardening and governance/evaluation surfaces
- Migration / compatibility notes:
  - active Release-1 routing must depend on this document and its declared companion surfaces only
  - bridge-era runtime behaviors may temporarily remain as migration supports, but they must not regain long-term owner status
- Operator or user re-entry requirements:
  - the operator should be able to re-enter Release-1 routing from this document alone without reconstructing execution posture from chat history or scattered docs
  - if a future plan attempts to become a Release-1 execution owner, it must explicitly supersede this document through the same current-spec-map change set

## Future Considerations
- Follow-up ideas:
  - Release-2 reactive synchronization and host-project domain routing
  - richer policy engines and approval workflows
  - expanded registry-backed UI/admin surfaces
- Known limitations:
  - this plan intentionally fixes the V1 end-state and migration order, but it does not itself implement the missing runtime contracts
  - the current workspace still contains bridge-era and launcher-heavy ownership concentration until follow-on implementation slices land
- Technical debt left intentionally:
  - crate extraction is deferred until module boundaries stabilize through the first bounded carve-out phases

## References
- Related specs:
  - `docs/product/spec/compiled-autonomous-delivery-runtime-architecture.md`
  - `docs/product/spec/canonical-runtime-layer-matrix.md`
  - `docs/product/spec/release-1-capability-matrix.md`
  - `docs/product/spec/release-1-seam-map.md`
  - `docs/product/spec/release-1-current-state.md`
  - `docs/product/spec/project-activation-and-configurator-model.md`
  - `docs/product/spec/execution-preparation-and-developer-handoff-model.md`
  - `docs/product/spec/operational-state-and-synchronization-model.md`
  - `docs/product/spec/runtime-paths-and-derived-cache-model.md`
  - `docs/product/spec/host-agent-layer-status-matrix.md`
- Related protocols:
  - `runtime.runtime-kernel-bundle-protocol`
  - `runtime.direct-runtime-consumption-protocol`
  - `work.taskflow-protocol`
  - `work.project-agent-extension-protocol`
  - verification and handoff protocols already promoted in current runtime law
- Related ADRs:
  - none yet; follow-up ADRs are recommended for lane-completion protocol and other durable control-plane decisions that need standalone history
- External references:
  - Airtable `Vida` base, `Spec` table, records refreshed on `2026-03-16`:
    - `System Purpose`
    - `Architectural Principles`
    - `Logical Architecture`
    - `Component Model`
    - `Canonical Agent Model`
    - `Execution Lifecycle`
    - `Cross-Layer Contracts`
    - `Knowledge and Retrieval Specification`
    - `Deployment Architecture`
    - `Agent Catalog Specification`
    - `Tool Catalog Specification`
    - `Architecture Decisions (ADR) Specification`
    - `Evaluation Benchmarks Specification`
    - vendor alignment matrices for OpenAI, Microsoft, AWS, Google Cloud, and Databricks / Snowflake

-----
artifact_path: product/spec/release-1-plan
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-03
schema_version: 1
status: canonical
source_path: docs/product/spec/release-1-plan.md
created_at: 2026-03-16T07:39:24.117630799Z
updated_at: 2026-04-03T18:40:00+03:00
changelog_ref: release-1-plan.changelog.jsonl
