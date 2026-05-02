# BIND Retrieval Fix — Design Specification

> **Date:** 2025-05-02  
> **Project:** mnemo — Agent memory database  
> **Status:** Approved  
> **Scope:** Improve BIND so high-confidence personal questions always return relevant memories

---

## 1. Problem Statement

The `bind` command detects user intent correctly but fails to retrieve memories when the query words don't lexically overlap with stored memory text.

**Example:**
- Stored: `"User mentioned they were tired after meetings yesterday"`
- User asks: `"What were my todos from yesterday?"`
- BIND detects `RetrieveAll` with confidence `0.85` — correct
- FTS5 search for `"my todos yesterday"` returns **0 results** — incorrect
- Root cause: "todos" and "meetings" share no lexical overlap, so FTS5 can't bridge them

**Impact:** High-confidence personal questions return empty results, breaking the agent's ability to recall context.

**Goals:**
1. Personal recall queries with confidence > 0.5 MUST return results if a semantically related memory exists
2. General knowledge queries MUST still skip memory (confidence = 0.0)
3. No external vector database — all data stays in `memory.db`
4. If no embedding provider is available, system MUST gracefully degrade to expanded-FTS5

---

## 2. Solution Overview

Two independent subsystems, deployed together:

### Subsystem A: Query Expansion (Immediate, No Dependencies)
- Expand query terms with synonym maps and morphology before FTS5 search
- Cast a wider lexical net so related terms are found

### Subsystem B: Vector Hybrid Search (Design Doc Match, Optional Provider)
- Store dense vectors alongside memories (via `sqlite-vec` extension)
- At query time, search by both FTS5 (exact word match) AND HNSW (semantic similarity)
- Merge scores: `final = 0.4 * bm25_norm + 0.6 * vec_norm`
- Vectors stored in the same `memory.db` file — no external database

---

## 3. Architecture

```
User prompt: "What were my todos from yesterday?"

Intent Detection (analyze_intent)
├── Detect: RetrieveAll, confidence 0.85 ✅
└── Query terms: ["my", "todos", "yesterday"]

Query Expansion (expand_query)
├── "todos" → ["todos", "tasks", "task", "todo"]
├── "meetings" → ["meetings", "meeting", "calls", "sync"]
├── Stemming: remove trailing s/ed/ing
└── Expanded query: "todos tasks task todo yesterday"

Parallel Search (run_recall)
├── Path A: FTS5 on memories_fts
│   └── Query: memories_fts MATCH expanded_query
│   └── Returns BM25 scores per memory
└── Path B: Vector search on memory_vectors (if available)
    ├── Embed: "What were my todos from yesterday?" → float[1536]
    ├── KNN: SELECT * FROM memory_vectors WHERE k = 5
    └── Returns cosine similarity scores

Score Merge
├── If vector results exist:
│   └── final = 0.4 * bm25_norm + 0.6 * vec_norm
├── Else:
│   └── final = bm25
└── Rank descending, top N returned
```

---

## 4. Subsystem A: Query Expansion

### 4.1 Synonym Map (Static, In-Code)

Organized by domain. Each domain has keyword clusters.

```rust
static SYNONYM_MAP: &[(&str, &[&str])] = &[
    // Communication / scheduling
    ("todos",       &["tasks", "task", "todo", "to-do", "checklist"]),
    ("meetings",    &["meeting", "calls", "sync", "standup", "review"]),
    ("yesterday",   &["yesterday", "last night", "previous day"]),

    // Work context
    ("work",        &["job", "project", "assignment", "task", "duty"]),
    ("deadline",    &["due", "due date", "milestone", "timeline", "schedule"]),
    ("email",       &["mail", "message", "inbox", "correspondence"]),

    // Preferences
    ("theme",       &["theme", "mode", "style", "appearance", "ui", "skin"]),
    ("dark mode",   &["dark", "night mode", "dark theme", "night theme"]),
    ("preference",  &["preference", "like", "dislike", "hate", "love", "want"]),

    // Tools
    ("editor",      &["editor", "ide", "vim", "emacs", "vscode", "code"]),
    ("terminal",    &["terminal", "shell", "command line", "cli", "bash", "zsh"]),
];
```

### 4.2 Expansion Algorithm

```rust
pub fn expand_query(terms: &[String]) -> Vec<String> {
    let mut expanded: Vec<String> = Vec::new();
    for term in terms {
        expanded.push(term.clone());
        // 1. Find in synonym map
        for (key, synonyms) in SYNONYM_MAP.iter() {
            if key == &term.to_lowercase() || synonyms.contains(&term.as_str()) {
                expanded.push(key.to_string());
                for s in *synonyms {
                    expanded.push(s.to_string());
                }
            }
        }
        // 2. Morphology: strip trailing s/ed/ing
        if let Some(stemmed) = simple_stem(term) {
            expanded.push(stemmed);
        }
    }
    expanded.sort();
    expanded.dedup();
    expanded
}
```

### 4.3 FTS5 Query Construction

FTS5 supports `OR` natively: `MATCH 'todos OR tasks OR task OR todo OR yesterday'`.

```rust
fn build_fts_query(terms: &[String]) -> String {
    terms.join(" OR ")
}
```

**Behavior:** A single match on any expanded term returns the memory — much wider catchment than exact lexical match.

---

## 5. Subsystem B: Vector Hybrid Search

### 5.1 Schema Addition

```sql
-- In store/db.rs, add to SCHEMA_SQL:
CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[1536]
);
```

- `vec0` is the `sqlite-vec` extension — loads as a native SQLite extension
- Stays in the same `memory.db` file — no external database
- Extension is bundled via `rusqlite`'s `load_extension` feature

### 5.2 Memory Storage Pipeline (Two-Phase)

```
REMEMBER command
│
├─ Phase 1: Synchronous (2ms)
│  ├── INSERT INTO memories(...) → text persistence
│  ├── FTS5 trigger → text indexed immediately
│  └── is_indexed = 0, queue embedding
│
└─ Phase 2: Asynchronous (~50-500ms)
   ├── Call embedding provider (OpenAI / Ollama)
   ├── Get float[1536] vector
   ├── INSERT INTO memory_vectors(memory_id, embedding) VALUES(...)
   └── UPDATE memories SET is_indexed = 1
```

### 5.3 Query Time Pipeline

```
BIND: "What were my todos from yesterday?"
│
├── 1. Expand query terms
├── 2. Build FTS5 query string
├── 3. Query FTS5 → returns [(memory_id, bm25_score)]
│
├── 4. Embed the raw query text → float[1536] query_vec
├── 5. Query memory_vectors via HNSW:
│   └── SELECT memory_id, distance FROM memory_vectors
│       WHERE embedding MATCH vec0 ?
│       ORDER BY distance
│       LIMIT 20;
│   → returns [(memory_id, cosine_score)]
│
└── 6. Merge and rank:
    ├── Normalize BM25 and cosine to [0, 1]
    ├── final_score = 0.4 * bm25_norm + 0.6 * vec_norm
    └── Return top N by final_score
```

### 5.4 Score Normalization

```rust
fn normalize_bm25(raw: f64, max_score: f64) -> f64 {
    if max_score == 0.0 { return 0.0; }
    (raw / max_score).min(1.0)
}

fn normalize_cosine(raw: f64, min_dist: f64) -> f64 {
    // sqlite-vec returns distance (lower = closer)
    // cosine similarity = 1 - (distance / 2) for normalized vectors
    let cos = 1.0 - (raw / 2.0);
    cos.max(0.0).min(1.0)
}
```

### 5.5 Embedding Provider Abstraction

```rust
pub trait EmbeddingProvider: Send + Sync {
    /// Embed a single text. Returns float[dim] or error.
    fn embed_single(&self, text: &str) -> Result<Vec<f32>, String>;
}

pub struct OpenAiProvider {
    api_key: String,
    model: String,
    dims: usize,
}

pub struct OllamaProvider {
    endpoint: String,
    model: String,
    dims: usize,
}
```

**Initialization from environment:**
| Variable | Default | Provider |
|---|---|---|
| `MNEMO_OPENAI_API_KEY` | — | OpenAI |
| `MNEMO_OPENAI_MODEL` | `text-embedding-3-small` | OpenAI |
| `MNEMO_OLLAMA_ENDPOINT` | `http://localhost:11434/api/embeddings` | Ollama |
| `MNEMO_OLLAMA_MODEL` | `nomic-embed-text` | Ollama |
| `MNEMO_EMBED_DIMS` | `1536` | Both |

---

## 6. Error Handling

| Scenario | Behavior |
|---|---|
| No embedding provider configured | Skip vector search. Warn in logs: `"No embedding provider configured, falling back to FTS5"`. Return FTS5 results only. |
| Embedding provider timeout on INSERT | Store text, queue embedding for retry. Search falls back to FTS5. |
| Embedding provider timeout on QUERY | Skip vector search. Return FTS5 results only. Warn once per session. |
| No `memory_vectors` table (old DB) | Graceful fallback to FTS5. No schema migration required in v0.1.1. |
| FTS5 returns 0, vector returns some | Return vector results only. Final score = cosine_norm. |
| Both FTS5 and vector return 0 | Return `"No matching memories found"`. Confidence unchanged. |
| Vector DB returns corrupted vectors | Treat as no vector results. Log error, return FTS5 results. |

---

## 7. Testing Strategy

### 7.1 Unit Tests

| Test | File | Purpose |
|---|---|---|
| `expand_query_basic` | `context/query_test.rs` | "todos" contains "tasks" |
| `expand_query_morphology` | `context/query_test.rs` | "meetings" produces "meeting" |
| `expand_query_dedup` | `context/query_test.rs` | No duplicate terms in expansion |
| `synonym_map_coverage` | `context/query_test.rs` | All map keys are lowercase, no empty entries |
| `build_fts_query_or` | `store/memory_test.rs` | Output contains OR keywords |
| `normalize_bm25_zero` | `store/search_test.rs` | max_score=0 returns 0.0, no panic |
| `normalize_cosine_edge` | `store/search_test.rs` | distance=0 returns 1.0; distance=2 returns 0.0 |

### 7.2 Integration Tests

| Test | File | Purpose |
|---|---|---|
| `bind_finds_related_with_expansion` | `integration_test.rs` | Store "tired after meetings yesterday", query "my todos yesterday" → finds it |
| `bind_skips_general_knowledge` | `integration_test.rs` | "capital of France" → skip memory, no results |
| `bind_hybrid_search_finds_semantic` | `integration_test.rs` | Mock embedding provider, "vim" finds "editor preference" |
| `bind_fallback_without_provider` | `integration_test.rs` | No API key, only FTS5 runs, returns empty for semantic mismatch |
| `bind_ranking_prefers_vector` | `integration_test.rs` | Two memories, vector scores higher for unseen query |

### 7.3 Manual Verification Checklist

- [ ] `mnemo bind "What were my todos from yesterday?"` finds "tired after meetings" when expansion enabled
- [ ] `mnemo bind "What is capital of France?"` still skips memory
- [ ] `mnemo status` shows pending_embeddings count when provider disabled
- [ ] Embedding works with `MNEMO_OPENAI_API_KEY` set (if you have a key)
- [ ] Embedding falls back to heuristic when key not set

---

## 8. Changes to Existing Code

### 8.1 New Files

| Path | Purpose |
|---|---|
| `src/context/query_expansion.rs` | `expand_query()`, synonym map, stemming |
| `src/context/query_expansion_test.rs` | Unit tests for expansion |
| `src/store/vector.rs` | `memory_vectors` CRUD, HNSW search |
| `src/embed/provider.rs` | `EmbeddingProvider` trait + implementations |

### 8.2 Modified Files

| Path | Change |
|---|---|
| `src/store/db.rs` | Add `memory_vectors` virtual table to SCHEMA_SQL |
| `src/store/memory.rs` | Add `search_content_expanded()` method |
| `src/store/memory.rs` | Add `search_hybrid()` method |
| `src/context/query.rs` | Export `expand_query` |
| `src/repl/runner.rs` | `cmd_bind()`: call `expand_query` before FTS5; call `search_hybrid` if provider configured |
| `src/mcp/mod.rs` | `handle_bind()`: same expansion + hybrid logic |
| `src/protocol/response.rs` | `Status` response already tracks vector_indexed — wire it up |

---

## 9. Limitations & Future Work

- **Synonym map is static** — no dynamic learning. v0.2 may extract synonyms from LLM or corpus.
- **No re-embedding on update** — if memory content is updated, its vector becomes stale. v0.2 handles this via versioned vectors.
- **Hybrid weight is fixed** — 0.4/0.6 is hardcoded. v0.3 may make this configurable per agent or memory type.
- **Dimension is fixed at 1536** — matches OpenAI. If using Ollama models with different dims, user must set `MNEMO_EMBED_DIMS`.

---

## 10. Backward Compatibility

- Old databases without `memory_vectors` still work — `search_hybrid` checks table existence and falls back to FTS5
- No breaking changes to protocol, CLI, or MCP interfaces
- New behavior only activates when `bind` is called with high-confidence personal questions

---

## 11. Acceptance Criteria

- [ ] Test 1 from demo (`bind "What were my todos from yesterday?"` with stored "tired after meetings yesterday") returns the stored memory
- [ ] Test 3 from demo (`bind "What is capital of France?"`) still returns 0 results and skips memory
- [ ] All existing integration tests pass without modification
- [ ] New integration tests for expansion and hybrid search pass
- [ ] Compilation succeeds with `cargo build --release`
- [ ] No clippy warnings
- [ ] Architecture matches this spec
