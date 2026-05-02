# AGENTS.md — mnemo

> Quick reference for AI agents working on the mnemo codebase.

## What is mnemo?

mnemo is a **zero-config, single-binary agent memory database** written in Rust. Think of it as `sqlite3` for AI agent memory — agents use it to store, recall, and consolidate memories across sessions without needing Docker, Postgres, or cloud APIs.

## Architecture

```
mnemo binary (~3-5MB, Rust, static-linked)
  ├── Text Protocol (stdin/stdout) ─ main.rs + protocol/
  ├── MCP Server (JSON-RPC stdio) ─ mcp/
  ├── Interactive REPL ─ repl/
  ├── Memory Tier Manager ─ tier/
  │   ├── Working Buffer (in-memory ring)
  │   ├── Episodic Store (SQLite + FTS5)
  │   └── Semantic Store (SQLite + FTS5)
  ├── Extract Engine (LLM / heuristic) ─ extract/
  ├── Embedding Provider ─ embed/
  ├── Lifecycle Hooks ─ lifecycle/
  └── Storage Layer ─ store/
```

### Source Layout

```
src/
  main.rs       # CLI entry point (clap subcommands)
  lib.rs        # Public module exports
  protocol/     # Text protocol parser + command types
  store/        # SQLite storage, database initialization
  tier/         # Memory tier logic (working/episodic/semantic)
  embed/        # Vector embedding provider interface
  extract/      # Memory extraction from natural language
  repl/         # Interactive REPL
  mcp/          # Model Context Protocol server
  lifecycle/    # Lifecycle hooks
```

## Memory Tiers

| Tier | Storage | Duration | Example |
|------|---------|----------|---------|
| **Working** | In-memory ring buffer | Seconds–minutes | "User just said they're tired" |
| **Episodic** | SQLite + FTS5 | Hours–months | "Last time user asked about X, liked Y" |
| **Semantic** | SQLite + FTS5 | Months–permanent | "User prefers dark mode" |

Consolidation flow: `Working → Episodic → Semantic`

## Key Commands

```bash
# Build
cargo build --release

# Run tests
cargo test

# Run with coverage
cargo tarpaulin

# One-shot usage
cargo run -- remember "User prefers dark mode" --type semantic
cargo run -- recall "dark mode"

# REPL
cargo run -- --repl

# MCP server
cargo run -- --mcp --agent-id my-agent
```

## CLI Subcommands (main.rs)

| Command | Purpose |
|---------|---------|
| `remember <TEXT> --type <TIER>` | Store a memory |
| `recall <QUERY> [--memory-type <T>] [--limit N]` | Search memories |
| `status` | Show memory counts |
| `init` | Initialize database |
| `consolidate <FROM> <TO>` | Move memories between tiers |
| `extract <TEXT>` | Auto-extract memories from text |
| `forget <ID>` | Delete a memory |
| `pragma [KEY] [VALUE]` | Config get/set |

## MCP Tools (mcp/)

When running with `--mcp`, the binary exposes JSON-RPC tools:

- `remember`
- `recall`
- `extract`
- `status`
- `forget`

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `MNEMO_OPENAI_API_KEY` | Enable LLM extraction | none (heuristic fallback) |
| `MNEMO_OPENAI_MODEL` | Model for extraction | `gpt-4o-mini` |
| `HOME` | Database location | `~/.mnemo/{agent_id}/memory.db` |

## Data Storage

- **Location**: `~/.mnemo/{agent-id}/memory.db`
- **Engine**: SQLite with FTS5 extension (bundled via `rusqlite`)
- **Schema**: Managed in `store/`

## Development Workflow

1. **TDD required**: Write failing tests BEFORE implementation code
2. Use `assert_cmd` + `predicates` + `tempfile` for integration tests
3. Run `cargo clippy` and `cargo fmt` before committing

## Important Files for Context

| File | What it does |
|------|--------------|
| `src/main.rs` | CLI argument parsing, mode dispatch (CLI / REPL / MCP) |
| `src/protocol/` | Central command/response types used by all interfaces |
| `src/store/` | All database I/O — the only place that touches SQLite |
| `src/tier/` | Memory tier promotion/consolidation logic |
| `src/mcp/` | JSON-RPC server for agent tool integration |
| `Cargo.toml` | Dependencies: rusqlite, tokio, clap, serde, reqwest, etc. |
| `README.md` | Full user-facing documentation |

## Testing Strategy

- Unit tests in each module (`#[cfg(test)]`)
- Integration tests in `tests/` using `assert_cmd`
- Test database isolation via `tempfile`

## Tips for Agents

- **Always use the REPL/CLI for manual testing**: `cargo run -- --repl`
- **Database is stateful**: Test DBs are created in `~/.mnemo/`; use `--agent-id test-agent` for isolation
- **MCP mode**: The binary communicates via stdin/stdout JSON-RPC when `--mcp` is passed
- **Protocol module is the source of truth**: `Command` and response types defined there are used by CLI, REPL, and MCP
- **Consolidation is manual**: No automatic consolidation; user or agent must call `consolidate working episodic`
