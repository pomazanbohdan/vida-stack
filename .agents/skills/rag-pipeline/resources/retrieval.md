# Retrieval Patterns and Optimization

> **Project context**: Retrieval runs in **Rust**. Python snippets are reference/prototype only.

---

## Retrieval Strategy Overview

```
Query
  │
  ├─── Dense Retrieval (Candle embedding → ANN search)
  │
  ├─── Sparse Retrieval (BM25 / TF-IDF keyword match)
  │
  └─── Hybrid Fusion (RRF) → Reranker → Top-K → LLM
```

---

## 1. Dense Retrieval (Semantic)

Use cosine similarity between query embedding and indexed chunk embeddings.

### Rust (with SurrealDB or FAISS)

```rust
use crate::embedder::CandleEmbedder;

pub struct DenseRetriever {
    embedder: CandleEmbedder,
    vector_store: VectorStore, // SurrealDB / FAISS wrapper
}

impl DenseRetriever {
    pub async fn search(
        &self,
        query: &str,
        top_k: usize,
        score_threshold: f32,
    ) -> anyhow::Result<Vec<RetrievedChunk>> {
        // BGE/E5 benefit from query prefix
        let prefixed = format!(
            "Represent this sentence for searching relevant passages: {}",
            query
        );
        let query_vec = self.embedder.embed(&[&prefixed])?;
        let query_slice = query_vec.to_vec1::<f32>()?; // candle Tensor → Vec<f32>

        let candidates = self.vector_store
            .similarity_search(&query_slice, top_k * 2)
            .await?;

        Ok(candidates
            .into_iter()
            .filter(|c| c.score >= score_threshold)
            .take(top_k)
            .collect())
    }
}
```

---

## 2. Sparse Retrieval (BM25)

BM25 excels at exact keyword matches and rare terms that embeddings may miss.

### Rust (BM25 implementation sketch)

```rust
use std::collections::HashMap;

pub struct Bm25Index {
    /// term → (doc_id → term_freq)
    inverted_index: HashMap<String, HashMap<String, f32>>,
    doc_lengths: HashMap<String, usize>,
    avg_doc_length: f32,
    k1: f32, // typically 1.2–2.0
    b: f32,  // typically 0.75
}

impl Bm25Index {
    pub fn score(&self, query_terms: &[&str], doc_id: &str) -> f32 {
        let n = self.doc_lengths.len() as f32;
        let dl = *self.doc_lengths.get(doc_id).unwrap_or(&0) as f32;
        let k1 = self.k1;
        let b = self.b;
        let avdl = self.avg_doc_length;

        query_terms.iter().map(|term| {
            let df = self.inverted_index.get(*term)
                .map(|m| m.len() as f32)
                .unwrap_or(0.0);

            if df == 0.0 { return 0.0; }

            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
            let tf = self.inverted_index
                .get(*term)
                .and_then(|m| m.get(doc_id))
                .copied()
                .unwrap_or(0.0);

            let tf_norm = tf * (k1 + 1.0)
                / (tf + k1 * (1.0 - b + b * dl / avdl));

            idf * tf_norm
        }).sum()
    }

    pub fn search(&self, query: &str, top_k: usize) -> Vec<(String, f32)> {
        let terms: Vec<&str> = query.split_whitespace().collect();
        let mut scores: HashMap<&str, f32> = HashMap::new();

        for (term, doc_map) in &self.inverted_index {
            if terms.contains(&term.as_str()) {
                for doc_id in doc_map.keys() {
                    let score = self.score(&terms, doc_id);
                    *scores.entry(doc_id).or_default() += score;
                }
            }
        }

        let mut ranked: Vec<(String, f32)> = scores
            .into_iter()
            .map(|(id, s)| (id.to_string(), s))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        ranked.truncate(top_k);
        ranked
    }
}
```

---

## 3. Hybrid Search with Reciprocal Rank Fusion (RRF)

Combine dense and sparse rankings without needing to normalize scores.

```rust
/// k=60 is the standard RRF constant (Cormack et al., 2009)
pub fn reciprocal_rank_fusion(
    rankings: &[Vec<(String, f32)>], // each inner Vec is one ranked list
    k: usize,
) -> Vec<(String, f32)> {
    let mut fused: std::collections::HashMap<String, f32> = Default::default();

    for ranking in rankings {
        for (rank, (doc_id, _score)) in ranking.iter().enumerate() {
            *fused.entry(doc_id.clone()).or_default() += 1.0 / (k + rank + 1) as f32;
        }
    }

    let mut result: Vec<(String, f32)> = fused.into_iter().collect();
    result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    result
}

// Usage:
// let dense = dense_retriever.search(query, 20).await?;
// let sparse = bm25_index.search(query, 20);
// let dense_ids: Vec<(String, f32)> = dense.iter().map(|c| (c.id.clone(), c.score)).collect();
// let fused = reciprocal_rank_fusion(&[dense_ids, sparse], 60);
```

### Weight Tuning

| Query Type | Dense Weight | Sparse Weight |
|------------|-------------|---------------|
| Conceptual / semantic | 0.8 | 0.2 |
| Keyword / fact lookup | 0.3 | 0.7 |
| Mixed | 0.5 | 0.5 |

Use BM25 weight ≥ 0.5 for queries that contain rare terms, product names, or codes.

---

## 4. Reranking

First-stage retrieval (dense + sparse) optimizes for recall. Reranking optimizes for precision.

### Cross-Encoder Reranking (Candle)

```rust
// Cross-encoder scores (query, passage) pairs jointly — more accurate than bi-encoder
// Load a cross-encoder model (e.g. cross-encoder/ms-marco-MiniLM-L-6-v2) via Candle
pub struct CrossEncoderReranker {
    model: BertForSequenceClassification, // or equivalent
    tokenizer: Tokenizer,
    device: Device,
}

impl CrossEncoderReranker {
    pub fn rerank(
        &self,
        query: &str,
        candidates: Vec<RetrievedChunk>,
        top_k: usize,
    ) -> anyhow::Result<Vec<RetrievedChunk>> {
        let pairs: Vec<(&str, &str)> = candidates.iter()
            .map(|c| (query, c.text.as_str()))
            .collect();

        let scores = self.score_pairs(&pairs)?;

        let mut scored: Vec<(RetrievedChunk, f32)> = candidates
            .into_iter()
            .zip(scores)
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.truncate(top_k);

        Ok(scored.into_iter().map(|(c, _)| c).collect())
    }
}
```

### Maximal Marginal Relevance (MMR) — diversity + relevance

```rust
pub fn mmr_select(
    query_embedding: &[f32],
    candidate_embeddings: &[Vec<f32>],
    candidates: Vec<RetrievedChunk>,
    lambda: f32, // 0.0 = max diversity, 1.0 = max relevance
    top_k: usize,
) -> Vec<RetrievedChunk> {
    let cosine = |a: &[f32], b: &[f32]| -> f32 {
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        dot // assumes L2-normalized vectors
    };

    let mut selected_indices: Vec<usize> = Vec::new();
    let mut remaining: Vec<usize> = (0..candidates.len()).collect();

    while selected_indices.len() < top_k && !remaining.is_empty() {
        let next = remaining.iter().max_by(|&&i, &&j| {
            let rel_i = cosine(query_embedding, &candidate_embeddings[i]);
            let rel_j = cosine(query_embedding, &candidate_embeddings[j]);

            let red_i = selected_indices.iter()
                .map(|&s| cosine(&candidate_embeddings[i], &candidate_embeddings[s]))
                .fold(f32::NEG_INFINITY, f32::max);
            let red_j = selected_indices.iter()
                .map(|&s| cosine(&candidate_embeddings[j], &candidate_embeddings[s]))
                .fold(f32::NEG_INFINITY, f32::max);

            let score_i = lambda * rel_i - (1.0 - lambda) * red_i;
            let score_j = lambda * rel_j - (1.0 - lambda) * red_j;

            score_i.partial_cmp(&score_j).unwrap()
        }).copied().unwrap();

        selected_indices.push(next);
        remaining.retain(|&i| i != next);
    }

    selected_indices.into_iter().map(|i| candidates[i].clone()).collect()
}
```

---

## 5. Advanced Patterns

### Multi-Query Retrieval

```rust
// Generate N query variations (via LLM), union the results, deduplicate
pub async fn multi_query_retrieve(
    original_query: &str,
    llm: &dyn LlmClient,
    retriever: &DenseRetriever,
    top_k: usize,
) -> Vec<RetrievedChunk> {
    let prompt = format!(
        "Generate 3 alternative phrasings of this query, one per line: {}",
        original_query
    );
    let variations = llm.complete(&prompt).await.unwrap_or_default();
    let queries: Vec<&str> = std::iter::once(original_query)
        .chain(variations.lines())
        .collect();

    let mut seen_ids = std::collections::HashSet::new();
    let mut results = Vec::new();

    for q in queries {
        if let Ok(hits) = retriever.search(q, top_k, 0.0).await {
            for hit in hits {
                if seen_ids.insert(hit.id.clone()) {
                    results.push(hit);
                }
            }
        }
    }
    results
}
```

### HyDE (Hypothetical Document Embeddings)

```rust
// 1. Generate a hypothetical answer with LLM
// 2. Embed the hypothetical answer (not the query)
// 3. Search with that embedding
pub async fn hyde_retrieve(
    query: &str,
    llm: &dyn LlmClient,
    embedder: &CandleEmbedder,
    vector_store: &VectorStore,
    top_k: usize,
) -> Vec<RetrievedChunk> {
    let hypothetical = llm.complete(&format!(
        "Write a short passage that directly answers: {}", query
    )).await.unwrap_or_else(|_| query.to_string());

    let hyp_embedding = embedder.embed(&[&hypothetical])
        .and_then(|t| t.to_vec1::<f32>().map_err(Into::into))
        .unwrap_or_default();

    vector_store.similarity_search(&hyp_embedding, top_k).await.unwrap_or_default()
}
```

### Metadata Filtering

```rust
// Pre-filter by category, date, source before vector search
pub async fn filtered_search(
    query_vec: &[f32],
    filters: &HashMap<String, serde_json::Value>,
    vector_store: &VectorStore,
    top_k: usize,
) -> Vec<RetrievedChunk> {
    vector_store
        .similarity_search_with_filter(query_vec, filters, top_k)
        .await
        .unwrap_or_default()
}
// Example filter: {"category": "technical", "source": "docs/api"}
```

---

## Python Reference (LangChain — prototype only)

```python
from langchain.retrievers import BM25Retriever, EnsembleRetriever
from langchain.retrievers.multi_query import MultiQueryRetriever
from langchain.retrievers import ContextualCompressionRetriever
from langchain.retrievers.document_compressors import LLMChainExtractor

# Hybrid retriever
bm25 = BM25Retriever.from_documents(chunks, k=10)
dense = vectorstore.as_retriever(search_kwargs={"k": 10})
hybrid = EnsembleRetriever(retrievers=[bm25, dense], weights=[0.4, 0.6])

# Multi-query
multi_q = MultiQueryRetriever.from_llm(retriever=dense, llm=llm)

# Contextual compression (extract only relevant sentences)
compressor = LLMChainExtractor.from_llm(llm)
compression = ContextualCompressionRetriever(
    base_compressor=compressor,
    base_retriever=hybrid
)
```

---

## Optimization Checklist

- [ ] Threshold filtering: drop chunks below cosine sim 0.6 (tune per dataset)
- [ ] Metadata pre-filtering before ANN search for large indexes
- [ ] Rerank top-20 candidates to return top-5 for LLM context
- [ ] Cache query embeddings for repeated queries
- [ ] Monitor retrieval latency P50/P99 separately from generation latency
- [ ] Profile ANN index type (HNSW vs flat vs IVF) for your corpus size
