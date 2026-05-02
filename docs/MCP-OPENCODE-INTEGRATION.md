# MCP Integration Guide — mnemo + OpenCode

> Step-by-step guide for integrating the mnemo memory database with OpenCode via MCP.

---

## What You Get

- **Zero-config memory** — mnemo stores, recalls, and auto-consolidates memories for your agent
- **Single binary** — no Docker, no Postgres, no cloud APIs required
- **Lifecycle hooks** — idle consolidation, decay, and context recall run automatically

---

## Prerequisites

1. **Rust toolchain** (>= 1.70) — install via [rustup.rs](https://rustup.rs)
2. **OpenCode** installed and configured
3. **Git** — you already have this (`ai-databases/mnemo`)

---

## Step 1: Build mnemo

From your project directory:

```bash
cd /Users/yenonnhiu/Developments/ai-databases/mnemo

cargo build --release
```

Wait ~2 minutes. The binary appears at:

```
target/release/mnemo
```

**Verify it works:**

```bash
./target/release/mnemo --help
```

You should see the CLI subcommands: `remember`, `recall`, `status`, `init`, `consolidate`, `extract`, `forget`, `pragma`.

---

## Step 2: Configure OpenCode MCP Server

OpenCode supports MCP servers via JSON-RPC over stdio. You need to register mnemo as an MCP server in your OpenCode configuration.

### Option A: Edit OpenCode Config File

Opencode stores MCP server configuration in its main config file. Depending on your scope, edit one of these:

- **Global** (all projects): `~/.config/opencode/opencode.json`
- **Project-level** (current directory only): `./opencode.json`

Add an `mcp` entry at the top level of the JSON (sibling to `provider`, not nested inside it). For a **local stdio** server, use `type: "local"` and pass the binary and all arguments as a single `command` **array**:

**Without OpenAI key (recommended for testing):**

```json
{
  "mcp": {
    "mnemo": {
      "type": "local",
      "command": [
        "/Users/yenonnhiu/Developments/ai-databases/mnemo/target/release/mnemo",
        "--mcp",
        "--agent-id",
        "opencode-agent"
      ],
      "enabled": true
    }
  }
}
```

**With OpenAI key (optional — enables LLM extraction + vector search):**

```json
{
  "mcp": {
    "mnemo": {
      "type": "local",
      "command": [
        "/Users/yenonnhiu/Developments/ai-databases/mnemo/target/release/mnemo",
        "--mcp",
        "--agent-id",
        "opencode-agent"
      ],
      "enabled": true,
      "environment": {
        "MNEMO_OPENAI_API_KEY": "sk-your-key-here"
      }
    }
  }
}
```

> **Important:**
> - Opencode **does not** read from `~/.config/opencode/mcp.json`.
> - The key must be `mcp` (not `mcpServers`).
> - For `type: "local"`, `command` is an **array** containing the full binary path followed by all arguments.
> - `disabled` and `autoApprove` are **not** valid fields here; use `enabled: true/false` instead.

**Important:**
- **No API key needed for core functionality** — remember, recall, status, lifecycle all work without it
- OpenAI key is only needed for `extract` (better quality) and semantic/vector search
- The `--mcp` flag puts mnemo in MCP server mode (JSON-RPC stdio)
- The `--agent-id` isolates this agent's memory from others
- Full path to binary is required (no `~` expansion)

---

## Step 3: Restart OpenCode

After updating the MCP config, restart OpenCode so it discovers the new MCP server.

**Check if mnemo is loaded:**

In OpenCode, ask:

> "What MCP servers are available?"

Or look for mnemo in your tool list. You should see these tools registered:

| Tool | Description |
|------|-------------|
| `remember` | Store a memory (working, episodic, or semantic) |
| `recall` | Search and retrieve memories |
| `extract` | Auto-extract memories from natural language |
| `status` | Show memory counts by tier |
| `forget` | Delete a memory by ID |

---

## Step 4: Test the Integration

### Test 1: Remember Something

Ask OpenCode:

> "Use mnemo to remember that I prefer dark mode"

Or, if you can call tools directly:

```
Tool: remember
Args: {"content": "User prefers dark mode", "memory_type": "semantic"}
```

**Expected:** Response says "Stored memory: mem-XXXX"

### Test 2: Recall

> "Use mnemo to recall what I prefer"

```
Tool: recall
Args: {"query": "dark mode"}
```

**Expected:** Returns the memory about dark mode preference.

### Test 3: Extract from Conversation

> "Use mnemo to extract memories from this: 'I love hiking and my dog is named Luna'"

```
Tool: extract
Args: {"content": "I love hiking and my dog is named Luna"}
```

**Expected:** Returns extracted facts (hobby: hiking, pet: Luna).

### Test 4: Status

> "Use mnemo to check memory status"

**Expected:** Shows counts like `Working: 0 | Episodic: 2 | Semantic: 1`

### Test 5: Lifecycle (Idle Consolidation)

This is the fun part. mnemo auto-consolidates working memories when idle.

1. Store a working memory:
   > "Use mnemo to remember I said 'hello' as a working memory"

2. Wait 60 seconds (default idle threshold) — or simulate by manually editing the DB:

   ```bash
   sqlite3 ~/.mnemo/opencode-agent/memory.db \
     "UPDATE _mnemo_meta SET value='1' WHERE key='lifecycle_last_activity';"
   ```

3. Trigger any mnemo command (e.g., `status`):
   > "Use mnemo to check status again"

**Expected:** You should see lifecycle output like:
```
[session-end] Consolidated 1 memories to mem-XXXX
[session-start] Recalled 1 memories
```

### Test 6: Context Recall

After idle consolidation, mnemo auto-recalls relevant episodic/semantic memories as working memories. These appear with `[context-recall]` prefix.

Check for them in the DB:

```bash
sqlite3 ~/.mnemo/opencode-agent/memory.db \
  "SELECT memory_type, content FROM memories WHERE content LIKE '%context-recall%';"
```

---

## Step 5: Verify Lifecycle is Active

Run the built-in status check:

```bash
/Users/yenonnhiu/Developments/ai-databases/mnemo/target/release/mnemo status --agent-id opencode-agent
```

Look for lifecycle metadata:

```
Lifecycle: enabled=true, last_activity=..., threshold=60s
```

---

## Troubleshooting

### mnemo binary not found

```bash
which /Users/yenonnhiu/Developments/ai-databases/mnemo/target/release/mnemo
```

If missing, rebuild:

```bash
cd /Users/yenonnhiu/Developments/ai-databases/mnemo && cargo build --release
```

### OpenCode not discovering tools

1. Check OpenCode MCP logs (location varies by install)
2. Verify JSON-RPC communication:

   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | \
     /Users/yenonnhiu/Developments/ai-databases/mnemo/target/release/mnemo --mcp --agent-id test
   ```

   Should output a JSON list of available tools.

### No lifecycle output

1. Check if lifecycle is enabled:

   ```bash
   /Users/yenonnhiu/Developments/ai-databases/mnemo/target/release/mnemo pragma lifecycle_enabled --agent-id opencode-agent
   ```

2. Check `last_activity` timestamp:

   ```bash
   sqlite3 ~/.mnemo/opencode-agent/memory.db \
     "SELECT value FROM _mnemo_meta WHERE key='lifecycle_last_activity';"
   ```

3. Ensure working memories exist:

   ```bash
   sqlite3 ~/.mnemo/opencode-agent/memory.db \
     "SELECT id, content FROM memories WHERE memory_type='working';"
   ```

### Database locked errors

mnemo uses SQLite with WAL mode. If you see "database is locked":

- Don't run multiple mnemo instances with the same `--agent-id`
- Close any open `sqlite3` CLI sessions to the DB

---

## Architecture Recap

```
OpenCode Agent
  └── MCP Client (stdio JSON-RPC)
        └── mnemo --mcp --agent-id opencode-agent
              ├── Working Buffer (in-memory ring)
              ├── Episodic Store (SQLite + FTS5)
              ├── Semantic Store (SQLite + FTS5)
              └── Lifecycle Engine
                    ├── Idle Detection → Consolidate
                    ├── Session Start → Auto-Recall
                    └── Periodic → Decay confidence
```

---

## Next Steps

- Adjust thresholds: `pragma lifecycle_idle_threshold 120` (2 minutes)
- Disable: `pragma lifecycle_enabled false`
- Explore: `cargo run -- --repl` for interactive testing
- Read: `README.md` section 9 for full lifecycle docs

---

## Quick Reference

| Command | Purpose |
|---------|---------|
| Build | `cargo build --release` |
| MCP Test | `echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' \| ./target/release/mnemo --mcp` |
| Status | `./target/release/mnemo status --agent-id opencode-agent` |
| DB Query | `sqlite3 ~/.mnemo/opencode-agent/memory.db "SELECT * FROM memories;"` |
| Disable lifecycle | `./target/release/mnemo pragma lifecycle_enabled false --agent-id opencode-agent` |

---

**Happy testing!** Report any issues to the mnemo repo.
