# RAG Evaluation Metrics

> **Project context**: Metric computation can run in Rust or Python (offline eval scripts).
> Python reference implementations are provided; port to Rust as needed.

---

## Why Evaluate Retrieval Separately

Generation quality is a lagging indicator. Retrieval quality must be measured independently:

```
Retrieval recall → Generation faithfulness → Answer accuracy
     ↑                      ↑                      ↑
Measure here first    Then here             Finally here
```

If retrieval recall is low, no amount of prompt engineering fixes it.

---

## Core Metrics

### Precision@k

"Of the top-k retrieved chunks, how many are relevant?"

```
Precision@k = |Relevant ∩ Retrieved_k| / k
```

- High Precision@k → fewer irrelevant chunks in LLM context → less noise
- Target: P@5 ≥ 0.7 for production systems

### Recall@k

"Of all relevant chunks, how many did we retrieve in top-k?"

```
Recall@k = |Relevant ∩ Retrieved_k| / |Relevant|
```

- High Recall@k → LLM has all the information it needs
- Target: R@10 ≥ 0.8 for knowledge-critical applications

### MRR (Mean Reciprocal Rank)

"How high in the ranking is the first relevant result?"

```
MRR = (1/|Q|) Σ_q 1/rank_first_relevant(q)
```

- MRR = 1.0 means the first result is always relevant
- MRR = 0.5 means the first relevant result is at rank 2 on average
- Especially useful for single-answer retrieval

### NDCG@k (Normalized Discounted Cumulative Gain)

"Are relevant chunks ranked higher than irrelevant ones, accounting for graded relevance?"

```
DCG@k  = Σ_{i=1}^{k} rel_i / log2(i+1)
NDCG@k = DCG@k / IDCG@k
```

- NDCG handles graded relevance (highly relevant = 2, relevant = 1, irrelevant = 0)
- NDCG@10 ≥ 0.7 is a reasonable production target

---

## Python Reference Implementation

```python
import numpy as np
from typing import List, Set, Dict

def precision_at_k(relevant: Set[str], retrieved: List[str], k: int) -> float:
    """Fraction of top-k results that are relevant."""
    retrieved_k = retrieved[:k]
    hits = len(set(retrieved_k) & relevant)
    return hits / k if k > 0 else 0.0


def recall_at_k(relevant: Set[str], retrieved: List[str], k: int) -> float:
    """Fraction of relevant docs retrieved in top-k."""
    retrieved_k = retrieved[:k]
    hits = len(set(retrieved_k) & relevant)
    return hits / len(relevant) if relevant else 0.0


def reciprocal_rank(relevant: Set[str], retrieved: List[str]) -> float:
    """1 / rank of first relevant result; 0 if none found."""
    for i, doc_id in enumerate(retrieved):
        if doc_id in relevant:
            return 1.0 / (i + 1)
    return 0.0


def ndcg_at_k(
    relevant: Set[str],
    retrieved: List[str],
    k: int,
    graded_relevance: Dict[str, int] = None,
) -> float:
    """
    NDCG@k supporting binary or graded relevance.
    graded_relevance: {doc_id: relevance_score} — if None, binary relevance used.
    """
    def rel(doc_id: str) -> float:
        if graded_relevance:
            return graded_relevance.get(doc_id, 0)
        return 1.0 if doc_id in relevant else 0.0

    dcg = sum(
        rel(doc) / np.log2(i + 2)
        for i, doc in enumerate(retrieved[:k])
    )
    # Ideal: best possible ordering of relevant docs
    ideal_rels = sorted(
        [rel(d) for d in (graded_relevance or {doc: 1 for doc in relevant})],
        reverse=True,
    )[:k]
    idcg = sum(r / np.log2(i + 2) for i, r in enumerate(ideal_rels))
    return dcg / idcg if idcg > 0 else 0.0


def evaluate_retrieval(
    test_cases: List[Dict],
    retriever_fn,  # fn(query: str, k: int) -> List[str]
    k: int = 10,
) -> Dict[str, float]:
    """
    Evaluate a retriever across a test set.

    test_cases format:
      [{"query": "...", "relevant_ids": ["doc1", "doc2"]}, ...]
    """
    results = {f"precision@{k}": [], f"recall@{k}": [], "mrr": [], f"ndcg@{k}": []}

    for case in test_cases:
        query = case["query"]
        relevant = set(case["relevant_ids"])
        retrieved = retriever_fn(query, k)

        results[f"precision@{k}"].append(precision_at_k(relevant, retrieved, k))
        results[f"recall@{k}"].append(recall_at_k(relevant, retrieved, k))
        results["mrr"].append(reciprocal_rank(relevant, retrieved))
        results[f"ndcg@{k}"].append(ndcg_at_k(relevant, retrieved, k))

    return {name: float(np.mean(vals)) for name, vals in results.items()}
```

### Example Usage

```python
# Ground-truth test set (build with SME annotation or LLM-assisted labeling)
test_cases = [
    {
        "query": "How does the memory MCP store embeddings?",
        "relevant_ids": ["chunk_42", "chunk_43", "chunk_101"]
    },
    {
        "query": "What is the SurrealDB schema for memory entries?",
        "relevant_ids": ["chunk_7", "chunk_88"]
    },
]

def my_retriever(query: str, k: int) -> list[str]:
    # Call your Rust retriever via subprocess or HTTP for evaluation
    import subprocess, json
    result = subprocess.check_output(
        ["./target/release/memory-mcp", "retrieve", "--query", query, "--k", str(k)]
    )
    return json.loads(result)["chunk_ids"]

metrics = evaluate_retrieval(test_cases, my_retriever, k=10)
print(metrics)
# {'precision@10': 0.73, 'recall@10': 0.88, 'mrr': 0.91, 'ndcg@10': 0.84}
```

---

## Rust Reference Implementation

```rust
pub struct RetrievalMetrics {
    pub precision_at_k: f64,
    pub recall_at_k: f64,
    pub mrr: f64,
    pub ndcg_at_k: f64,
}

pub fn evaluate_retrieval(
    relevant: &std::collections::HashSet<String>,
    retrieved: &[String],
    k: usize,
) -> RetrievalMetrics {
    let retrieved_k: Vec<&String> = retrieved.iter().take(k).collect();

    // Precision@k
    let hits = retrieved_k.iter().filter(|id| relevant.contains(*id)).count();
    let precision = hits as f64 / k as f64;

    // Recall@k
    let recall = if relevant.is_empty() {
        0.0
    } else {
        hits as f64 / relevant.len() as f64
    };

    // MRR
    let mrr = retrieved.iter()
        .enumerate()
        .find(|(_, id)| relevant.contains(*id))
        .map(|(i, _)| 1.0 / (i + 1) as f64)
        .unwrap_or(0.0);

    // NDCG@k
    let dcg: f64 = retrieved_k.iter()
        .enumerate()
        .map(|(i, id)| {
            let rel = if relevant.contains(*id) { 1.0 } else { 0.0 };
            rel / (i as f64 + 2.0).log2()
        })
        .sum();

    let ideal_hits = relevant.len().min(k);
    let idcg: f64 = (0..ideal_hits)
        .map(|i| 1.0 / (i as f64 + 2.0).log2())
        .sum();

    let ndcg = if idcg > 0.0 { dcg / idcg } else { 0.0 };

    RetrievalMetrics { precision_at_k: precision, recall_at_k: recall, mrr, ndcg_at_k: ndcg }
}
```

---

## End-to-End RAG Evaluation

Beyond retrieval, measure the full pipeline:

| Metric | Measures | Tool |
|--------|----------|------|
| **Faithfulness** | Answer grounded in retrieved context | LLM-as-judge or NLI model |
| **Answer relevance** | Answer addresses the question | Embedding similarity |
| **Context precision** | Retrieved context is useful | Human annotation / LLM judge |
| **Context recall** | All needed info was retrieved | Ground-truth comparison |

### LLM-as-Judge Pattern

```python
def judge_faithfulness(answer: str, context: str, llm) -> float:
    prompt = f"""Rate 0-1 how faithful the answer is to the context.
Context: {context}
Answer: {answer}
Score (0.0-1.0):"""
    score_str = llm.complete(prompt).strip()
    try:
        return float(score_str)
    except ValueError:
        return 0.0
```

---

## Benchmarking Targets

| Metric | Acceptable | Good | Excellent |
|--------|------------|------|-----------|
| Precision@5 | ≥ 0.5 | ≥ 0.7 | ≥ 0.85 |
| Recall@10 | ≥ 0.6 | ≥ 0.8 | ≥ 0.92 |
| MRR | ≥ 0.5 | ≥ 0.75 | ≥ 0.90 |
| NDCG@10 | ≥ 0.5 | ≥ 0.70 | ≥ 0.85 |

---

## Building a Test Set

1. **Curate representative queries** — cover all query types users will issue
2. **Annotate relevant chunks** — SME review or LLM-assisted with human check
3. **Include hard negatives** — near-miss documents that should NOT rank highly
4. **Version the test set** — track metric changes as the index evolves
5. **Automate in CI** — run retrieval eval on every index rebuild or model change

### LLM-Assisted Test Set Generation

```python
def generate_test_cases(chunks: list[str], llm, n_per_chunk: int = 2) -> list[dict]:
    """Generate question → relevant_chunk pairs using LLM."""
    test_cases = []
    for i, chunk in enumerate(chunks):
        prompt = f"""Generate {n_per_chunk} questions answered by this passage:
---
{chunk}
---
Output one question per line."""
        questions = llm.complete(prompt).strip().split("\n")
        for q in questions:
            if q.strip():
                test_cases.append({"query": q.strip(), "relevant_ids": [f"chunk_{i}"]})
    return test_cases
```

---

## Monitoring in Production

Track these over time:

- **Mean retrieval latency** (P50, P95, P99)
- **Cache hit rate** for repeated queries
- **NDCG drift** — alert if drops >5% week-over-week
- **Zero-result rate** — queries returning no chunks above threshold
- **Embedding staleness** — fraction of indexed chunks older than N days
