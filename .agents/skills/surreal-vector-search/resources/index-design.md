# SurrealDB Vector Index Design

Reference for choosing, configuring, and tuning vector indexes in SurrealDB 3.0.

---

## 1. Index Type Selection

```
Data Size            Recommended Strategy
──────────────────────────────────────────────────────────
< 10 K vectors   →   Flat exact scan (no HNSW, low overhead)
10 K – 1 M       →   HNSW (default parameters)
1 M – 100 M      →   HNSW + INT8 quantization
> 100 M          →   HNSW + INT8/PQ + tiered storage / sharding
```

SurrealDB 3.0 exposes HNSW as the primary ANN index type via `DEFINE INDEX`.

---

## 2. SurrealDB HNSW DDL

```sql
-- Basic HNSW index (cosine distance, 1536-dim embeddings)
DEFINE INDEX memory_vec_idx
  ON TABLE memories
  FIELDS embedding
  TYPE HNSW
  DIMENSION 1536
  DIST COSINE
  M 16
  EFC 128;

-- High-recall variant (more memory, slower build)
DEFINE INDEX memory_vec_idx_hq
  ON TABLE memories
  FIELDS embedding
  TYPE HNSW
  DIMENSION 1536
  DIST COSINE
  M 32
  EFC 256;

-- Memory-optimised variant
DEFINE INDEX memory_vec_idx_lean
  ON TABLE memories
  FIELDS embedding
  TYPE HNSW
  DIMENSION 768
  DIST EUCLIDEAN
  M 8
  EFC 64;
```

### Rust (`surrealdb` crate) – define index at startup

```rust
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

pub async fn ensure_vector_index(db: &Surreal<Client>) -> surrealdb::Result<()> {
    db.query(
        "DEFINE INDEX IF NOT EXISTS memory_vec_idx \
         ON TABLE memories FIELDS embedding \
         TYPE HNSW DIMENSION 1536 DIST COSINE M 16 EFC 128"
    )
    .await?;
    Ok(())
}
```

---

## 3. HNSW Parameters

| Parameter | DDL Keyword | Default | Effect |
|-----------|-------------|---------|--------|
| **M** | `M` | 16 | Bi-directional edges per node — ↑ = better recall, more memory |
| **efConstruction** | `EFC` | 128 | Build-time beam width — ↑ = denser graph, slower build |
| **efSearch** | query-time `<\|K,EF\|>` | = K | Search beam width — ↑ = higher recall, slower query |

### Parameter recommendations by scale

| Vectors | M | EFC | Notes |
|---------|---|-----|-------|
| < 100 K | 16 | 100 | SurrealDB defaults, good starting point |
| 100 K – 1 M | 32 | 200 | Improves recall for denser datasets |
| > 1 M | 48 | 256 | High memory; pair with quantization |

### recall vs efSearch (empirical heuristics)

| Target recall | efSearch |
|---------------|----------|
| ≥ 0.99 | 256 |
| ≥ 0.95 | 128 |
| ≥ 0.90 | 64 |
| Speed priority | 32 |

---

## 4. Distance Metrics

| SurrealDB `DIST` | Mathematical definition | Ideal use case |
|-----------------|------------------------|----------------|
| `COSINE` | `1 − (A·B)/(‖A‖‖B‖)` | L2-normalised embeddings (most LLM models) |
| `EUCLIDEAN` | `√Σ(aᵢ−bᵢ)²` | Raw / unnormalised embeddings |
| `MANHATTAN` | `Σ\|aᵢ−bᵢ\|` | Sparse or high-dim vectors |
| `MINKOWSKI` | Generalised Lp norm | Custom distance families |
| `HAMMING` | Bit-count of XOR | Binary quantised vectors |
| `PEARSON` | Correlation coefficient | Magnitude-independent comparison |

**Rule of thumb**: normalise embeddings before insertion → use `COSINE`.
If embeddings are already unit-normalised, `COSINE` and `EUCLIDEAN` are equivalent.

---

## 5. Quantization Strategies

Quantization reduces memory footprint at the cost of some recall. Always re-score the top-N
candidates with full-precision vectors to recover quality.

### Memory comparison (1 M × 1536-dim vectors)

```
Precision     Bytes/dim   Total (index + vectors)
──────────────────────────────────────────────────
FP32          4           ~6.1 GB
FP16          2           ~3.1 GB
INT8          1           ~1.5 GB
PQ (8 sub)    ~0.05       ~200 MB  (lossy)
Binary        0.125       ~400 MB  (very lossy)
```

### INT8 scalar quantization (generic algorithm)

```python
import numpy as np

def scalar_quantize_int8(vectors: np.ndarray):
    """Compress FP32 → INT8, return quantized array and calibration params."""
    min_val = vectors.min()
    max_val = vectors.max()
    scale   = 255.0 / (max_val - min_val)
    quantized = np.clip(
        np.round((vectors - min_val) * scale), 0, 255
    ).astype(np.uint8)
    return quantized, {"min": min_val, "scale": scale}

def dequantize_int8(quantized: np.ndarray, params: dict) -> np.ndarray:
    return quantized.astype(np.float32) / params["scale"] + params["min"]
```

### Product Quantization (PQ) — generic algorithm

```python
from sklearn.cluster import KMeans
import numpy as np

def product_quantize(vectors: np.ndarray, n_subvectors: int = 8, n_centroids: int = 256):
    """
    Compress vectors using PQ. Returns (codes, codebooks).
    Storage: n_subvectors bytes per vector (vs dim*4 bytes for FP32).
    """
    n, dim = vectors.shape
    assert dim % n_subvectors == 0
    sub_dim  = dim // n_subvectors
    codebooks = []
    codes     = np.zeros((n, n_subvectors), dtype=np.uint8)

    for i in range(n_subvectors):
        sub = vectors[:, i*sub_dim:(i+1)*sub_dim]
        km  = KMeans(n_clusters=n_centroids, random_state=42)
        codes[:, i] = km.fit_predict(sub)
        codebooks.append(km.cluster_centers_)

    return codes, codebooks
```

---

## 6. Memory Estimation

```python
def estimate_memory(num_vectors: int, dimensions: int,
                    quantization: str = "fp32", hnsw_m: int = 16) -> dict:
    bytes_per_dim = {"fp32": 4, "fp16": 2, "int8": 1, "pq": 0.05, "binary": 0.125}
    vec_bytes   = num_vectors * dimensions * bytes_per_dim[quantization]
    # HNSW graph: ~M*2 edges per node, 4 bytes each
    graph_bytes = num_vectors * hnsw_m * 2 * 4
    total       = vec_bytes + graph_bytes
    return {
        "vectors_mb":  vec_bytes   / 1024**2,
        "graph_mb":    graph_bytes / 1024**2,
        "total_mb":    total       / 1024**2,
        "total_gb":    total       / 1024**3,
    }
```

---

## 7. Performance Monitoring

### Recall calculation

```python
def recall_at_k(predictions: list[list[str]], ground_truth: list[list[str]], k: int) -> float:
    correct = sum(
        len(set(p[:k]) & set(t[:k]))
        for p, t in zip(predictions, ground_truth)
    )
    return correct / (len(predictions) * k)
```

### Latency benchmarking (language-agnostic pattern)

```python
import time, numpy as np

def benchmark_search(search_fn, queries, k=10, iterations=5):
    latencies = []
    for _ in range(iterations):
        for q in queries:
            t0 = time.perf_counter()
            search_fn(q, k=k)
            latencies.append((time.perf_counter() - t0) * 1000)
    arr = np.array(latencies)
    return {
        "p50_ms":  np.percentile(arr, 50),
        "p95_ms":  np.percentile(arr, 95),
        "p99_ms":  np.percentile(arr, 99),
        "qps":     len(latencies) / (arr.sum() / 1000),
    }
```

### Safety checklist

- [ ] Benchmark on staging with production-representative data before deploying index changes
- [ ] Keep a rollback DDL (`REMOVE INDEX …; DEFINE INDEX …`) ready
- [ ] Track recall@10 as a continuous metric; alert on regressions > 2 %
- [ ] Validate index usage with `EXPLAIN SELECT … FROM … WHERE … <|K,EF|> (embedding, $vec)`
- [ ] Never reindex in production without a phased rollout or fallback
