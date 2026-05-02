# Design: Expand ALL Recall Queries with Synonym Expansion

**Date:** 2026-05-02
**Status:** Approved
**Approach:** A — Expand all recall queries across CLI, MCP, and REPL

## Problem Statement

The `recall` command uses raw user queries against SQLite FTS5 `MATCH` directly. When a user searches `"code coverage preferences"`, FTS5 looks for that exact phrase. The stored memory says *"User prefers code coverage of at least 80%…"* — the words are there but not adjacent, so FTS5 phrase matching returns nothing.

Meanwhile the `BIND` command already does the right thing: it expands queries with synonyms and morphology variants, and finds the memories. The `recall` command should behave the same way.

## Goals

1. `recall "code coverage preferences"` should find `semantic` memories about code coverage
2. MCP `recall` tool should also use expanded queries
3. REPL `recall` command should also use expanded queries
4. Keep existing `limit` and ranking behavior unchanged
5. No schema or DB migration changes

## Design

### Core Change

All `recall` paths will go through `expand_query()` before executing FTS5 search. The flow is:

```
User query → split into terms → remove stop words → simple stemming → SYNONYM_MAP expansion → OR-joined FTS5 query → search
```

### Files to Modify

| File | Change |
|------|--------|
| `src/repl/runner.rs` | In `cmd_recall`, expand `query` before calling `manager.recall()` |
| `src/mcp/mod.rs` | In `recall` MCP handler, expand `query` before calling `manager.recall()` |

### API Change

`TierManager::recall()` currently accepts a raw `&str` query and passes it straight to `search_content()`. The `cmd_recall` and MCP handler will instead:

1. Split the raw query into whitespace-separated terms
2. Call `expand_query(&terms)` → `Vec<String>`
3. Build `fts_query = terms.join(" OR ")`
4. Pass the `fts_query` OR-joined string to `manager.recall()` (or better, call `manager.recall_expanded()` which already does the OR-join)

### Alternative: Use `recall_expanded`

`TierManager` already has `recall_expanded(expanded_terms, memory_types, limit)` which calls `search_content_expanded()` that joins terms with `OR`. Reusing this method is cleaner than duplicating the OR-join logic.

So the change is:
- `cmd_recall`: `manager.recall_expanded(&expanded, &types_to_search, limit)` instead of `manager.recall(&query, ...)`
- MCP `recall`: same change

### SYNONYM_MAP Additions

Add two synonym clusters to help with this specific use case:

```rust
("coverage", &["coverage", "test coverage", "code coverage"]),
("preference", &["preference", "preferences", "prefer", "prefers"]),
```

(These map terms `coverage` → `coverage`, `test coverage`, `code coverage` and vice versa.)

## Testing Strategy

1. **Unit test** in `context::query_expansion` that `"coverage"` expands to include `"test coverage"` and `"code coverage"`.
2. **Unit test** in `context::query_expansion` that `"preference"` expands to include `"preferences"`, `"prefer"`, `"prefers"`.
3. **Integration test** in `tests/` that stores a memory with "code coverage" content, then recalls with `"coverage preferences"` and asserts it finds the memory.
4. **Integration test** via MCP JSON-RPC that the same recall finds the memory.

## Non-Goals

- Rebuilding FTS5 with Porter tokenizer (Approach C) — too invasive, requires DB migration
- Fallback-only expansion (Approach B) — adds latency without clear benefit since OR-joins on a small index are fast
- Changing the `Command::Recall` protocol struct — keep the wire format unchanged

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| OR-joined queries return too many noisy results | Keep existing `limit` parameter; results already truncated at DB level |
| Synonym expansion changes exact-match semantics | Existing `MATCH` exact query is discarded; this is intentional per user request |
| Performance hit from term expansion | FTS5 `OR` on ≤20 terms against a user-local DB is negligible |

