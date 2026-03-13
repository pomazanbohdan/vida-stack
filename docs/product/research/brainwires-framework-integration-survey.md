# Brainwires Framework Integration Survey

Purpose: Assess what Brainwires Framework can cover for VIDA TaskFlow, DocFlow, and Release 2, and identify the remaining gaps, risks, and recommended integration posture.

## 1. Research Question

What does `Brainwires Framework` already provide as a Rust agent/runtime library stack, which parts can be borrowed into the current VIDA `TaskFlow` and `DocFlow` implementation effort, and which parts remain outside its coverage for both `Release 1` and `Release 2`?

## 2. Verified Framework Shape

`Brainwires Framework` is a broad Rust agent-framework workspace rather than a narrow execution-kernel or documentation-kernel runtime.

Verified current shape:

1. one facade crate plus a large multi-crate workspace,
2. provider abstraction for multiple LLM backends,
3. tool execution and multi-agent orchestration,
4. MCP client and MCP server surfaces,
5. A2A protocol support,
6. RAG and code-search subsystems,
7. semantic memory and knowledge storage,
8. distributed mesh and relay layers,
9. permissions, audit, and trust controls,
10. training, autonomy, and audio-adjacent subsystems.

This makes it a strong library donor for agent infrastructure, but not an obvious match for VIDA's current product-law-driven runtime kernel.

## 3. Potential Coverage For Current TaskFlow

The strongest potential borrowing areas for the current `TaskFlow` line are:

1. provider abstraction and model-routing surfaces,
2. tool-system execution scaffolding,
3. multi-agent pool coordination,
4. file/resource lock infrastructure,
5. MCP and A2A transport interoperability,
6. workflow DAG helpers for bounded orchestration slices,
7. permissions/audit primitives for agent-side action control.

These capabilities can help as implementation accelerators around the execution runtime.

However, `Brainwires` does not currently cover the core VIDA `TaskFlow` product kernel requirements:

1. `TaskFlow` as the primary execution authority,
2. DB-first runtime truth under the current `SurrealDB` direction,
3. compiled control bundle consumption and runtime binding,
4. protocol-binding authority and receipts,
5. `orchestrator-init`, `agent-init`, and `project-activator` boot surfaces,
6. fail-closed project activation and migration law,
7. current run-graph, recovery, checkpoint, and readiness semantics,
8. VIDA-specific operator command contracts and closure proof surfaces.

Conclusion for current `TaskFlow`:

1. useful as donor libraries,
2. not suitable as the replacement runtime foundation.

## 4. Potential Coverage For Current DocFlow

The strongest potential borrowing areas for the current `DocFlow` line are narrower:

1. document ingestion and chunking,
2. semantic and hybrid retrieval,
3. code/document search utilities,
4. memory-backed knowledge lookup,
5. MCP exposure of search-oriented helper services.

This can help a retrieval or auxiliary search subsystem.

`Brainwires` does not currently cover the current VIDA `DocFlow` kernel responsibilities:

1. canonical markdown artifact law,
2. footer metadata and changelog sidecar mutation model,
3. canonical inventory and relation law,
4. readiness, proof, activation, and protocol-coverage operator surfaces,
5. lawful `finalize-edit`, `touch`, `move`, and bounded documentation mutation flow,
6. canonical-path registry/readiness export behavior,
7. VIDA-specific documentation closure semantics.

Conclusion for current `DocFlow`:

1. useful as an auxiliary retrieval/search donor,
2. not suitable as the primary `DocFlow` runtime owner.

## 5. Suitability For Release 2

`Release 2` is the first place where `Brainwires` becomes materially more relevant.

The most relevant future-facing areas are:

1. relay/server surfaces for long-lived background coordination,
2. mesh and federation layers for distributed runtime topology,
3. A2A interoperability for cross-agent or cross-system integration,
4. RAG and memory layers for richer retrieval services,
5. autonomy-oriented background services for daemon-stage experimentation.

These capabilities align with parts of the current `Release 2` direction around:

1. daemon mode,
2. background workers,
3. reactive orchestration,
4. richer control-plane services,
5. optional heavier retrieval/runtime assistance.

Even for `Release 2`, important gaps remain:

1. no VIDA-style DB-first operational evidence model,
2. no compiled control bundle owner model,
3. no current VIDA host-project integration law,
4. no shared CLI/UI authority model over one runtime truth,
5. no current VIDA approval/evidence/governance semantics,
6. no current VIDA synchronization, projection, and migration ownership.

Conclusion for `Release 2`:

1. promising infrastructure substrate for selected subsystems,
2. still not a drop-in base for the whole VIDA product runtime.

## 6. Architectural Risks

The main adoption risks are:

1. architectural drift from product-runtime law into generic agent-framework assumptions,
2. storage-model mismatch because `Brainwires` centers on `LanceDB` and `SQLite` patterns rather than the current VIDA DB-first runtime direction,
3. blurred runtime ownership if agent-framework crates begin replacing `TaskFlow` or `DocFlow` kernel responsibilities,
4. framework youth and limited maturity signals relative to the breadth of claimed scope,
5. broad dependency footprint and subsystem surface area that could raise integration cost.

## 7. Recommended Integration Posture

The recommended posture is:

1. do not adopt `Brainwires Framework` wholesale as the new product base,
2. do not transfer `TaskFlow` ownership into `brainwires-agents` or `brainwires-storage`,
3. do not transfer `DocFlow` ownership into `brainwires-rag` or `brainwires-storage`,
4. evaluate only bounded donor use of:
   - provider abstraction,
   - MCP/A2A interoperability,
   - relay/mesh for future `Release 2`,
   - optional RAG/memory helpers where they do not become runtime truth.

The recommended near-term practical sequence is:

1. keep current `TaskFlow` and `DocFlow` ownership inside VIDA-native runtime crates,
2. treat `Brainwires` as a comparison/donor framework,
3. test bounded spikes on `providers`, `mcp`, `a2a`, and `mesh` before any deeper adoption decision.

## 8. Sources

Primary external sources used for this survey:

1. Brainwires Framework repository root
   - https://github.com/Brainwires/brainwires-framework
2. Brainwires README
   - https://github.com/Brainwires/brainwires-framework/blob/main/README.md
3. Brainwires workspace manifest
   - https://github.com/Brainwires/brainwires-framework/blob/main/Cargo.toml
4. `brainwires-agents`
   - https://github.com/Brainwires/brainwires-framework/blob/main/crates/brainwires-agents/README.md
5. `brainwires-mcp`
   - https://github.com/Brainwires/brainwires-framework/blob/main/crates/brainwires-mcp/README.md
6. `brainwires-relay`
   - https://github.com/Brainwires/brainwires-framework/blob/main/crates/brainwires-relay/README.md
7. `brainwires-a2a`
   - https://github.com/Brainwires/brainwires-framework/blob/main/crates/brainwires-a2a/README.md
8. `brainwires-rag`
   - https://github.com/Brainwires/brainwires-framework/blob/main/crates/brainwires-rag/README.md
9. `brainwires-storage`
   - https://github.com/Brainwires/brainwires-framework/blob/main/crates/brainwires-storage/README.md
10. `brainwires-permissions`
   - https://github.com/Brainwires/brainwires-framework/blob/main/crates/brainwires-permissions/README.md
11. `brainwires-mesh`
   - https://github.com/Brainwires/brainwires-framework/blob/main/crates/brainwires-mesh/README.md
12. `brainwires-brain`
   - https://github.com/Brainwires/brainwires-framework/blob/main/crates/brainwires-brain/README.md
13. GitHub repository metadata and tags
   - https://api.github.com/repos/Brainwires/brainwires-framework
   - https://api.github.com/repos/Brainwires/brainwires-framework/tags?per_page=10

-----
artifact_path: product/research/brainwires-framework-integration-survey
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/product/research/brainwires-framework-integration-survey.md
created_at: '2026-03-13T08:10:33+02:00'
updated_at: '2026-03-13T08:12:43+02:00'
changelog_ref: brainwires-framework-integration-survey.changelog.jsonl
