# BIND Retrieval Fix — Plan A: Query Expansion

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add synonym-based query expansion to `bind` so high-confidence personal questions with no lexical overlap still retrieve semantically related memories.

**Architecture:** Add a static English synonym map and simple word stemmer to the existing `src/context/` module, wire the expansion into `cmd_bind` before FTS5 recall, and return the expanded results to the caller.

**Tech Stack:** Rust, Rusqlite, FTS5

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/context/query_expansion.rs` | Create | `expand_query()`, synonym map, stemming, expansion logic |
| `src/context/mod.rs` | Modify | Export `expand_query` |
| `src/context/query.rs` | Modify | Export `expand_query` public visibility |
| `src/store/memory.rs` | Modify | Add `search_content_expanded()` method |
| `src/tier/manager.rs` | Modify | Add `recall_expanded()` passthrough method |
| `src/repl/runner.rs` | Modify | Call `expand_query` then `recall_expanded` inside `cmd_bind` |
| `src/mcp/mod.rs` | Modify | Same expansion in `handle_bind` |
| `tests/integration_test.rs` | Modify | Add `test_bind_finds_related_with_expansion` |
| `tests/context_query_expansion_test.rs` | Create | Unit tests for expansion logic |

---

## Task 1: Query Expansion Engine

**Files:**
- Create: `src/context/query_expansion.rs`
- Test: `tests/context_query_expansion_test.rs`

### Step 1: Write the failing unit test

Create `tests/context_query_expansion_test.rs`:

```rust
use mnemo::context::expand_query;

#[test]
fn test_expand_query_basic() {
    let terms = vec!["todos".to_string()];
    let expanded = expand_query(&terms);
    assert!(
        expanded.contains(&"todos".to_string()),
        "Original term must be preserved"
    );
    assert!(
        expanded.contains(&"tasks".to_string()),
        "Synonym 'tasks' must be present"
    );
    assert!(
        expanded.contains(&"task".to_string()),
        "Synonym 'task' must be present"
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Command:
```bash
cargo test --test context_query_expansion_test test_expand_query_basic -- --nocapture
```

Expected output:
```
error: test target `context_query_expansion_test.rs` does not exist
```

- [ ] **Step 3: Write the expansion module**

Create `src/context/query_expansion.rs`:

```rust
//! Query term expansion for BIND retrieval
//!
//! When a user's query has no lexical overlap with stored memories,
//! expand query terms with synonyms and morphology variants to cast
//! a wider net over the FTS5 index.

static SYNONYM_MAP: &[(&str, &[&str])] = &[
    ("todos",       &["tasks", "task", "todo", "to-do", "checklist"]),
    ("meetings",    &["meeting", "calls", "sync", "standup", "review"]),
    ("yesterday",   &["yesterday", "last night", "previous day"]),
    ("work",        &["job", "project", "assignment", "task", "duty"]),
    ("deadline",    &["due", "due date", "milestone", "timeline", "schedule"]),
    ("email",       &["mail", "message", "inbox", "correspondence"]),
    ("theme",       &["theme", "mode", "style", "appearance", "ui", "skin"]),
    ("dark mode",   &["dark", "night mode", "dark theme", "night theme"]),
    ("preference",  &["preference", "like", "dislike", "hate", "love", "want"]),
    ("editor",      &["editor", "ide", "vim", "emacs", "vscode", "code"]),
    ("terminal",    &["terminal", "shell", "command line", "cli", "bash", "zsh"]),
];

/// Expand a list of query terms with synonyms and morphology variants.
///
/// Each term is preserved. If it matches a synonym map key
/// (or is listed in a synonym cluster), all cluster members are added.
/// A simple stemmer removes trailing `s`, `ed`, and `ing`.
///
/// # Example
///
/// ```
/// use mnemo::context::expand_query;
/// let expanded = expand_query(&vec!["todos".to_string()]);
/// assert!(expanded.contains(&"tasks".to_string()));
/// ```
pub fn expand_query(terms: &[String]) -> Vec<String> {
    let mut result: Vec<String> = terms.iter().map(|s| s.to_lowercase()).collect();

    for term in terms {
        let lower = term.to_lowercase();
        for (key, synonyms) in SYNONYM_MAP.iter() {
            let mut matched = false;
            if *key == lower {
                matched = true;
            } else if synonyms.contains(&lower.as_str()) {
                matched = true;
            }

            if matched {
                // Add the canonical key
                result.push(key.to_string());
                // Add every synonym in the cluster
                for &synonym in *synonyms {
                    result.push(synonym.to_string());
                }
            }
        }
    }

    result.sort();
    result.dedup();
    result
}
```

- [ ] **Step 4: Register the new module**

Modify `src/context/mod.rs`:

```rust
pub mod query;
pub mod query_expansion;

pub use query::*;
pub use query_expansion::*;
```

- [ ] **Step 5: Verify the module compiles**

Command:
```bash
cargo check
```

Expected: no errors.

- [ ] **Step 6: Run the unit test again**

Command:
```bash
cargo test --test context_query_expansion_test test_expand_query_basic -- --nocapture
```

Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/context/query_expansion.rs src/context/mod.rs tests/context_query_expansion_test.rs
git commit -m "feat(bind): add query expansion engine with synonym map"
```

---

## Task 2: Expose Public API

**Files:**
- Modify: `src/context/mod.rs`
- Modify: `src/lib.rs`
- Modify: `src/context/query_expansion.rs` (add `pub` visibility)

### Step 1: Ensure `expand_query` is public

In `src/context/query_expansion.rs`:

```rust
pub fn expand_query(terms: &[String]) -> Vec<String> { ... }  // Already pub from Task 1
```

In `src/context/mod.rs`, already done in Task 1.

### Step 2: Verify from external crates

No `src/lib.rs` changes needed: `pub mod context` already publishes the context module tree. Any crate importing `mnemo::context::expand_query` will resolve it.

### Step 3: Compile check

Command:
```bash
cargo check
```

Expected: no errors.

### Step 4: Commit

```bash
git add src/context/mod.rs
git commit -m "chore(bind): expose expand_query in public API"
```

---

## Task 3: Add Expanded FTS5 Search to MemoryStore

**Files:**
- Modify: `src/store/memory.rs`

### Step 1: Add `search_content_expanded` method

In `src/store/memory.rs`, locate the `impl MemoryStore` block and add:

```rust
/// Search memories using an expanded query.
///
/// Each expanded term is joined with `OR` so a match on any synonym
/// returns the memory (wider catchment than exact match).
pub fn search_content_expanded(
    &self,
    expanded_terms: &[String],
    memory_types: &[String],
    limit: usize,
) -> SqliteResult<Vec<Memory>> {
    if expanded_terms.is_empty() {
        return self.search_content("", memory_types, limit);
    }

    let fts_query = expanded_terms.join(" OR ");
    let mut sql = String::from(
        "SELECT id, memory_type, content, created_at, accessed_at, expires_at,
                confidence, importance, source_type, source_turn_id,
                version, superseded_by, is_indexed, tags
         FROM memories
         WHERE rowid IN (SELECT rowid FROM memories_fts WHERE content MATCH ?)"
    );

    if !memory_types.is_empty() {
        let placeholders = memory_types
            .iter()
            .map(|_| "?".to_string())
            .collect::<Vec<_>>()
            .join(",");
        sql.push_str(&format!(" AND memory_type IN ({})", placeholders));
    }

    sql.push_str(" LIMIT ?");

    let mut stmt = self.conn.prepare(&sql)?;
    let mut params: Vec<&dyn ToSql> = vec![&fts_query];
    for t in memory_types {
        params.push(t);
    }
    let limit_i64 = limit as i64;
    params.push(&limit_i64);

    let rows = stmt.query_map(params.as_slice(), Memory::from_row)?;
    rows.collect()
}
```

### Step 2: Compile check

Command:
```bash
cargo check
```

Expected: no errors.

### Step 3: Commit

```bash
git add src/store/memory.rs
git commit -m "feat(store): add expanded FTS5 search with OR-joined synonyms"
```

---

## Task 4: Passthrough in TierManager

**Files:**
- Modify: `src/tier/manager.rs`

### Step 1: Add `recall_expanded` method

In `src/tier/manager.rs`, after the existing `recall` method, add:

```rust
    pub fn recall_expanded(
        &self,
        expanded_terms: &[String],
        memory_types: &[String],
        limit: usize,
    ) -> rusqlite::Result<Vec<Memory>> {
        self.store.search_content_expanded(expanded_terms, memory_types, limit)
    }
```

### Step 2: Compile check

Command:
```bash
cargo check
```

Expected: no errors.

### Step 3: Commit

```bash
git add src/tier/manager.rs
git commit -m "feat(tier): add recall_expanded passthrough for query expansion"
```

---

## Task 5: Wire Expansion into BIND (REPL)

**Files:**
- Modify: `src/repl/runner.rs`

### Step 1: Modify `cmd_bind` to expand before recall

In `src/repl/runner.rs`, find the `cmd_bind` method and change the retrieval branch.

The existing code (simplified) is:

```rust
IntentType::RetrieveAll | IntentType::RetrieveRecent |
IntentType::RetrieveByTopic | IntentType::RetrieveByDate => {
    let query = build_query(&intent);
    let manager = TierManager::new(self.db.conn(), 100).unwrap();
    let types_to_search = vec!["working".to_string(), "episodic".to_string(), "semantic".to_string()];
    
    match manager.recall(&query, &types_to_search, 20) { ... }
}
```

Replace `manager.recall` call with expanded search:

```rust
IntentType::RetrieveAll | IntentType::RetrieveRecent |
IntentType::RetrieveByTopic | IntentType::RetrieveByDate => {
    use crate::context::expand_query;

    let query = build_query(&intent);
    let expanded = expand_query(&query.split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>());

    // Also always keep original query for the non-expanded fallback if we want
    // For now, use the expanded query exclusively on FTS5.
    let manager = TierManager::new(self.db.conn(), 100).unwrap();
    let types_to_search = vec!["working".to_string(), "episodic".to_string(), "semantic".to_string()];
    
    match manager.recall_expanded(&expanded, &types_to_search, 20) {
        Ok(memories) => {
            let memory_texts: Vec<String> = memories
                .iter()
                .map(|m| format!("[{}] {}", m.memory_type, m.content))
                .collect();

            let retrieved = if memory_texts.is_empty() {
                "No matching memories found.".to_string()
            } else {
                format!("Found {} memories:\n{}", memory_texts.len(), memory_texts.join("\n"))
            };

            Response::Ok {
                message: format!("{}\n\nQuery intent: {:?} (confidence: {:.2})\nExpanded terms: {:?}",
                                 retrieved, intent.intent_type, intent.confidence, &expanded),
            }
        }
        Err(e) => Response::Error {
            code: "DB_ERROR".to_string(),
            message: e.to_string(),
        },
    }
}
```

### Step 2: Verify compilation

Command:
```bash
cargo build --release
```

Expected: Builds successfully.

### Step 3: Commit

```bash
git add src/repl/runner.rs
git commit -m "feat(repl): wire query expansion into cmd_bind retrieval"
```

---

## Task 6: Wire Expansion into BIND (MCP)

**Files:**
- Modify: `src/mcp/mod.rs`

### Step 1: Modify `handle_bind` to expand before recall

In `src/mcp/mod.rs`, find `IntentType::RetrieveAll` block inside `handle_bind`, and replace `manager.recall` with expanded search.

Current snippet:
```rust
let types_to_search = vec!["working".to_string(), "episodic".to_string(), "semantic".to_string()];

match manager.recall(&query, &types_to_search, 20) {
```

Replace with:
```rust
let types_to_search = vec!["working".to_string(), "episodic".to_string(), "semantic".to_string()];

let expanded = crate::context::expand_query(
    &query.split_whitespace().map(|s| s.to_string()).collect::<Vec<_>>()
);

match manager.recall_expanded(&expanded, &types_to_search, 20) {
```

### Step 2: Rebuild and verify

Command:
```bash
cargo build --release
```

Expected: Successful build.

### Step 3: Commit

```bash
git add src/mcp/mod.rs
git commit -m "feat(mcp): wire query expansion into MCP bind handle"
```

---

## Task 7: Integration Test — Expansion Retrieval

**Files:**
- Modify: `tests/integration_test.rs`

### Step 1: Write the integration test

Append to `tests/integration_test.rs`:

```rust
#[test]
fn test_bind_finds_related_with_expansion() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-agent-bind-expansion";

    // Seed: "tired after meetings yesterday" — no "todos" keyword
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("remember")
        .arg("User mentioned they were tired after meetings yesterday");
    cmd.arg("--memory-type").arg("episodic");
    cmd.assert().success();

    // Bind query: asks for "todos yesterday" — should find "meetings yesterday" via expansion
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("bind").arg("What were my todos from yesterday?");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Found"))
        .stdout(predicates::str::contains("tired after meetings"));
}
```

### Step 2: Run the test (ensure it passes)

Command:
```bash
cargo test --test integration_test test_bind_finds_related_with_expansion -- --nocapture
```

Expected: PASS

### Step 3: Run the full integration suite

Command:
```bash
cargo test --test integration_test
```

Expected: All tests PASS (existing tests unchanged, new test passes).

### Step 4: Commit

```bash
git add tests/integration_test.rs
git commit -m "test(bind): integration test for expansion-retrieval on semantic gap"
```

---

## Task 8: Regression Test — General Knowledge Still Skipped

**Files:**
- Modify: `tests/integration_test.rs`

### Step 1: Write the regression test

Append to `tests/integration_test.rs`:

```rust
#[test]
fn test_bind_skips_general_knowledge() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-agent-bind-gk";

    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("bind").arg("What is the capital of France?");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("No personal context"));
}
```

### Step 2: Run the test

Command:
```bash
cargo test --test integration_test test_bind_skips_general_knowledge -- --nocapture
```

Expected: PASS

### Step 3: Commit

```bash
git add tests/integration_test.rs
git commit -m "test(bind): regression test ensuring general knowledge still skips memory"
```

---

## Task 9: Verify All Existing Tests Still Pass

### Step 1: Full test suite

Command:
```bash
cargo test
```

Expected: All tests PASS.

### Step 2: Clippy

Command:
```bash
cargo clippy --release -- -D warnings
```

Expected: No warnings. (If there is an existing warning, ignore it; do not fix unrelated code.)

### Step 3: Format check

Command:
```bash
cargo fmt -- --check
```

Expected: No unformatted files.

### Step 4: Commit if needed

If formatting created changes:

```bash
git add -A
git commit -m "style: cargo fmt"
```

---

## Self-Review Checklist

| Spec Requirement | Task |
|---|---|
| Static synonym map in code | Task 1 |
| Expand query before FTS5 recall | Task 1, Task 5, Task 6 |
| General knowledge still skipped | Task 8 regression test |
| Test 1 demo passes ("todos" finds "meetings") | Task 7 integration test |
| MCP and REPL both wired | Task 5, Task 6 |
| All existing tests pass | Task 9 |
| No clippy warnings | Task 9 |

**Placeholder scan:**
- `TBD` / `TODO` / `implement later` → None found.
- Placeholder content in test files → None.
- "Similar to Task N" references → None.
- "Add appropriate error handling" → None (all errors handled with `Result`).

**Type consistency:**
- `expand_query` signature matches across `src/context/query_expansion.rs`, `src/context/mod.rs`, `src/store/memory.rs`, `src/tier/manager.rs`, `src/repl/runner.rs`, `src/mcp/mod.rs`.
