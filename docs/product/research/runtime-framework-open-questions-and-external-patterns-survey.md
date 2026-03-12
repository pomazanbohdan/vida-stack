# Runtime Framework Open Questions And External Patterns Survey

Purpose: map the still-open VIDA runtime-framework questions that previously had no dedicated external research to primary external patterns, and distinguish which tracks now have enough outside evidence to proceed versus which remain mostly VIDA-specific design work.

## 1. Research Question

For the unresolved VIDA runtime-framework tracks that did not yet have dedicated external research, what do primary external sources suggest, and where do those sources stop being enough so VIDA must define its own product law?

## 2. Scope

This survey focuses on the unresolved runtime-framework tracks around:

1. compiled control bundles,
2. DB authority, import, and migration,
3. runtime-home placement under `.vida/**`,
4. derived cache between DB truth and runtime serving,
5. embedded runtime and editable projection loops,
6. protocol admission and runtime binding.

It uses only primary or official references from:

1. OpenAI
2. Anthropic
3. Microsoft Agent Framework / Semantic Kernel
4. LangGraph
5. vLLM
6. XDG Base Directory Specification

Rule:

1. these sources provide implementation patterns and constraints,
2. they do not replace VIDA product law,
3. if no strong external analogue exists, the remaining contract stays VIDA-owned.

## 3. Summary

The strongest external support now exists for:

1. one strict declarative compiled contract,
2. durable execution identity and resumable state,
3. hard separation of config, state, cache, and runtime file classes,
4. cache systems that reuse exact stable prefixes only,
5. fail-closed admission of declarative components and tools.

The weakest external support remains around:

1. the exact VIDA `known project protocol -> compiled executable protocol` promotion path,
2. the exact `.vida/**` export/import UX for project activation and hidden runtime config,
3. the precise DB receipt model for installer/init/re-import cutovers.

## 4. Track A: Compiled Control Bundle Contract

### 4.1 External Findings

Microsoft Agent Framework and Semantic Kernel both lean toward declarative agent/config surfaces that are:

1. file-backed,
2. auditable,
3. loadable from YAML/JSON,
4. validated before runtime construction.

OpenAI guidance reinforces that:

1. structured outputs and function schemas should constrain data flow,
2. strict schemas reduce ambiguous downstream behavior,
3. prompt/version objects benefit from explicit versioning and rollback.

### 4.2 VIDA Implications

The current VIDA direction is strongly supported:

1. the compiled control bundle should be one strict declarative contract, not a loose collection of ad hoc payloads,
2. it should remain versioned and inspectable,
3. missing referenced components should fail construction rather than silently degrade,
4. stable declarative sections should be separable from dynamic runtime evidence.

### 4.3 Remaining VIDA-Owned Questions

External sources do not answer:

1. the exact VIDA top-level split between `control_core`, `activation_bundle`, `protocol_binding_registry`, and `cache_delivery_contract`,
2. the exact root metadata block,
3. the exact minimal valid Release-1 bundle.

Suggested bounded follow-up research:

1. `compiled-control-bundle-contract-research`

## 5. Track B: DB Authority, Import, And Migration

### 5.1 External Findings

Microsoft human-in-the-loop patterns treat approvals as resumable workflow state bound to the same session/workflow context.

LangGraph treats durable execution, checkpointed state, and resumable execution identity as first-class runtime primitives.

Semantic Kernel agent threads also reinforce:

1. continuing execution through a stable thread/session identity,
2. reusing existing execution context instead of inferring continuation from chat alone.

### 5.2 VIDA Implications

The current VIDA direction is again supported:

1. project-local DB truth should remain the durable runtime authority,
2. installer/init/import/re-import should be receipt-bearing lifecycle transitions,
3. execution identity, waiting state, and imported configuration state should survive process boundaries,
4. migration should update authoritative state, not just regenerate files.

### 5.3 Remaining VIDA-Owned Questions

External sources do not answer:

1. the exact VIDA import receipt schema,
2. the exact cutover order for framework import, project activation import, and protocol-binding import,
3. the exact rollback or stale-revision law during re-init.

Suggested bounded follow-up research:

1. `db-authority-and-migration-runtime-research`

## 6. Track C: Runtime Home Placement Under `.vida/**`

### 6.1 External Findings

The XDG Base Directory Specification gives the clearest general rule for separating:

1. configuration,
2. data,
3. state,
4. cache,
5. runtime-only files.

Its strongest signal is not the exact directory names but the ownership split itself.

### 6.2 VIDA Implications

This strongly supports the VIDA move away from mixed root/runtime placement:

1. `.vida/config/**` for runtime-owned config,
2. `.vida/db/**` for authoritative state,
3. `.vida/cache/**` for non-authoritative derived cache,
4. `.vida/runtime/**` or adjacent runtime-only files for transient process/runtime artifacts,
5. explicit path overrides only through canonical config, not by ad hoc drift.

### 6.3 Remaining VIDA-Owned Questions

External standards do not answer:

1. the exact sublayout for framework versus project runtime artifacts inside `.vida/**`,
2. the exact migration path from bridge-era root files,
3. the exact precedence between project-local paths and installed/global defaults.

Suggested bounded follow-up research:

1. `runtime-home-and-surface-migration-research`

## 7. Track D: Derived Cache Between DB Truth And Runtime

### 7.1 External Findings

OpenAI prompt caching, Anthropic prompt caching, and vLLM APC converge on the same operational rule:

1. cache only exact or stable prefixes,
2. keep static/reused content at the front,
3. treat dynamic tail content as cache-busting,
4. expect cache invalidation when tool/config/message ordering changes.

Anthropic adds a particularly useful hierarchy:

1. `tools`
2. `system`
3. `messages`

and shows that cache invalidation depends on where a change occurs in that hierarchy.

### 7.2 VIDA Implications

This supports the VIDA derived-cache direction:

1. cache should sit under DB truth, not replace it,
2. cache keys should derive from stable revision tuples and stable bundle partitions,
3. dynamic evidence, receipts, and per-task deltas must stay outside the cache-stable prefix,
4. tool and schema ordering should remain stable for cache reuse,
5. prompt-serving cache and CLI query cache should both remain derived views.

### 7.3 Remaining VIDA-Owned Questions

External caching docs do not answer:

1. the exact VIDA cache manifest schema,
2. which CLI/status families may read stale cache,
3. whether non-prompt query views and prompt-prefix views share one invalidation contract or two.

Suggested bounded follow-up research:

1. `derived-cache-delivery-and-invalidation-research`

## 8. Track E: Embedded Runtime And Editable Projection Loops

### 8.1 External Findings

OpenAI prompt objects and Microsoft/Semantic Kernel declarative specs all reinforce a versioned, auditable artifact model:

1. stable declarative artifacts can be stored centrally,
2. versions can be promoted or rolled back,
3. runtime should load validated declarative artifacts rather than freeform text,
4. required referenced components must exist at load time.

### 8.2 VIDA Implications

This supports the VIDA binary-plus-projection direction:

1. embedded runtime artifacts should be compiled and versioned,
2. editable projections should be exportable and re-importable,
3. export artifacts should not automatically become truth until validated and imported,
4. re-import should remain explicit and fail-closed.

### 8.3 Remaining VIDA-Owned Questions

External sources do not give a direct analogue for:

1. the exact VIDA projection artifact schema,
2. the exact user-facing export/edit/import UX for hidden `.vida/**` runtime surfaces,
3. the exact interaction between embedded framework artifacts and project-local editable projections.

Suggested bounded follow-up research:

1. `embedded-runtime-bootstrap-and-projection-research`

## 9. Track F: Protocol Admission And Runtime Binding

### 9.1 External Findings

OpenAI function-calling guidance reinforces:

1. tool definitions are explicit schemas,
2. strict mode should be enabled,
3. the runtime may restrict available tools through an allowed subset,
4. tool results must be routed back through explicit call ids.

Microsoft Agent Framework and Semantic Kernel declarative specs reinforce:

1. referenced tools/plugins/components must already exist at construction time,
2. declarative loaders should not invent missing components on the fly,
3. missing runtime dependencies should fail agent construction.

OpenAI safety guidance reinforces:

1. structured outputs should constrain data flow,
2. tool approvals should remain on for sensitive operations,
3. untrusted inputs must not silently drive privileged downstream behavior.

### 9.2 VIDA Implications

These patterns support a stronger VIDA runtime-binding model:

1. a project protocol should not become executable merely because it is discoverable,
2. promotion into executable/runtime-bound state should require explicit admission checks,
3. missing runtime owners or missing gates should block activation,
4. runtime binding should remain schema-backed and fail-closed,
5. tool/action admission and protocol admission should stay connected to approval and execution-boundary policy.

### 9.3 Remaining VIDA-Owned Questions

External sources still do not define:

1. the exact VIDA `known` versus `compiled executable` protocol split,
2. the exact binding-matrix shape for project protocols,
3. the exact promotion lifecycle and receipts for executable protocol admission.

Suggested bounded follow-up research:

1. `protocol-admission-and-runtime-binding-research`

## 10. Recommended Research Split

The unresolved runtime-framework space now separates cleanly into these bounded future research tracks:

1. `compiled-control-bundle-contract-research`
2. `db-authority-and-migration-runtime-research`
3. `runtime-home-and-surface-migration-research`
4. `derived-cache-delivery-and-invalidation-research`
5. `embedded-runtime-bootstrap-and-projection-research`
6. `protocol-admission-and-runtime-binding-research`
7. `runtime-memory-state-and-retrieval-research`

Rule:

1. each track now has enough external grounding to proceed as a bounded VIDA-specific contract exercise,
2. but none of them can be solved by vendor copying alone.

## 11. Sources

Primary references used for this survey:

1. OpenAI Prompt Caching
   - https://developers.openai.com/api/docs/guides/prompt-caching
2. OpenAI Prompting
   - https://developers.openai.com/api/docs/guides/prompting
3. OpenAI Function Calling
   - https://platform.openai.com/docs/guides/function-calling
4. OpenAI Safety in Building Agents
   - https://developers.openai.com/api/docs/guides/agent-builder-safety
5. Anthropic Prompt Caching
   - https://platform.claude.com/docs/en/build-with-claude/prompt-caching
6. Microsoft Agent Framework Declarative Agents
   - https://learn.microsoft.com/en-us/agent-framework/agents/declarative
7. Microsoft Agent Framework Tool Approval
   - https://learn.microsoft.com/en-us/agent-framework/agents/tools/tool-approval
8. Semantic Kernel YAML Schema Reference
   - https://learn.microsoft.com/en-us/semantic-kernel/concepts/prompts/yaml-schema
9. Semantic Kernel Agent Architecture
   - https://learn.microsoft.com/en-us/semantic-kernel/frameworks/agent/agent-architecture
10. Semantic Kernel OpenAI Assistant Agent
   - https://learn.microsoft.com/en-us/semantic-kernel/frameworks/agent/agent-types/assistant-agent
11. LangGraph Durable Execution
   - https://docs.langchain.com/oss/python/langgraph/durable-execution
12. LangGraph Human In The Loop
   - https://docs.langchain.com/oss/python/langgraph/human-in-the-loop
13. vLLM Automatic Prefix Caching
   - https://docs.vllm.ai/en/latest/features/automatic_prefix_caching/
14. XDG Base Directory Specification
   - https://specifications.freedesktop.org/basedir-spec/0.8

-----
artifact_path: product/research/runtime-framework-open-questions-and-external-patterns-survey
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/research/runtime-framework-open-questions-and-external-patterns-survey.md
created_at: '2026-03-12T23:50:00+02:00'
updated_at: '2026-03-12T23:59:00+02:00'
changelog_ref: runtime-framework-open-questions-and-external-patterns-survey.changelog.jsonl
