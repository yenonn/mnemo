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
| **Semantic** | SQLite + HNSW (via sqlite-vec) | Months–permanent | "User prefers dark mode" |

Memories naturally consolidate up the tiers:
```
Working → Episodic → Semantic
```

## Quick Start

```bash
# 1. Download (placeholder — not yet released)
curl -fsSL https://get.mnemo.dev | sh

# 2. Remember something
mnemo remember "User likes minimal UI"

# 3. Recall later
mnemo recall "minimal UI"

# 4. Interactive REPL
mnemo --repl
mnemo> REMEMBER "User prefers dark mode" AS semantic WITH importance=0.9;
mnemo> RECALL "dark mode" FROM semantic;
mnemo> STATUS;
```

## Architecture

```
mnemo binary (~3-5MB, Rust, static-linked)
  ├── Text Protocol (stdin/stdout)
  ├── Memory Tier Manager
  │   ├── Working Buffer (in-memory ring)
  │   ├── Episodic Store (SQLite + FTS5)
  │   └── Semantic Store (SQLite + HNSW via sqlite-vec)
  ├── Lifecycle Engine (confidence decay, consolidation, GC)
  ├── Embedding Gateway (Ollama / OpenAI HTTP client)
  └── SQLite Storage (single ~/.mnemo/{agent_id}/memory.db file)
```

## Status

**v0.1** — In development. MVP with working, episodic, and semantic memory tiers, text protocol, and external embedding gateway.

See [`docs/2026-05-01-mnemo-agent-memory-database-design.md`](docs/2026-05-01-mnemo-agent-memory-database-design.md) for the full specification.

See [`docs/2026-05-01-mnemo-v0.1-implementation-plan.md`](docs/2026-05-01-mnemo-v0.1-implementation-plan.md) for the implementation plan.

## License

MIT

## Contributing

This project is in early design phase. The spec and plan are the source of truth. Implementation follows TDD with failing tests written before code.
