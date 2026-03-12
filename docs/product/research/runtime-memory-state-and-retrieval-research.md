# Runtime Memory State And Retrieval Research

Purpose: define the external and donor-backed research basis for treating `memory` as a first-class VIDA runtime subsystem rather than a cache, and record `memory-mcp-1file` as the current strongest donor repository for practical implementation borrowing.

## 1. Research Question

How should VIDA model runtime memory, retrieval, validity, and indexing, and what can be borrowed from `memory-mcp-1file` plus official agent-runtime guidance before the remaining memory contract becomes VIDA-owned law?

## 2. Core Result

The strongest current conclusion is:

1. `memory` is not a cache layer,
2. `memory` is not part of the sealed compiled control bundle as authoritative truth,
3. `memory` is a separate DB-backed runtime state family,
4. runtime may consume memory through bounded retrieval and task-dynamic context injection,
5. memory-derived indexes or prompt views may be cached, but memory truth itself remains authoritative state.

Release-1 boundary correction:

1. current runtime memory retrieval should assume ordinary search only,
2. vector or semantic retrieval should remain deferred until the daemon/reactive stage is admitted,
3. donor support for richer retrieval does not make those capabilities active Release-1 law automatically.

## 3. External Research Signals

### 3.1 OpenAI

OpenAI agent sessions behave as persistent memory across runs and interrupted execution.

Implication for VIDA:

1. memory must survive process boundaries,
2. memory belongs to runtime state, not to ephemeral prompt-only context,
3. resume and session continuity should be able to consume persisted memory.

Primary source:

1. `OpenAI Agents SDK Sessions`
   - https://openai.github.io/openai-agents-python/sessions/

### 3.2 Anthropic

Anthropic memory tooling exposes explicit memory CRUD and retrieval behavior with application-owned storage.

Implication for VIDA:

1. memory should have explicit write/read/update/delete semantics,
2. storage ownership remains application/runtime-owned,
3. memory retrieval should be bounded and deliberate rather than inferred from broad transcript reuse.

Primary source:

1. `Anthropic Memory Tool`
   - https://anthropic.mintlify.app/en/docs/agents-and-tools/tool-use/memory-tool

### 3.3 Microsoft

Microsoft separates conversation/session continuity from longer-term agent memory and treats both as stateful, resumable runtime concerns.

Implication for VIDA:

1. short-term session continuity and long-term memory should not be collapsed automatically,
2. memory is a runtime subsystem with explicit storage/provider boundaries,
3. rehydration and continuation should consume stored memory/state rather than only chat history.

Primary sources:

1. `Agent Framework Conversations`
   - https://learn.microsoft.com/en-us/agent-framework/agents/conversations/
2. `Agent Framework Agent Memory`
   - https://learn.microsoft.com/en-us/agent-framework/user-guide/agents/agent-memory

### 3.4 LangGraph

LangGraph treats long-term memory as DB-backed stored documents with search and namespace semantics, distinct from short-term thread state.

Implication for VIDA:

1. memory should likely have explicit namespaces/scopes,
2. memory reads and writes should be distinguishable from short-term execution state,
3. semantic retrieval belongs to memory serving, not to the sealed control bundle.

Primary source:

1. `LangGraph Memory`
   - https://docs.langchain.com/oss/python/langgraph/memory

## 4. Donor Repository

Current strongest donor repository:

1. `pomazanbohdan/memory-mcp-1file`
   - https://github.com/pomazanbohdan/memory-mcp-1file

Key donor surface already identified:

1. `src/storage/surrealdb/memory_ops.rs`
   - https://raw.githubusercontent.com/pomazanbohdan/memory-mcp-1file/refs/heads/master/src/storage/surrealdb/memory_ops.rs

Why this donor matters:

1. it is already Rust-based,
2. it already uses SurrealDB-backed memory operations,
3. it already treats memory as operational state rather than as a cache,
4. it already includes more than simple CRUD:
   - search
   - temporal validity
   - supersession/invalidation
   - graph/semantic memory direction

Current borrow boundary:

1. ordinary search, storage, validity, and supersession are the strongest near-term borrow candidates,
2. vector or semantic retrieval should stay deferred until a later daemon-backed memory/index stage is admitted.

## 5. What VIDA Can Borrow Directly From The Donor

The donor can likely be connected locally as a bounded source repository and mined for implementation elements rather than copied wholesale.

The most promising borrowable families are:

1. storage operations over SurrealDB,
2. memory CRUD semantics,
3. validity-window handling,
4. supersession and invalidation logic,
5. ordinary search helpers,
6. graph-memory building blocks for later direction,
7. indexing helpers for later direction,
8. memory tool surface patterns for external agent/runtime use.

Recommended donor-handling rule:

1. attach the repository under a bounded local donor folder,
2. decompose it into borrowable elements,
3. pull only the needed pieces into VIDA runtime architecture,
4. do not adopt donor ownership, naming, or API surfaces blindly as root law.

## 6. Proposed VIDA Runtime Placement

`memory` should become its own authoritative DB-backed family:

1. `framework_state`
2. `project_activation_state`
3. `protocol_binding_state`
4. `memory_state`
5. `runtime_operational_state`

Placement rule:

1. `memory_state` lives under the same project-local DB authority in `.vida/db/**`,
2. derived memory indexes or query caches may live under `.vida/cache/**`,
3. memory truth must not move into `cache_delivery_contract`,
4. memory truth must not become part of the sealed `control_core`.

## 7. Current Open Questions

The following still need bounded closure inside VIDA:

1. short-term versus long-term versus operational memory taxonomy,
2. namespace or scope model:
   - project
   - user
   - team
   - task/thread
   - agent
3. write-path policy:
   - synchronous
   - deferred
   - background
4. retrieval boundary:
   - always preload
   - triggered retrieval
   - task-dynamic injection only
5. validity and supersession law,
6. index boundary:
   - authoritative records versus derived search indexes,
7. governance and approval rules for memory mutation,
8. exact cutoff between ordinary search in Release 1 and deferred semantic/vector retrieval in later daemon-backed stages.

## 8. Recommended Next Track

The next bounded research track should be:

1. `runtime-memory-state-and-retrieval-contract`

It should define:

1. memory classes,
2. namespace model,
3. write/read/retrieve lifecycle,
4. validity and supersession rules,
5. ordinary-search boundary for Release 1 versus deferred vector/semantic retrieval,
6. runtime-consumption boundary,
7. donor extraction plan from `memory-mcp-1file`.

## 9. Result

This research is strong enough to fix one architectural decision now:

1. `memory` is a first-class runtime state family, not a cache.

It is not yet strong enough to close:

1. the exact memory schema,
2. the exact namespace model,
3. the exact retrieval/default preload policy,
4. the exact ordinary-search contract for Release 1,
5. the exact donor extraction map from `memory-mcp-1file`.

-----
artifact_path: product/research/runtime-memory-state-and-retrieval-research
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/research/runtime-memory-state-and-retrieval-research.md
created_at: '2026-03-12T23:59:00+02:00'
updated_at: '2026-03-12T23:59:30+02:00'
changelog_ref: runtime-memory-state-and-retrieval-research.changelog.jsonl
