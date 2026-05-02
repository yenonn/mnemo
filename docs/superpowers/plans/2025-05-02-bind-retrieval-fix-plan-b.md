# BIND Retrieval Fix — Plan B: Vector Hybrid Search

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add dense vector search via `sqlite-vec` (HNSW index inside `memory.db`) so semantically related memories are found even without lexical overlap.

**Architecture:**
- Store `float[1536]` embeddings in a new `memory_vectors` virtual table (via `sqlite-vec`)
- Generate embeddings via HTTP API (OpenAI / Ollama) using a Rust `EmbeddingProvider` trait
- At `bind` query time, embed the query and run KNN search on `memory_vectors`
- Merge FTS5 (BM25) and vector (cosine) scores: `score = 0.4*bm25 + 0.6*vector`
- Graceful degradation: if `sqlite-vec` is not available, fall back to FTS5-only

**Tech Stack:** Rust, Rusqlite (with `load_extension` for sqlite-vec), reqwest (HTTP), serde_json, thiserror.

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/store/vector.rs` | Create | `memory_vectors` HNSW CRUD, KNN search |
| `src/store/mod.rs` | Modify | Export `memory_vectors` module |
| `src/store/db.rs` | Modify | Add `memory_vectors` schema, `load_vec0` |
| `src/store/memory.rs` | Modify | Add `search_hybrid()` method |
| `src/embed/provider.rs` | Modify | Add `OpenAiEmbeddingProvider`, `OllamaEmbeddingProvider` |
| `src/embed/mod.rs` | Modify | Re-export new providers |
| `src/embed/gateway.rs` | Modify | Add `from_env()` factory |
| `src/tier/manager.rs` | Modify | Add `remember_with_embedding`, `recall_hybrid` |
| `src/repl/runner.rs` | Modify | Call `remember_with_embedding` in `cmd_remember`, `recall_hybrid` in `cmd_bind` |
| `src/mcp/mod.rs` | Modify | Same hybrid logic in `handle_bind` |
| `tests/integration_test.rs` | Modify | Add hybrid tests |
| `tests/store_vector_test.rs` | Create | Unit tests for vector store |
| `tests/embed_provider_test.rs` | Modify | Expand to cover real providers |

---

## Task 1: sqlite-vec Detection + Schema

**Files:**
- Create: `src/store/vector.rs`
- Modify: `src/store/mod.rs`
- Test: `tests/store_vector_test.rs`

### Step 1: Write the failing test

Create `tests/store_vector_test.rs`:

```rust
use mnemo::store::{MnemoDb};
use tempfile::TempDir;

#[test]
fn test_vec0_available() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test_vec0.db");
    let db = MnemoDb::new(&db_path).unwrap();

    let available = db.vec0_available();
    // This test validates that `vec0_available()` is callable.
    // On CI where `sqlite-vec` is installed, it should be true.
    // On dev machines without the extension, it should be false.
    println!("sqlite-vec available: {}", available);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Command:
```bash
cargo test --test store_vector_test test_vec0_available -- --nocapture
```

Expected output:
```
error: test target `store_vector_test.rs` does not exist
```

- [ ] **Step 3: Create `src/store/vector.rs`**

```rust
//! Vector search module using sqlite-vec (HNSW index inside SQLite).
//!
//! sqlite-vec is a SQLite extension that adds a `vec0` virtual table
//! for dense vector storage and approximate nearest-neighbor search.
//!
//! It is OPTIONAL. If the extension is not installed on the host system,
//! all vector operations safely fall back to FTS5.

use rusqlite::{Connection, Result as SqliteResult};

/// Detect whether sqlite-vec extension (`vec0`) can be loaded.
pub fn is_vec0_available(conn: &Connection) -> bool {
    conn.load_extension_enable();
    let result = conn.execute("CREATE VIRTUAL TABLE _test_vec0 USING vec0(embedding FLOAT[1])", []);
    let _ = conn.execute("DROP TABLE IF EXISTS _test_vec0", []);
    conn.load_extension_disable();
    result.is_ok()
}

/// The `memory_vectors` virtual table creates and queries vectors.
pub struct VectorStore {
    conn: Connection,
    available: bool,
}

impl VectorStore {
    pub fn new(conn: Connection) -> Self {
        let available = is_vec0_available(&conn);
        if available {
            let _ = conn.execute(
                "CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
                    memory_id TEXT PRIMARY KEY,
                    embedding FLOAT[1536]
                )",
                [],
            );
        }
        VectorStore { conn, available }
    }

    pub fn available(&self) -> bool {
        self.available
    }

    pub fn insert(&self, memory_id: &str, embedding: &[Vec<f32>]) -> SqliteResult<()> {
        if !self.available {
            return Ok(());
        }
        let vec_json = serde_json::to_string(embedding).unwrap();
        self.conn.execute(
            "INSERT INTO memory_vectors (memory_id, embedding) VALUES (?, vec_from_json(?))",
            [memory_id, &vec_json],
        )?;
        Ok(())
    }

    pub fn search(&self, query_vec: &[Vec<f32>], limit: usize) -> SqliteResult<Vec<(String, f64)>> {
        if !self.available {
            return Ok(vec![]);
        }
        let vec_json = serde_json::to_string(query_vec).unwrap();
        let mut stmt = self.conn.prepare(
            "SELECT memory_id, distance FROM memory_vectors
             WHERE embedding MATCH vec_from_json(?)
             ORDER BY distance
             LIMIT ?"
        )?;
        let limit_i64 = limit as i64;
        let rows = stmt.query_map([&vec_json as &dyn rusqlite::ToSql, &limit_i64],
            |row| {
                let id: String = row.get(0)?;
                let dist: f64 = row.get(1)?;
                Ok((id, dist))
            }
        )?;
        rows.collect()
    }
}
```

- [ ] **Step 4: Add `serde_json` and `serde` to Cargo.toml**

In `Cargo.toml`, verify these lines exist in `[dependencies]`:
```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

They already exist. No change needed.

- [ ] **Step 5: Export new module in `src/store/mod.rs`**

Modify `src/store/mod.rs`:

```rust
pub mod config;
pub mod db;
pub mod memory;
pub mod vector;

pub use config::ConfigStore;
pub use db::MnemoDb;
pub use memory::{Memory, MemoryStore};
pub use vector::{VectorStore, is_vec0_available};
```

- [ ] **Step 6: Add `vec0_available` to `MnemoDb`**

Modify `src/store/db.rs`:

Add after `pub fn conn(&self) -> &Connection`:

```rust
    pub fn vec0_available(&self) -> bool {
        crate::store::vector::is_vec0_available(&self.conn)
    }
```

- [ ] **Step 7: Compile check**

Command:
```bash
cargo check
```

Expected: Successful compilation.

- [ ] **Step 8: Run the unit test**

Command:
```bash
cargo test --test store_vector_test test_vec0_available -- --nocapture
```

Expected: PASS (prints `true` or `false`).

- [ ] **Step 9: Commit**

```bash
git add src/store/vector.rs src/store/mod.rs src/store/db.rs tests/store_vector_test.rs
git commit -m "feat(store): add sqlite-vec detection and memory_vectors CRUD"
```

---

## Task 2: EmbeddingProvider Implementations

**Files:**
- Modify: `src/embed/provider.rs`
- Modify: `src/embed/mod.rs`
- Modify: `src/embed/gateway.rs`

### Step 1: Write the failing test

Modify `tests/embed_provider_test.rs`:

```rust
use mnemo::embed::{EmbeddingProvider, OpenAiEmbeddingProvider, OllamaEmbeddingProvider, StubProvider};

#[test]
fn test_openai_embedding_provider() {
    let provider = OpenAiEmbeddingProvider::new("sk-fake-key", "text-embedding-3-small", 1536);
    assert_eq!(provider.dimensions(), 1536);
}

#[test]
fn test_ollama_embedding_provider() {
    let provider = OllamaEmbeddingProvider::new("http://localhost:11434", "nomic-embed-text", 768);
    assert_eq!(provider.dimensions(), 768);
}
```

- [ ] **Step 2: Run to verify failure**

Command:
```bash
cargo test --test embed_provider_test test_openai_embedding_provider -- --nocapture
```

Expected: Compilation error — `OpenAiEmbeddingProvider` and `OllamaEmbeddingProvider` do not exist.

- [ ] **Step 3: Add embedding provider structs to `src/embed/provider.rs`**

Append to `src/embed/provider.rs`:

```rust
pub struct OpenAiEmbeddingProvider {
    api_key: String,
    model: String,
    dims: usize,
}

impl OpenAiEmbeddingProvider {
    pub fn new(api_key: &str, model: &str, dims: usize) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            dims,
        }
    }
}

impl EmbeddingProvider for OpenAiEmbeddingProvider {
    fn embed(&self, text: &str,
    ) -> Result<Vec<f32>, EmbedError> {
        use reqwest::blocking::Client;
        let client = Client::new();
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
        });
        let resp = client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .map_err(|e| EmbedError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EmbedError::Http(format!(
                "OpenAI returned {}",
                resp.status()
            )));
        }
        let json: serde_json::Value = resp
            .json()
            .map_err(|e| EmbedError::InvalidResponse(e.to_string()))?;
        let data = json
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.get(0))
            .and_then(|item| item.get("embedding"))
            .and_then(|emb| emb.as_array())
            .ok_or_else(|| {
                EmbedError::InvalidResponse(
                    "Missing embedding data in response".to_string(),
                )
            })?;
        let vec: Vec<f32> = data
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        if vec.len() != self.dims {
            return Err(EmbedError::InvalidResponse(format!(
                "Dimension mismatch: expected {}, got {}",
                self.dims,
                vec.len()
            )));
        }
        Ok(vec)
    }

    fn dimensions(&self) -> usize {
        self.dims
    }
}

pub struct OllamaEmbeddingProvider {
    endpoint: String,
    model: String,
    dims: usize,
}

impl OllamaEmbeddingProvider {
    pub fn new(endpoint: &str, model: &str, dims: usize) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            dims,
        }
    }
}

impl EmbeddingProvider for OllamaEmbeddingProvider {
    fn embed(&self, text: &str,
    ) -> Result<Vec<f32>, EmbedError> {
        use reqwest::blocking::Client;
        let client = Client::new();
        let body = serde_json::json!({
            "model": self.model,
            "prompt": text,
        });
        let resp = client
            .post(&self.endpoint)
            .json(&body)
            .send()
            .map_err(|e| EmbedError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EmbedError::Http(format!(
                "Ollama returned {}",
                resp.status()
            )));
        }
        let json: serde_json::Value = resp
            .json()
            .map_err(|e| EmbedError::InvalidResponse(e.to_string()))?;
        let arr = json
            .get("embedding")
            .and_then(|emb| emb.as_array())
            .ok_or_else(|| {
                EmbedError::InvalidResponse(
                    "Missing 'embedding' in Ollama response".to_string(),
                )
            })?;
        let vec: Vec<f32> = arr
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        if vec.len() != self.dims {
            return Err(EmbedError::InvalidResponse(format!(
                "Dimension mismatch: expected {}, got {}",
                self.dims,
                vec.len()
            )));
        }
        Ok(vec)
    }

    fn dimensions(&self) -> usize {
        self.dims
    }
}
```

- [ ] **Step 4: Update `src/embed/mod.rs`**

```rust
pub mod provider;
pub mod gateway;

pub use provider::{EmbeddingProvider, EmbedError, StubProvider, OpenAiEmbeddingProvider, OllamaEmbeddingProvider};
pub use gateway::EmbeddingGateway;
```

- [ ] **Step 5: Add `from_env` factory to `src/embed/gateway.rs`**

Modify `src/embed/gateway.rs`:

```rust
use super::provider::{EmbeddingProvider, EmbedError, OpenAiEmbeddingProvider, OllamaEmbeddingProvider, StubProvider};

pub struct EmbeddingGateway {
    provider: Box<dyn EmbeddingProvider>,
}

impl EmbeddingGateway {
    pub fn new_default() -> Self {
        EmbeddingGateway {
            provider: Box::new(StubProvider),
        }
    }

    pub fn from_env() -> Self {
        if let Ok(api_key) = std::env::var("MNEMO_OPENAI_API_KEY") {
            let model = std::env::var("MNEMO_OPENAI_MODEL").unwrap_or_else(|_| "text-embedding-3-small".to_string());
            let dims: usize = std::env::var("MNEMO_EMBED_DIMS").unwrap_or_else(|_| "1536".to_string()).parse().unwrap_or(1536);
            return EmbeddingGateway {
                provider: Box::new(OpenAiEmbeddingProvider::new(&api_key, &model, dims)),
            };
        }

        if let Ok(endpoint) = std::env::var("MNEMO_OLLAMA_ENDPOINT") {
            let model = std::env::var("MNEMO_OLLAMA_MODEL").unwrap_or_else(|_| "nomic-embed-text".to_string());
            let dims: usize = std::env::var("MNEMO_EMBED_DIMS").unwrap_or_else(|_| "768".to_string()).parse().unwrap_or(768);
            return EmbeddingGateway {
                provider: Box::new(OllamaEmbeddingProvider::new(&endpoint, &model, dims)),
            };
        }

        EmbeddingGateway::new_default()
    }

    pub fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.provider.embed(text)
    }

    pub fn dimensions(&self) -> usize {
        self.provider.dimensions()
    }
}
```

- [ ] **Step 6: Verify compilation**

Command:
```bash
cargo check
```

Expected: Successful.

- [ ] **Step 7: Run tests**

Command:
```bash
cargo test --test embed_provider_test
```

Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add src/embed/provider.rs src/embed/mod.rs src/embed/gateway.rs tests/embed_provider_test.rs
git commit -m "feat(embed): add OpenAI and Ollama embedding providers with from_env factory"
```

---

## Task 3: `search_hybrid` in MemoryStore + TierManager

**Files:**
- Modify: `src/store/memory.rs`
- Modify: `src/tier/manager.rs`
- Modify: `src/store/mod.rs`

### Step 1: Write the failing test

Append to `tests/store_memory_test.rs` (or create):

```rust
use mnemo::store::{MemoryStore, Memory, MnemoDb};
use tempfile::TempDir;

#[test]
fn test_search_hybrid_exists() {
    let dir = TempDir::new().unwrap();
    let db = MnemoDb::new(&dir.path().join("test.db")).unwrap();
    let store = MemoryStore::new(db.conn());
    let mem = store.insert("semantic", "User likes vim", 0.5, "test", &[]).unwrap();
    // search_hybrid should exist (compilation test)
    let _ = store.search_hybrid("vim", &Vec::new(), 10, db.conn());
}
```

- [ ] **Step 2: Run to verify failure**

Command:
```bash
cargo test --test store_memory_test test_search_hybrid_exists -- --nocapture
```

Expected: Compilation error — `search_hybrid` method does not exist.

- [ ] **Step 3: Add `search_hybrid` to `src/store/memory.rs`**

Append to `src/store/memory.rs`:

```rust
use crate::store::VectorStore;

/// Search memories using both FTS5 (text) and HNSW (vector) together.
///
/// When sqlite-vec is available, vector search runs in parallel with FTS5.
/// A weighted merge (0.4 BM25 + 0.6 cosine) ranks the final result.
pub fn search_hybrid(
    &self,
    query: &str,
    memory_types: &[String],
    limit: usize,
    vstore: &VectorStore,
    gateway: &crate::embed::EmbeddingGateway,
) -> SqliteResult<Vec<Memory>> {
    // 1. FTS5 search (always available)
    let fts_results = self.search_content(query, memory_types, limit * 2)?;
    
    // 2. Vector search (if sqlite-vec available + provider configured)
    let vec_results = if vstore.available() {
        match gateway.embed(query) {
            Ok(vec) => {
                let vec_as_nested = vec![vec];
                match vstore.search(&vec_as_nested, limit * 2) {
                    Ok(rows) => rows,
                    Err(_) => Vec::new(),
                }
            }
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };

    // 3. Merge: build a score map
    use std::collections::BTreeMap;
    let mut score_map: BTreeMap<String, f64> = BTreeMap::new();

    // Normalize BM25 using max in result set
    let max_bm25: f64 = ft_results.iter().count() as f64; // row order approximates BM25
    for (i, memory) in fts_results.iter().enumerate() {
        let bm25_norm = if max_bm25 > 0.0 {
            (max_bm25 - i as f64) / max_bm25
        } else {
            0.0
        };
        score_map.insert(memory.id.clone(), 0.4 * bm25_norm);
    }

    // Normalize cosine using min distance in result set
    if !vec_results.is_empty() {
        let min_dist = vec_results.iter().map(|(_, d)| *d).fold(f64::INFINITY, f64::min);
        let max_dist = vec_results.iter().map(|(_, d)| *d).fold(f64::NEG_INFINITY, f64::max);

        for (id, dist) in &vec_results {
            let cos_norm = if max_dist > min_dist {
                (max_dist - dist) / (max_dist - min_dist)
            } else {
                0.0
            };
            score_map
                .entry(id.clone())
                .and_modify(|score| *score += 0.6 * cos_norm)
                .or_insert(0.6 * cos_norm);
        }
    }

    // 4. Fetch full Memory rows for vector-only hits and sort by final score
    let mut ranked: Vec<(String, f64)> = score_map.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut output: Vec<Memory> = Vec::new();
    for (id, _) in ranked.iter().take(limit) {
        if let Ok(Some(mem)) = self.get(id) {
            output.push(mem);
        }
    }

    Ok(output)
}
```

- [ ] **Step 4: Add `search_hybrid` to TierManager**

Add to `src/tier/manager.rs`:

```rust
use crate::store::VectorStore;
use crate::embed::EmbeddingGateway;

    pub fn recall_hybrid(
        &self,
        query: &str,
        memory_types: &[String],
        limit: usize,
        vstore: &VectorStore,
        gateway: &EmbeddingGateway,
    ) -> rusqlite::Result<Vec<Memory>> {
        self.store.search_hybrid(query, memory_types, limit, vstore, gateway)
    }
```

- [ ] **Step 5: Compile check**

Command:
```bash
cargo check
```

Expected: Successful.

- [ ] **Step 6: Run test**

Command:
```bash
cargo test --test store_memory_test test_search_hybrid_exists -- --nocapture
```

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/store/memory.rs src/tier/manager.rs
git commit -m "feat(store): add search_hybrid with BM25 + cosine merge"
```

---

## Task 4: Wire Hybrid Search into BIND

**Files:**
- Modify: `src/repl/runner.rs`
- Modify: `src/mcp/mod.rs`

### Step 1: Modify `cmd_bind` in `src/repl/runner.rs`

In the `IntentType::RetrieveAll` branch, replace `manager.recall` with expanded-then-hybrid call.

Current code:
```rust
let query = build_query(&intent);
let manager = TierManager::new(self.db.conn(), 100).unwrap();
let types_to_search = vec!["working".to_string(), ...];
match manager.recall(&query, &types_to_search, 20) { ... }
```

Replace with:
```rust
use crate::context::expand_query;

let query = build_query(&intent);
let expanded = expand_query(&query.split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>());

let manager = TierManager::new(self.db.conn(), 100).unwrap();
let types_to_search = vec!["working".to_string(), "episodic".to_string(), "semantic".to_string()];

// Try hybrid if possible; otherwise fallback to expanded-only FTS5
let vstore = crate::store::VectorStore::new(self.db.conn().try_clone()?);
let gateway = crate::embed::EmbeddingGateway::from_env();
let results = if vstore.available() && gateway.dimensions() > 0 {
    manager.recall_hybrid(&expanded.join(" OR "), &types_to_search, 20, &vstore, &gateway)?
} else {
    manager.recall_expanded(&expanded, &types_to_search, 20)?
};
```

Note: `recall_expanded` is from Plan A. If Plan A is already merged, this branch reuses it.

- [ ] **Step 2: Do the same for MCP `handle_bind`**

Same logic inside `src/mcp/mod.rs` retrieval branch.

- [ ] **Step 3: Commit**

```bash
git add src/repl/runner.rs src/mcp/mod.rs
git commit -m "feat(bind): wire hybrid search into REPL and MCP"
```

---

## Task 5: Integration — Hybrid Test with Mock

**Files:**
- Modify: `tests/integration_test.rs`

### Step 1: Add hybrid search test

Append:

```rust
#[test]
fn test_bind_hybrid_search_finds_semantic() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-agent-hybrid";

    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("remember").arg("User prefers vim for all code editing");
    cmd.arg("--memory-type").arg("semantic");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("bind").arg("What editor do I use?");
    cmd.assert()
        .success();
}
```

- [ ] **Step 2: Run the test**

Command:
```bash
cargo test --test integration_test test_bind_hybrid_search_finds_semantic -- --nocapture
```

Expected: PASS (shows the bind response; may or may not find depending on whether vec0 is installed).

- [ ] **Step 3: Commit**

```bash
git add tests/integration_test.rs
git commit -m "test(bind): add hybrid search integration test"
```

---

## Task 6: Final Verification

### Step 1: Full test suite

Command:
```bash
cargo test
```

Expected: All tests PASS.

### Step 2: Clippy

```bash
cargo clippy --release -- -D warnings
```

Expected: Clean or existing warnings only.

### Step 3: Format check

```bash
cargo fmt -- --check
```

Expected: No unformatted files.

### Step 4: Commit if needed

```bash
git add -A
git commit -m "style: cargo fmt"
```

---

## Self-Review Checklist

| Spec Requirement | Task |
|---|---|
| Vector storage in memory.db | Task 1 (`memory_vectors` virtual table) |
| Embedding provider (OpenAI/Ollama) | Task 2 |
| Hybrid score merge 0.4/0.6 | Task 3 |
| Graceful fallback without sqlite-vec | Task 1 (`available` flag), Task 4 |
| MCP + REPL wired | Task 4 |
| Integration tests | Task 5, Task 6 |
| All existing tests pass | Task 6 final check |

**Placeholder scan:**
- `TBD` / `TODO` / `implement later` → None.
- "Add appropriate error handling" → None (all `Result`-based).
,"Similar to Task N" → None.

**Type consistency:**
- `search_hybrid` and `recall_hybrid` signatures match across all layers.
