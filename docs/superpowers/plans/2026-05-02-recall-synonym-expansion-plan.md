# Expand ALL Recall Queries with Synonym Expansion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `recall`, MCP `recall`, and REPL `recall` commands use synonym + morphology expansion so they find memories even when exact phrase matching fails.

**Architecture:** Reuse the existing `expand_query()` + `recall_expanded()` pipeline from the `BIND` command. Raw user query strings are split into terms, expanded with the static `SYNONYM_MAP` and heuristic stemming, then joined with `OR` for FTS5 search. Two new synonym clusters (coverage, preference) augment the map.

**Tech Stack:** Rust, SQLite + FTS5, `rusqlite`. No schema changes.

---

## File Map

| File | Responsibility |
|------|----------------|
| `src/context/query_expansion.rs` | Static synonym map + morphology stemmer + `expand_query()` |
| `src/repl/runner.rs` | REPL command dispatch — `cmd_recall` currently calls `manager.recall()` |
| `src/mcp/mod.rs` | MCP JSON-RPC server — recall handler currently calls `manager.recall()` |
| `src/store/memory.rs` | `search_content()` + `search_content_expanded()` already implemented |
| `tests/recall_expansion.rs` | New integration test: store memory → recall with different phrasing → assert found |

---

## Task 1: Add Synonym Clusters

**Files:**
- Modify: `src/context/query_expansion.rs` (around line 46)

- [ ] **Step 1.1: Add "coverage" synonym cluster**

Insert into `SYNONYM_MAP` after the existing clusters:

```rust
("coverage", &["coverage", "test coverage", "code coverage"]),
```

- [ ] **Step 1.2: Add "preference" synonym cluster**

Insert after coverage:

```rust
("preference", &["preference", "preferences", "prefer", "prefers"]),
```

- [ ] **Step 1.3: Verify compilation**

Run: `cargo check`
Expected: PASS (no errors)

- [ ] **Step 1.4: Commit**

```bash
git add src/context/query_expansion.rs
git commit -m "feat(query-expansion): add coverage and preference synonym clusters"
```

---

## Task 2: Unit Tests — New Synonym Clusters

**Files:**
- Modify: `src/context/query_expansion.rs` (in existing `#[cfg(test)]` block)

- [ ] **Step 2.1: Write failing test for "coverage" expansion**

Add to the `tests` module at the bottom of `query_expansion.rs`:

```rust
#[test]
fn test_expand_query_coverage() {
    let expanded = expand_query(&vec!["coverage".to_string()]);
    assert!(expanded.contains(&"coverage".to_string()));
    assert!(expanded.contains(&"test coverage".to_string()));
    assert!(expanded.contains(&"code coverage".to_string()));
}
```

- [ ] **Step 2.2: Write failing test for "preference" expansion**

Add below the coverage test:

```rust
#[test]
fn test_expand_query_preference() {
    let expanded = expand_query(&vec!["preference".to_string()]);
    assert!(expanded.contains(&"preference".to_string()));
    assert!(expanded.contains(&"preferences".to_string()));
    assert!(expanded.contains(&"prefer".to_string()));
    assert!(expanded.contains(&"prefers".to_string()));
}
```

- [ ] **Step 2.3: Run tests to verify they fail**

Run: `cargo test test_expand_query_coverage test_expand_query_preference -- --nocapture`
Expected: These tests FAIL (not yet committed)

- [ ] **Step 2.4: Run tests to verify they pass**

Run: `cargo test test_expand_query_coverage test_expand_query_preference -- --nocapture`
Expected: PASS (the synonym clusters added in Task 1 make them pass)

- [ ] **Step 2.5: Commit**

```bash
git add src/context/query_expansion.rs
git commit -m "test(query-expansion): add unit tests for coverage and preference synonyms"
```

---

## Task 3: Wire Recall Expansion into REPL

**Files:**
- Modify: `src/repl/runner.rs` (lines 148–181)

- [ ] **Step 3.1: Understand current code**

In `cmd_recall`, the current code is:

```rust
fn cmd_recall(&self, query: String, memory_types: Vec<String>, limit: usize) -> Response {
    let manager = TierManager::new(self.db.conn(), 100).unwrap();
    let types_to_search = if memory_types.is_empty() {
        vec!["episodic".to_string(), "semantic".to_string()]
    } else {
        memory_types
    };

    match manager.recall(&query, &types_to_search, limit) {
        // ...
    }
}
```

- [ ] **Step 3.2: Edit cmd_recall to use recall_expanded**

Replace the `cmd_recall` body (keep the signature unchanged). Add import for `expand_query` if not already present (check line 346 where `cmd_bind` imports it):

```rust
fn cmd_recall(&self, query: String, memory_types: Vec<String>, limit: usize) -> Response {
    use crate::context::expand_query; // already imported at module level in this file

    let manager = TierManager::new(self.db.conn(), 100).unwrap();
    let types_to_search = if memory_types.is_empty() {
        vec!["episodic".to_string(), "semantic".to_string()]
    } else {
        memory_types
    };

    // Split query into terms and expand with synonyms + morphology
    let query_terms: Vec<String> = query.split_whitespace().map(|s| s.to_string()).collect();
    let expanded = expand_query(&query_terms);

    // Use recall_expanded which joins expanded terms with OR
    match manager.recall_expanded(&expanded, &types_to_search, limit) {
        // ... keep existing match arm exactly as-is ...
    }
}
```

**Important:** Only change the call from `manager.recall(&query, ...)` to `manager.recall_expanded(&expanded, ...)`. The match arm and response mapping stays identical.

- [ ] **Step 3.3: Verify compilation**

Run: `cargo check`
Expected: PASS

- [ ] **Step 3.4: Commit**

```bash
git add src/repl/runner.rs
git commit -m "feat(repl): expand recall queries with synonyms and morphology"
```

---

## Task 4: Wire Recall Expansion into MCP

**Files:**
- Modify: `src/mcp/mod.rs` (around line 320)

- [ ] **Step 4.1: Understand current code**

The MCP recall handler currently does:

```rust
match manager.recall(query, &types_to_search, limit) {
```

- [ ] **Step 4.2: Add expand_query import and expansion logic**

Before the `match manager.recall` line, add:

```rust
// Expand query with synonyms and morphology for broader recall
let query_terms: Vec<String> = query.split_whitespace().map(|s| s.to_string()).collect();
let expanded = crate::context::expand_query(&query_terms);
```

Then change the call to:

```rust
match manager.recall_expanded(&expanded, &types_to_search, limit) {
```

- [ ] **Step 4.3: Verify compilation**

Run: `cargo check --features mcp` (or just `cargo check` if mcp is built by default)
Expected: PASS

- [ ] **Step 4.4: Commit**

```bash
git add src/mcp/mod.rs
git commit -m "feat(mcp): expand recall queries with synonyms and morphology"
```

---

## Task 5: Integration Test — End-to-End Recall Expansion

**Files:**
- Create: `tests/recall_expansion.rs`

- [ ] **Step 5.1: Create integration test file**

```rust
// tests/recall_expansion.rs
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_recall_finds_memory_with_different_phrasing() {
    let tmp = TempDir::new().unwrap();
    let db_path = tmp.path().join("memory.db");
    let agent_id = tmp.path().file_name().unwrap().to_str().unwrap();

    // Use --agent-id pointing to temp dir so ~/.mnemo stays clean
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.arg("--agent-id")
        .arg(agent_id)
        .env("HOME", tmp.path().parent().unwrap());

    // Step 1: Remember a memory about code coverage
    let mut remember = Command::cargo_bin("mnemo").unwrap();
    remember.arg("--agent-id").arg(agent_id)
        .env("HOME", tmp.path().parent().unwrap())
        .arg("remember")
        .arg("User prefers code coverage of at least 80%")
        .arg("--type")
        .arg("semantic");
    remember.assert().success();

    // Step 2: Recall with different phrasing — "coverage preferences"
    let mut recall = Command::cargo_bin("mnemo").unwrap();
    recall.arg("--agent-id").arg(agent_id)
        .env("HOME", tmp.path().parent().unwrap())
        .arg("recall")
        .arg("coverage preferences")
        .arg("--limit")
        .arg("10");

    let output = recall.assert().success().get_output().clone();
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should find the stored memory
    assert!(
        stdout.contains("code coverage") || stdout.contains("80%"),
        "recall should find the stored memory, got: {}", stdout
    );
}
```

- [ ] **Step 5.2: Run test — expect failure**

Run: `cargo test test_recall_finds_memory_with_different_phrasing --test recall_expansion -- --nocapture`
Expected: FAIL (before the code changes)

- [ ] **Step 5.3: Run test — expect pass (Tasks 1–4 done)**

Run: `cargo test test_recall_finds_memory_with_different_phrasing --test recall_expansion -- --nocapture`
Expected: PASS

- [ ] **Step 5.4: Commit**

```bash
git add tests/recall_expansion.rs
git commit -m "test(integration): verify recall finds memories with synonym-expanded queries"
```

---

## Task 6: Full Test Suite + Clippy + Format

**Files:** none (just verification)

- [ ] **Step 6.1: Run full test suite**

Run: `cargo test`
Expected: All tests pass (including existing + new)

- [ ] **Step 6.2: Run Clippy**

Run: `cargo clippy`
Expected: Clean (no warnings, or only pre-existing ones)

- [ ] **Step 6.3: Run Formatter**

Run: `cargo fmt`
Expected: No uncommitted style changes

- [ ] **Step 6.4: Commit**

```bash
git add -A
git commit -m "chore: clippy + fmt pass; all recall expansion tests green"
```

---

## Spec Coverage Check

| Spec Requirement | Covered By |
|------------------|------------|
| Add coverage synonym cluster | Task 1 |
| Add preference synonym cluster | Task 1 |
| REPL `recall` uses expanded queries | Task 3 |
| MCP `recall` uses expanded queries | Task 4 |
| Unit tests for new synonyms | Task 2 |
| Integration test for end-to-end recall | Task 5 |
| No schema changes | Not needed (uses existing FTS5 + recall_expanded) |
| Keep existing limit/ranking | Preserved (same `limit` param passed through) |

## Placeholder Scan

No TBDs, TODOs, or vague instructions. Each task has exact file paths, line ranges, code blocks, and commands.

## Type Consistency Check

- `expand_query` returns `Vec<String>` — used correctly in Step 3.2 and Step 4.2
- `recall_expanded` takes `&[String]`, `&[String]`, `usize` — matches usage
- `query_terms` is `Vec<String>` — compatible with both `expand_query` and `recall_expanded`
