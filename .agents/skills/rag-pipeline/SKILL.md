---
name: rag-pipeline
description: "Unified RAG pipeline skill for Rust/MCP projects. Covers architecture, local embeddings via Candle ML framework, chunking strategies, retrieval patterns, hybrid search, reranking, and evaluation (NDCG/MRR). Use when: building RAG, vector search, semantic search, document retrieval, embeddings in Rust."
source: consolidated from rag-engineer, rag-implementation, embedding-strategies
---

# RAG Pipeline (Rust + Candle)

**Role**: RAG Systems Architect for Rust-native pipelines

The core pipeline is implemented in **Rust** using the [Candle](https://github.com/huggingface/candle) ML framework for local, offline embeddings. Python code in `resources/` is reference material only — do not default to Python implementations when Rust is the project language.

Retrieval quality determines generation quality. Obsess over chunking boundaries, embedding dimensions, and similarity metrics.

---

## Architecture Overview

```
Documents
   │
   ▼
[Chunker]  ←── chunking strategies (see resources/chunking.md)
   │
   ▼
[Embedder] ←── Candle (local) — BERT / BGE / E5 loaded via HuggingFace Hub
   │
   ▼
[Vector Store]  ←── SurrealDB / FAISS / in-process
   │
   ▼
[Retriever]  ←── dense + sparse hybrid (see resources/retrieval.md)
   │
   ▼
[Reranker]  ←── cross-encoder (Candle) or heuristic MMR
   │
   ▼
[LLM Context Window]
```

---

## Candle: Local Embeddings in Rust

Candle is HuggingFace's minimalist ML framework for Rust. It runs BERT-family models locally — no OpenAI API, no Python runtime.

### Adding Candle to Cargo.toml

```toml
[dependencies]
candle-core = { version = "0.8", features = ["cuda"] }  # or remove cuda for CPU-only
candle-nn = "0.8"
candle-transformers = "0.8"
hf-hub = { version = "0.3", features = ["tokio"] }
tokenizers = "0.19"
```

### Loading a BGE / E5 Model

```rust
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config};
use hf_hub::{api::tokio::Api, Repo, RepoType};
use tokenizers::Tokenizer;

pub struct CandleEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl CandleEmbedder {
    pub async fn from_hub(model_id: &str) -> anyhow::Result<Self> {
        let device = Device::Cpu; // or Device::new_cuda(0)?

        let api = Api::new()?;
        let repo = api.repo(Repo::new(model_id.to_string(), RepoType::Model));

        let config_path = repo.get("config.json").await?;
        let tokenizer_path = repo.get("tokenizer.json").await?;
        let weights_path = repo.get("model.safetensors").await?;

        let config: Config = serde_json::from_str(&std::fs::read_to_string(config_path)?)?;
        let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(anyhow::Error::msg)?;

        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], candle_core::DType::F32, &device)?
        };
        let model = BertModel::load(vb, &config)?;

        Ok(Self { model, tokenizer, device })
    }

    /// Embed a batch of texts; returns shape [batch, hidden_dim]
    pub fn embed(&self, texts: &[&str]) -> anyhow::Result<Tensor> {
        let encoded = self.tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(anyhow::Error::msg)?;

        let max_len = encoded.iter().map(|e| e.len()).max().unwrap_or(0);

        let input_ids: Vec<Vec<u32>> = encoded.iter()
            .map(|e| {
                let mut ids = e.get_ids().to_vec();
                ids.resize(max_len, 0);
                ids
            })
            .collect();

        let attention_mask: Vec<Vec<u32>> = encoded.iter()
            .map(|e| {
                let mut mask = e.get_attention_mask().to_vec();
                mask.resize(max_len, 0);
                mask
            })
            .collect();

        let input_ids = Tensor::new(input_ids, &self.device)?;
        let attention_mask = Tensor::new(attention_mask, &self.device)?;

        let embeddings = self.model.forward(&input_ids, &attention_mask, None)?;

        // CLS pooling (index 0) — adjust for mean pooling if model requires it
        let cls = embeddings.i((.., 0, ..))?;

        // L2-normalize for cosine similarity
        let norm = cls.sqr()?.sum_keepdim(1)?.sqrt()?;
        Ok(cls.broadcast_div(&norm)?)
    }
}
```

### Recommended Local Models

| Model | Dims | Notes |
|-------|------|-------|
| `BAAI/bge-small-en-v1.5` | 384 | Fast, good quality, CPU-friendly |
| `BAAI/bge-large-en-v1.5` | 1024 | Higher accuracy, more RAM |
| `intfloat/e5-small-v2` | 384 | Strong zero-shot, compact |
| `intfloat/multilingual-e5-base` | 768 | Multi-language support |
| `sentence-transformers/all-MiniLM-L6-v2` | 384 | Very fast, reasonable quality |

**Do not use OpenAI embedding APIs** in this project — all embeddings must be local via Candle.

---

## Capabilities

- Local vector embeddings via Candle (no API calls)
- Document chunking and preprocessing in Rust
- Dense + sparse (BM25) hybrid retrieval
- Reranking with cross-encoders or MMR
- Metadata-filtered similarity search
- Evaluation with NDCG, MRR, Precision@k

---

## Anti-Patterns

- **Fixed chunk size without overlap** — breaks sentence/context boundaries
- **Embedding everything indiscriminately** — index only what benefits retrieval
- **Skipping evaluation** — retrieval quality must be measured (see `resources/evaluation.md`)
- **Mixing embedding models** — vectors from different models are incompatible
- **OpenAI embeddings in this project** — violates local-first constraint

---

## Sharp Edges

| Issue | Severity | Solution |
|-------|----------|----------|
| Fixed-size chunking breaks context | High | Semantic chunking respecting document structure |
| Pure semantic search misses keywords | Medium | Hybrid BM25 + dense retrieval |
| Same model for code and prose | Medium | Use code-specialized models for code |
| First-stage results used directly | Medium | Add cross-encoder reranking step |
| Cramming max context into prompt | Medium | Relevance threshold filtering |
| Not measuring retrieval separately | High | Track NDCG/MRR per query type |
| Embeddings stale after doc updates | Medium | Incremental re-indexing on change |
| Candle CUDA feature missing on CPU server | High | Gate CUDA behind feature flag |

---

## Resources

- `resources/chunking.md` — chunking strategies and Rust implementations
- `resources/retrieval.md` — dense, sparse, hybrid, and reranking patterns
- `resources/evaluation.md` — NDCG, MRR, Precision@k with Python reference code

---

## Related Skills

Works well with: `ai-agents-architect`, `prompt-engineer`, `database-architect`, `backend`, `data-engineer`
