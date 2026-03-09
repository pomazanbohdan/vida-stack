# VIDA 0.3 Instruction Kernel Spec

Purpose: define the authoritative instruction model for direct `1.0`, freeze the instruction semantics that must survive the rewrite, and separate instruction-owned behavior from provider/rendering topology and from the command, state, route, and migration kernels.

Status: canonical `0.3` spec artifact for the direct `1.0` program.

Date: 2026-03-08

---

## 1. Executive Decision

The direct `1.0` instruction kernel freezes exactly three authoritative instruction entities:

1. `Agent Definition`
2. `Instruction Contract`
3. `Prompt Template Configuration`

Direct `1.0` instruction law is:

1. `Agent Definition` is the umbrella runtime object,
2. `Instruction Contract` is the canonical logic source,
3. `Prompt Template Configuration` is the rendering/configuration layer only,
4. effective instruction behavior is composed from explicit activation, precedence, validated overlay inputs, and referenced instruction entities,
5. no implied behavior is allowed,
6. fallback, escalation, output, and proof obligations are instruction-owned behavior,
7. provider/prompt/transport surfaces are topology, not product law.

The instruction kernel does **not** own:

1. root command families or command syntax,
2. authoritative task/workflow state,
3. route/authorization/receipt schemas,
4. migration procedures or rollback execution,
5. shell/Python/provider transport topology.

Compact rule:

`freeze instruction entities, precedence, activation, and explicit behavior law; keep rendering and transport topology discardable`

---

## 2. Why This Spec Comes Next

The command-tree spec already froze the future operator families, and the state-kernel spec already froze the authoritative state facts those commands will read and mutate.

The next blocker is the missing instruction model that tells direct `1.0`:

1. which instruction entities are authoritative,
2. how instruction behavior is versioned and composed,
3. how overlays participate without becoming logic owners,
4. where explicit fallback, escalation, output, and proof law live,
5. where instruction ownership ends so later route/receipt and migration work can start cleanly.

Without this artifact:

1. command behavior would still depend on scattered protocol prose,
2. prompt rendering could silently become the logic source,
3. migration would not know which instruction surfaces are versioned and compatibility-bearing,
4. route/receipt work could incorrectly absorb instruction-owned proof and escalation semantics.

---

## 3. Source Basis

Primary local source basis:

1. `AGENTS.md`
2. `docs/framework/ORCHESTRATOR-ENTRY.MD`
3. `docs/framework/thinking-protocol.md`
4. `vida.config.yaml`
5. `docs/framework/history/research/2026-03-08-agentic-master-index.md`
6. `docs/framework/history/research/2026-03-08-vida-direct-1.0-next-agent-compact-instruction.md`
7. `docs/framework/history/research/2026-03-08-vida-instruction-kernel-next-step-after-compact-instruction.md`
8. `docs/framework/history/plans/2026-03-08-vida-0.3-command-tree-spec.md`
9. `docs/framework/history/plans/2026-03-08-vida-0.3-state-kernel-schema-spec.md`
10. `docs/framework/history/plans/2026-03-08-vida-0.1-to-1.0-direct-binary-transition-plan.md`
11. `docs/framework/history/plans/2026-03-08-vida-semantic-extraction-layer-map.md`
12. `docs/framework/history/plans/2026-03-08-vida-0.2-semantic-freeze-spec.md`
13. `docs/framework/history/plans/2026-03-08-vida-0.2-bridge-policy.md`
14. `docs/framework/history/plans/2026-03-08-vida-direct-1.0-local-spec-program.md`
15. `docs/framework/history/research/2026-03-08-agentic-agent-definition-system.md`
16. `docs/framework/agent-definition-protocol.md`
17. `docs/framework/templates/instruction-contract.yaml`
18. `docs/framework/templates/prompt-template-config.yaml`
19. `docs/framework/project-overlay-protocol.md`
20. `docs/framework/instruction-activation-protocol.md`
21. `docs/framework/agent-system-protocol.md`
22. `docs/framework/framework-memory-protocol.md`
23. `docs/framework/silent-framework-diagnosis-protocol.md`

Bounded explorer lanes used for synthesis:

1. canonical instruction entities, hierarchy rules, and frozen invariants,
2. precedence, activation, overlay, and effective-composition inputs,
3. topology leakage and cross-kernel boundary inventory.

Source synthesis rule:

1. the semantic freeze defines what instruction law must survive,
2. the agent-definition system and protocol define the canonical entities and minimum contracts,
3. the activation and overlay protocols define how instruction surfaces become active and validated,
4. the command-tree and state-kernel specs define the downstream boundaries the instruction kernel must not absorb.

---

## 4. Purpose Of The Instruction Kernel Spec

This artifact answers one question:

1. what authoritative instruction entities and composition law must the direct `1.0` binary own before migration, route/receipt, and conformance work can be finalized?

This spec defines:

1. authoritative instruction entities,
2. hierarchy and ownership,
3. versioning law,
4. precedence and activation law,
5. effective instruction composition law,
6. explicit instruction-owned fallback, escalation, output, and proof law,
7. boundaries against topology and against other kernels,
8. instruction-level invariants and non-goals.

This spec does not define:

1. exact provider prompt bodies,
2. exact CLI/provider transport wiring,
3. exact command syntax below frozen root families,
4. exact state schema or lifecycle enums,
5. exact route/receipt payloads,
6. exact migration procedures or rollback execution,
7. exact Rust module/storage layout.

---

## 5. Authoritative Instruction Entities

Direct `1.0` freezes exactly three authoritative instruction entities.

### 5.1 `Agent Definition`

Purpose:

1. umbrella runtime object for agent behavior,
2. stable assembly boundary for role identity, behavior, rendering, permissions, output, and conformance,
3. the authoritative reference that selects which instruction contract and prompt-template configuration belong together.

Minimum semantic contents:

1. identity / role class,
2. `instruction_contract_ref`,
3. `prompt_template_config_ref`,
4. tool/permission policy linkage,
5. output/proof contract linkage,
6. version metadata,
7. conformance/eval hooks.

Rule:

1. `Agent Definition` owns assembly, not lower-level behavioral logic.

### 5.2 `Instruction Contract`

Purpose:

1. canonical behavior law,
2. normative source for what the agent must do, must not do, and what evidence it needs,
3. semantic owner of deterministic behavior, fallback, escalation, output, and proof obligations.

Minimum semantic contents:

1. `contract_id`
2. `version`
3. `role_id`
4. `mission`
5. `scope_boundary`
6. `mandatory_reads`
7. `input_contract`
8. `decision_rules`
9. `allowed_actions`
10. `forbidden_actions`
11. `tool_permission_policy`
12. `fallback_ladder`
13. `escalation_rules`
14. `output_contract`
15. `proof_requirements`

Rule:

1. `Instruction Contract` is the canonical logic source.

### 5.3 `Prompt Template Configuration`

Purpose:

1. render/config layer for a concrete runtime, provider, or templating target,
2. bridge from provider-neutral behavior into provider-specific prompt materialization,
3. configuration owner for template syntax, bindings, and runtime-facing prompt assembly.

Minimum semantic contents:

1. `config_id`
2. `version`
3. `instruction_contract_ref`
4. `rendering_target`
5. `template_format`
6. `system_prompt_template`
7. `parameter_bindings`
8. `runtime_bindings`
9. `tool_exposure`
10. `output_rendering`

Rule:

1. `Prompt Template Configuration` renders the contract and must not redefine behavior.

---

## 6. Supporting Inputs And Derived Surfaces

The following surfaces are instruction-kernel-adjacent, but they are not peer authoritative entities in the canonical three-term hierarchy.

### 6.1 Supporting Inputs

Supporting inputs:

1. `Role Profile`
2. validated overlay data from `vida.config.yaml`
3. framework-owned activation and precedence surfaces
4. tool/permission, output, and fallback/escalation subcontracts referenced by the authoritative entities

Rules:

1. `Role Profile` remains an upstream identity/stance input, not the full runtime behavior object.
2. Overlay data is project-owned input data, not the owner of instruction logic.
3. Framework-owned activation and precedence law stays above project overlay inputs.

### 6.2 `Effective Instruction Bundle`

Direct `1.0` needs one provider-neutral derived runtime surface:

1. `Effective Instruction Bundle`

Purpose:

1. resolve the active instruction surfaces for one lane/route/phase,
2. make precedence resolution inspectable,
3. carry the resolved behavior into rendering without turning rendered prompts into the logic source.

Minimum contents:

1. active instruction-surface references,
2. resolved `Agent Definition`, `Instruction Contract`, and `Prompt Template Configuration` references,
3. source-version tuple,
4. validated overlay bindings that were actually applied,
5. resolved decision/fallback/escalation/output/proof surfaces,
6. activation class and lane/route context,
7. precedence-resolution result,
8. mandatory chained instruction reads in resolved order,
9. optional triggered instruction reads separated from mandatory boot chain.

Rule:

1. `Effective Instruction Bundle` is derived runtime state, not a fourth peer logic authority.
2. the bundle may include resolved sidecar effects, but those effects must remain explicit and inspectable.

---

## 7. Canonical Hierarchy And Ownership

The canonical hierarchy is frozen as:

1. `Agent Definition`
2. `Instruction Contract`
3. `Prompt Template Configuration`

Ownership split:

1. `Agent Definition` owns assembly and version boundary,
2. `Instruction Contract` owns behavioral logic,
3. `Prompt Template Configuration` owns rendering/configuration only,
4. `Role Profile` is upstream identity input,
5. overlay data is validated parameter input,
6. rendered prompt/provider transport is downstream topology.

Hard rules:

1. provider text must not become the hidden logic source,
2. tone/persona wording must not hide authority,
3. output and proof obligations must remain inspectable after rendering,
4. future artifacts must preserve this hierarchy unless a higher-precedence spec explicitly replaces it,
5. durable storage for these artifacts belongs in the instruction-memory slice, not in project memory or transcript state.

---

## 8. Versioning Law

Direct `1.0` instruction versioning must follow these rules:

1. every authoritative instruction entity is explicitly versioned,
2. behavior-changing changes require a new `Instruction Contract` version,
3. assembly/reference changes require a new `Agent Definition` version,
4. render/config-only changes require a new `Prompt Template Configuration` version,
5. a render/config change that would alter behavior is invalid unless the referenced `Instruction Contract` or `Agent Definition` version also changes accordingly,
6. overlay version values remain input metadata and do not replace instruction-entity versioning,
7. the `Effective Instruction Bundle` must record the exact source-version tuple it was built from,
8. missing, incompatible, or unresolved version references are blocking and must fail closed.

Versioning boundary:

1. this artifact freezes the existence and responsibility of versioned instruction surfaces,
2. exact migration procedures for those versions belong to the migration kernel,
3. durable version records and immutable framework-bundled instruction rows belong to the instruction-memory slice.

---

## 9. Precedence And Activation Law

### 9.1 Precedence

The frozen instruction precedence order that must survive is:

1. `AGENTS.md`
2. lane-entry contract
3. active canonical domain protocol
4. validated overlay data
5. command docs / wrappers
6. helper or script behavior

Rules:

1. lower-precedence conflicts are drift, not alternative valid paths,
2. overlay data may parameterize behavior, but may not weaken framework invariants,
3. `Instruction Contract` outranks `Prompt Template Configuration` within the kernel-owned entity stack,
4. provider/runtime rendering or transport data has zero authority to redefine behavior.

### 9.2 Activation

Activation law is frozen around exactly four classes:

1. `always_on`
2. `lane_entry`
3. `triggered_domain`
4. `closure_reflection`

Rules:

1. only activated instruction surfaces participate in effective composition,
2. instruction surfaces are activated by phase and trigger, not by broad reread,
3. each instruction surface must have one clear activation class,
4. mixed-phase instruction ownership is invalid and should be refactored,
5. the boot read-set must not silently widen beyond the trigger matrix.

---

## 10. Effective Instruction Composition Law

The direct `1.0` composition path is:

1. determine current lane, phase, route, and active triggers,
2. activate the relevant instruction surfaces by activation class,
3. apply frozen precedence across those active surfaces,
4. select the relevant `Agent Definition`,
5. load the referenced `Instruction Contract`,
6. validate overlay input and bind only allowlisted overlay values,
7. attach the referenced `Prompt Template Configuration`,
8. resolve mandatory chained instruction references required by the selected artifacts,
9. resolve active sidecars attached to the selected instruction artifacts,
10. produce one provider-neutral `Effective Instruction Bundle`,
11. render that bundle for the chosen runtime/provider.

Composition rules:

1. activation happens before precedence resolution,
2. precedence resolution happens before overlay binding,
3. overlay binding happens before rendering,
4. rendering materializes behavior but does not author it,
5. unresolved conflicts, missing mandatory inputs, invalid overlay schema, or logic drift must fail closed,
6. rendered provider text is an output of composition, not the canonical input to future composition,
7. effective composition must remain inspectable from explicit sources rather than transcript memory,
8. sidecars may alter effective composition only through explicit recorded sidecar effects,
9. immutable base instruction artifacts may be projected through sidecar patch operations, but not mutated in place,
10. mandatory follow-on instruction reads must be auto-resolved and emitted in deterministic order once their base instruction is selected,
11. optional or trigger-only instruction reads must not contaminate the mandatory chain unless their activation condition is satisfied.

---

## 11. Instruction-Owned Behavior Law

Instruction-owned behavior is the part of runtime law that belongs in the `Instruction Contract` rather than in downstream route, state, or migration kernels.

### 11.1 Fallback

Fallback law must be:

1. explicit,
2. ordered,
3. condition-driven,
4. bounded by scope and authority.

Hard rules:

1. no fallback may silently widen scope,
2. no fallback may invent undefined behavior,
3. if no lawful fallback exists, escalation is mandatory,
4. if escalation is unavailable, fail closed.

### 11.2 Escalation

Escalation law must declare:

1. trigger,
2. route or owner,
3. blocking condition,
4. fail-closed behavior when escalation cannot proceed.

Instruction-owned escalation covers:

1. ambiguity on scope or authority,
2. missing law-bearing instruction inputs,
3. invalid instruction/overlay composition,
4. no lawful fallback.

Boundary:

1. route-specific authorization and escalation receipts belong later to the route/receipt kernel.

### 11.3 Output

Output law must declare:

1. required format,
2. required sections or schema,
3. downstream readability for verification,
4. user-facing synthesis expectations.

Rules:

1. output obligations belong to the instruction contract,
2. prompt-template rendering may format them, but must not change the obligation,
3. raw worker output is not the default user-facing deliverable when orchestrator synthesis is required.

### 11.4 Proof

Proof law must declare:

1. what evidence classes are required before completion,
2. which outputs remain inspectable after rendering,
3. which blocking conditions prevent completion when proof is missing.

Boundary:

1. instruction law owns the requirement that proof exist,
2. exact receipt schemas, approval identities, and authorization payloads belong later to route/receipt law.

---

## 12. Boundaries With Topology And Other Kernels

### 12.1 Instruction Semantics Vs Provider / Rendering / Prompt Transport Topology

Direct `1.0` instruction semantics must preserve:

1. authoritative entities,
2. hierarchy and ownership,
3. versioning and precedence,
4. activation and effective composition,
5. explicit fallback/escalation/output/proof law.

Direct `1.0` must **not** freeze as instruction law:

1. `rendering_target`
2. `template_format`
3. provider-specific prompt scaffolding
4. concrete template syntax
5. runtime/provider bindings such as model hints or temperature modes
6. tool exposure transport wiring
7. CLI dispatch commands, flags, output modes, prompt modes, cache paths, or probe flags
8. packet-rendering helper scripts
9. current file/path layouts for prompt packets
10. shell/Python/provider transport boundaries

Topology rule:

1. rendering and transport are allowed to change as long as canonical instruction behavior does not.

### 12.2 Boundary With Command Kernel

The command kernel owns:

1. root command families,
2. command-family boundaries,
3. command syntax and command-home semantics,
4. which root family owns mutation or diagnosis.

The instruction kernel may own:

1. behavior bound to those command families,
2. provider-neutral command capsules or effective instruction bundles.

It must not own:

1. root-family creation,
2. root-family syntax,
3. mutation-home decisions already frozen by command-tree law.

### 12.3 Boundary With State Kernel

The state kernel owns:

1. authoritative lifecycle state,
2. dependency and blocker posture,
3. execution telemetry,
4. run-graph and resumability state,
5. governance and reconciliation state.

The instruction kernel may:

1. read those state surfaces,
2. declare behavior that depends on them.

It must not:

1. redefine state vocabularies,
2. become a second state engine,
3. absorb state mutation law.

### 12.4 Boundary With Route / Receipt Kernel

The route/receipt kernel owns:

1. authorization law,
2. analysis/writer/coach/verifier/approval gating,
3. receipt schemas,
4. approval/escalation/verification payloads,
5. closure-ready proof artifacts.

The instruction kernel owns only:

1. fallback and escalation semantics that must exist,
2. output and proof requirements that must be declared.

It must not own:

1. exact route receipt schema,
2. exact approval payloads,
3. exact verification artifact schema,
4. exact closure authorization law.

### 12.5 Boundary With Migration Kernel

The migration kernel owns:

1. migration states,
2. compatibility checks,
3. fail-closed startup behavior for incompatible versions,
4. rollback notes and execution path,
5. migration receipts,
6. bridge import/export from `0.1`.

The instruction kernel owns only:

1. the versioned instruction surfaces that migration must later move or validate,
2. the compatibility-bearing boundaries of those surfaces.

It must not own:

1. migration procedure,
2. cutover execution,
3. rollback execution,
4. startup repair flow.

---

## 13. Instruction-Level Invariants

The direct `1.0` instruction kernel must preserve these invariants:

1. the authoritative instruction hierarchy is exactly `Agent Definition -> Instruction Contract -> Prompt Template Configuration`,
2. `Instruction Contract` is the canonical logic source,
3. `Prompt Template Configuration` never becomes the logic source,
4. undefined behavior is forbidden by default,
5. no implied behavior is allowed,
6. no silent autonomy expansion is allowed,
7. fallback and escalation are explicit and ordered,
8. output and proof obligations remain inspectable after rendering,
9. activation is phase/trigger-based, not broad bulk loading,
10. overlay data cannot weaken framework invariants,
11. provider/render/transport topology cannot become product law,
12. instruction semantics remain separate from command, state, route/receipt, and migration kernels,
13. effective instruction behavior must remain reconstructible from explicit artifacts rather than transcript memory,
14. future instruction artifacts and next-step prompts must carry forward behavioral inheritance explicitly.

---

## 14. Instruction-Level Non-Goals

This spec does not:

1. define exact provider prompt bodies,
2. define exact provider/model/runtime selection policy,
3. define exact command syntax below the frozen command tree,
4. define authoritative task/workflow state,
5. define route/receipt payload schemas,
6. define migration procedures or rollback execution,
7. define prompt-packet file layout,
8. preserve shell/Python/provider transport topology,
9. widen the program into `MCP`, `A2A`, `A2UI`, remote identity, gateways, or remote memory,
10. start Rust implementation work.

---

## 15. Open Ambiguities

The following remain intentionally open:

1. the exact serialized storage shape for `Agent Definition`, `Instruction Contract`, and `Prompt Template Configuration` inside the direct `1.0` binary,
2. the exact normalized schema for `Effective Instruction Bundle`,
3. the exact cross-entity version-negotiation algorithm across instruction entities and overlay versions,
4. the exact allowlist of overlay fields that may parameterize instruction behavior beyond the currently known overlay surfaces,
5. the precise line between command capsules defined by the instruction kernel and command syntax defined by the command tree,
6. the exact receipt schemas that later prove instruction-owned proof and escalation obligations were satisfied,
7. the exact migration states and compatibility receipts that will move versioned instruction surfaces at boot.

Ambiguity rule:

1. later specs may refine these details,
2. they must not violate the hierarchy and ownership freeze defined here.

---

## 16. Downstream Contracts Unlocked By This Spec

This artifact unlocks:

1. `Migration Kernel Spec`
   - because the instruction surfaces that need versioning, compatibility checks, and migration are now frozen.
2. `Route And Receipt Spec`
   - because instruction-owned fallback/escalation/output/proof law is now separated from route-owned authorization and receipt schema.
3. `Parity And Conformance Spec`
   - because authoritative instruction entities, version boundaries, and effective-composition invariants can now be tested.
4. later `Memory` and `Doctor` runtime contracts
   - because future boot/doctor behavior can now rely on a frozen instruction compatibility boundary without redefining instruction semantics.

---

## 17. Immediate Next Artifact

The next artifact is:

1. `docs/framework/history/plans/2026-03-08-vida-0.3-migration-kernel-spec.md`

Reason:

1. the command surface is frozen,
2. the state model is frozen,
3. the instruction hierarchy and composition law are now frozen,
4. the next blocker is defining how `0.1` artifacts and versions migrate safely into direct `1.0` with fail-closed startup checks and bounded rollback law.
