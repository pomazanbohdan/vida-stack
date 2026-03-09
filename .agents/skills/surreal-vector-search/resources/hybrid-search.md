# SurrealDB Hybrid Search

Patterns for combining BM25 keyword ranking with HNSW vector similarity in SurrealDB 3.0,
including RRF fusion, linear combination, cascade pipelines, and Rust integration.

---

## 1. Architecture Overview

```
Query ──► ┬─► HNSW vector search  <|K,EF|>  ──► candidates ─┐
          │                                                    │
          └─► BM25 full-text search (SEARCH index) ──────────┴─► Fusion ──► Results
```

Hybrid search improves recall for queries that need both:
- **Semantic understanding** — handled by the vector branch
- **Exact term matching** — handled by the BM25 branch (names, codes, rare tokens)

---

## 2. Fusion Methods

| Method | Description | Best for |
|--------|-------------|----------|
| **RRF** (Reciprocal Rank Fusion) | Score = Σ 1/(k + rank) across lists | General purpose; robust without weight tuning |
| **Linear combination** | Score = α·vec_score + (1−α)·bm25_score | When you can tune α on labelled data |
| **Cross-encoder rerank** | Neural model scores each (query, doc) pair | Highest quality; highest latency |
| **Cascade** | ANN first, then keyword filter on candidates | Latency-sensitive, high selectivity keyword filter |

**Default recommendation**: use RRF. It is parameter-free and consistently competitive.

---

## 3. SurrealDB DDL: BM25 Full-Text Index

```sql
-- 1. Define a text analyser (tokeniser + filters)
DEFINE ANALYZER text_analyzer
  TOKENIZERS class
  FILTERS lowercase, ascii, snowball(english);

-- 2. Create BM25 full-text index on the content field
DEFINE INDEX memory_bm25_idx
  ON TABLE memories
  FIELDS content
  TYPE SEARCH ANALYZER text_analyzer BM25;
```

---

## 4. SurrealQL Hybrid Queries

### Separate queries + application-layer RRF (recommended)

Run two queries, merge results in Rust using RRF.

```sql
-- Vector branch
SELECT id, content,
       vector::distance::cosine(embedding, $query_vec) AS vec_score
FROM memories
WHERE embedding <|50,128|> $query_vec
ORDER BY vec_score;

-- BM25 branch
SELECT id, content,
       search::score(1) AS bm25_score
FROM memories
WHERE content @@ $query_text
ORDER BY bm25_score DESC
LIMIT 50;
```

### Inline RRF in SurrealQL (LET block pattern)

SurrealDB supports CTEs via `LET` + sub-queries. The RRF formula is applied in pure SurrealQL.

```sql
LET $k = 60;

LET $vec_results = (
    SELECT id, ROW_NUMBER() OVER (ORDER BY vector::distance::cosine(embedding, $query_vec)) AS vec_rank
    FROM memories
    WHERE embedding <|50,128|> $query_vec
);

LET $bm25_results = (
    SELECT id, ROW_NUMBER() OVER (ORDER BY search::score(1) DESC) AS bm25_rank
    FROM memories
    WHERE content @@ $query_text
    LIMIT 50
);

-- Merge on id, compute RRF score
SELECT
    id,
    (IF vec_rank  IS NOT NONE THEN 1.0 / ($k + vec_rank)  ELSE 0.0 END +
     IF bm25_rank IS NOT NONE THEN 1.0 / ($k + bm25_rank) ELSE 0.0 END) AS rrf_score
FROM $vec_results FULL JOIN $bm25_results ON id
ORDER BY rrf_score DESC
LIMIT $top_k;
```

---

## 5. RRF Algorithm (language-agnostic)

```python
from collections import defaultdict
from typing import List, Tuple

def reciprocal_rank_fusion(
    result_lists: List[List[Tuple[str, float]]],
    k: int = 60,
    weights: List[float] | None = None,
) -> List[Tuple[str, float]]:
    """
    Combine ranked result lists with RRF.

    Args:
        result_lists: Each is a list of (doc_id, score) sorted best-first.
        k:            RRF constant. Higher k → more weight to lower ranks.
                      k=60 is the standard default from the original paper.
        weights:      Optional per-list multipliers (default: uniform).

    Returns:
        Merged list of (doc_id, rrf_score) sorted best-first.
    """
    if weights is None:
        weights = [1.0] * len(result_lists)

    scores: dict[str, float] = defaultdict(float)

    for result_list, weight in zip(result_lists, weights):
        for rank, (doc_id, _) in enumerate(result_list):
            scores[doc_id] += weight / (k + rank + 1)

    return sorted(scores.items(), key=lambda x: x[1], reverse=True)
```

**Properties of RRF**:
- Rank-based → immune to score-scale differences between vector and BM25
- `k=60` consistently works without dataset-specific tuning
- Easily extended to 3+ lists (add a re-ranker score as a third list)

---

## 6. Linear Combination (weighted score fusion)

Use when you have labelled data and can optimise α.

```python
def linear_fusion(
    vector_results: List[Tuple[str, float]],
    keyword_results: List[Tuple[str, float]],
    alpha: float = 0.5,          # weight for vector branch
) -> List[Tuple[str, float]]:
    """Min-max normalise both score lists, then combine linearly."""

    def normalise(results: List[Tuple[str, float]]) -> dict[str, float]:
        if not results:
            return {}
        scores = [s for _, s in results]
        lo, hi = min(scores), max(scores)
        denom = (hi - lo) or 1.0
        return {doc_id: (score - lo) / denom for doc_id, score in results}

    vec_norm = normalise(vector_results)
    kw_norm  = normalise(keyword_results)

    all_ids = set(vec_norm) | set(kw_norm)
    combined = {
        doc_id: alpha * vec_norm.get(doc_id, 0.0) + (1 - alpha) * kw_norm.get(doc_id, 0.0)
        for doc_id in all_ids
    }
    return sorted(combined.items(), key=lambda x: x[1], reverse=True)
```

**Tuning α**: start at 0.5, sweep [0.3, 0.4, 0.5, 0.6, 0.7] against your evaluation set (MRR or
NDCG), pick the best.

---

## 7. Rust Integration — Hybrid Search

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use surrealdb::{engine::remote::ws::Client, Surreal};

#[derive(Debug, Deserialize)]
pub struct RawResult {
    pub id: String,
    pub content: String,
    pub score: f64,
}

#[derive(Debug)]
pub struct HybridResult {
    pub id: String,
    pub content: String,
    pub rrf_score: f64,
}

/// Fetch vector candidates from SurrealDB.
async fn vector_branch(
    db: &Surreal<Client>,
    query_vec: &[f32],
    limit: usize,
) -> surrealdb::Result<Vec<RawResult>> {
    let mut res = db
        .query(
            "SELECT id, content, \
             vector::distance::cosine(embedding, $vec) AS score \
             FROM memories \
             WHERE embedding <|$lim,128|> $vec \
             ORDER BY score",
        )
        .bind(("vec", query_vec.to_vec()))
        .bind(("lim", limit))
        .await?;
    Ok(res.take(0)?)
}

/// Fetch BM25 keyword candidates from SurrealDB.
async fn keyword_branch(
    db: &Surreal<Client>,
    query_text: &str,
    limit: usize,
) -> surrealdb::Result<Vec<RawResult>> {
    let mut res = db
        .query(
            "SELECT id, content, search::score(1) AS score \
             FROM memories \
             WHERE content @@ $text \
             ORDER BY score DESC \
             LIMIT $lim",
        )
        .bind(("text", query_text))
        .bind(("lim", limit))
        .await?;
    Ok(res.take(0)?)
}

/// Merge two ranked lists with Reciprocal Rank Fusion.
fn rrf_merge(
    vec_results: &[RawResult],
    kw_results: &[RawResult],
    k: f64,
) -> Vec<HybridResult> {
    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut content_map: HashMap<String, String> = HashMap::new();

    for (rank, r) in vec_results.iter().enumerate() {
        *scores.entry(r.id.clone()).or_default() += 1.0 / (k + rank as f64 + 1.0);
        content_map.entry(r.id.clone()).or_insert_with(|| r.content.clone());
    }
    for (rank, r) in kw_results.iter().enumerate() {
        *scores.entry(r.id.clone()).or_default() += 1.0 / (k + rank as f64 + 1.0);
        content_map.entry(r.id.clone()).or_insert_with(|| r.content.clone());
    }

    let mut merged: Vec<HybridResult> = scores
        .into_iter()
        .map(|(id, rrf_score)| HybridResult {
            content: content_map[&id].clone(),
            id,
            rrf_score,
        })
        .collect();

    merged.sort_by(|a, b| b.rrf_score.partial_cmp(&a.rrf_score).unwrap());
    merged
}

/// Full hybrid search: parallel ANN + BM25, then RRF fusion.
pub async fn hybrid_search(
    db: &Surreal<Client>,
    query_vec: &[f32],
    query_text: &str,
    top_k: usize,
    oversample: usize,       // fetch oversample * top_k candidates per branch
) -> surrealdb::Result<Vec<HybridResult>> {
    let limit = top_k * oversample;

    // Run both branches concurrently
    let (vec_res, kw_res) = tokio::try_join!(
        vector_branch(db, query_vec, limit),
        keyword_branch(db, query_text, limit),
    )?;

    let mut merged = rrf_merge(&vec_res, &kw_res, 60.0);
    merged.truncate(top_k);
    Ok(merged)
}
```

---

## 8. Cascade Pipeline (latency-optimised)

When keyword search is very selective, use it as a fast pre-filter before the ANN step:

```
1. BM25 keyword search → small candidate set (e.g. 200 docs)
2. Vector KNN restricted to those IDs → final ranked results
```

```sql
-- Step 1: collect BM25 candidate IDs
LET $bm25_ids = (
    SELECT id FROM memories
    WHERE content @@ $query_text
    ORDER BY search::score(1) DESC
    LIMIT 200
);

-- Step 2: ANN over candidate set only
SELECT id, content,
       vector::distance::cosine(embedding, $query_vec) AS score
FROM memories
WHERE id INSIDE $bm25_ids
  AND embedding <|10,128|> $query_vec
ORDER BY score;
```

Use cascade when:
- BM25 selectivity is high (keyword filter removes > 90 % of records)
- Latency budget is tight and you cannot afford parallel dual-branch search

---

## 9. Best Practices

### Do's
- **Default to RRF** — it is robust, parameter-free, and hard to beat without labelled data
- **Oversample** — fetch 3–5× more candidates per branch than the final top_k
- **Run branches concurrently** — `tokio::try_join!` in Rust, parallel queries elsewhere
- **Log individual branch scores** — essential for debugging recall regressions
- **A/B test** — measure MRR or NDCG on real user queries before and after tuning α or k

### Don'ts
- **Don't assume one α fits all query types** — factual vs semantic queries may need different weights
- **Don't skip keyword search** — exact term matching handles names, codes, and rare tokens
- **Don't over-fetch without bound** — larger candidate sets improve recall but increase latency
- **Don't ignore empty-result edge cases** — either branch may return zero hits for some queries
