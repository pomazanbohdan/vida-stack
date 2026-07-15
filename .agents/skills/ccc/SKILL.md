---
name: ccc
description: "This skill should be used when code search is needed (whether explicitly requested or as part of completing a task), when indexing the codebase after changes, or when the user asks about ccc, cocoindex-code, or the codebase index. Trigger phrases include 'search the codebase', 'find code related to', 'update the index', 'ccc', 'cocoindex-code'."
---

# ccc - Semantic Code Search & Indexing

`ccc` is the CLI for CocoIndex Code, providing semantic search over the current codebase and index management.

## Indexing Safety Boundary

The agent owns `ccc` searches for the current project, but indexing can send source
text to the configured embedding provider. Before running `ccc init`, `ccc index`,
or `ccc search --refresh`, first inspect the effective `ccc` embedding
configuration without starting indexing:

- If the effective provider is local-only (`sentence-transformers` or another
  explicitly local provider such as Ollama), the agent may initialize or refresh
  the index automatically.
- If the effective provider is remote/cloud (`litellm`, OpenAI, Gemini, Voyage,
  or any API-key-backed provider), do not index automatically. Ask the user for
  explicit opt-in for this repository and session before running an indexing
  command.
- If the effective provider cannot be determined, fail closed: do not run
  indexing commands until the user confirms that indexing may proceed.

- **Initialization**: If `ccc search` or `ccc index` fails with an initialization error (e.g., "Not in an initialized project directory"), apply the indexing safety boundary above before running `ccc init` from the project root directory, building the index with `ccc index`, and retrying the original command.
- **Index freshness**: Keep the index up to date only after applying the indexing safety boundary above. For local providers, run `ccc index` (or `ccc search --refresh`) when the index may be stale — e.g., at the start of a session, or after making significant code changes (new files, refactors, renamed modules). There is no need to re-index between consecutive searches if no code was changed in between.
- **Installation**: If `ccc` itself is not found (command not found), refer to [management.md](references/management.md) for installation instructions and inform the user.

## Searching the Codebase

To perform a semantic search:

```bash
ccc search <query terms>
```

The query should describe the concept, functionality, or behavior to find, not exact code syntax. For example:

```bash
ccc search database connection pooling
ccc search user authentication flow
ccc search error handling retry logic
```

### Filtering Results

- **By language** (`--lang`, repeatable): restrict results to specific languages.

  ```bash
  ccc search --lang python --lang markdown database schema
  ```

- **By path** (`--path`): restrict results to a glob pattern relative to project root. If omitted, defaults to the current working directory (only results under that subdirectory are returned).

  ```bash
  ccc search --path 'src/api/*' request validation
  ```

### Pagination

Results default to the first page. To retrieve additional results:

```bash
ccc search --offset 5 --limit 5 database schema
```

If all returned results look relevant, use `--offset` to fetch the next page — there are likely more useful matches beyond the first page.

### Working with Search Results

Search results include file paths and line ranges. To explore a result in more detail:

- Use the editor's built-in file reading capabilities (e.g., the `Read` tool) to load the matched file and read lines around the returned range for full context.
- When working in a terminal without a file-reading tool, use `sed -n '<start>,<end>p' <file>` to extract a specific line range.

## Settings

To view or edit embedding model configuration, include/exclude patterns, or language overrides, see [settings.md](references/settings.md).

## Management & Troubleshooting

For installation, initialization, daemon management, troubleshooting, and cleanup commands, see [management.md](references/management.md).
