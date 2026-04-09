# Mempalace Against Vida Specs Project Design Design

Purpose: define a bounded VIDA memory implementation path that uses VIDA specs as owner law, MemPalace as a product-pattern donor, and `memory-mcp-1file` as the strongest current technical donor for storage, search, validity, and supersession mechanics.

## 1. Problem

VIDA already has enough research to conclude that memory is a first-class runtime state family rather than a cache, but the exact implementation contract is still open.

Current state:

1. memory architecture direction exists in research,
2. Release 1 still requires ordinary search first,
3. governance and runtime-boundary law already belong to VIDA specs,
4. donor repositories expose useful patterns but do not define VIDA law,
5. the project still lacks one bounded design document that ties donor mechanics to VIDA-owned runtime constraints.

This design closes that gap.

## 2. Core Decision

VIDA memory will be implemented as:

1. a separate authoritative DB-backed runtime state family,
2. distinct from sealed compiled control-bundle truth,
3. distinct from ephemeral prompt-only context,
4. consumed through bounded retrieval and task-dynamic context injection,
5. governed by VIDA-owned approval, TTL, correction, deletion, and trace rules.

The primary architectural decision is:

1. memory truth is not cache,
2. memory truth is not transcript-only reuse,
3. memory truth is not donor-defined product law,
4. memory truth is runtime-owned state.

## 3. Donor Split

### 3.1 MemPalace

MemPalace is the stronger donor for product and retrieval-shaping patterns.

Borrow candidates:

1. explicit memory scope organization,
2. room or namespace style grouping,
3. raw-context-first retention instead of aggressive early summarization,
4. retrieval bounded by topical or contextual scope before broad search,
5. layered memory navigation metaphors that improve operator reasoning.

Do not borrow as root law:

1. Chroma-centric or vector-first default architecture,
2. donor naming as canonical VIDA terminology,
3. donor implementation assumptions that bypass VIDA governance law,
4. store-everything posture without stronger lifecycle controls.

### 3.2 `memory-mcp-1file`

`memory-mcp-1file` is the stronger donor for near-term technical implementation mechanics.

Borrow candidates:

1. SurrealDB-backed storage operations,
2. ordinary search helpers,
3. validity-window handling,
4. supersession and invalidation logic,
5. graph-memory building blocks for later evolution,
6. CRUD patterns for operational memory state.

Do not borrow as root law:

1. donor API surface as canonical VIDA surface,
2. semantic or vector retrieval as an admitted Release-1 default,
3. donor schema names without VIDA-owned runtime integration.

## 4. Release 1 Boundary

Release 1 keeps memory intentionally narrow.

Active Release-1 posture:

1. ordinary search only,
2. bounded retrieval only,
3. no default vector retrieval,
4. no default semantic search index as authoritative serving layer,
5. no donor-driven expansion beyond VIDA-owned runtime law.

Reason:

1. current project research already narrows Release 1 to ordinary search,
2. governance and runtime proof are more important than retrieval sophistication at this stage,
3. vector and semantic retrieval remain valid later directions, but not automatic Release-1 law.

## 5. VIDA Memory Model

VIDA memory should become its own authoritative state family alongside other DB-backed runtime families.

Recommended family split:

1. `framework_state`
2. `project_activation_state`
3. `protocol_binding_state`
4. `memory_state`
5. `runtime_operational_state`

Placement rule:

1. memory truth belongs under project-local DB authority,
2. derived memory indexes remain separate from memory truth,
3. prompt views and serving caches may be derived,
4. memory truth must not move into cache-delivery or sealed control-core surfaces.

## 6. Memory Record Classes

The initial bounded model should distinguish records by operational role rather than by donor metaphors.

Minimum classes:

1. `fact`
   - durable declarative knowledge with trace and validity
2. `decision`
   - chosen direction with source trace and possible supersession
3. `preference`
   - stable but revisable user or project preference
4. `context_anchor`
   - bounded situational context worth later retrieval
5. `relationship`
   - explicit link between memory records, topics, actors, or projects

Minimum shared fields:

1. `memory_id`
2. `memory_class`
3. `scope`
4. `content`
5. `source_trace_ref`
6. `created_at`
7. `valid_from`
8. `valid_until`
9. `supersedes_memory_id`
10. `invalidated_by`
11. `governance_policy_ref`

## 7. Scope Model

MemPalace is most useful here as a donor for organization, not as a schema authority.

Recommended initial VIDA scopes:

1. `project`
2. `user`
3. `team`
4. `task_thread`
5. `agent`

Scope rule:

1. retrieval must always start from explicit bounded scope when one exists,
2. cross-scope retrieval should be explicit and narrow,
3. memory injection should prefer the smallest scope that can satisfy the task,
4. broad global retrieval should not become the default fallback.

## 8. Retrieval Model

Release 1 retrieval should remain ordinary-search-first and scope-bounded.

Recommended retrieval order:

1. identify active scope,
2. filter records by governance and validity,
3. run ordinary search inside the scope,
4. apply limited expansion only when direct-scope recall is insufficient,
5. inject only task-relevant memory slices into runtime context.

Injection rule:

1. memory retrieval is deliberate runtime consumption,
2. memory is not automatically preloaded wholesale,
3. raw transcripts are not the only retrieval unit,
4. derived summaries may exist, but authoritative source remains the memory record plus source trace.

## 9. Validity And Supersession

`memory-mcp-1file` is the strongest donor here.

VIDA should adopt:

1. explicit validity windows,
2. explicit supersession chains,
3. explicit invalidation or correction refs,
4. non-destructive lineage where possible.

Operational rule:

1. new memory does not silently overwrite old memory,
2. correction and invalidation must stay auditable,
3. retrieval must prefer currently valid records,
4. stale or superseded records may remain visible for audit but must not silently dominate live retrieval.

## 10. Governance

Governance remains VIDA-owned and higher-precedence than donor behavior.

Required governance controls:

1. approval for sensitive memory classes,
2. TTL policy where required,
3. correction path,
4. deletion path,
5. source-trace requirement,
6. auditability of mutations.

Rule:

1. donors can inform storage and retrieval mechanics,
2. donors do not weaken approval, TTL, deletion, or trace law.

## 11. Runtime Consumption Boundary

Memory must be consumed by runtime as bounded state, not as always-on transcript baggage.

Recommended boundary:

1. memory retrieval happens at task time,
2. memory records are selected for explicit relevance,
3. runtime context receives only bounded slices,
4. retrieval should remain explainable and inspectable,
5. evidence of what was injected should remain recoverable.

This keeps memory compatible with:

1. session continuity,
2. resumed execution,
3. task-dynamic context assembly,
4. future daemon-backed serving evolution.

## 12. Implementation Track

### Phase 1: Authoritative Foundation

Implement:

1. memory schema for the bounded record classes,
2. SurrealDB-backed CRUD,
3. source trace linkage,
4. validity windows,
5. supersession and invalidation,
6. ordinary search inside bounded scopes.

Primary donor emphasis:

1. `memory-mcp-1file`

### Phase 2: Runtime Retrieval Integration

Implement:

1. scope-first retrieval planner,
2. bounded retrieval injection,
3. retrieval traceability,
4. minimal operator-facing introspection for what memory was injected and why.

Primary donor emphasis:

1. MemPalace organization and retrieval-shaping patterns
2. VIDA-owned runtime boundary rules

### Phase 3: Deferred Rich Retrieval

Consider later:

1. semantic retrieval,
2. vector indexes,
3. graph-memory traversal,
4. richer ranking or fusion.

Admission rule:

1. none of these become active by donor existence alone,
2. each requires separate VIDA-owned design and proof.

## 13. Dependency On Takeover Surface

This design is blocked operationally by the current takeover-surface gap.

Reason:

1. current runtime still leaves root-local write protocol-invalid under the active delegated-cycle deadlock,
2. design-doc completion and subsequent code work need a lawful write path,
3. the takeover surface is the current critical-path unblocker.

Therefore sequencing remains:

1. close the exception-path takeover surface,
2. finalize this design document through canonical docflow path,
3. shape the implementation packet from the bounded file set and proof targets,
4. only then implement the memory runtime track.

## 14. Result

VIDA memory should proceed with a split-donor strategy:

1. MemPalace shapes namespace and retrieval semantics,
2. `memory-mcp-1file` shapes storage and search mechanics,
3. VIDA specs remain the owner of governance, runtime boundaries, and Release-1 admission.

That gives VIDA a practical implementation path without importing donor architecture as root law.

-----
artifact_path: product/spec/mempalace-against-vida-specs-project-design-design
artifact_type: product_spec
artifact_version: 1
artifact_revision: 2026-04-08
schema_version: 1
status: canonical
source_path: docs/product/spec/mempalace-against-vida-specs-project-design-design.md
created_at: 2026-04-08T20:55:39.055339209Z
updated_at: 2026-04-09T05:41:17.943049337Z
changelog_ref: mempalace-against-vida-specs-project-design-design.changelog.jsonl
