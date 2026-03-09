---
name: surreal-vector-search
description: "SurrealDB 3.0 vector search expert. Covers HNSW index design, distance metrics, quantization, BM25 hybrid fusion (RRF/linear), and production tuning for Rust MCP projects. Use when building semantic search, RAG retrieval, or hybrid search with SurrealDB."
---

# SurrealDB Vector Search

Expert skill for vector search on **SurrealDB 3.0** in Rust-based MCP projects. Covers the full
lifecycle: index design, similarity patterns, quantization, hybrid BM25+vector fusion, and
production performance tuning.

## SurrealDB 3.0 Vector Capabilities

- **HNSW index** — native `DEFINE INDEX … TYPE HNSW` with configurable `M`, `EFC`, and distance
- **Distance metrics** — `EUCLIDEAN`, `COSINE`, `MANHATTAN`, `MINKOWSKI`, `HAMMING`, `PEARSON`
- **BM25 full-text** — `DEFINE ANALYZER` + `DEFINE INDEX … TYPE SEARCH ANALYZER … BM25`
- **Hybrid fusion** — combine `<|K,EF|>` KNN operator with BM25 ranking in a single SurrealQL query
- **Quantization** — INT8 / FP16 storage reduces memory; re-score with full precision at query time

## Use this skill when

- Defining or tuning a SurrealDB HNSW vector index
- Choosing distance metrics and quantization strategies
- Implementing ANN (approximate nearest-neighbour) queries in SurrealQL or Rust `surrealdb` crate
- Building hybrid search that combines BM25 keyword ranking with vector similarity
- Debugging recall regression or high latency in a SurrealDB vector index
- Scaling vector collections from thousands to millions of records

## Do not use this skill when

- You need a different vector database (Qdrant, Pinecone, pgvector) — those are out of scope
- The task is pure relational/graph queries with no vector component
- You only need exact (flat/brute-force) search on a tiny dataset (< 5 K records)

## Instructions

1. Clarify the data size, target recall, latency budget, and query patterns.
2. Choose an index strategy using the decision table in `resources/index-design.md`.
3. Select distance metric and quantization from `resources/index-design.md`.
4. Implement similarity patterns (KNN, filtered, metadata-conditioned) from
   `resources/similarity-patterns.md`.
5. Add BM25 hybrid fusion if keyword recall matters — see `resources/hybrid-search.md`.
6. Benchmark, tune HNSW parameters, and validate recall with the monitoring checklist in
   `resources/index-design.md`.

## Resources

| File | Contents |
|------|----------|
| `resources/index-design.md` | Index type selection, HNSW parameter tuning, quantization strategies, memory estimation, performance monitoring |
| `resources/similarity-patterns.md` | Distance metrics, ANN search patterns, metadata filtering, re-ranking |
| `resources/hybrid-search.md` | BM25 + vector fusion (RRF, linear), SurrealQL hybrid queries, cascade pipelines |

## Workflow

1. Profile query patterns and data characteristics
2. Define HNSW index with appropriate M / EFC / distance metric
3. Validate baseline recall vs latency on a staging dataset
4. Add metadata filters or hybrid BM25 layer as needed
5. Apply quantization if memory is constrained; re-score at query time
6. Monitor recall and latency continuously; reindex on drift

## Best Practices

- Start with SurrealDB defaults (M=16, EFC=128) then benchmark before tuning
- Always measure recall@K against ground truth before and after parameter changes
- Use `EXPLAIN` in SurrealQL to inspect query plans and confirm index usage
- Prefer RRF for hybrid fusion — it is robust without per-dataset weight tuning
- Plan an offline re-index path before going to production (no hot reindex without rollback)
- Cache frequently repeated query embeddings at the application layer
