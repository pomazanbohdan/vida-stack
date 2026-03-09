# Chunking Strategies

Chunking is the most impactful preprocessing decision. Poor chunks produce poor embeddings, regardless of model quality.

---

## Principles

1. **Preserve semantic units** — sentences, paragraphs, code blocks, sections
2. **Use overlap** — 10–20% overlap maintains context at boundaries
3. **Attach metadata** — source, page, section header, timestamp
4. **Respect token limits** — model max is a hard constraint, not a target

---

## Strategy Comparison

| Strategy | Best For | Chunk Size | Overlap |
|----------|----------|------------|---------|
| Fixed token | Simple prose, quick prototyping | 512 tokens | 50–100 |
| Sentence boundary | Conversational, QA | 3–10 sentences | 1–2 sentences |
| Recursive character | Mixed content | 500–1000 chars | 100–200 chars |
| Markdown/header | Documentation, wikis | Section-sized | None (boundaries are natural) |
| Semantic similarity | Dense technical text | Variable | None |
| Hierarchical | Large documents, books | Parent 2000 / child 400 | Child overlap |

---

## Rust Implementations

### Fixed-Size with Overlap (character-based)

```rust
pub fn chunk_fixed(text: &str, size: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut chunks = Vec::new();
    let step = size.saturating_sub(overlap);
    let mut start = 0;

    while start < chars.len() {
        let end = (start + size).min(chars.len());
        chunks.push(chars[start..end].iter().collect());
        if end == chars.len() {
            break;
        }
        start += step;
    }
    chunks
}
```

### Sentence Boundary Chunking

```rust
/// Splits on sentence-ending punctuation + whitespace.
/// For production use, consider the `unicode-segmentation` crate.
pub fn chunk_by_sentences(
    text: &str,
    max_chars: usize,
    min_chars: usize,
) -> Vec<String> {
    // Naive sentence splitter — replace with proper tokenizer for production
    let sentence_ends = [". ", "! ", "? ", ".\n", "!\n", "?\n"];
    let mut chunks = Vec::new();
    let mut current = String::new();

    let mut remaining = text;
    while !remaining.is_empty() {
        // Find the next sentence boundary
        let boundary = sentence_ends
            .iter()
            .filter_map(|&sep| remaining.find(sep).map(|i| i + sep.len()))
            .min();

        match boundary {
            Some(pos) => {
                let sentence = &remaining[..pos];
                if current.len() + sentence.len() > max_chars && current.len() >= min_chars {
                    chunks.push(current.trim().to_string());
                    current = sentence.to_string();
                } else {
                    current.push_str(sentence);
                }
                remaining = &remaining[pos..];
            }
            None => {
                current.push_str(remaining);
                break;
            }
        }
    }

    if !current.trim().is_empty() {
        chunks.push(current.trim().to_string());
    }
    chunks
}
```

### Markdown Header Splitter

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MarkdownChunk {
    pub header: String,
    pub level: usize,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

pub fn chunk_markdown(text: &str) -> Vec<MarkdownChunk> {
    let mut chunks: Vec<MarkdownChunk> = Vec::new();
    let mut current_header = String::new();
    let mut current_level = 0usize;
    let mut current_content = String::new();

    for line in text.lines() {
        if let Some(header_text) = line.strip_prefix("### ") {
            flush_chunk(&mut chunks, &current_header, current_level, &current_content);
            current_header = header_text.to_string();
            current_level = 3;
            current_content.clear();
        } else if let Some(header_text) = line.strip_prefix("## ") {
            flush_chunk(&mut chunks, &current_header, current_level, &current_content);
            current_header = header_text.to_string();
            current_level = 2;
            current_content.clear();
        } else if let Some(header_text) = line.strip_prefix("# ") {
            flush_chunk(&mut chunks, &current_header, current_level, &current_content);
            current_header = header_text.to_string();
            current_level = 1;
            current_content.clear();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }
    flush_chunk(&mut chunks, &current_header, current_level, &current_content);
    chunks
}

fn flush_chunk(
    chunks: &mut Vec<MarkdownChunk>,
    header: &str,
    level: usize,
    content: &str,
) {
    let trimmed = content.trim();
    if !trimmed.is_empty() || !header.is_empty() {
        chunks.push(MarkdownChunk {
            header: header.to_string(),
            level,
            content: trimmed.to_string(),
            metadata: HashMap::new(),
        });
    }
}
```

### Hierarchical (Parent-Child) Chunking

Index small child chunks for precise retrieval; return the parent chunk as context to the LLM.

```rust
#[derive(Debug, Clone)]
pub struct HierarchicalChunk {
    pub id: String,
    pub parent_id: Option<String>,
    pub text: String,
    pub level: &'static str,  // "document" | "section" | "paragraph"
}

pub fn chunk_hierarchical(
    doc_id: &str,
    text: &str,
    parent_size: usize,
    child_size: usize,
    child_overlap: usize,
) -> Vec<HierarchicalChunk> {
    let mut result = Vec::new();
    let parent_chunks = chunk_fixed(text, parent_size, 0);

    for (pi, parent_text) in parent_chunks.iter().enumerate() {
        let parent_id = format!("{doc_id}_p{pi}");
        result.push(HierarchicalChunk {
            id: parent_id.clone(),
            parent_id: None,
            text: parent_text.clone(),
            level: "section",
        });

        let children = chunk_fixed(parent_text, child_size, child_overlap);
        for (ci, child_text) in children.iter().enumerate() {
            result.push(HierarchicalChunk {
                id: format!("{parent_id}_c{ci}"),
                parent_id: Some(parent_id.clone()),
                text: child_text.clone(),
                level: "paragraph",
            });
        }
    }
    result
}
```

---

## Python Reference (do not use in Rust project)

```python
# Reference only — equivalent logic, Python-style
from langchain.text_splitters import RecursiveCharacterTextSplitter

splitter = RecursiveCharacterTextSplitter(
    chunk_size=1000,
    chunk_overlap=200,
    separators=["\n\n", "\n", ". ", " ", ""]
)
chunks = splitter.split_text(text)
```

---

## Best Practices

- Chunk size 400–1000 chars (or ~128–512 tokens) covers most use cases
- Overlap of 10–20% preserves cross-boundary context
- Always store `source_id`, `chunk_index`, `header` in metadata for filtering
- For code: chunk by function/class boundaries, not character count (use `tree-sitter`)
- Test chunking quality by sampling 20 chunks and checking for broken sentences
