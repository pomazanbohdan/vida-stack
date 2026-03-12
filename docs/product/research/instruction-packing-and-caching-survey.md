# Instruction Packing And Caching Survey

Purpose: summarize the current practical options for reducing repeated instruction-token cost in agent runtimes and map those options to the VIDA Release-1 architecture without prematurely promoting research findings into active product law.

## 1. Research Question

How can VIDA package framework and project instructions so runtime prompts consume fewer tokens while preserving protocol correctness, activation control, and fail-closed behavior?

## 2. Evaluated Families

### 2.1 Prompt Caching / Prefix Caching

This is the strongest immediate lever for token savings when the runtime can keep a stable prompt prefix.

Findings:

1. OpenAI supports prompt caching for repeated prompt prefixes and recommends keeping stable instructions at the beginning of the prompt while moving dynamic task state later.
2. Anthropic supports prompt caching across stable prompt prefixes, including tools/system blocks, and requires identical cached sections for reliable hits.
3. vLLM provides automatic prefix caching for self-hosted inference paths.

Implication for VIDA:

1. canonical always-on instructions, lane bundles, tool schemas, and stable output contracts should be kept deterministic,
2. dynamic state, receipts, task-specific evidence, and operator deltas should stay outside the cached prefix whenever possible.

### 2.2 Compiled Instruction Bundles

This is the best architectural match for VIDA.

Findings:

1. large raw protocol rereads are expensive even when model context windows allow them,
2. token savings improve when human-readable canon is compiled into short runtime-bearing instruction contracts,
3. bundle compilation also improves cache-hit rate because the runtime can reuse stable machine-readable prefixes.

Recommended bundle split:

1. `always_on_core`
2. `lane_bundle`
3. `triggered_domain_bundle`
4. `task_specific_dynamic_context`

Implication for VIDA:

1. Release 1 should not rely on sending full markdown law for every step,
2. compiled bundle outputs should become the cache-friendly runtime surface for repeated execution.

### 2.3 Retrieval Instead Of Prompt Stuffing

Retrieval helps when the corpus is large but only a subset is needed per request.

Findings:

1. retrieval/file-search style systems can reduce prompt size by loading only relevant chunks,
2. this is useful for large documentation or protocol corpora that are not always active,
3. retrieval is weaker than compiled bundles for mandatory invariants because retrieval miss behavior can become unsafe if treated as the only source.

Implication for VIDA:

1. retrieval is a good fit for optional or low-frequency instruction families,
2. mandatory framework invariants still need direct always-on or activation-controlled bundle delivery.

### 2.4 Semantic Routing

Semantic routing is useful before loading optional instruction families.

Findings:

1. semantic routing can choose which domain instructions, models, or context depth to load,
2. this can reduce prompt size by avoiding irrelevant domain bundles,
3. it is not sufficient as the only safety mechanism for mandatory framework rules.

Implication for VIDA:

1. semantic routing should select optional lane/domain slices,
2. it must not decide whether hard invariants exist at all.

### 2.5 Fine-Tuning And Distillation

This can reduce prompt size by moving repetitive behavior into model weights, but it is a later-stage optimization.

Findings:

1. fine-tuning and distillation help only after the runtime contract is already stable,
2. they are higher-cost and higher-lock-in than caching plus bundle compilation,
3. without stable evals they risk encoding current runtime drift into the model.

Implication for VIDA:

1. fine-tuning should stay behind bundle compilation, runtime evals, and cache metrics,
2. it is not the first Release-1 move.

### 2.6 Prompt Compression

Prompt compression research is promising but risky for normative instruction law.

Findings:

1. compression systems can reduce prompt size significantly,
2. they may be acceptable for large informational context,
3. they are riskier for hard protocol constraints because compression can erase small but important normative details.

Implication for VIDA:

1. prompt compression should remain experimental for protocol-bearing instruction surfaces,
2. it should not replace compiled law bundles in Release 1.

## 3. Recommended VIDA Strategy

The best practical stack for VIDA is:

1. keep markdown canon as the human-owned source of truth,
2. compile runtime-bearing instruction bundles from canon,
3. deliver only always-on and activated bundles to the model,
4. keep that prefix stable so provider-level prompt caching can reuse it,
5. use retrieval for large optional corpora rather than sending them every time,
6. reserve semantic routing for optional slice selection,
7. defer fine-tuning and compression until compiled bundles plus evals are stable.

## 4. Release-1 Fit

This research maps most directly to Wave 3 of Release 1.

Why:

1. Wave 3 already owns compiled runtime bundles,
2. cache-friendly bundle packaging is a natural extension of machine-readable runtime initialization,
3. this work reduces repeated instruction-token cost without weakening protocol activation boundaries.

Release-1 recommendation:

1. treat instruction caching as a bounded cache-system slice of Wave 3 rather than as a detached late optimization,
2. keep the first implementation discussion focused on bundle shape, cache keys, provider compatibility, and activation-safe prefix layout,
3. do not block Wave 1 or Wave 2 on full caching closure.

## 5. Suggested First Implementation Discussion

The next implementation discussion should answer:

1. what exact compiled bundle format VIDA will use for cache-friendly runtime delivery,
2. which parts of the runtime prefix are stable enough for provider cache reuse,
3. how activation-controlled bundles change the cache key,
4. where retrieval begins and always-on bundle delivery ends,
5. what metrics prove that the cache system is actually saving tokens without hiding protocol drift.

## 6. Sources

Primary external references:

1. OpenAI Prompt Caching
   - https://developers.openai.com/api/docs/guides/prompt-caching
2. Anthropic Prompt Caching
   - https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching
3. Anthropic Context Editing
   - https://docs.anthropic.com/en/docs/build-with-claude/context-editing
4. vLLM Automatic Prefix Caching
   - https://docs.vllm.ai/en/latest/features/automatic_prefix_caching.html
5. OpenAI File Search
   - https://developers.openai.com/api/docs/guides/tools-file-search
6. OpenAI Fine-Tuning Best Practices
   - https://developers.openai.com/api/docs/guides/fine-tuning-best-practices
7. OpenAI Model Distillation
   - https://openai.com/index/api-model-distillation/
8. vLLM Semantic Router
   - https://github.com/vllm-project/semantic-router
9. LLMLingua
   - https://aclanthology.org/2023.emnlp-main.825/
10. LongLLMLingua
   - https://arxiv.org/abs/2310.06839

-----
artifact_path: product/research/instruction-packing-and-caching-survey
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-12'
schema_version: '1'
status: canonical
source_path: docs/product/research/instruction-packing-and-caching-survey.md
created_at: '2026-03-12T17:03:32+02:00'
updated_at: '2026-03-12T17:03:32+02:00'
changelog_ref: instruction-packing-and-caching-survey.changelog.jsonl
