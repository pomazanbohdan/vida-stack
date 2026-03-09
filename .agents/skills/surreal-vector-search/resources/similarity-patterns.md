# SurrealDB Similarity Search Patterns

Reference for ANN queries, filtered search, metadata conditioning, and re-ranking in SurrealDB 3.0.

---

## 1. Distance Metrics

| SurrealDB `DIST` | Formula | Ideal use case |
|-----------------|---------|----------------|
| `COSINE` | `1 − (A·B)/(‖A‖‖B‖)` | L2-normalised embeddings (most LLM models) |
| `EUCLIDEAN` | `√Σ(aᵢ−bᵢ)²` | Raw / unnormalised embeddings |
| `MANHATTAN` | `Σ|aᵢ−bᵢ|` | Sparse or high-dimensional vectors |
| `MINKOWSKI` | Generalised Lp norm | Custom distance families |
| `HAMMING` | Bit-count of XOR | Binary quantised vectors |
| `PEARSON` | Correlation coefficient | Magnitude-independent comparison |

**Rule of thumb**: normalise embeddings before insertion → use `COSINE`.
If embeddings are already unit-normalised, `COSINE` and `EUCLIDEAN` are equivalent.

---

## 2. Index Types by Dataset Size

```
Data Size           Recommended Strategy
──────────────────────────────────────────────────────
< 10 K vectors  →   Flat exact scan (no HNSW needed)
10 K – 1 M      →   HNSW (default parameters)
1 M – 100 M     →   HNSW + INT8 quantization
> 100 M         →   HNSW + INT8/PQ + sharding
```

Approximate complexity:
- **Flat (exact)**: O(n) — 100 % recall, only for small datasets
- **HNSW**: O(log n) — ~95–99 % recall at typical ef settings
- **IVF+PQ**: O(√n) — ~90–95 % recall, extreme scale

---

## 3. SurrealQL KNN Queries

### Basic KNN (top-K nearest neighbours)

```sql
-- Retrieve the 10 nearest memories to a query vector
SELECT id, content, vector::distance::cosine(embedding, $query_vec) AS score
FROM memories
WHERE embedding <|10|> $query_vec
ORDER BY score;
```

### KNN with explicit efSearch (beam width)

The `<|K,EF|>` operator controls ef at query time, overriding the index default.

```sql
-- High-recall query: ef=256, return top 10
SELECT id, content
FROM memories
WHERE embedding <|10,256|> $query_vec;
```

| Target recall | EF |
|---------------|-----|
| ≥ 0.99 | 256 |
| ≥ 0.95 | 128 |
| ≥ 0.90 | 64 |
| Speed priority | 32 |

### Rust (`surrealdb` crate) — basic KNN

```rust
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

#[derive(Debug, Deserialize)]
pub struct MemoryResult {
    pub id: String,
    pub content: String,
    pub score: f32,
}

/// Search the top-K nearest memories.
pub async fn knn_search(
    db: &Surreal<Client>,
    query_vec: &[f32],
    top_k: usize,
) -> surrealdb::Result<Vec<MemoryResult>> {
    let query = format!(
        "SELECT id, content, \
         vector::distance::cosine(embedding, $vec) AS score \
         FROM memories \
         WHERE embedding <|{top_k}|> $vec \
         ORDER BY score"
    );
    let mut res = db.query(query).bind(("vec", query_vec.to_vec())).await?;
    Ok(res.take(0)?)
}
```

---

## 4. Filtered / Metadata-Conditioned Search

### Pre-filter (reduce search space before ANN)

```sql
-- Vector search scoped to a specific user
SELECT id, content,
       vector::distance::cosine(embedding, $query_vec) AS score
FROM memories
WHERE user_id = $user_id
  AND embedding <|10,128|> $query_vec
ORDER BY score;
```

### Post-filter (ANN first, then filter candidates)

Useful when the metadata selectivity is low (most records match the filter).

```sql
-- Over-fetch candidates, then apply soft filter
SELECT id, content, tags,
       vector::distance::cosine(embedding, $query_vec) AS score
FROM memories
WHERE embedding <|50,128|> $query_vec   -- fetch 50
  AND tags CONTAINS $tag                 -- filter after ANN
ORDER BY score
LIMIT 10;
```

### Rust — filtered search

```rust
pub async fn filtered_knn(
    db: &Surreal<Client>,
    query_vec: &[f32],
    user_id: &str,
    top_k: usize,
) -> surrealdb::Result<Vec<MemoryResult>> {
    let mut res = db
        .query(
            "SELECT id, content, \
             vector::distance::cosine(embedding, $vec) AS score \
             FROM memories \
             WHERE user_id = $uid \
               AND embedding <|$k|> $vec \
             ORDER BY score",
        )
        .bind(("vec", query_vec.to_vec()))
        .bind(("uid", user_id))
        .bind(("k", top_k))
        .await?;
    Ok(res.take(0)?)
}
```

---

## 5. Oversampling and Re-ranking

Retrieve more candidates than needed (oversampling), then re-rank with a more expensive model or
cross-encoder to improve final precision without hurting recall.

### Pattern

```
1. ANN query → top N*oversampling candidates (fast, approximate)
2. Re-score with cross-encoder or full-precision distance (slow, exact)
3. Return top N after re-ranking
```

### SurrealQL — oversample then re-rank in application layer

```sql
-- Fetch 5× candidates for re-ranking
SELECT id, content, embedding,
       vector::distance::cosine(embedding, $query_vec) AS approx_score
FROM memories
WHERE embedding <|50,64|> $query_vec
ORDER BY approx_score;
```

Application then re-ranks the 50 results and returns the top 10.

### Rust — oversample utility

```rust
/// Fetch oversampled candidates for external re-ranking.
pub async fn oversampled_knn(
    db: &Surreal<Client>,
    query_vec: &[f32],
    top_k: usize,
    oversample: usize,
) -> surrealdb::Result<Vec<MemoryResult>> {
    let fetch = top_k * oversample;
    let mut res = db
        .query(
            "SELECT id, content, \
             vector::distance::cosine(embedding, $vec) AS score \
             FROM memories \
             WHERE embedding <|$fetch,64|> $vec \
             ORDER BY score",
        )
        .bind(("vec", query_vec.to_vec()))
        .bind(("fetch", fetch))
        .await?;
    Ok(res.take(0)?)
}
```

---

## 6. Recall Measurement

```python
def recall_at_k(predictions: list[list[str]], ground_truth: list[list[str]], k: int) -> float:
    """Fraction of true top-K neighbours found in predicted top-K."""
    correct = sum(
        len(set(p[:k]) & set(t[:k]))
        for p, t in zip(predictions, ground_truth)
    )
    return correct / (len(predictions) * k)
```

---

## 7. Best Practices

### Do's
- **Normalise embeddings** before insertion for numerically stable cosine similarity
- **Use pre-filter** when the filter has high selectivity (few records match)
- **Use post-filter / oversampling** when selectivity is low (most records match)
- **Increase efSearch** incrementally until target recall is met; stop there
- **Measure recall@K on a held-out query set** before and after any parameter change

### Don'ts
- **Don't skip evaluation** — measure first, tune second
- **Don't use flat scan** (no index) on datasets > 10 K vectors in production
- **Don't ignore P99 latency** — outlier queries define user experience
- **Don't store unnormalised vectors** if using `COSINE` distance (results are incorrect)
- **Don't hardcode efSearch** — expose it as a tunable parameter per query profile
