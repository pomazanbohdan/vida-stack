# Framework Map Protocol (FMP)

Purpose: one canonical map for VIDA repository structure, documentation architecture, artifact taxonomy, runtime layering, and promotion/projection rules.

This file is the single framework-owned answer to five questions:

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
2. `docs/framework/`
   - framework-owned human-readable law, maps, plans, and domain protocols,
   - active canon unless a file is explicitly marked pointer-only or historical-only.
3. `docs/framework/plans/`
   - active strategic and execution-spec program artifacts,
   - not legacy by default.
4. `docs/framework/history/`
   - evidence, provenance, prior source artifacts, and historical copies,
   - not active canon unless a current canonical document cites the artifact explicitly.
5. `docs/product/spec/`
   - promoted stable product prose canon.
6. `vida/config/`
   - executable product law and runtime-readable projections.
7. `vida-v0/`
   - transitional implementation runtime,
   - current execution substrate for the `0.2.0` line.
8. `docs/project-memory/`
   - Git-resident source tree for project-memory artifacts.
9. `docs/process/`
   - project operational runbooks when present.

Rule:

1. directory placement is semantic, not cosmetic,
2. no artifact should exist in two active homes with equal authority,
3. if the same concept appears in multiple places, exactly one location must be canonical and the others must be projections, pointers, or evidence.

## 2. Layer Map

VIDA uses one normalized documentation/runtime stack:

1. `Bootstrap Layer`
   - `AGENTS.md`
   - `docs/framework/ORCHESTRATOR-ENTRY.MD`
   - `docs/framework/WORKER-ENTRY.MD`
   - `docs/framework/WORKER-THINKING.MD`
2. `Framework Program Layer`
   - `docs/framework/plans/**`
   - strategic plan plus execution/spec artifacts that implement that strategy
3. `Framework Protocol Layer`
   - `docs/framework/*-protocol.md`
   - runtime law for bounded domains
4. `Framework Map / Index Layer`
   - this file
   - `docs/framework/protocol-index.md`
   - thin indexes/pointers only
5. `Product Spec Layer`
   - `docs/product/spec/**`
   - stable promoted product law
6. `Instruction Artifact Layer`
   - `vida/config/instructions/**`
   - product-owned instruction authoring/projection home
7. `Executable Law Layer`
   - `vida/config/**`
   - machine-readable law consumed by runtime
8. `Implementation Layer`
   - `vida-v0/**`
   - current transitional implementation
9. `History / Evidence Layer`
   - `docs/framework/history/**`

## 3. Canonical Artifact Taxonomy

Canonical artifact classes:

1. `plan`
   - strategic or execution-program artifact,
   - active home: `docs/framework/plans/**`
2. `runtime_spec`
   - human-readable runtime law/specification that may be shared across implementations,
   - active homes: `docs/framework/plans/**`, then promoted parts in `docs/product/spec/**` when stabilized
3. `framework_protocol`
   - bounded framework runtime law for one domain,
   - active home: `docs/framework/*-protocol.md`
4. `instruction_artifact`
   - human-readable or projected agent-facing/runtime-facing instruction artifact,
   - active home: `vida/config/instructions/**`
5. `product_spec`
   - stable promoted product prose canon,
   - active home: `docs/product/spec/**`
6. `executable_law`
   - machine-readable runtime projection/config,
   - active home: `vida/config/**`
7. `implementation`
   - concrete runtime code,
   - active home: `vida-v0/**` today
8. `history_evidence`
   - non-canonical source trail, historical artifacts, or migration evidence,
   - active home: `docs/framework/history/**`
9. `pointer`
   - short document whose job is to redirect readers to the canonical source,
   - allowed in `docs/framework/**` and other map/index locations only when it clearly names the canonical target.

Hard rule:

1. a document must have one primary artifact class,
2. mixed documents should be split or explicitly marked as transitional,
3. “human-readable” alone does not define the class; ownership and function do.

## 4. Canonical Glossary

Use one normalized vocabulary.

### 4.1 Program and Spec Terms

1. `strategic plan`
   - top-level direction-setting plan,
   - current canonical source: `docs/framework/plans/2026-03-08-vida-0.1-to-1.0-direct-binary-transition-plan.md`
2. `execution-spec artifact`
   - detailed plan/spec artifact that concretizes the strategic plan,
   - current active home: `docs/framework/plans/**`
3. `runtime spec`
   - shared semantic runtime model independent of one implementation substrate.

### 4.2 Agent-System Terms

1. `agent system`
   - orchestration/runtime layer above one delegated execution
2. `agent backend`
   - concrete execution backend such as `internal` or `external_cli`
3. `agent role`
   - semantic route role such as `analyst`, `writer`, `coach`, `verifier`, `approver`, `synthesizer`
4. `worker`
   - bounded delegated executor posture
5. `worker packet`
   - canonical dispatch artifact for a worker lane

Normalization rule:

1. Canonical docs use `worker`, `agent backend`, `agent role`, and `worker packet`.
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
3. `docs/framework/plans/**` are active by default unless marked otherwise,
4. `docs/framework/history/**` are evidence by default unless cited by an active canonical document.

## 6. Shared Runtime-Spec Spine

VIDA `0.2.0` and VIDA `1.0` must share one runtime-spec spine.

Current rule:

1. `vida 0.2.0` and `vida 1.0` do not own separate semantic runtime models,
2. they share one canonical runtime-spec foundation,
3. they differ by implementation substrate and maturity, not by core runtime law.

Implementation posture:

1. `vida 0.2.0`
   - prototype / proving / continuation runtime,
   - current implementation substrate: `vida-v0/**`,
   - expected to continue development and refine instruction/runtime behavior in practice
2. `vida 1.0`
   - target durable runtime implementation,
   - expected to consume the same semantic runtime spine with a stronger final implementation

Current shared runtime-spec sources:

1. `docs/framework/plans/2026-03-08-vida-0.1-to-1.0-direct-binary-transition-plan.md`
   - strategic master plan
2. `docs/framework/plans/2026-03-08-vida-0.3-state-kernel-schema-spec.md`
3. `docs/framework/plans/2026-03-08-vida-0.3-route-and-receipt-spec.md`
4. `docs/framework/plans/2026-03-08-vida-0.3-instruction-kernel-spec.md`
5. `docs/framework/plans/2026-03-08-vida-0.3-migration-kernel-spec.md`
6. `docs/framework/plans/2026-03-08-vida-0.3-command-tree-spec.md`
7. `docs/framework/plans/2026-03-08-vida-0.3-parity-and-conformance-spec.md`

Promotion rule:

1. when a runtime-spec boundary is stable and no longer merely program-facing, promote its settled product-law portion into `docs/product/spec/**`,
2. executable projections then belong in `vida/config/**`,
3. implementation-specific details remain in `vida-v0/**` or later target runtimes.

## 7. Promotion And Projection Rules

Artifact movement is deterministic.

### 7.1 Plan -> Product Spec

Promote from `docs/framework/plans/**` to `docs/product/spec/**` when all are true:

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

Move or mirror content from `docs/framework/**` into `vida/config/instructions/**` only when:

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
2. one protocol registry: `docs/framework/protocol-index.md`
3. one current product-spec map: `docs/product/spec/current-spec-map.md`
4. one canonical source per semantic decision

Avoid:

1. multiple competing framework maps,
2. repeated narrative summaries across `README`, `index`, `map`, and `protocol-index`,
3. duplicated active law in both `docs/framework/**` and `vida/config/**`.

Index reduction rule:

1. `docs/framework/index.md` should remain a thin pointer into this map and the protocol index,
2. `docs/framework/protocol-index.md` should remain a registry, not a second architecture map.

## 9. Normalized Worker Model

Canonical state:

1. one agent-system protocol,
2. one worker-dispatch protocol,
3. one backend lifecycle protocol,
4. one normalized role vocabulary,
5. prompt bodies living in `vida/config/instructions/prompt_templates/**`.

## 10. Consistency Rules

When changing framework structure, in the same change set:

1. update this file,
2. update `docs/framework/protocol-index.md` if protocol ownership or canonical source changed,
3. update `docs/product/spec/current-spec-map.md` if a runtime-spec promotion changed current product canon,
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
3. `docs/framework/protocol-index.md` is the canonical domain-protocol registry,
4. `docs/product/spec/current-spec-map.md` is the canonical promoted product-spec map.
