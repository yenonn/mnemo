# Lifecycle Hooks Design — mnemo v0.3

> Self-inferring, mode-agnostic lifecycle engine for automatic memory management.

## Overview

mnemo v0.3 introduces **lifecycle hooks**: an inline, self-managing engine that infers session boundaries from usage patterns and automatically consolidates working memories, decays stale episodic memories, and recalls relevant context — all without requiring the agent framework to explicitly coordinate.

The engine is **mode-agnostic**: it runs identically in REPL, one-shot CLI, and MCP modes, because the source of truth is the database, not an ephemeral in-memory buffer.

## Goals

1. **Automatic consolidation**: Working memories are consolidated into episodic memories on session boundaries or buffer overflow.
2. **Confidence decay**: Older episodic memories gradually lose confidence so that stale or low-relevance data surfaces less often.
3. **Context recall**: On session start, relevant semantic + episodic context is loaded into the working buffer (or returned in the response for stateless modes).
4. **Zero-config by default**: Tuned for typical agent workflows; user can adjust via `PRAGMA`.
5. **Non-breaking**: All hooks are opt-out via config. Legacy behavior is preserved if hooks are disabled.

## Non-Goals

- Automatic episodic → semantic promotion. This remains manual (`consolidate` command).
- Background threads / server mode. All lifecycle runs on the command execution path.
- Multi-agent locking. Documented as a known limitation for v0.3.

## Background

In mnemo v0.1–v0.2, memory tiers (`working`, `episodic`, `semantic`) exist but promotion between tiers is entirely manual. If an agent crash occurs while the `WorkingBuffer` has unconsolidated entries, those memories are lost because the in-memory ring is not the source of truth. The agent framework also must explicitly call `consolidate` — a burden for agent integrations.

Existing systems (Letta, Mem0, Zep) handle some lifecycle but never tie automatic tiering to a zero-config, static-file database. mnemo v0.3 fills this gap.

## Design

### 1. Core Insight: Working Memory Is DB-Backed

`TierManager::new()` hydrates the `WorkingBuffer` from SQLite. This means:

- Working memories are not ephemeral across requests.
- Consolidation and overflow checks can use the DB row count, not a transient counter.
- All modes (REPL, one-shot, MCP) observe the same lifecycle state.

### 2. Lifecycle Engine Sequence

On every command execution (`Repl::execute()` and every MCP handler), the engine runs **inline** in a single SQLite transaction:

1. **Read `_mnemo_meta`** for `lifecycle_last_activity`, `lifecycle_enabled`, thresholds.
2. **Calculate idle seconds** = `now - last_activity`.
3. **Buffer overflow check**: count `working` rows in DB. If count >= `capacity * 0.8`, fire `OVERFLOW`.
4. **Idle session boundary**: if idle >= `lifecycle_idle_threshold` (default 60s), fire `SESSION_END`, then `SESSION_START`.
5. **Confidence decay**: decrement `confidence` on episodic memories based on age and `lifecycle_decay_rate`.
6. **Update `lifecycle_last_activity`**.
7. **Execute user command**.

All steps (1–6) run in **one SQLite transaction**, committed before the user's command executes. If lifecycle operations fail, the error is logged (or returned as a warning in REPL), but the user's command still executes. Lifecycle is **opportunistic**, not **mandatory**.

### 3. Hook Definitions

| Event | Trigger | Action | Applies To |
|-------|---------|--------|------------|
| `SESSION_END` | idle >= threshold | `TierManager::consolidate_working_to_episodic()` | All modes |
| `SESSION_START` | after SESSION_END | Query episodic + semantic, push top N into working buffer | REPL and MCP (returned in response) |
| `OVERFLOW` | DB working count >= 80% capacity | `TierManager::consolidate_working_to_episodic()` | All modes |
| `DECAY` | every command | Update `confidence` on old episodic rows | All modes |

### 4. Hook Reporting

Lifecycle hooks that fire yield `HookResult` values:

```rust
pub enum HookResult {
    SessionEnd { consolidated_count: usize, new_episodic_id: Option<String> },
    SessionStart { recalled_count: usize },
    Overflow { consolidated_count: usize },
    Decay { affected_count: usize },
}
```

In REPL: results are printed as `<lifecycle>` tags.
In MCP: results are appended to the tool response text.

Example REPL output:
```
mnemo> recall "dark mode"
<lifecycle>
  [session-start] Recalled 3 memories
</lifecycle>
<result count="3">
  ...
</result>
```

Example MCP response:
```json
{
  "content": [{
    "type": "text",
    "text": "Recalled 3 memories for session start.\n\nFound 1 memories:\n[semantic] User prefers dark mode"
  }]
}
```

### 5. Configuration via `_mnemo_meta`

All configuration is stored in the existing `_mnemo_meta` table, readable and writable via `pragma`.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `lifecycle_enabled` | bool | `true` | Master on/off switch |
| `lifecycle_idle_threshold` | i64 (seconds) | `60` | Idle before session end/start |
| `lifecycle_decay_rate` | f64 (per day) | `0.1` | Proportional confidence decay |
| `lifecycle_consolidate_on_flush` | bool | `true` | If false, SESSION_END only runs `FORGET working` |

### 6. Decay Algorithm

On every command run:

1. Select all episodic memories where `confidence > 0.1`.
2. For each memory: `age_days = (now - created_at) / ms_per_day`.
3. `new_confidence = confidence * (1.0 - decay_rate)^age_days`.
4. Round to 2 decimals.
5. Update the row with the new confidence.

If `new_confidence < 0.1`, set to `0.1` as a floor to prevent permanent deletion without explicit `FORGET`.

### 7. Auto-Recall on Session Start

After `SESSION_END` consolidates working memories, the engine queries the DB:

- Top 5 **episodic** memories by `created_at DESC`, filtered by `confidence > 0.3`.
- Top 5 **semantic** memories by `importance DESC`.

These are inserted back into the `working` tier with a `[context-recall]` prefix:

```
[context-recall] User prefers dark mode
```

This ensures a new session starts with relevant context, and the `[context-recall]` prefix allows humans (and agents) to distinguish recalled context from fresh observation.

### 8. Consolidation Behavior

`TierManager::consolidate_working_to_episodic()` drains the in-memory `WorkingBuffer` and produces a single episodic memory:

```text
[Consolidated session] <memory 1>; <memory 2>; ...
```

Because `TierManager::new()` hydrates from DB first, this automatically includes all "orphan" working memories that were not in the previous in-memory buffer.

After consolidation, the DB rows with `memory_type = 'working'` are deleted (not just from the in-memory buffer). The `WorkingBuffer` is cleared. FTS5 rowid triggers handle index cleanup.

## Implementation Plan

### Files to Change

| File | Action |
|------|--------|
| `src/lifecycle/mod.rs` | Implement `LifecycleEngine` struct and `check_and_fire()` |
| `src/lifecycle/engine.rs` | New file: core logic |
| `src/lifecycle/hook.rs` | New file: `HookResult` enums |
| `src/lifecycle/decay.rs` | New file: decay algorithm |
| `src/lifecycle/recall.rs` | New file: auto-recall query builder |
| `src/repl/runner.rs` | Call `LifecycleEngine` inside `execute()` |
| `src/mcp/mod.rs` | Call `LifecycleEngine` in every handler; append to response |
| `src/tier/manager.rs` | Ensure `consolidate_working_to_episodic()` deletes DB rows |
| `tests/lifecycle_test.rs` | New file: integration tests |

### Testing Strategy

1. **Idle session boundary**: mock `lifecycle_last_activity` to be in the past; assert consolidate + recall run.
2. **Overflow**: insert 81 working memories; assert next command triggers consolidation.
3. **Decay**: insert episodic memory with `confidence = 1.0` and `created_at = now - 7 days`; assert confidence reduced.
4. **Config opt-out**: set `lifecycle_enabled = false`; assert no hooks fire.
5. **Transaction safety**: cause a command error mid-lifecycle; assert DB rolls back, no side effects.

### Backwards Compatibility

- Default `lifecycle_enabled = true`. If a user upgrades, hooks start firing automatically.
- To disable, `mnemo pragma lifecycle_enabled false`.
- The `CONSOLIDATE` command still works identically — manual consolidation is always available.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Conflicts with multi-agent DB sharing | Data loss or double consolidation | Documented as "single process per DB" limitation for v0.3. |
| Premature consolidation on fast commands | Context lost in < 1 minute | Buffer-overflow check only fires at 80%+; idle threshold is 60s. |
| Confidence decay deletes useful memories | Low but non-zero | Floor at 0.1; decay is multiplicative not linear. |
| MCP server stateless, hooks only on request | Delayed lifecycle | Hooks fire on first command after idle — acceptable for v0.3. |
| SQLite transaction bloat with many rows | Latency on large DBs | Decay query uses `WHERE confidence > 0.1 AND created_at < now - 5min` to avoid full scan. |

## Out of Scope (Future Work)

1. **Background thread**: Not needed for v0.3. CLI is inherently request-driven.
2. **Episodic → semantic**: Manual only. May add in v0.4.
3. **Knowledge graph**: `memory_links` table exists in schema but not yet populated. v0.4.
4. **Multi-agent locking**: Requires row-level locking or WAL checkpoint coordination.

## Appendix: Example Session Flow

```
# Agent idle for 10 minutes

User: "What were my todos yesterday?"
→ mnemo mcp: recall "todos"
→ Lifecycle: idle=600s > 60s → SESSION_END (consolidate working → episodic)
→ Lifecycle: SESSION_START (recall preferences, recent episodic)
→ Lifecycle: DECAY (reduce confidence on old episodic)
→ Command: RECALL "todos"
→ Response: (recalled context + search results)
→ Update last_activity

# Agent sends 50 rapid requests in 30 seconds
→ All lifecycle checks evaluate
→ Overflow threshold not reached (requires 80 working)
→ No idle threshold crossed
→ Only DECAY fires (lightweight)

# Agent silent for 2 hours, then resumes
→ First command triggers SESSION_END (large consolidation)
→ SESSION_START (blank slate + fresh recall)
→ DECAY (heavier, many affected rows)
→ Command executes
```

## Appendix: PRAGMA Commands

```bash
# Check current lifecycle config
mnemo pragma lifecycle_enabled
mnemo pragma lifecycle_idle_threshold
mnemo pragma lifecycle_decay_rate

# Adjust
mnemo pragma lifecycle_idle_threshold 120
mnemo pragma lifecycle_decay_rate 0.05

# Disable completely
mnemo pragma lifecycle_enabled false
```

## References

1. `docs/2026-05-01-mnemo-agent-memory-database-design.md` — Section 11.2
2. `AGENTS.md` — Memory Tiers and Consolidation Flow
3. `README.md` — Lifecycle Hooks (v0.3) roadmap item
