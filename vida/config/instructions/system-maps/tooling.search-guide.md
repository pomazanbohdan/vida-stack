# Tooling Guide

This document contains detailed operational guidance for project code-search tools.
`AGENTS.md` and `AGENTS.sidecar.md` keep normative policy; this file keeps examples and lookup details.

## Search Routing Contract

Use the smallest search surface that answers the current question, and keep discovery separate from proof.

| Tool | Contract | Use it for | Not sufficient for |
|---|---|---|---|
| `ccc` | Semantic discovery and ranked concept search | Finding likely files, concepts, and related implementation areas | Proof, tests, or current runtime truth |
| `ctx_compose` → `ctx_search` | Context and filesystem-aware confirmation | Confirming source context, impact, and nearby symbols after discovery | Proof, tests, or runtime truth |
| `rg` | Exact symbol, string, path, or high-recall fallback | Verifying exact occurrences and locating precise edit anchors | Proof, tests, or runtime truth |

### `ccc` — semantic discovery

Use `ccc` for concept-oriented discovery. Keep its index initialized and refresh it after significant code changes.

```text
ccc init                         # only when the project is not initialized
ccc index                       # refresh after refactors or renamed/new files
ccc search --lang rust --limit 10 "TaskFlow dispatch run graph"
```

If the index is stale, incomplete, or unavailable, continue with `ctx_search` and `rg`, and record the fallback when it affects evidence quality.

### `ctx_search` — confirmation and context

Run `ctx_compose` first for the bounded task, then use `ctx_search` to confirm the live filesystem context and impact around discovery results.

```text
ctx_compose(task="TaskFlow dispatch run graph")
ctx_search(action="regex", pattern="dispatch_contract_execution_lane_sequence")
ctx_search(action="symbol", name="derive_advanced_run_graph_state")
```

Use bounded reads around confirmed matches; do not treat a compressed or indexed result as proof.

### `rg` — exact search and fallback

Use `rg` for exact symbols, strings, paths, and high-recall confirmation.

```text
rg -n "SessionExpired|AccessDenied" app test
rg -n "RetryInterceptor|reAuthenticate" app
rg --files | rg "vida|protocol|beads|runtime"
```

When to use:
- "Where is error X handled?"
- "How is auth implemented?"
- "Where is cli worker Y used?"

## Search Strategy

```text
Need concept discovery?
  -> ccc search <concept>
  -> ctx_compose + ctx_search to confirm live context

Know an exact symbol/string/path?
  -> rg -n <pattern> <paths>
  -> ctx_search or ctx_read for bounded context

Need callers, impact, or architecture context?
  -> ccc discovery
  -> ctx_search confirmation
  -> bounded source reads

Need proof?
  -> run the focused test, runtime command, or project quality gate
  -> attach the result; search output alone is never proof
```

Batch independent read-only searches when scopes are disjoint. Keep tests, builds, runtime commands, and other stateful proof serialized and separate from discovery output.

## Log-Search Guardrails

For runtime artifacts (`.vida/logs`, `.vida/state`, `.beads`):

1. Prefer one exact file plus exact-key search.
2. Prefer `sed -n start,end` after locating the exact anchor.
3. Avoid broad recursive `rg` sweeps unless a documented escalation requires wider correlation.
4. Avoid dumping full JSON/JSONL payloads when a bounded excerpt or targeted key lookup is sufficient.

## Tool Capability Contract

For runtime tool availability/fallback consistency, use:

```text
bash tool-capability.sh matrix
bash tool-capability.sh resolve <required_tool>
bash tool-capability.sh evidence <task_id> <required_tool> [impact]
bash tool-capability.sh use <task_id> <required_tool> [impact]
```

Fallback evidence line format (mandatory for non-trivial fallback):

```text
required_tool -> fallback_used -> impact
```

Critical rule:

- If the DB-backed task runtime is unavailable, stop execution (no SSOT fallback path).

## Known Tool Limitations (2026-03-02)

| Tool | Known issue |
|---|---|
| `symbol_graph` with callers/callees | `Invalid thing ID format` |
| `recall_code` with vector mode | Falls back to BM25 (`vector_score=0.0`) |
| `search_memory` | Vector cosine errors or empty results |
| `knowledge_graph` | `table relations does not exist` |

-----
artifact_path: config/system-maps/tooling
artifact_type: system_map
artifact_version: '1'
artifact_revision: '2026-03-09'
schema_version: '1'
status: canonical
source_path: vida/config/instructions/system-maps/tooling.search-guide.md
created_at: '2026-03-06T22:42:30+02:00'
updated_at: '2026-07-14T00:49:42.699Z'
changelog_ref: tooling.search-guide.changelog.jsonl
