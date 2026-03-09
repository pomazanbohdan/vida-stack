# Tooling Guide

This document contains detailed operational guidance for MCP search tools.
`AGENTS.md` keeps only normative policy; this file keeps examples and lookup details.

## Search Tools

### `rg` — primary cross-file search

Primary tool for cross-file code discovery.

```text
rg -n "SessionExpired|AccessDenied" app test
rg -n "RetryInterceptor|reAuthenticate" app
rg --files | rg "vida|protocol|beads|runtime"
```

When to use:
- "Where is error X handled?"
- "How is auth implemented?"
- "Where is cli subagent Y used?"

### `recall_code` — optional semantic search

Use only when available in the current runtime and `rg` search is insufficient.

### `search_symbols` — symbol-name lookup

Use when class/function name is known or partially known.

```text
search_symbols(query="ApiClient", limit=5)
search_symbols(query="Repository", symbol_type="class", limit=5)
search_symbols(query="authProvider", symbol_type="function", limit=5)
```

Returns `file_path`, `start_line`, `end_line`, `signature`, `symbol_type`.

### `symbol_graph` — symbol dependency graph

Use for relationship discovery between symbols.

```text
symbol_graph(symbol_id="248264f35a46f13e", action="related", depth=1)
```

Important limitations:
- Only `action="related"` is reliable in current setup.
- `symbol_id` must come from `search_symbols` result `id`.

### `recall` — notes/memory search

Use for previous architecture decisions/context.

```text
recall(query="authentication architecture decision", limit=3)
recall(query="API integration pattern", limit=3)
```

Notes:
- Correct tool is `recall` (not `recall_memory`)
- `search_memory` is unreliable in current environment

## Search Strategy

```text
Know exact symbol name?
  YES -> search_symbols + symbol_graph(related)
  NO  -> recall_code(query)
          -> narrower filters (pathPrefix/chunkType)
          -> explore agent only if still insufficient

Need architecture note from past sessions?
  -> recall(query)

Need filename pattern?
  -> rg --files | rg <pattern>

Need exact text lines?
  -> Grep
```

## Log-Search Guardrails

For runtime artifacts (`.vida/logs`, `.vida/state`, `.beads`):

1. Prefer one exact file plus exact-key search.
2. Prefer `sed -n start,end` after locating the exact anchor.
3. Avoid broad recursive `rg` sweeps unless a documented escalation requires wider correlation.
4. Avoid dumping full JSON/JSONL payloads when a bounded excerpt or targeted key lookup is sufficient.

## Tool Capability Contract

For runtime tool availability/fallback consistency, use:

```text
bash _vida/scripts/tool-capability.sh matrix
bash _vida/scripts/tool-capability.sh resolve <required_tool>
bash _vida/scripts/tool-capability.sh evidence <task_id> <required_tool> [impact]
bash _vida/scripts/tool-capability.sh use <task_id> <required_tool> [impact]
```

Fallback evidence line format (mandatory for non-trivial fallback):

```text
required_tool -> fallback_used -> impact
```

Critical rule:

- If `br` is unavailable, stop execution (no SSOT fallback path).

## Known Tool Limitations (2026-03-02)

| Tool | Known issue |
|---|---|
| `symbol_graph` with callers/callees | `Invalid thing ID format` |
| `recall_code` with vector mode | Falls back to BM25 (`vector_score=0.0`) |
| `search_memory` | Vector cosine errors or empty results |
| `knowledge_graph` | `table relations does not exist` |
