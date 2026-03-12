# Framework Map Protocol (FMP)

Purpose: define the canonical framework topology, layer model, artifact taxonomy, and promotion/projection rules beneath the top-level `vida/` root map.

Primary framework root:

1. `vida/root-map.md`

This file answers five topology questions:

1. where each major directory belongs,
2. which artifact class each document/config family represents,
3. what is active canon vs transitional vs historical,
4. how `0.2.0` and `1.0` relate to one shared runtime-spec spine,
5. how artifacts move from plan -> spec -> executable law.

## 1. Catalog Map

Top-level repository layout:

1. `AGENTS.md`
   - bootstrap router and cross-lane invariants,
   - framework-owned,
   - active canon.
2. `vida/config/instructions/`
   - active framework instruction canon in flat latest-revision Markdown form plus executable projections,
   - canonical home for agent instructions, command instructions, runtime instructions, diagnostic instructions, and system maps.
3. `docs/product/spec/`
   - promoted stable product prose canon.
4. `vida/config/`
   - executable product law and runtime-readable projections.
5. `docs/process/framework-source-lineage-index.md`
   - project-owned provenance index for deleted framework-formation plan/research inputs,
   - not active framework law.
6. `taskflow-v0/`
   - transitional implementation runtime,
   - current execution substrate for the `0.2.0` line.
7. `codex-v0/`
   - bounded transitional `DocFlow` donor/documentation/operator runtime surface,
   - independently usable, but still part of the unified VIDA framework runtime family.
8. additional runtime families may exist or be added later,
   - each must remain both independently understandable and discoverable through the unified VIDA framework map layer.
9. `docs/project-memory/`
   - Git-resident source tree for project-memory artifacts.
10. `docs/process/`
   - project operational runbooks when present,
   - including project-owned development/build/install/run-condition records such as `docs/process/vida1-development-conditions.md`, kept current through `vida/config/instructions/runtime-instructions/work.development-evidence-sync-protocol.md`.
11. `projects/`
   - extracted or quarantined secondary project bundles preserved locally during repository cleanup or staged separation,
   - not part of the default active project-doc bootstrap path.

Rule:

1. directory placement is semantic, not cosmetic,
2. no artifact should exist in two active homes with equal authority,
3. if the same concept appears in multiple places, exactly one location must be canonical and the others must be projections, pointers, or evidence.

Bootstrap routing rule:

1. framework bootstrap resolves downstream documentation discovery through framework-owned maps first,
2. `AGENTS.md` and `AGENTS.sidecar.md` must not directly bear project/product document discovery details beyond routing to the correct framework-owned map/index surface,
3. once this framework map resolves the owner layer, bootstrap may continue into `docs/product/**` or `docs/process/**` as discovered targets rather than as implicit bootstrap knowledge.

Runtime-family map rule:

1. the unified framework map must expose all active runtime families,
2. each runtime family such as `docflow`, `taskflow`, or a future runtime must have a bounded map/discovery surface of its own,
3. runtime-family discoverability must not depend on ad hoc filesystem guessing,
4. one runtime family must not silently absorb the identity of the others.
5. current runtime-family discovery entrypoint: `vida/config/instructions/system-maps/runtime-family.index.md`

Tooling routing rule:

1. concrete documentation/runtime tooling discovery does not belong in this topology map,
2. use `vida/config/instructions/system-maps/runtime-family.index.md` for runtime-family routing,
3. use `vida/config/instructions/system-maps/template.map.md` for template routing,
4. use `vida/config/instructions/system-maps/governance.map.md` for policy/gate discovery.

Core-cluster routing rule:

1. bounded discovery and stitching for the framework `core cluster` does not belong inside the core protocols themselves,
2. use `vida/config/instructions/system-maps/framework.core-protocols-map.md` when the task is about how the core protocols fit together as one package,
3. keep tooling, project-environment notes, and backend-specific lifecycle law outside the core cluster map and outside the core protocols.

Protocol-domain routing rule:

1. domain-level classification of protocol-bearing artifacts does not belong inside `protocol.index` rows or owner-layer maps,
2. use `vida/config/instructions/system-maps/framework.protocol-domains-map.md` when the task is about which protocol family a topic belongs to, especially when distinguishing orchestration architecture from adjacent protocol families such as thinking, documentation, diagnostics, naming, or artifact governance,
3. keep this topology map as the higher framework architecture owner and the protocol-domains map as the thinner domain-classification surface.

Protocol-layer routing rule:

1. one-pass placement of a protocol-bearing artifact into the correct owner layer does not belong inside individual protocols,
2. use `vida/config/instructions/system-maps/framework.protocol-layers-map.md` when the task is about whether an artifact belongs to framework canon, agent-role, bootstrap/environment, human-governance, or project documentation,
3. keep this topology map as the higher framework architecture owner and the protocol-layers map as the thinner layer-placement surface.

Development-evidence routing rule:

1. when implementation/build/install work just succeeded and that success changes what can now be run, built, installed, or verified, activate `vida/config/instructions/runtime-instructions/work.development-evidence-sync-protocol.md`,
2. route the resulting project-owned mutation to `docs/process/vida1-development-conditions.md` unless the active project overlay names a more specific target,
3. do not postpone this routing until the end of a larger wave once the condition is already proven.

Document maturity rule:

1. active canonical documents are working artifacts by default unless a stricter completion rule is stated elsewhere,
2. current framework/product/instruction documents are expected to continue evolving during work on VIDA `0.2.0` and VIDA `1.0.0`,
3. this ongoing status is a policy-level fact and must be recorded in canonical maps/specs, not spammed as repetitive per-file changelog noise.

Governance boundary rule:

1. root-bootstrap exceptions, contribution/publication rules, approval gates, and lifecycle policies belong to the governance map,
2. keep only topology, layer, and artifact-boundary rules here.

Template-map rule:

1. template discovery must be available through an explicit template-map surface,
2. top-level framework discovery must point to that template-map surface,
3. template families must remain distinguishable by owner and by activation trigger.
4. current template discovery entrypoint: `vida/config/instructions/system-maps/template.map.md`

Map trigger rule:

1. every active framework map or map-like index must state explicit `Activation Triggers`,
2. those triggers must describe when the map should be read, not merely restate its title,
3. every active framework map or map-like index must also state explicit `Routing` or equivalent next-hop behavior,
4. trigger text must be concrete enough to distinguish topology lookup, protocol lookup, runtime-family lookup, governance lookup, template lookup, observability lookup, and project-doc handoff,
5. adding a new active map without explicit trigger and routing language is blocking framework-map drift.

## 2. Layer Map

VIDA uses one normalized documentation/runtime stack:

1. `Bootstrap Layer`
   - `AGENTS.md`
   - `vida/config/instructions/agent-definitions/entry.orchestrator-entry.md`
   - `vida/config/instructions/agent-definitions/entry.worker-entry.md`
   - `vida/config/instructions/instruction-contracts/role.worker-thinking.md`
2. `Instruction Artifact Layer`
   - `vida/config/instructions/**`
   - active latest-revision Markdown canon plus adjacent changelogs and projections
3. `Framework Map / Index Layer`
   - `vida/root-map.md`
   - this file
   - `vida/config/instructions/system-maps/framework.protocol-domains-map.md`
   - `vida/config/instructions/system-maps/protocol.index.md`
   - `vida/config/instructions/system-maps/governance.map.md`
   - thin indexes/pointers only
4. `Product Spec Layer`
   - `docs/product/spec/**`
   - stable promoted product law
5. `Executable Law Layer`
   - `vida/config/**`
   - machine-readable law consumed by runtime
6. `Implementation Layer`
   - `taskflow-v0/**`
   - current transitional implementation
   - future `vida 1.0` implementation must be a Rust workspace with reusable crates, not only one CLI-bound binary crate
## 3. Canonical Artifact Taxonomy

Canonical artifact classes:

1. `plan`
   - strategic or execution-program artifact,
   - historical formation artifact after promotion,
   - lineage preserved in `docs/process/framework-source-lineage-index.md`
2. `runtime_spec`
   - human-readable runtime law/specification that may be shared across implementations,
   - active home: `docs/product/spec/**` with executable projections in `vida/config/**`
3. `instruction_artifact`
   - human-readable or projected agent-facing/runtime-facing instruction artifact,
   - active home: `vida/config/instructions/**`
4. `product_spec`
   - stable promoted product prose canon,
   - active home: `docs/product/spec/**`
6. `executable_law`
   - machine-readable runtime projection/config,
   - active home: `vida/config/**`
7. `implementation`
   - concrete runtime code,
   - active home: `taskflow-v0/**` today
8. `history_evidence`
   - non-canonical source trail carried by Git history and sidecar changelogs,
   - no separate active directory is required for this class in the clean architecture.
9. `pointer`
   - short document whose job is to redirect readers to the canonical source,
   - allowed in non-canonical map/index locations only when it clearly names the canonical target.

Hard rule:

1. a document must have one primary artifact class,
2. mixed documents should be split or explicitly marked as transitional,
3. “human-readable” alone does not define the class; ownership and function do.
4. until runtime registry consumption is complete, only the latest active Markdown revision remains in the canonical tree.

## 4. Canonical Glossary

Use one normalized vocabulary.

### 4.1 Program and Spec Terms

1. `strategic plan`
   - top-level direction-setting plan whose settled semantics have already been promoted,
   - historical lineage is preserved in `docs/process/framework-source-lineage-index.md`
2. `execution-spec artifact`
   - detailed plan/spec artifact that concretizes a strategic direction before promotion,
   - after promotion its lineage is preserved only in `docs/process/framework-source-lineage-index.md`
3. `runtime spec`
   - shared semantic runtime model independent of one implementation substrate.

### 4.2 Agent-System Terms

1. `agent system`
   - orchestration/runtime layer above one delegated execution
2. `agent backend`
   - concrete execution backend such as `internal` or `external_cli`
3. `agent lane class`
   - semantic execution lane class such as `analyst`, `writer`, `coach`, `verifier`, `approver`, `synthesizer`
4. `worker`
   - bounded delegated executor posture
5. `worker packet`
   - canonical dispatch artifact for a worker lane

### 4.3 Binary Packaging Terms

1. `workspace crate architecture`
   - `vida 1.0` implementation is split into bounded Rust crates with explicit ownership
   - the minimal required split includes a dedicated `taskflow` crate and a dedicated `docflow` crate
2. `embeddable crate`
   - a crate that can be consumed by another host program without depending on the full VIDA CLI binary
   - for VIDA this applies at minimum to both `taskflow` and `docflow`
3. `CLI shell`
   - the standalone `vida` binary as one consumer of reusable VIDA crates, not the only runtime surface
4. `independent CLI surface`
   - a bounded crate may also expose its own CLI tool without requiring the top-level `vida` shell

Normalization rule:

1. Canonical docs use `worker`, `agent backend`, `agent lane class`, and `worker packet`.
2. Legacy `SUBAGENT-*` naming is not part of the active canon.

## 5. Legacy / Transitional State Model

Canonical artifact states:

1. `canonical`
   - current source of truth
2. `active_transitional`
   - current and valid, but expected to be replaced by a cleaner canonical form later
3. `projected`
   - derived or machine-readable projection of another canonical artifact
4. `pointer_only`
   - navigation aid that must not carry unique source-of-truth semantics
5. `history_evidence`
   - historical input only

Legacy rule:

1. “legacy” must never mean merely “older date”,
2. an artifact is legacy only when its state is explicitly `history_evidence`,
3. deleted framework-formation plan/research trees are not active canon,
4. historical lineage is retained through Git history, active artifact sidecars, and `docs/process/framework-source-lineage-index.md`.

## 6. Shared Runtime-Spec Spine

VIDA `0.2.0` and VIDA `1.0` must share one runtime-spec spine.

Current rule:

1. `vida 0.2.0` and `vida 1.0` do not own separate semantic runtime models,
2. they share one canonical runtime-spec foundation,
3. they differ by implementation substrate and maturity, not by core runtime law.

Packaging rule:

1. `vida 1.0` must be implemented as a Rust workspace with explicit crate boundaries,
2. `taskflow` and `docflow` must exist as separate bounded crates in that workspace,
3. `taskflow` must work independently as a library and independently as a CLI tool,
4. `docflow` must work independently as a library and independently as a CLI tool,
5. the agent engine and other bounded runtime subsystems must be embeddable into other programs,
6. the standalone `vida` CLI binary is one packaging target over those crates, not the sole integration surface.

Implementation posture:

1. `vida 0.2.0`
   - prototype / proving / continuation runtime,
   - current implementation substrate: `taskflow-v0/**`,
   - expected to continue development and refine instruction/runtime behavior in practice
2. `vida 1.0`
   - target durable runtime implementation,
   - expected to consume the same semantic runtime spine with a stronger final implementation

Current shared runtime-spec lineage:

1. active promoted owners live in `docs/product/spec/**`, `vida/config/instructions/**`, and `vida/config/**`
2. deleted formation sources are indexed in `docs/process/framework-source-lineage-index.md`

Promotion rule:

1. when a runtime-spec boundary is stable, keep its settled product-law portion in `docs/product/spec/**`,
2. executable projections then belong in `vida/config/**`,
3. implementation-specific details remain in `taskflow-v0/**` or later target runtimes,
4. historical formation inputs are preserved only through `docs/process/framework-source-lineage-index.md`, active artifact sidecars, and Git history.

## 7. Promotion And Projection Rules

Artifact movement is deterministic.

### 7.1 Formation Artifact -> Product Spec

Promote from a framework formation artifact into `docs/product/spec/**` when all are true:

1. the semantics are settled enough to act as stable product law,
2. the document is no longer primarily a sequencing/program artifact,
3. implementation-independent rules can be stated cleanly,
4. the spec should outlive one implementation wave.

### 7.2 Product Spec -> Executable Law

Project from `docs/product/spec/**` into `vida/config/**` when all are true:

1. runtime needs machine-readable configuration or schema,
2. the executable shape can be derived from the prose canon,
3. the projection does not become the only understandable source of meaning.

### 7.3 Framework Protocol -> Product Instruction Artifact

Move or mirror content from pre-cutover framework locations into `vida/config/instructions/**` only when:

1. the content is an instruction-bearing artifact consumed as agent/worker behavior law or prompt authoring surface,
2. the content belongs to the instruction artifact model rather than a generic framework domain protocol,
3. canonical ownership is clearer in the product-owned instruction home.

### 7.4 Pointer Reduction

Reduce a document to `pointer_only` when:

1. a clearer canonical source already exists elsewhere,
2. keeping two full bodies would duplicate active law,
3. the old location is still useful for navigation or compatibility.

## 8. Minimal Duplication Policy

Use this anti-duplication rule:

1. one master map: this file
2. one protocol registry: `vida/config/instructions/system-maps/protocol.index.md`
3. one current project/product docs map resolved through the project docs bootstrap surface
4. one canonical source per semantic decision

Avoid:

1. multiple competing framework maps,
2. repeated narrative summaries across `README`, `index`, `map`, and `protocol-index`,
3. duplicated active law in more than one active home.

Index reduction rule:

1. `vida/config/instructions/system-maps/framework.index.md` should remain a thin pointer into this map and the protocol index,
2. `vida/config/instructions/system-maps/protocol.index.md` should remain a registry, not a second architecture map.

## 9. Normalized Worker Model

Canonical state:

1. one agent-system protocol,
2. one worker-dispatch protocol,
3. one backend lifecycle protocol,
4. one normalized role vocabulary,
5. prompt bodies living in `vida/config/instructions/**`.

## 10. Consistency Rules

When changing framework structure, in the same change set:

1. update this file,
2. update `vida/config/instructions/system-maps/protocol.index.md` if protocol ownership or canonical source changed,
3. update the project docs map if a runtime-spec promotion changed current project/product canon,
4. update instruction projection docs when an instruction-bearing artifact moves,
5. remove outdated active-body duplicates immediately or mark them pointer-only.

## 11. Decision Boundary

Use this protocol when:

1. deciding where a document belongs,
2. deciding whether something is canonical, projected, or historical,
3. deciding whether a runtime rule should stay in plans, promote to product spec, or project into config,
4. normalizing terminology across framework, product, and runtime layers.

Conflict rule:

1. `AGENTS.md` remains stronger for bootstrap behavior,
2. this file is the canonical repository/documentation architecture map,
3. `vida/config/instructions/system-maps/protocol.index.md` is the canonical domain-protocol registry,
4. the active project docs map is the canonical promoted project/product-spec map for the current repository.

-----
artifact_path: config/system-maps/framework.map
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/framework.map.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-03-12T11:05:54+02:00'
changelog_ref: framework.map.changelog.jsonl
