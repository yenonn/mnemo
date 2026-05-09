# Semantic Storage Pipeline for sqlite-vec — Design Specification

> **Date:** 2026-05-09  
> **Project:** mnemo — Agent memory database  
> **Status:** Draft  
> **Scope:** Make the `vec` feature actually provide semantic search by writing embeddings at store-time  

---

## 1. Problem Statement

The codebase has full **query-time** infrastructure for vector search (`VectorStore`, `search_hybrid`, `recall_hybrid`) but zero **store-time** infrastructure. The `memory_vectors` table is created but never populated. Building with `--features vec` compiles ~200 lines of dead code.

**Impact:** Users who compile with `--features vec` get zero benefit. Hybrid search always falls back to FTS5-only because `memory_vectors` is empty.

---

## 2. Goals

| # | Goal |
|---|------|
| G1 | Every new **episodic** and **semantic** memory gets its embedding stored when `vec` feature + provider available |
| G2 | `recall` and `bind` use hybrid search and return semantically related results |
| G3 | System degrades gracefully: no provider → FTS5 only; no sqlite-vec → FTS5 only |
| G4 | No breaking changes to CLI, REPL, or MCP interfaces |
| G5 | Working tier stays lightweight (no embeddings for transient memories) |

---

## 3. Non-Goals

- Async/background embedding (out of scope — keep it synchronous and simple)
- Embedding existing memories (migration not required for v0.1)
- On-device embedding without external provider (wait for v0.4 pure-Rust embedder)
- Storing embeddings for working memories (only episodic + semantic)

---

## 4. Architecture

```
Store Path (when feature=vec + provider available):
  User: REMEMBER "User prefers dark mode" AS semantic
    → MemoryStore::insert("semantic", ...)
    → Embed content via EmbeddingGateway
    → VectorStore::insert(mem_id, embedding)
    → memory.db now has BOTH memories row + memory_vectors row

Query Path (same conditions):
  User: BIND "What are my visual settings?"
    → analyze_intent → RetrieveByTopic
    → build_query → "visual settings"
    → EmbeddingGateway::embed("visual settings")
    → VectorStore::search(query_vec, limit)
    → search_hybrid: merge BM25 + cosine
    → Return "User prefers dark mode" (semantic match, no word overlap)
```

---

## 5. Implementation Steps

### Step 1: Wire Embedding into `MemoryStore::insert()`

**File:** `src/store/memory.rs`

Add optional embedding storage inside `insert()`, gated by `#[cfg(feature = "vec")]`:

```rust
#[cfg(feature = "vec")]
fn maybe_store_embedding(
    conn: &rusqlite::Connection,
    memory_id: &str,
    content: &str,
) {
    use crate::store::VectorStore;
    use crate::embed::EmbeddingGateway;

    let vstore = VectorStore::new(conn);
    if !vstore.available() {
        return;
    }

    let gateway = match EmbeddingGateway::from_env() {
        Some(g) => g,
        None => return,
    };

    if let Ok(vec) = gateway.embed(content) {
        let _ = vstore.insert(memory_id, &vec);
    }
}
```

Call it at the end of `insert()`:

```rust
pub fn insert(...) -> SqliteResult<String> {
    // ... existing insert logic ...
    
    #[cfg(feature = "vec")]
    maybe_store_embedding(self.conn, &id, content);
    
    Ok(id)
}
```

**Rationale:** This is the single point where ALL memory creation flows (CLI, REPL, MCP, extract, lifecycle). One hook covers every path.

---

### Step 2: Tier-Conditional Embedding

**File:** `src/store/memory.rs`

Modify `maybe_store_embedding` to skip working memories:

```rust
#[cfg(feature = "vec")]
fn maybe_store_embedding(
    conn: &rusqlite::Connection,
    memory_id: &str,
    content: &str,
    memory_type: &str,
) {
    if memory_type == "working" {
        return; // G5: working tier stays lightweight
    }
    // ... rest of embedding logic
}
```

Update the call site to pass `memory_type`.

**Rationale:** Working memories are seconds-old and get consolidated or dropped quickly. Embedding them wastes storage and API calls.

---

### Step 3: Update `VectorStore::insert()` to Handle Dimension Mismatch

**File:** `src/store/vector.rs`

The current `insert()` blindly uses `FLOAT[1536]`. This breaks with Ollama's 768-dim models.

Change the virtual table creation to use a dynamic dimension, or at minimum support multiple dimension profiles:

```rust
// Option A: Create table with provider's actual dimension
pub fn insert(&self, memory_id: &str, embedding: &[f32]) -> SqliteResult<()> {
    if !self.available {
        return Ok(());
    }
    // Validate dimension matches table schema, or recreate if needed
    let vec_json = vec_to_json(embedding);
    self.conn.execute(
        "INSERT INTO memory_vectors (memory_id, embedding) VALUES (?, vec_from_json(?))",
        rusqlite::params![memory_id, vec_json],
    )?;
    Ok(())
}
```

**Important:** `sqlite-vec` virtual tables have **fixed dimensions at creation time**. If the user switches from OpenAI (1536) to Ollama (768), the table must be recreated. Add a `_mnemo_meta` config key `vector_dimensions` and drop + recreate `memory_vectors` if it changes.

---

### Step 4: Add `EmbeddingGateway::from_env()` Caching

**File:** `src/embed/gateway.rs`

Currently `from_env()` is called repeatedly (every `bind`, every `remember`). It reads environment variables and allocates a new provider each time.

Add interior mutability or a simple once-cell cache:

```rust
use std::sync::OnceLock;

static GATEWAY_CACHE: OnceLock<Option<EmbeddingGateway>> = OnceLock::new();

impl EmbeddingGateway {
    pub fn from_env_cached() -> Option<&'static EmbeddingGateway> {
        GATEWAY_CACHE.get_or_init(|| Self::from_env())
    }
}
```

Update all call sites (`runner.rs`, `mcp/mod.rs`, `memory.rs`) to use `from_env_cached()`.

**Rationale:** Prevents repeated env lookups and redundant provider initialization.

---

### Step 5: Verify Hybrid Search End-to-End

**File:** `src/store/memory.rs` (already exists)

Confirm `search_hybrid()` behavior when `memory_vectors` has data:

1. `FTS5` search returns `["User prefers dark mode"]` — score = 0.4
2. `VectorStore::search("visual settings")` returns `["mem-abc123"]` — score = 0.6
3. Merge: `0.4 + 0.6 = 1.0` → ranks first
4. `recall_expanded()` fallback returns same result if vectors unavailable

No code changes needed here — just verify via integration test.

---

### Step 6: Integration Tests

**New file:** `tests/vector_search_test.rs`

```rust
#[cfg(feature = "vec")]
mod vector_tests {
    use mnemo::store::{MnemoDb, MemoryStore, VectorStore};
    use mnemo::embed::EmbeddingGateway;
    use mnemo::tier::TierManager;
    use tempfile::TempDir;

    #[test]
    fn test_semantic_memory_gets_embedded() {
        // Skip if no provider configured
        let _ = std::env::var("MNEMO_OPENAI_API_KEY")
            .expect("Set MNEMO_OPENAI_API_KEY to run this test");

        let dir = TempDir::new().unwrap();
        let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
        let store = MemoryStore::new(db.conn());
        
        let id = store.insert("semantic", "User prefers dark mode", 0.8, "test", &[]).unwrap();
        
        let vstore = VectorStore::new(db.conn());
        let count = vstore.count().unwrap();
        assert_eq!(count, 1, "Semantic memory should have embedding stored");
    }

    #[test]
    fn test_working_memory_skips_embedding() {
        // ... same setup, insert working memory, assert vstore.count() == 0
    }

    #[test]
    fn test_hybrid_search_finds_paraphrase() {
        // Insert "User prefers dark mode"
        // Query with "visual settings"
        // Assert result contains the dark mode memory via hybrid search
    }
}
```

**New file:** `tests/vector_fallback_test.rs`

```rust
// Tests that run WITHOUT vec feature or WITHOUT provider
#[cfg(not(feature = "vec"))]
mod fallback_tests {
    // Verify recall_expanded still works, no crashes
}
```

---

### Step 7: Documentation Updates

**Files:**
- `README.md` — Update section on hybrid search to say "Functional when provider configured"
- `AGENTS.md` — Note that `VectorStore::insert()` must be called for semantic memories
- `docs/superpowers/specs/2025-05-02-bind-retrieval-fix-design.md` — Mark store-time embedding tasks as complete

---

## 6. Test Plan

| Test | Type | Condition |
|------|------|-----------|
| `test_semantic_memory_gets_embedded` | Integration | `vec` + `MNEMO_OPENAI_API_KEY` set |
| `test_working_memory_skips_embedding` | Integration | `vec` + provider available |
| `test_hybrid_search_finds_paraphrase` | Integration | `vec` + provider + sqlite-vec installed |
| `test_fallback_when_no_provider` | Integration | `vec` feature on, no env vars |
| `test_fallback_when_no_sqlite_vec` | Integration | `vec` feature on, sqlite-vec not installed |
| `test_build_without_vec_feature` | Compile | `cargo test` (default features) |
| `test_build_with_vec_feature` | Compile | `cargo test --features vec` |

---

## 7. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Embedding API call fails during `remember` | High | Catch error silently; memory is still stored in `memories`, just no vector |
| Dimension mismatch (OpenAI → Ollama switch) | Medium | Store `vector_dimensions` in `_mnemo_meta`; recreate table on mismatch |
| sqlite-vec extension not installed | Low | `vstore.available()` returns false; graceful FTS5 fallback |
| Slow writes due to embedding latency | Medium | Skip working tier (G5); only embed episodic + semantic |
| Binary bloat from `reqwest` + embedding code | Low | Already compiled in; feature gate keeps `VectorStore` out |

---

## 8. Acceptance Criteria

- [ ] `cargo test --features vec` passes all existing + new tests
- [ ] `cargo test` (without `vec`) passes — no regression
- [ ] `mnemo remember "User prefers dark mode" --type semantic` stores an embedding when provider configured
- [ ] `mnemo bind "What are my visual settings?"` returns the dark mode memory via hybrid search
- [ ] `mnemo remember "transient thought" --type working` does NOT store an embedding
- [ ] MCP `remember` tool behaves identically to CLI
- [ ] No API key configured → same behavior as before (FTS5-only)

---

## 9. Estimated Effort

| Step | Effort |
|------|--------|
| Step 1–2: Wire `maybe_store_embedding` | 1 hour |
| Step 3: Dynamic dimension handling | 2 hours |
| Step 4: Gateway caching | 1 hour |
| Step 5: Verify hybrid merge | 30 min |
| Step 6: Integration tests | 2 hours |
| Step 7: Documentation | 30 min |
| **Total** | **~7 hours** |

---

## 10. Decision Log

| Decision | Rationale |
|----------|-----------|
| Embed in `MemoryStore::insert()` (not `TierManager`) | Single chokepoint; covers CLI, MCP, extract, lifecycle |
| Skip working tier | Transient memories don't need semantic retrieval |
| Synchronous embedding | Simple, predictable; async is v0.4 scope |
| No migration of old memories | SQLite-vec table stays empty for old DBs; new memories get vectors |
| Gateway caching via `OnceLock` | Avoids env lookups; thread-safe; minimal code |

---

## Related Documents

- `docs/superpowers/specs/2025-05-02-bind-retrieval-fix-design.md` — Original hybrid search design
- `docs/superpowers/plans/2025-05-02-bind-retrieval-fix-plan-b.md` — Implementation plan for vector search
- `src/store/vector.rs` — Existing `VectorStore` implementation
- `src/store/memory.rs` — `MemoryStore::insert()` and `search_hybrid()`
