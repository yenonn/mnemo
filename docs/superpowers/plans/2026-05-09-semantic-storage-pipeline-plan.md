# Semantic Storage Pipeline — Atomic Implementation Plan

> **Spec:** MNEMO-VEC-001  
> **Date:** 2026-05-09  
> **Constraint:** Each task ≤ 1 hour  

---

## How to Use This Plan

1. Pick one task at a time  
2. Read its **Entry Criteria** and **Exit Criteria**  
3. Implement only what is described — no scope creep  
4. Run the verification command before declaring done  
5. Commit after each task (small, atomic commits)  

---

## Task 1: Wire Store-Time Embedding Hook (30–45 min)

**Goal:** When a memory is inserted, also store its embedding — but only for episodic and semantic tiers.

**Files:** `src/store/memory.rs`

**What to do:**
1. Add a private helper `maybe_store_embedding()` gated with `#[cfg(feature = "vec")]` inside `memory.rs`
2. It receives `conn`, `memory_id`, `content`, and `memory_type`
3. If `memory_type == "working"`, return immediately
4. Create `VectorStore`, check `available()`
5. Call `EmbeddingGateway::from_env()`, embed content, call `vstore.insert()`
6. Silently swallow any errors (memory must still be stored even if embedding fails)
7. Call this helper at the end of `MemoryStore::insert()`

**What NOT to do:**
- Do NOT change `MemoryStore::insert()` signature (keep return type as `SqliteResult<String>`)
- Do NOT add async/background logic
- Do NOT touch `VectorStore::insert()` internals yet

**Verification:**
```bash
cargo test --features vec
cargo test  # without vec — must pass too
```

**Commit message:** `feat(store): add maybe_store_embedding hook in MemoryStore::insert()`

---

## Task 2: Cache EmbeddingGateway to Avoid Repeated Env Lookups (20–30 min)

**Goal:** Stop re-reading env vars and re-allocating the provider on every `remember` and `bind` call.

**Files:** `src/embed/gateway.rs`

**What to do:**
1. Add `use std::sync::OnceLock;`
2. Add a static cache: `static GATEWAY_CACHE: OnceLock<Option<EmbeddingGateway>> = OnceLock::new();`
3. Add a new method `from_env_cached() -> Option<&'static EmbeddingGateway>`
4. The body: `GATEWAY_CACHE.get_or_init(|| Self::from_env())`
5. Keep `from_env()` unchanged for direct callers

**Verification:**
```bash
cargo test --features vec
cargo test
```

**Commit message:** `perf(embed): cache EmbeddingGateway with OnceLock`

---

## Task 3: Update Call Sites to Use Cached Gateway (15–20 min)

**Goal:** Switch the query-time callers to the cached gateway.

**Files:** `src/repl/runner.rs`, `src/mcp/mod.rs`

**What to do:**
1. In `runner.rs` `cmd_bind()`, replace `EmbeddingGateway::from_env()` with `EmbeddingGateway::from_env_cached()`
2. In `mcp/mod.rs` `handle_bind()` and `handle_recall()`, do the same
3. Adjust types if needed (`Option<&'static EmbeddingGateway>` vs `Option<EmbeddingGateway>`)

**What NOT to do:**
- Do NOT change `maybe_store_embedding` in `memory.rs` to use the cache yet (Task 4 does that)

**Verification:**
```bash
cargo test --features vec
cargo test
```

**Commit message:** `refactor(repl,mcp): use cached EmbeddingGateway in bind/recall paths`

---

## Task 4: Switch Store-Time Hook to Cached Gateway (10–15 min)

**Goal:** The embedding hook written in Task 1 should also use the cached gateway.

**Files:** `src/store/memory.rs`

**What to do:**
1. Inside `maybe_store_embedding()`, replace `EmbeddingGateway::from_env()` with `EmbeddingGateway::from_env_cached()`
2. Handle the `Option<&'static>` lifetime correctly

**Verification:**
```bash
cargo test --features vec
cargo test
```

**Commit message:** `perf(store): use cached gateway in maybe_store_embedding`

---

## Task 5: Dynamic Dimension Handling in VectorStore (45–60 min)

**Goal:** If the user switches from OpenAI (1536 dims) to Ollama (768 dims), recreate the `memory_vectors` table instead of failing silently.

**Files:** `src/store/vector.rs`, `src/store/db.rs` or `src/lifecycle/config.rs`

**What to do:**
1. When `VectorStore::new()` is called, read `_mnemo_meta` key `vector_dimensions` (via `ConfigStore` or direct SQL)
2. After a successful `insert()`, compare `embedding.len()` with the stored dimension:
   - If no stored dimension → store `embedding.len()` in `_mnemo_meta`
   - If stored dimension == `embedding.len()` → continue
   - If stored dimension != `embedding.len()` → `DROP TABLE memory_vectors`, recreate with new `FLOAT[N]`, store new dimension
3. Wrap this logic inside `VectorStore::insert()` or a new `ensure_dimension_match()` helper

**What NOT to do:**
- Do NOT migrate existing rows (acceptable loss for v0.1)
- Do NOT over-engineer a complex migration system

**Verification:**
```bash
cargo test --features vec
cargo test
```

**Commit message:** `feat(store): auto-recreate memory_vectors on dimension mismatch`

---

## Task 6: Integration Test — Semantic Gets Embedded, Working Skips (30–45 min)

**Goal:** Prove that the store-time hook actually writes embeddings.

**File:** `tests/vector_search_test.rs` (new)

**What to do:**
1. Create new test file, gated with `#[cfg(feature = "vec")]`
2. Write `test_semantic_memory_gets_embedded()`:
   - Skip test early if `MNEMO_OPENAI_API_KEY` is not set (or use a mock/stub if you prefer)
   - Create temp DB, insert semantic memory, assert `vstore.count() == 1`
3. Write `test_working_memory_skips_embedding()`:
   - Create temp DB, insert working memory, assert `vstore.count() == 0`
4. Use `tempfile::TempDir` and `MnemoDb` for isolation

**Verification:**
```bash
# Run only the new test file
cargo test --features vec --test vector_search_test

# If no API key, the test should be skipped gracefully (or fail fast with a clear message)
```

**Commit message:** `test(vec): add store-time embedding tests for semantic vs working`

---

## Task 7: Integration Test — Hybrid Search Paraphrase (45–60 min)

**Goal:** Prove that hybrid search finds memories with no lexical overlap.

**File:** `tests/vector_search_test.rs` (append)

**What to do:**
1. Write `test_hybrid_search_finds_paraphrase()`:
   - Insert semantic memory: `"User prefers dark mode"`
   - Query via `TierManager::recall_hybrid()` with `"visual settings"`
   - Assert result contains the dark mode memory
2. Requires `MNEMO_OPENAI_API_KEY` or `MNEMO_OLLAMA_ENDPOINT` to be set
3. If provider not available, `#[ignore]` the test with a clear reason

**What NOT to do:**
- Do NOT test exact scores (embedding models change, scores drift)
- Do NOT depend on external services in CI if you can avoid it

**Verification:**
```bash
cargo test --features vec --test vector_search_test -- --ignored
```

**Commit message:** `test(vec): add hybrid search paraphrase test`

---

## Task 8: Integration Test — Fallback Paths (30 min)

**Goal:** Prove that the system never crashes when sqlite-vec or provider is missing.

**Files:** `tests/vector_fallback_test.rs` (new), or append to existing

**What to do:**
1. `test_fallback_when_no_provider()` — with `vec` feature, no env vars, call `remember` + `recall`, assert FTS5 results still work
2. `test_fallback_when_no_sqlite_vec()` — same but on a system without sqlite-vec (or use in-memory DB where `vstore.available()` is false)
3. `test_recall_without_vec_feature()` — compile without `vec`, assert `recall` still works

**Verification:**
```bash
cargo test --test vector_fallback_test
cargo test  # default features
```

**Commit message:** `test(vec): add fallback tests for missing provider and sqlite-vec`

---

## Task 9: README + AGENTS.md Documentation Update (20–30 min)

**Goal:** Users should know that `--features vec` is now functional.

**Files:** `README.md`, `AGENTS.md`

**What to do:**
1. In `README.md` line ~267-280:
   - Change "Optional hybrid search" to "Hybrid search (requires sqlite-vec + provider)"
   - Add a note: "Embeddings are generated automatically at store-time for episodic and semantic memories"
2. In `AGENTS.md`:
   - Under "Tips for Agents", add: "When `--features vec` is enabled and an embedding provider is configured, `bind` uses hybrid FTS5+cosine search"

**Verification:**
```bash
cargo test  # no features — must still pass
cargo test --features vec
```

**Commit message:** `docs: update README and AGENTS.md for functional vec feature`

---

## Final Verification (Run Once After All Tasks)

```bash
# 1. Default build — no regression
cargo test

# 2. With vec feature — all tests including new ones
cargo test --features vec

# 3. Clippy + format
cargo clippy --features vec
cargo fmt --check
```

---

## Time Budget Summary

| Task | Est. Time |
|------|-----------|
| 1. Store-time embedding hook | 30–45 min |
| 2. Gateway caching | 20–30 min |
| 3. Update query-time call sites | 15–20 min |
| 4. Switch hook to cached gateway | 10–15 min |
| 5. Dynamic dimension handling | 45–60 min |
| 6. Test: semantic gets embedded | 30–45 min |
| 7. Test: hybrid paraphrase | 45–60 min |
| 8. Test: fallback paths | 30 min |
| 9. Documentation | 20–30 min |
| **Total** | **~5.5–7 hours** |

Each task is self-contained and can be paused/resumed independently.

---

## Commit Sequence

```
cb7f478 docs(spec): add semantic storage pipeline design for sqlite-vec  ← already done
├─ T1  feat(store): add maybe_store_embedding hook in MemoryStore::insert()
├─ T2  perf(embed): cache EmbeddingGateway with OnceLock
├─ T3  refactor(repl,mcp): use cached EmbeddingGateway in bind/recall paths
├─ T4  perf(store): use cached gateway in maybe_store_embedding
├─ T5  feat(store): auto-recreate memory_vectors on dimension mismatch
├─ T6  test(vec): add store-time embedding tests for semantic vs working
├─ T7  test(vec): add hybrid search paraphrase test
├─ T8  test(vec): add fallback tests for missing provider and sqlite-vec
└─ T9  docs: update README and AGENTS.md for functional vec feature
```

---

**Start with Task 1. Stop after any task. Resume later.**
