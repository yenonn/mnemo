# Lifecycle Hooks Implementation Plan

> **REQUIRED SUB-SKILL:** Use `superpowers:subagent-driven-development` OR `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the lifecycle hooks engine (auto-consolidate, decay, recall) for mnemo v0.3 with TDD, frequent commits, and full integration test coverage.

**Architecture:** Inline lifecycle engine on every command; mode-agnostic because working memory is DB-backed; configuration via `_mnemo_meta`; hooks reported via `HookResult`.

**Tech Stack:** Rust, rusqlite, chrono, tokio (for MCP tests)

---

## File Structure

| File | Action | Responsibility |
|------|--------|--------------|
| `src/lifecycle/mod.rs` | Modify | Module exports |
| `src/lifecycle/engine.rs` | Create | `LifecycleEngine` and `check_and_fire()` |
| `src/lifecycle/hook.rs` | Create | `HookResult` enum |
| `src/lifecycle/decay.rs` | Create | Decay algorithm |
| `src/lifecycle/recall.rs` | Create | Auto-recall query builder |
| `src/lifecycle/config.rs` | Create | Lifecycle config helpers |
| `src/repl/runner.rs` | Modify | Integrate `LifecycleEngine::check_and_fire()` into `execute()` |
| `src/mcp/mod.rs` | Modify | Integrate lifecycle into every handler; append results to response |
| `src/tier/manager.rs` | Modify | Ensure `consolidate_working_to_episodic()` deletes DB rows |
| `src/lib.rs` | Modify | Add `pub mod context` if missing (already present) |
| `src/protocol/response.rs` | Modify | Add `Lifecycle` variant |
| `src/protocol/commands.rs` | Modify | Add `Pragma` defaults for lifecycle keys |
| `tests/lifecycle_test.rs` | Create | Integration tests for lifecycle engine |
| `README.md` | Modify | Document lifecycle hooks and config |

---

### Task 1: Add lifecycle config keys to store schema

**Files:**
- Modify: `src/store/db.rs`
- Test: `tests/store_config_test.rs`

- [ ] **Step 1: Add `_mnemo_meta` defaults migration** to `db.rs` or `lifecycle/config.rs`

Ensure `lifecycle_enabled`, `lifecycle_idle_threshold`, `lifecycle_decay_rate`, `lifecycle_consolidate_on_flush` are seeded with defaults if not present.

- [ ] **Step 2: Write test** `test_lifecycle_default_config()` in `tests/lifecycle_test.rs`

```rust
// Should assert that a new MnemoDb has lifecycle config keys with correct defaults
```

- [ ] **Step 3: Implement config seeding** in `lifecycle/config.rs`

Helper functions:
```rust
pub fn get_lifecycle_config_bool(conn: &Connection, key: &str, default: bool) -> rusqlite::Result<bool>
pub fn get_lifecycle_config_i64(conn: &Connection, key: &str, default: i64) -> rusqlite::Result<i64>
pub fn get_lifecycle_config_f64(conn: &Connection, key: &str, default: f64) -> rusqlite::Result<f64>
pub fn lifecycle_config_seed(conn: &Connection) -> rusqlite::Result<()>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test lifecycle_test::test_lifecycle_default_config`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lifecycle/config.rs
# also modify src/store/db.rs if we add seed call there
git commit -m "feat(lifecycle): add lifecycle config keys to _mnemo_meta with defaults"
```

---

### Task 2: Implement HookResult enum

**Files:**
- Create: `src/lifecycle/hook.rs`
- Modify: `src/lifecycle/mod.rs`
- Test: `tests/lifecycle_test.rs`

- [ ] **Step 1: Write failing test** `test_hook_result_display()`

```rust
use mnemo::lifecycle::HookResult;

#[test]
fn test_hook_result_display() {
    let session_end = HookResult::SessionEnd {
        consolidated_count: 5,
        new_episodic_id: Some("mem-abc123".to_string()),
    };
    assert!(format!("{}", session_end).contains("session-end"));
}
```

- [ ] **Step 2: Implement `HookResult`** in `src/lifecycle/hook.rs`

```rust
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum HookResult {
    SessionEnd { consolidated_count: usize, new_episodic_id: Option<String> },
    SessionStart { recalled_count: usize },
    Overflow { consolidated_count: usize },
    Decay { affected_count: usize },
    None,
}

impl fmt::Display for HookResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HookResult::SessionEnd { consolidated_count, new_episodic_id } => {
                write!(f, "[session-end] Consolidated {} memories", consolidated_count)?;
                if let Some(id) = new_episodic_id {
                    write!(f, " to {}", id)?;
                }
                Ok(())
            }
            HookResult::SessionStart { recalled_count } => {
                write!(f, "[session-start] Recalled {} memories", recalled_count)
            }
            HookResult::Overflow { consolidated_count } => {
                write!(f, "[overflow] Consolidated {} memories (buffer full)", consolidated_count)
            }
            HookResult::Decay { affected_count } => {
                write!(f, "[decay] {} memories decayed", affected_count)
            }
            HookResult::None => write!(f, "[lifecycle] No action"),
        }
    }
}
```

- [ ] **Step 3: Re-export from `mod.rs`**

```rust
pub mod engine;
pub mod hook;
pub mod decay;
pub mod recall;
pub mod config;
```

- [ ] **Step 4: Run test to verify it passes**

Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lifecycle/hook.rs src/lifecycle/mod.rs tests/lifecycle_test.rs
git commit -m "feat(lifecycle): add HookResult enum for lifecycle reporting"
```

---

### Task 3: Implement Decay Algorithm

**Files:**
- Create: `src/lifecycle/decay.rs`
- Modify: `src/lifecycle/mod.rs`
- Test: `tests/lifecycle_test.rs`

- [ ] **Step 1: Write failing test** `test_decay_reduces_confidence()`

```rust
fn test_decay_reduces_confidence() {
    // Insert episodic memory with confidence 1.0, created 7 days ago
    // Run decay
    // Assert confidence is lower (e.g., ~0.93 for decay_rate=0.1, floor=0.1)
}
```

- [ ] **Step 2: Implement `decay_episodic()`** in `src/lifecycle/decay.rs`

```rust
use chrono::Utc;
use rusqlite::Connection;

pub fn decay_episodic(conn: &Connection, decay_rate: f64) -> rusqlite::Result<usize> {
    let now = Utc::now().timestamp_millis();
    let five_min_ago = now - (5 * 60 * 1000);
    
    let mut stmt = conn.prepare(
        "UPDATE memories
         SET confidence = MAX(0.1, confidence * POWER(?1, (CAST(?2 AS REAL) - created_at) / 86400000))
         WHERE memory_type = 'episodic'
         AND created_at < ?3
         AND confidence > 0.1
    ")?;
    
    let affected = stmt.execute(rusqlite::params![1.0 - decay_rate, now, five_min_ago])?;
    Ok(affected)
}
```

- [ ] **Step 3: Run test to verify it passes**

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/lifecycle/decay.rs src/lifecycle/mod.rs tests/lifecycle_test.rs
git commit -m "feat(lifecycle): implement confidence decay algorithm"
```

---

### Task 4: Implement Auto-Recall Query Builder

**Files:**
- Create: `src/lifecycle/recall.rs`
- Modify: `src/lifecycle/mod.rs`
- Test: `tests/lifecycle_test.rs`

- [ ] **Step 1: Write failing test** `test_auto_recall_inserts_working()`

```rust
fn test_auto_recall_inserts_working() {
    // Insert 2 semantic and 3 episodic memories
    // Call auto_recall
    // Assert 5 new working memories exist with prefix [context-recall]
}
```

- [ ] **Step 2: Implement `auto_recall()`** in `src/lifecycle/recall.rs`

```rust
use chrono::Utc;
use rusqlite::Connection;
use nanoid::nanoid;

pub fn auto_recall(conn: &Connection) -> rusqlite::Result<usize> {
    // Query top 5 episodic by created_at, confidence > 0.3
    // Query top 5 semantic by importance desc
    // Insert as working memories with [context-recall] prefix
    let now = Utc::now().timestamp_millis();
    
    let mut count = 0;
    
    // Episodic
    let mut stmt = conn.prepare(
        "SELECT id, content FROM memories
         WHERE memory_type = 'episodic' AND confidence > 0.3
         ORDER BY created_at DESC LIMIT 5"
    )?;
    
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let content: String = row.get(1)?;
        Ok((id, content))
    })?;
    
    for (id, content) in rows {
        let new_id = format!("mem-recall-{}", nanoid!(8));
        conn.execute(
            "INSERT INTO memories (id, memory_type, content, created_at, importance, source_type)
             VALUES (?1, 'working', ?2, ?3, 0.5, 'lifecycle_recall')",
            rusqlite::params![new_id, format!("[context-recall] {}", content), now],
        )?;
        count += 1;
    }
    
    // Semantic — similar query but importance sort
    let mut stmt = conn.prepare(
        "SELECT id, content FROM memories
         WHERE memory_type = 'semantic'
         ORDER BY importance DESC LIMIT 5"
    )?;
    
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let content: String = row.get(1)?;
        Ok((id, content))
    })?;
    
    for (id, content) in rows {
        let new_id = format!("mem-recall-{}", nanoid!(8));
        conn.execute(
            "INSERT INTO memories (id, memory_type, content, created_at, importance, source_type)
             VALUES (?1, 'working', ?2, ?3, 0.7, 'lifecycle_recall')",
            rusqlite::params![new_id, format!("[context-recall] {}", content), now],
        )?;
        count += 1;
    }
    
    Ok(count)
}
```

- [ ] **Step 3: Run test to verify it passes**

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/lifecycle/recall.rs src/lifecycle/mod.rs tests/lifecycle_test.rs
git commit -m "feat(lifecycle): implement auto-recall for session start"
```

---

### Task 5: Implement LifecycleEngine Core

**Files:**
- Create: `src/lifecycle/engine.rs`
- Modify: `src/lifecycle/mod.rs`
- Test: `tests/lifecycle_test.rs`

- [ ] **Step 1: Write failing test** `test_lifecycle_engine_idles_and_consolidates()`

```rust
fn test_lifecycle_engine_idles_and_consolidates() {
    // Create TierManager with 100-capacity
    // Insert 3 working memories via remember_working()
    // Manually set lifecycle_last_activity to 2 minutes ago
    // Run check_and_fire()
    // Assert SESSION_END fired (consolidated 3)
    // Assert SESSION_START fired (recalled N)
    // Assert working buffer is empty
}
```

- [ ] **Step 2: Implement `LifecycleEngine::check_and_fire()`** in `src/lifecycle/engine.rs`

```rust
use chrono::Utc;
use rusqlite::Connection;
use crate::tier::TierManager;
use crate::lifecycle::{HookResult, decay, recall, config};

pub struct LifecycleEngine;

impl LifecycleEngine {
    pub fn check_and_fire(conn: &Connection, manager: &mut TierManager) -> Vec<HookResult> {
        let mut results = Vec::new();
        
        // Guard: disabled
        let enabled = config::get_lifecycle_config_bool(conn, "lifecycle_enabled", true)
            .unwrap_or(true);
        if !enabled {
            return results;
        }
        
        // Read thresholds
        let idle_threshold: i64 = config::get_lifecycle_config_i64(conn, "lifecycle_idle_threshold", 60)
            .unwrap_or(60);
        let decay_rate: f64 = config::get_lifecycle_config_f64(conn, "lifecycle_decay_rate", 0.1)
            .unwrap_or(0.1);
        let consolidate_on_flush = config::get_lifecycle_config_bool(conn, "lifecycle_consolidate_on_flush", true)
            .unwrap_or(true);
        
        let now = Utc::now().timestamp_millis();
        let last_activity = config::get_lifecycle_config_i64(conn, "lifecycle_last_activity", 0)
            .unwrap_or(0);
        let idle_seconds = (now - last_activity) / 1000;
        
        // 1. Overflow check
        // Count working rows in DB
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM memories WHERE memory_type = 'working'").unwrap();
        let working_count: i64 = stmt.query_row([], |row| row.get(0)).unwrap_or(0);
        let capacity = 100; // TODO: pass from manager or config
        let overflow_threshold = ((capacity as f64) * 0.8) as i64;
        
        if working_count >= overflow_threshold {
            let consolidated = manager.working_count(); // in-memory count before drain
            manager.consolidate_working_to_episodic(); // this deletes DB rows
            results.push(HookResult::Overflow {
                consolidated_count: consolidated,
            });
        }
        
        // 2. Idle session check
        if idle_seconds >= idle_threshold {
            let mut consolidated_count = 0;
            let mut new_episodic_id: Option<String> = None;
            
            // Consolidate working → episodic
            if consolidate_on_flush {
                consolidated_count = manager.working_count();
                new_episodic_id = manager.consolidate_working_to_episodic().ok().flatten();
            } else {
                // Just forget working
                let _ = conn.execute("DELETE FROM memories WHERE memory_type = 'working'", []);
                manager.clear_working(); // need to add this method to WorkingBuffer
            }
            
            results.push(HookResult::SessionEnd { consolidated_count, new_episodic_id });
            
            // Session start: auto-recall
            let recalled = recall::auto_recall(conn).unwrap_or(0);
            results.push(HookResult::SessionStart { recalled_count: recalled });
        }
        
        // 3. Decay
        let affected = decay::decay_episodic(conn, decay_rate).unwrap_or(0);
        if affected > 0 {
            results.push(HookResult::Decay { affected_count: affected });
        }
        
        // 4. Update last_activity
        let _ = config::set_lifecycle_config_i64(conn, "lifecycle_last_activity", now);
        
        results
    }
}
```

- [ ] **Step 3: Add `clear_working()` to WorkingBuffer** in `src/tier/working.rs`

```rust
pub fn clear(&mut self) {
    self.entries.clear();
}
```

- [ ] **Step 4: Modify `TierManager`** in `src/tier/manager.rs`

Add `pub fn clear_working(&mut self)` that calls `self.working.clear()`.

- [ ] **Step 5: Run test to verify it passes**

Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/lifecycle/engine.rs src/lifecycle/mod.rs src/tier/working.rs src/tier/manager.rs tests/lifecycle_test.rs
git commit -m "feat(lifecycle): implement LifecycleEngine with idle, overflow, and decay"
```

---

### Task 6: Ensure Consolidation Deletes DB Rows

**Files:**
- Modify: `src/tier/manager.rs`
- Test: `tests/tier_manager_test.rs`

- [ ] **Step 1: Write failing test** `test_consolidate_deletes_working_rows()`

```rust
fn test_consolidate_deletes_working_rows() {
    // remember 3 working
    // consolidate
    // DB count of working should be 0
    // episodic count should be 1
}
```

- [ ] **Step 2: Modify `consolidate_working_to_episodic()`** to clear DB working rows

```rust
pub fn consolidate_working_to_episodic(&mut self) -> rusqlite::Result<Option<String>> {
    let entries = self.working.drain();
    if entries.is_empty() {
        return Ok(None);
    }
    
    // Delete DB working rows too
    self.conn.execute("DELETE FROM memories WHERE memory_type = 'working'", [])?;
    
    let contents: Vec<String> = entries.iter().map(|e| e.content.clone()).collect();
    let summary = format!("[Consolidated] {}", contents.join("; "));
    
    let id = self.store.insert("episodic", &summary, 0.5, "consolidation", &[])?;
    Ok(Some(id))
}
```

- [ ] **Step 3: Run test to verify it passes**

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/tier/manager.rs tests/tier_manager_test.rs
git commit -m "fix(tier): consolidate deletes working rows from DB"
```

---

### Task 7: Integrate Lifecycle into REPL Runner

**Files:**
- Modify: `src/repl/runner.rs`
- Test: `tests/lifecycle_test.rs`

- [ ] **Step 1: Write failing integration test** `test_repl_runs_lifecycle_hooks()`

```rust
fn test_repl_runs_lifecycle_hooks() {
    // Create Repl, add working memory
    // Wait 2 minutes (or mock)
    // Send STATUS command
    // Output should contain lifecycle tags
}
```

- [ ] **Step 2: Modify `Repl::execute()`** in `src/repl/runner.rs`

```rust
pub fn execute(&mut self, cmd: Command) -> Response {
    use crate::lifecycle::LifecycleEngine;
    
    // Run lifecycle
    let mut hook_results = Vec::new();
    if let Ok(mut manager) = TierManager::new(self.db.conn(), 100) {
        hook_results = LifecycleEngine::check_and_fire(self.db.conn(), &mut manager);
    }
    
    // Now execute the user's command
    let mut response = self.execute_internal(cmd);
    
    // Append lifecycle results
    for hook in hook_results {
        response = Response::Lifecycle { message: hook.to_string() };
        // Or append to a new field
    }
    
    response
}
```

**Issue:** Need a way to append lifecycle to response. Two options:
a) Add `lifecycle_messages: Vec<String>` to `Response` enum variants
b) Print lifecycle as `<lifecycle>` tag before the actual response

Option (b) is simpler. Modify `Display` for `Response` to optionally print lifecycle header.

```rust
fn execute(&mut self, cmd: Command) -> Response {
    use crate::lifecycle::LifecycleEngine;
    
    let hook_results = if let Ok(mut manager) = TierManager::new(self.db.conn(), 100) {
        LifecycleEngine::check_and_fire(self.db.conn(), &mut manager)
    } else {
        Vec::new()
    };
    
    // Print lifecycle messages
    for hook in &hook_results {
        println!("<lifecycle>\n  {}\n</lifecycle>", hook);
    }
    
    // ... rest of execute match
}
```

- [ ] **Step 3: Run test to verify it passes**

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/repl/runner.rs tests/lifecycle_test.rs
git commit -m "feat(repl): integrate lifecycle engine into execute()"
```

---

### Task 8: Integrate Lifecycle into MCP Handlers

**Files:**
- Modify: `src/mcp/mod.rs`
- Test: `tests/mcp_test.rs`

- [ ] **Step 1: Write failing test** `test_mcp_lifecycle_fires_on_idle()`

```rust
fn test_mcp_lifecycle_fires_on_idle() {
    // Set up MCP server
    // Manually set lifecycle_last_activity to past
    // Send "remember" request
    // Response should include lifecycle message in content text
}
```

- [ ] **Step 2: Modify every MCP handler** to call `LifecycleEngine::check_and_fire()`

Example for `handle_remember()`:

```rust
fn handle_remember(
    id: Option<serde_json::Value>,
    args: serde_json::Value,
    agent_id: &str,
) -> McpResponse {
    let content = args.get("content").and_then(|c| c.as_str()).unwrap_or("");
    // ...
    
    match crate::store::MnemoDb::new(&db_path) {
        Ok(db) => {
            let mut manager = match crate::tier::TierManager::new(db.conn(), 100) {
                Ok(m) => m,
                Err(e) => return McpResponse::error(id, -32603, format!("DB error: {}", e)),
            };
            
            // Run lifecycle
            let hook_results = crate::lifecycle::LifecycleEngine::check_and_fire(
                db.conn(), &mut manager
            );
            
            let mut hook_texts: Vec<String> = hook_results
                .iter()
                .map(|h| h.to_string())
                .collect();
            
            // ... execute remember logic ...
            
            let response_text = if hook_texts.is_empty() {
                format!("Stored memory: {}", mem_id)
            } else {
                format!("{}\n\n{}", hook_texts.join("\n"), format!("Stored memory: {}", mem_id))
            };
            
            McpResponse::success(
                id,
                json!({"content": [{"type": "text", "text": response_text}]}),
            )
        }
        // ...
    }
}
```

Do the same for `handle_recall`, `handle_extract`, `handle_bind`, `handle_status`, `handle_forget`.

- [ ] **Step 3: Run test to verify it passes**

Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/mcp/mod.rs tests/mcp_test.rs
git commit -m "feat(mcp): integrate lifecycle engine into all handlers"
```

---

### Task 9: Add Full Integration Tests

**Files:**
- Create: `tests/lifecycle_integration_test.rs`

- [ ] **Step 1: Write tests**

```rust
// Test 1: idle session boundary
fn test_lifecycle_end_to_end_idle() {
    // Create Repl
    // Remember working memory
    // Mock/Manually set last_activity to 2 mins ago
    // Execute STATUS command
    // Assert working count is 0, episodic count is 1
    // Assert response contains session-start with recalled memories
}

// Test 2: overflow consolidation
fn test_lifecycle_overflow() {
    // Insert 81 working memories directly into DB
    // Execute a command
    // Assert working count drops below 80
    // Assert episodic count increased
}

// Test 3: decay
fn test_lifecycle_decay() {
    // Insert old episodic memory with confidence 1.0
    // Execute any command
    // Assert confidence is lower
}

// Test 4: opt-out
fn test_lifecycle_disabled() {
    // Set lifecycle_enabled = false
    // Wait, insert, etc.
    // Assert no consolidation happened
}
```

- [ ] **Step 2: Run all tests**

```bash
cargo test
```

Expected: All pass

- [ ] **Step 3: Commit**

```bash
git add tests/lifecycle_integration_test.rs
git commit -m "test(lifecycle): add comprehensive integration tests"
```

---

### Task 10: Update README.md

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add "Lifecycle Hooks" section** after Quick Start

```markdown
### 9. Lifecycle Hooks (Automatic Memory Management)

mnemo automatically manages your memory tiers:

- **Session boundaries**: After 1 minute of idle time, working memories are consolidated into episodic.
- **Decay**: Episodic memories gradually lose confidence over time so stale data surfaces less.
- **Recall**: On session start, relevant semantic + episodic context is loaded into your working buffer.
- **Overflow**: When working buffer hits 80% capacity, auto-consolidation triggers.

All hooks are configurable via `pragma`:

```bash
mnemo pragma lifecycle_enabled true        # master switch
mnemo pragma lifecycle_idle_threshold 120  # seconds (default: 60)
mnemo pragma lifecycle_decay_rate 0.1      # per day (default: 0.1)
```
```

- [ ] **Step 2: Update status table** to mark lifecycle as ✅

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs(readme): document v0.3 lifecycle hooks"
```

---

## Spec Coverage Checklist

| Spec Section | Task |
|--------------|------|
| Core insight (DB-backed working) | Task 5, Task 6 |
| Lifecycle engine sequence | Task 5 |
| HookResult enum | Task 2 |
| Hook reporting | Task 7, Task 8 |
| Configuration | Task 1 |
| Decay algorithm | Task 3 |
| Auto-recall | Task 4 |
| Consolidation deletes working rows | Task 6 |
| Example session flow | Task 9 (test coverage) |
| PRAGMA commands | Task 7, Task 8 (usage), Task 10 (docs) |

## Risk Coverage Checklist

| Risk | Task |
|------|------|
| Multi-agent DB sharing | Documented in spec, out of scope |
| Premature consolidation | 60s idle + 80% overflow, tested in Task 9 |
| Decay deletes useful memories | Floor at 0.1, tested in Task 3 |
| MCP delayed lifecycle | Tested in Task 8 |
| SQLite bloat | 5min guard in decay query, Task 3 |

## Final Verification

```bash
# Run all tests
cargo test

# Run clippy
cargo clippy -- -D warnings

# Format
cargo fmt
```
