# Instruction Packing And Caching Survey

Purpose: summarize the current practical options for reducing repeated instruction-token cost in agent runtimes and map those options to the VIDA Release-1 architecture without prematurely promoting research findings into active product law.

## 1. Research Question

How can VIDA package framework and project instructions so runtime prompts consume fewer tokens while preserving protocol correctness, activation control, and fail-closed behavior?

## 2. Evaluated Families

Cross-provider canon that now appears stable across current official guidance:

1. keep stable instructions structurally separated from volatile task state,
2. keep the reusable prefix deterministic so provider-side caching can hit reliably,
3. load only the active domain/context slice instead of replaying the whole corpus,
4. avoid sending tokens that do not change the current decision,
5. place long reference context and documents before the final question in long-context setups,
6. treat compression of normative law as higher-risk than compression of informational context.

### 2.1 Prompt Caching / Prefix Caching

This is the strongest immediate lever for token savings when the runtime can keep a stable prompt prefix.

Findings:

1. OpenAI supports prompt caching for repeated prompt prefixes and recommends keeping stable instructions at the beginning of the prompt while moving dynamic task state later.
2. Anthropic supports prompt caching across stable prompt prefixes, including tools/system blocks, and requires identical cached sections for reliable hits.
3. vLLM provides automatic prefix caching for self-hosted inference paths.
4. OpenAI's current latency guidance also recommends maximizing the shared prompt prefix and moving dynamic portions later so fewer input tokens need to be reprocessed.
5. Google's current Gemini guidance recommends context caching when the same large context is reused across many queries.

Implication for VIDA:

1. canonical always-on instructions, lane bundles, tool schemas, and stable output contracts should be kept deterministic,
2. dynamic state, receipts, task-specific evidence, and operator deltas should stay outside the cached prefix whenever possible.
3. cache-friendly packaging should be evaluated not only for direct token price but also for repeated bootstrap latency and cache-hit stability.

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
3. the highest-value compilation target is not "all canon into one blob" but a small number of stable bundle classes with explicit activation triggers.

### 2.3 Retrieval Instead Of Prompt Stuffing

Retrieval helps when the corpus is large but only a subset is needed per request.

Findings:

1. retrieval/file-search style systems can reduce prompt size by loading only relevant chunks,
2. this is useful for large documentation or protocol corpora that are not always active,
3. retrieval is weaker than compiled bundles for mandatory invariants because retrieval miss behavior can become unsafe if treated as the only source.

Implication for VIDA:

1. retrieval is a good fit for optional or low-frequency instruction families,
2. mandatory framework invariants still need direct always-on or activation-controlled bundle delivery.
3. retrieval should prefer section-level loading over whole-document replay when the active protocol owner is already known.

### 2.4 Semantic Routing

Semantic routing is useful before loading optional instruction families.

Findings:

1. semantic routing can choose which domain instructions, models, or context depth to load,
2. this can reduce prompt size by avoiding irrelevant domain bundles,
3. it is not sufficient as the only safety mechanism for mandatory framework rules.

Implication for VIDA:

1. semantic routing should select optional lane/domain slices,
2. it must not decide whether hard invariants exist at all.
3. routing must fail closed when more than one bounded unit or protocol family is plausible and no higher-evidence binding exists.

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

### 2.7 Current Provider Prompt-Structure Guidance

These points now appear in official vendor guidance closely enough to treat them as stable external canon for prompt-layout decisions.

Findings:

1. OpenAI recommends putting instructions at the beginning of the prompt, separating instructions from context clearly, and reducing fluffy or imprecise wording.
2. OpenAI's current latency guidance recommends using fewer input tokens only after the larger latency levers are exhausted, then maximizing shared prefix and moving dynamic content later.
3. Anthropic recommends prompt templates with fixed and variable parts, plus XML tags for explicit structure and hierarchy.
4. Anthropic's long-context guidance recommends placing longform documents near the top of the prompt and the actual query near the end.
5. Google Gemini's long-context guidance recommends avoiding unnecessary tokens, placing the query at the end for long contexts, and using context caching when the same context is reused repeatedly.

Implication for VIDA:

1. large framework/project markdown bodies should not be replayed as one mixed prose block,
2. stable law should sit in a deterministic prefix or compiled control bundle,
3. active receipts, run-graph state, retrieved evidence, and current ask should be injected later as variable slots,
4. long optional documents should be loaded only when activated and placed before the final task/query when a long-context pattern is unavoidable.

## 3. Recommended VIDA Strategy

The best practical stack for VIDA is:

1. keep markdown canon as the human-owned source of truth,
2. compile runtime-bearing instruction bundles from canon,
3. deliver only always-on and activated bundles to the model,
4. keep that prefix stable so provider-level prompt caching can reuse it,
5. use retrieval for large optional corpora rather than sending them every time,
6. reserve semantic routing for optional slice selection,
7. defer fine-tuning and compression until compiled bundles plus evals are stable.

Concrete packaging guidance for VIDA:

1. move repeated bootstrap law into a stable `always_on_core` prefix,
2. compile lane-specific control law into small deterministic `lane_bundle` artifacts,
3. compile high-frequency route law such as anti-stop, open-delegation, and reporting gates into one active orchestration bundle instead of repeating long prose across many surfaces,
4. keep run-graph gates, receipts, active task ids, and user-turn deltas in the variable suffix,
5. prefer section-level activation for large surfaces like step-thinking rather than whole-file loading,
6. keep project-level narrowing short and referential when framework law already owns the full rule.

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

Additional implementation questions after the current external review:

1. which current protocol surfaces are true `always_on` law versus merely human-readable reference,
2. which large documents can be compiled into section-addressable runtime projections instead of full markdown replay,
3. how the runtime will guarantee stable bundle ordering so cache keys stay deterministic,
4. what evals prove that compact bundle delivery preserves fail-closed behavior for anti-stop, exception-path, and active-unit binding rules,
5. how to separate normative law compression from informational-context compression so high-risk protocol details are not lost.

## 6. Sources

Primary external references:

1. OpenAI Prompt Engineering Best Practices
   - https://help.openai.com/en/articles/6654000-best-practices-for-prompt-engineering-with-the-openai-api
2. OpenAI Latency Optimization
   - https://developers.openai.com/api/docs/guides/latency-optimization
3. Anthropic Prompting Best Practices
   - https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices
4. Anthropic Prompt Templates And Variables
   - https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-tools
5. Google Gemini Long Context
   - https://ai.google.dev/gemini-api/docs/long-context
6. Anthropic Prompt Caching
   - https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching
7. Anthropic Context Editing
   - https://docs.anthropic.com/en/docs/build-with-claude/context-editing
8. vLLM Automatic Prefix Caching
   - https://docs.vllm.ai/en/latest/features/automatic_prefix_caching.html
9. OpenAI File Search
   - https://developers.openai.com/api/docs/guides/tools-file-search
10. OpenAI Fine-Tuning Best Practices
   - https://developers.openai.com/api/docs/guides/fine-tuning-best-practices
11. OpenAI Model Distillation
   - https://openai.com/index/api-model-distillation/
12. vLLM Semantic Router
   - https://github.com/vllm-project/semantic-router
13. LLMLingua
   - https://aclanthology.org/2023.emnlp-main.825/
14. LongLLMLingua
   - https://arxiv.org/abs/2310.06839

-----
artifact_path: product/research/instruction-packing-and-caching-survey
artifact_type: product_research_doc
artifact_version: '1'
artifact_revision: '2026-03-13'
schema_version: '1'
status: canonical
source_path: docs/product/research/instruction-packing-and-caching-survey.md
created_at: '2026-03-12T17:03:32+02:00'
updated_at: '2026-03-13T21:10:00+02:00'
changelog_ref: instruction-packing-and-caching-survey.changelog.jsonl
