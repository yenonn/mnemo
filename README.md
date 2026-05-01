# mnemo — Agent Memory Database

> Pronounced /NEH-moh/ (silent M, like "mnemonic")

mnemo is to agent memory what `sqlite3` is to tabular data: a single, zero-config, embeddable binary that any AI agent can use to store, recall, and consolidate its short-term and long-term memory.

## The Problem

Every AI agent starts from scratch on every session. There is no `sqlite3` equivalent for memory. Existing solutions like Letta and Mem0 are powerful but require Python runtimes, Docker, Postgres, or cloud API keys — too heavy for a simple agent that just needs to remember.

## The Solution

```bash
$ mnemo remember "User prefers dark mode" --type=semantic
$ mnemo recall "dark mode"
<result count="1">
  <memory id="mem-abc123" type="semantic" confidence="0.95">
    User prefers dark mode
  </memory>
</result>
```

mnemo is:
- **A single binary** (~3-5MB, static-linked, no dependencies)
- **Zero-config** — runs out of the box, no Docker, no Redis, no Postgres
- **Agent-agnostic** — works with Claude Code, OpenCode, Cursor, Codex, or any custom agent
- **Model-agnostic** — memory is just text; vectors from any embedding provider

## Memory Tiers

| Tier | Storage | Duration | Example |
|------|---------|----------|---------|
| **Working** | In-memory ring buffer | Seconds–minutes | "User just said they're tired" |
| **Episodic** | SQLite + FTS5 | Hours–months | "Last time user asked about X, liked Y" |
| **Semantic** | SQLite + FTS5 | Months–permanent | "User prefers dark mode" |

Memories naturally consolidate up the tiers:
```
Working → Episodic → Semantic
```

## Installation

### From Source
```bash
git clone https://github.com/yenonn/mnemo.git
cd mnemo
cargo build --release
# Binary will be at target/release/mnemo
cp target/release/mnemo /usr/local/bin/
```

## Quick Start

### 1. Store a Memory
```bash
# Explicit storage
mnemo remember "User prefers dark mode" --type semantic

# With importance score
mnemo remember "User hates popups" --type semantic --importance 0.9
```

### 2. Retrieve Memories
```bash
# Search all tiers
mnemo recall "dark mode"

# Filter by tier
mnemo recall "vim" --memory-type semantic

# Limit results
mnemo recall "work" --limit 5
```

### 3. Extract from Natural Language
```bash
# Without API key: heuristic fallback
mnemo extract "I had a bad day. Dark mode helps my eyes."
# → Extracted and stored 2 memories

# With API key: LLM extraction
export MNEMO_OPENAI_API_KEY=sk-...
mnemo extract "I prefer minimal UI and I use vim for everything."
# → Extracted and stored 2 memories
```

### 4. Check Status
```bash
mnemo status
# <status>
#   Agent: default
#   Working buffer: 0
#   Episodic memories: 1
#   Semantic memories: 3
# </status>
```

### 5. Interactive REPL
```bash
mnemo --repl
mnemo> REMEMBER "User prefers dark mode" AS semantic;
mnemo> RECALL "dark mode" FROM semantic;
mnemo> EXTRACT "I had a rough day but I love vim";
mnemo> STATUS;
mnemo> quit
```

### 6. Consolidate Memories
```bash
# Move all working memories to episodic
mnemo consolidate working episodic
```

### 7. Delete a Memory
```bash
mnemo forget mem-abc123
```

## MCP Server Mode (For OpenCode / Claude Code)

Mnemo supports the **Model Context Protocol** — agents can call memory operations as tools via JSON-RPC.

### Start the MCP Server
```bash
mnemo --mcp --agent-id my-agent
```

### Available MCP Tools

| Tool | Description | Example Args |
|------|-------------|--------------|
| `remember` | Store a memory explicitly | `{"content":"User likes dark mode","memory_type":"semantic"}` |
| `recall` | Search memories | `{"query":"dark mode","limit":5}` |
| `extract` | Auto-extract from natural language | `{"text":"I prefer dark mode"}` |
| `status` | Show memory counts | `{}` |
| `forget` | Delete by ID | `{"id":"mem-abc123"}` |

### OpenCode Integration

**1. Configure your agent:**
```json
{
  "mcpServers": {
    "mnemo": {
      "command": "mnemo",
      "args": ["--mcp", "--agent-id", "my-agent"]
    }
  }
}
```

**2. The agent automatically calls tools:**
```
User: "I had a bad day but I prefer dark mode"
Agent → calls mnemo.extract("I had a bad day but I prefer dark mode")
mnemo → returns extracted memories
Agent → responds with context
```

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `MNEMO_OPENAI_API_KEY` | Enable LLM extraction | none (uses heuristic) |
| `MNEMO_OPENAI_MODEL` | Model for extraction | `gpt-4o-mini` |
| `HOME` | Database location | `~/.mnemo/{agent_id}/memory.db` |

## Architecture

```
mnemo binary (~3-5MB, Rust, static-linked)
  ├── Text Protocol (stdin/stdout)
  ├── MCP Server (JSON-RPC stdio)
  │   ├── remember
  │   ├── recall
  │   ├── extract
  │   ├── status
  │   └── forget
  ├── Memory Tier Manager
  │   ├── Working Buffer (in-memory ring)
  │   ├── Episodic Store (SQLite + FTS5)
  │   └── Semantic Store (SQLite + FTS5)
  ├── Extract Engine (LLM / heuristic)
  └── SQLite Storage (single ~/.mnemo/{agent_id}/memory.db file)
```

## Data Location

```
~/.mnemo/
└── {agent-id}/
    └── memory.db          # Single SQLite file
```

## Status

**v0.1** — In development.

Features:
- ✅ CLI commands (remember, recall, status, forget)
- ✅ Interactive REPL mode
- ✅ Automatic memory extraction from natural language
- ✅ MCP server mode for agent integration
- ✅ Memory tier management (working → episodic → semantic)
- ✅ Consolidation
- ⏳ Vector embeddings (v0.2)
- ⏳ Lifecycle hooks (v0.3)

See [`docs/2026-05-01-mnemo-agent-memory-database-design.md`](docs/2026-05-01-mnemo-agent-memory-database-design.md) for the full specification.

## License

MIT

## Contributing

This project follows TDD with failing tests written before code. See `tests/` for examples.
