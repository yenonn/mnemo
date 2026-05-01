# mnemo — Agent Memory Database Design Specification

> **Date:** 2026-05-01  
> **Project:** mnemo — A sqlite3-scale binary for AI agent long/short-term memory  
> **Status:** v0.1 Design Approved  

---

## 1. Vision & Problem Statement

### 1.1 Core Insight

The best AI agents today have no reliable, portable memory. Letta and Mem0 are powerful but require Python runtimes, Docker, Postgres, or cloud API keys. SQLite is the closest thing — but `sqlite3` stores data, not *memories*.

**mnemo** is to agent memory what `sqlite3` is to tabular data: a single, zero-config, embeddable binary that any agent can use to store, recall, and consolidate its short-term and long-term memory.

### 1.2 Target User

- Any AI agent (Claude Code, OpenCode, Cursor, Codex, custom Python/JS agents)
- Any developer building an agent who needs persistent memory without Redis/Postgres/Pinecone infrastructure
- Any workflow where `pip install` or `curl` to a cloud API is too heavy

### 1.3 What mnemo Is NOT

- ❌ Not a vector database (not competing with Pinecone/Qdrant)
- ❌ Not a distributed system (single-agent, single-node v0.1)
- ❌ Not a full agent framework (not competing with Letta/CrewAI)
- ❌ Not a cloud service (no API keys, no network required after binary download)

### 1.4 The Existing Gap

| Existing Solution | Strengths | Why It Doesn't Fit |
|-------------------|-----------|-------------------|
| Letta | Memory tiers, tool execution | Requires Python 3.11+, Postgres, Redis, Docker |
| Mem0 | Smart extraction, dedup | Requires Python/Node SDK, optional Docker stack |
| Pinecone/Qdrant | Fast vector search | Requires network API, no memory semantics |
| SQLite + sqlite-vec | Embedded, portable | No memory types, no lifecycle, no consolidation |
| Redis | Fast in-memory | Volatile, no semantic search, no structured storage |

**mnemo fills the gap:** Memory semantics, structured storage, and semantic search, all inside a single binary.

---

## 2. Design Overview

```
┌──────────────────────────────────────────────────────────────┐
│                     mnemo binary (~5-10MB)                   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Text Protocol Handler (stdin → parse → dispatch)    │   │
│  │  Commands: REMEMBER, RECALL, CONSOLIDATE, FORGET,    │   │
│  │  REFLECT, STATUS, PRAGMA, INIT                       │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Memory Tier Manager                                 │   │
│  │  ├─ Working Buffer  (in-memory ring, seconds–mins)  │   │
│  │  ├─ Episodic Store   (SQLite: interactions, events) │   │
│  │  └─ Semantic Store   (SQLite + HNSW: facts, prefs)│   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Lifecycle Engine (v0.1: manual triggers + periodic BG)│   │
│  │  ├─ Confidence decay (hourly cron)                  │   │
│  │  ├─ Consolidation triggers (explicit + auto)        │   │
│  │  └─ Garbage collection (decayed memory removal)     │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Embedding Gateway (v0.1: HTTP client)              │   │
│  │  ├─ OpenAI / Ollama / custom local API             │   │
│  │  └─ Async queue: text indexed immediately, vector   │   │
│  │                    searchable after embedding       │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  SQLite Storage Layer (embedded)                    │   │
│  │  ├─ memories, memory_chunks, memory_links tables    │   │
│  │  ├─ memory_vectors (sqlite-vec HNSW virtual table)│   │
│  │  ├─ memory_access_log (relevance feedback)          │   │
│  │  └─ memories_fts (FTS5 full-text index)             │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  └─  ~/.mnemo/{agent_id}/memory.db (single .db file)      │   │
└──────────────────────────────────────────────────────────────┘
              ↕ POSIX stdin / stdout / argv
        Agent Process (any language, any model)
```

### 2.1 Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Single binary** | Zero dependency deployment — copy binary, run. |
| **Text protocol** | Language-agnostic, easy to debug, REPL-like for humans. |
| **SQLite backend** | Proven ACID storage, WAL mode, no ops burden. Correctness before speed. |
| **Working memory in RAM** | Not persisted — ring buffer for current conversation flow. |
| **Episodic/Semantic in SQLite** | Persistent across restarts. |
| **External embedding gateway** | Keeps binary small. On-device embedding is a v0.2+ feature. |
| **sqlite-vec for HNSW** | Native SQLite extension, no separate server needed. |
| **Per-agent `.db` file** | Isolation, simplicity, easy to backup/inspect with `sqlite3` CLI. |

---

## 3. v0.1 Scope

### 3.1 In Scope

- Single-agent operation (no multi-agent ACL or sharing in v0.1)
- Three memory tiers: working, episodic, semantic
- Text protocol over stdin/stdout (REPL + pipe modes)
- SQLite storage with WAL mode
- Full-text search via FTS5
- Vector search via sqlite-vec (HNSW)
- Hybrid recall (vector + full-text weighted merge)
- Confidence decay (exponential, configurable rate)
- Manual consolidation (`CONSOLIDATE` command)
- Embedding gateway (HTTP to OpenAI / Ollama)
- Relevance feedback (access logging)
- CLI-like one-shot execution: `mnemo remember "..."`

### 3.2 Out of Scope (v0.1)

- Multi-agent access control (ACL)
- Cross-agent memory sharing
- Automatic entity extraction / knowledge graph
- On-device embedding model
- Chunk-entity hierarchy (flat memories only)
- Storage tiering (RAM/NVMe/object)
- Model versioning for embeddings (fixed model per instance)
- Distribution / replication / clustering
- WebSocket / HTTP server mode
- SQL interface (not `mnemo> SELECT ...`)

### 3.3 Future Releases (Roadmap)

| Version | Feature |
|---------|---------|
| **v0.2** | On-device embedding (FastEmbed/ONNX), chunk hierarchy, PRAGMA configs |
| **v0.3** | Agent skill auto-save (MCP / lifecycle hooks) |
| **v0.4** | Multi-agent mode, ACL, cross-agent memory sharing |
| **v0.5** | Memory broker, consolidation scheduling, conflict resolution |
| **v1.0** | Custom storage backend (native Rust/C instead of SQLite), server mode |

---

## 4. Architecture

### 4.1 Memory Model

Four memory types adapted from the design requirements:

| Type | Lifetime | Structure | Retrieval | Example | Storage |
|------|----------|-----------|-----------|---------|---------|
| **Working** | Seconds–minutes | Conversation buffer, ring buffer | Temporal (most recent) | "User just said they're tired" | RAM only |
| **Episodic** | Hours–months | Event + context + outcome | Similarity + temporal | "Last time user asked about X, they liked Y" | SQLite |
| **Semantic** | Months–permanent | Entity + attribute + value | Structured + semantic | "User prefers dark mode" | SQLite + HNSW |
| **Procedural** | Permanent | Task → steps + conditions | Task matching | "When asked about deployment, check staging" | **v0.2+** |

**Promotion flow:**

```
User message → Working buffer
   |
   v
CONSOLIDATE (manual / auto-trigger)
   |
   +-- working → episodic (summarize recent turns)
   |
   v
CONSOLIDATE (periodic / manual)
   |
   +-- episodic → semantic (extract persistent facts)
```

### 4.2 Component Diagram

```
                    ┌──────────────────┐
     stdin          │  Protocol Parser  │      stdout
    ─────→          │  (line-oriented) │     ─────→
                    └────────┬─────────┘
                             v
                    ┌──────────────────┐
                    │  Command Router   │
                    │   (dispatch)      │
                    └────────┬─────────┘
                             v
        ┌────────────────────┼────────────────────┐
        v                    v                    v
  ┌───────────┐      ┌───────────┐      ┌───────────┐
  │ Working   │      │ Episodic  │      │ Semantic  │
  │ Manager   │      │ Manager   │      │  Manager  │
  │ (in-mem)  │      │ (SQLite)  │      │ (SQLite   │
  └─────┬─────┘      └─────┬─────┘      │  + HNSW)  │
        │                  │            └─────┬─────┘
        v                  v                  v
  ┌───────────┐      ┌──────────────────────┐
  │ Ring      │      │    SQLite Core       │
  │ Buffer    │      │  (~3 backend files)  │
  └───────────┘      │  • memories table    │
                     │  • hnsw virtual tbl  │
                     │  • fts5 virtual tbl  │
                     └──────────────────────┘
```

### 4.3 Lifecycle Flow

```
┌───────────────┐     ┌───────────────┐     ┌───────────────┐
│   Working     │────→│   Episodic    │────→│   Semantic    │
│   (RAM)       │     │   (SQLite)    │     │   (SQLite    │
│               │     │               │     │   + HNSW)    │
│  CONSOLIDATE  │     │  CONSOLIDATE  │     │               │
│    trigger    │     │    trigger    │     │               │
└───────────────┘     └───────────────┘     └───────────────┘
       │                       │                     │
       v                       v                     v
┌─────────────────────────────────────────────────────────────┐
│                    Confidence Decay Engine                 │
│  • Every N hours: decay all episodic memories              │
│  • Every N*10 hours: decay semantic memories               │
│  • Remove if confidence < threshold                        │
└─────────────────────────────────────────────────────────────┘
```

### 4.4 Data Model (SQLite Schema)

```sql
-- Core memory unit
CREATE TABLE memories (
    id          TEXT PRIMARY KEY,           -- mem-{nanoid}
    memory_type TEXT NOT NULL CHECK (memory_type IN ('working', 'episodic', 'semantic')),
    content     TEXT NOT NULL,
    
    -- Temporal
    created_at  INTEGER NOT NULL,           -- unix epoch ms
    accessed_at INTEGER,
    expires_at  INTEGER,                    -- NULL = permanent
    
    -- Scoring
    confidence  REAL DEFAULT 1.0,         -- 0.0 to 1.0
    importance  REAL DEFAULT 0.5,         -- 0.0 to 1.0
    
    -- Provenance
    source_type TEXT,                     -- observation, inference, user_stated
    source_turn_id TEXT,
    
    -- Versioning
    version     INTEGER DEFAULT 1,
    superseded_by TEXT,                   -- linked-list of updates
    
    -- HNSW index availability flag
    is_indexed  INTEGER DEFAULT 0,        -- 0 if embedding pending, 1 when available
    
    -- Metadata
    tags        TEXT                      -- comma-separated
);

-- HNSW vector table via sqlite-vec
CREATE VIRTUAL TABLE memory_vectors USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[1536]
);

-- Relationships between memories
CREATE TABLE memory_links (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id   TEXT NOT NULL REFERENCES memories(id),
    target_id   TEXT NOT NULL REFERENCES memories(id),
    link_type   TEXT NOT NULL CHECK (link_type IN (
        'supports', 'contradicts', 'derived_from', 'supersedes', 'context_of'
    )),
    confidence  REAL DEFAULT 1.0,
    created_at  INTEGER NOT NULL
);

-- Access audit (relevance feedback)
CREATE TABLE memory_access_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    memory_id   TEXT NOT NULL REFERENCES memories(id),
    access_type TEXT NOT NULL CHECK (access_type IN ('read', 'write', 'forget')),
    query_text  TEXT,
    relevance   REAL,                       -- explicit feedback, NULL if implicit
    accessed_at INTEGER NOT NULL
);

-- FTS5 for full-text search
CREATE VIRTUAL TABLE memories_fts USING fts5(
    content, 
    content='memories', 
    content_rowid='rowid'
);

-- Configuration / metadata
CREATE TABLE _mnemo_meta (
    key     TEXT PRIMARY KEY,
    value   TEXT
);
```

### 4.5 Storage Layout

Each agent gets an isolated directory:

```
~/.mnemo/
├── {agent_id}/
│   ├── memory.db             -- Main SQLite database
│   ├── memory.db-wal         -- WAL journal (auto-created)
│   ├── memory.db-shm         -- Shared memory (auto-created)
│   └── config.toml             -- Per-agent overrides (optional)
└── global.toml                 -- Global defaults
```

**Default:** If no `--agent-id` is provided, mnemo falls back to `default` agent.

### 4.6 Confidence Decay Model (v0.1 Simplified)

```
current_confidence = base_confidence * boost * exp(-decay_rate * age_hours)

where:
  decay_rate = config.decay_episodic (default: 0.01/hr → half-life ~69 hrs)
  decay_rate = config.decay_semantic (default: 0.001/hr → half-life ~693 hrs)
  boost = 1.0 + 0.05 * min(reinforcement_count, 20)
  reinforcement_count = number of accesses with relevance > 0.7
```

**Garbage collection trigger:**
- When `confidence < config.min_confidence` (default: 0.1)
- When `expires_at < now` (for working/episodic with TTL)
- Run automatically every `config.gc_interval` (default: every 1 hour)

---

## 5. Text Protocol

### 5.1 Design Philosophy

- **Line-oriented** — easy to parse with `readline()`
- **Verb-noun structure** — `REMEMBER`, `RECALL`, `FORGET`, `CONSOLIDATE`, etc.
- **Block responses** — multi-line responses wrapped in XML-like tags for clear delimiters
- **No SQL** — agents don't need to know SQL, but humans can inspect `.db` with `sqlite3`

### 5.2 Command Syntax

```
<command> [<args>...] [<options>...];
```

Commands end with a semicolon (like SQL CLIs). Each command produces a response block.

### 5.3 Command Reference

#### INIT
Initialize a memory database for this agent.

```
INIT;
```

**Response:**
```
<ok>
Database initialized at ~/.mnemo/{agent_id}/memory.db
</ok>
```

---

#### REMEMBER
Store a new memory.

```
REMEMBER "User prefers dark mode and hates popups"
  AS semantic
  WITH importance=0.8, source=user_stated, tags="ui,preferences"
```

**Fields:**
- `AS <memory_type>` — `working`, `episodic`, `semantic`
- `WITH <key>=<value>, ...` — Optional metadata (importance, source, tags)

**Response (immediate text persistence):**
```
<memory id="mem-abc123" status="text-indexed">
  Text indexed (embedding queued)
</memory>
```

**Response (after embedding completes):**
```
<memory id="mem-abc123" status="vector-indexed">
  Text + vector indexed
</memory>
```

(Note: In v0.1, the agent can choose to block for embedding with `--sync` flag.)

---

#### RECALL
Retrieve memories matching a query.

```
RECALL "user preferences about UI"
  FROM semantic, episodic
  WHERE confidence > 0.5
  ORDER BY score DESC
  LIMIT 5;
```

**Fields:**
- `FROM <type1>, <type2>` — Optional filter by types. Default: all.
- `WHERE <conditions>` — Optional filters: confidence, importance, created_at, tags.
- `ORDER BY score|confidence|importance|created_at` — Optional ordering.
- `LIMIT N` — Optional limit. Default: 10.

**Response:**
```
<result count="2">
  <memory id="mem-abc123" type="semantic" confidence="0.95" importance="0.80" score="0.87">
    User prefers dark mode and hates popups
  </memory>
  <memory id="mem-def456" type="episodic" confidence="0.60" importance="0.50" score="0.72">
    User dismissed notification dialog quickly
  </memory>
</result>
```

(Scores are normalized [0,1]. Hybrid score = 0.6 * vector_score + 0.4 * bm25_score.)

---

#### FORGET
Delete memories by filter.

```
FORGET WHERE memory_type = 'working' AND created_at < ago('1 hour');
FORGET id('mem-abc123');
```

**Response:**
```
<ok>
Deleted 12 memories
</ok>
```

---

#### CONSOLIDATE
Promote memories up the hierarchy.

```
CONSOLIDATE WORKING TO EPISODIC;
CONSOLIDATE EPISODIC TO SEMANTIC WHERE tags LIKE '%deployment%'
```

**Behavior:**
- `WORKING → EPISODIC`: Summarize recent working memories into episodic events
- `EPISODIC → SEMANTIC`: Extract persistent facts from episodes

**Response:**
```
<ok>
Consolidated 3 working memories into 1 episodic memory
New memory id: mem-ghi789
</ok>
```

---

#### REFLECT
Analyze memory state.

```
REFLECT;
REFLECT ON SEMANTIC WHERE confidence < 0.5;
```

**Response:**
```
<analysis>
Memory count: 120 episodic, 45 semantic
Low-confidence items: 12
Potential contradictions: 2
  - mem-abc123: "User prefers dark mode"
  - mem-def456: "User prefers light mode"
Stale memories (>30 days): 8
</analysis>
```

---

#### STATUS
Print database statistics.

```
STATUS;
```

**Response:**
```
<status>
Agent: default
Database: ~/.mnemo/default/memory.db (2.4 MB)
Working buffer: 8 items (max 100)
Episodic memories: 120
Semantic memories: 45
Full-text indexed: 165
Vector indexed: 45 (3 pending embedding)
Last consolidation: 2026-05-01 14:22:00 UTC
Last GC: 2026-05-01 13:00:00 UTC
</status>
```

---

#### PRAGMA
Set or get configuration.

```
PRAGMA embedding_provider = 'ollama';
PRAGMA embedding_model = 'nomic-embed-text';
PRAGMA decay_episodic = 0.01;
PRAGMA decay_semantic = 0.001;
PRAGMA min_confidence = 0.1;
PRAGMA gc_interval = 3600;
PRAGMA working_buffer_size = 100;
PRAGMA;  -- list all
```

**Response:**
```
<config>
embedding_provider = ollama
embedding_model = nomic-embed-text
decay_episodic = 0.01
decay_semantic = 0.001
min_confidence = 0.1
gc_interval = 3600
working_buffer_size = 100
</config>
```

---

### 5.4 Error Protocol

All errors use the same block format:

```
<error code="NO_MATCH">
  No memories matched the query
</error>
```

**Common error codes:**

| Code | Meaning |
|------|---------|
| `INVALID_TYPE` | Unknown memory type |
| `INVALID_SYNTAX` | Malformed command |
| `NO_MATCH` | Query returned no results |
| `EMBED_TIMEOUT` | Embedding provider didn't respond |
| `EMBED_ERROR` | Embedding provider returned an error |
| `DB_ERROR` | SQLite internal error |
| `NOT_FOUND` | Memory ID doesn't exist |
| `CONFIG_ERROR` | Invalid PRAGMA value |

### 5.5 One-Shot Mode

For one-shot execution (like a CLI tool):

```bash
$ mnemo remember "User prefers blue theme" --type=semantic --importance=0.9
<memory id="mem-xyz789" status="text-indexed">
  Text indexed (embedding queued)
</memory>

$ mnemo recall "blue theme" --limit=1
<result count="1">
  <memory id="mem-xyz789" type="semantic" confidence="0.95" importance="0.90" score="0.91">
    User prefers blue theme
  </memory>
</result>

$ mnemo status
<status>
  Agent: default
  Database: ~/.mnemo/default/memory.db (2.4 MB)
  ...
</status>
```

One-shot mode is the default. REPL mode is triggered with `mnemo --repl` or when input is a TTY.

---

## 6. Embedding Model Strategy

### 6.1 v0.1: External Gateway

No neural models shipped in the binary. The binary makes HTTP requests to configurable providers.

**Configuration:**
```
PRAGMA embedding_provider = 'ollama';   -- or 'openai', 'custom'
PRAGMA embedding_model = 'nomic-embed-text';
PRAGMA embedding_endpoint = 'http://localhost:11434/api/embeddings';
PRAGMA embedding_timeout = 30;
PRAGMA embedding_dimensions = 768;      -- must match vec0 declaration
```

**Two-phase indexing:**

```
REMEMBER "some text"
  |
  +--> Sync phase (2ms)
  |     1. Write text to `memories` table
  |     2. Write text to FTS5 index
  |     3. Set `is_indexed = 0`
  |     4. Queue embedding task
  |     5. Return `<status="text-indexed">`
  |
  +--> Async phase (50-500ms)
        1. Call embedding API
        2. On success: write to `memory_vectors` table, set `is_indexed = 1`
        3. On failure: retry N times, then mark `is_indexed = -1` (error)
```

**Search fallback:** If `is_indexed = 0`, the memory is still discoverable via full-text search and structured filters, just not vector recall.

### 6.2 Why Not Ship a Model in v0.1?

- Binary size: FastEmbed models start at 20MB, ONNX runtime adds another 10-20MB. This makes `curl | tar` deployment unattractive.
- Many agents already run Ollama locally for LLM inference — reusing it for embeddings is natural.
- Proving the protocol and data model is higher priority than optimizing latency.

### 6.3 v0.2+: On-Device Embedding

Once the design is proven, embed a quantized ONNX model (via `ort` crate in Rust) directly into the binary. This makes mnemo fully offline.

---

## 7. Implementation Language

### 7.1 Decision: Rust

**Why Rust over C/C++:**

| Factor | Rust | C/C++ |
|--------|------|-------|
| Binary size | ~3MB (stripped) | ~1MB but more custom code |
| Safety | Memory safety guaranteed | Manual; segfaults are common |
| SQLite bindings | `rusqlite` mature, with feature flags | `sqlite3.c` but more plumbing |
| Async HTTP | `reqwest` / `tokio` excellent | libcurl / custom HTTP |
| Cross-compilation | `cross` tool works well | Harder for multiple targets |
| Static linking | Easy with musl | Requires musl toolchain |
| Community | Smaller but growing for CLI tools | Larger but fragmented |

**Specific crates (Rust):**
- `rusqlite` — SQLite bindings (bundled, no system SQLite needed)
- `sqlite-vec` — via `rusqlite` extension, or FFI if needed
- `tokio` — Async runtime (embedding gateway, background tasks)
- `serde` / `toml` — Config parsing and protocol serialization
- `clap` — `mnemo` CLI argument parsing
- `reqwest` — HTTP to OpenAI/Ollama
- `nanoid` — Memory IDs

**Build targets:**
- Linux x86_64, aarch64 (musl static)
- macOS x86_64, aarch64
- Windows x86_64, aarch64 (v0.2+)

---

## 8. Error Handling & Recovery

### 8.1 Principles

- **No panics** — all crashes caught at top-level, returned as `<error>` block
- **WAL durability** — If mmap WAL mode, commit is fsynced on every command
- **Graceful embedding failure** — If Ollama/OpenAI is down, store text anyway, retry later
- **Corruption detection** — On startup, run `PRAGMA integrity_check;`. Repair or warn.

### 8.2 Recovery Scenarios

| Scenario | Behavior |
|----------|----------|
| Binary crash mid-command | WAL replay on next startup restores committed state |
| Embedding provider down | Memories stored as text-only, retry queue persists in SQLite |
| Invalid `.db` file | Warn, suggest `mnemo init --force` or manual `sqlite3` |
| Disk full | Return `<error code="DB_ERROR">`, pause GC, don't corrupt |

---

## 9. Testing Strategy

### 9.1 Test Levels

| Level | Tool | What |
|-------|------|------|
| **Unit** | `cargo test` | Protocol parser, tier manager, decay math, command dispatch |
| **Integration** | `assert_cmd` + `predicates` | Spawn `mnemo` subprocess, pipe commands, assert output |
| **Property** | `proptest` | Round-trip: `REMEMBER` → `RECALL` must find memory |
| **Bench** | `criterion` | RECALL latency vs. memory count |

### 9.2 Key Test Cases

- Parser: Every command with every argument combination
- Parser: Malformed commands (missing semicolons, unclosed quotes)
- Working buffer: Fill to capacity, overflow wraps correctly
- Decay: Manually set `created_at` to past, verify `REFLECT` marks as stale
- Consolidation: `REMEMBER` 5 working → `CONSOLIDATE` → verify 1 episodic exists
- Embedding fallback: Block Ollama port → verify text-only recall still works

### 9.3 CI Pipeline

```yaml
# GitHub Actions
# Build on Linux/macOS/Windows
# Run unit + integration on every PR
# Run benchmark on schedule (weekly against main)
```

---

## 10. Build, Package, & Distribution

### 10.1 Source Build

```bash
git clone https://github.com/mnemonics/mnemo.git
cd mnemo
cargo build --release
# Binary at: target/release/mnemo
```

### 10.2 Binary Distribution

Goal: `curl -sSL https://get.mnemo.dev | sh` installs `mnemo` in one line.

| Platform | Package |
|----------|---------|
| Linux | `mnemo-linux-x86_64.tar.gz` (static, musl) |
| macOS | `mnemo-darwin-aarch64.tar.gz` (M1/M2), `mnemo-darwin-x86_64.tar.gz` |
| Homebrew | `brew install mnemo` |
| Cargo | `cargo install mnemo` |

### 10.3 Versioning

Follow **SemVer**:
- `0.1.0` — MVP working, episodic, semantic
- `0.2.0` — On-device embeddings, chunk hierarchy
- `0.3.0` — Auto-save skills, MCP integration
- `1.0.0` — stable API, multi-agent, proven at scale

---

## 11. Skill / Auto-Save Integration (v0.3+)

The long-term vision: AI agents automatically persist memory without explicit commands.

### 11.1 MCP (Model Context Protocol) Integration

When mnemo runs as an MCP server (`mnemo --mcp`):

```
Agent (Claude Code / OpenCode)
  |
  +-- MCP tool: `save_memories`
      +-- mnemo REMEMBER ... (auto-called on session end / pre-compact)
  +-- MCP tool: `recall_memories`
      +-- mnemo RECALL ... (auto-called at session start / on context window pressure)
```

### 11.2 Lifecycle Hooks (Language Agnostic)

Agents call `mnemo` at key lifecycle events:

| Event | mnemo command | Purpose |
|-------|-------------|---------|
| Session start | `RECALL` | Load relevant memories into context |
| Pre-compact | `REMEMBER` + `CONSOLIDATE` | Persist before LLM summarizes away context |
| User prompt | `RECALL` | Fetch memories relevant to current task |
| Task complete | `REMEMBER` + `CONSOLIDATE` | Save outcomes, update success/failure patterns |
| Session end | `FORGET working` | Clear temporary working state |

---

## 12. Open Questions & Assumptions

1. **sqlite-vec license & compatibility** — Must verify it ships with `rusqlite` and doesn't require a separate compilation step that breaks static linking.
2. **Embedding dimensions** — Default `1536` (OpenAI `text-embedding-3-small`). Configurable via `PRAGMA`. HNSW virtual table must be recreated if dimension changes.
3. **Consolidation LLM call** — v0.1 manual consolidation only. `CONSOLIDATE` is a command, not automatic. v0.2 may use an LLM call to do summarization.
4. **Working memory eviction** — When working buffer overflows, oldest entries are silently dropped. No automatic promotion to episodic in v0.1.
5. **No multi-thread access** — Single `mnemo` process per agent database. Don't run two `mnemo --repl` processes on the same `.db`.
6. **Security** — No encryption at rest in v0.1. Agent data is in `~/.mnemo/`. If operating system disk encryption is off, data is plaintext. v0.4 may add SQLCipher.

---

## 13. Comparison with Existing Systems

| Capability | Letta | Mem0 | mnemo v0.1 |
|---|---|---|---|
| Single binary | ✗ | ✗ | **✓** |
| Zero external deps | ✗ | ✗ | **✓** |
| Memory tiers | ✓ (core/recall/archival) | Working/episodic implied | **✓ (working/episodic/semantic)** |
| In-DB vector search | ✗ (pgvector) | ✓ (Qdrant/Pinecone) | **✓ (sqlite-vec)** |
| Full-text search | ✗ | ✗ | **✓ (FTS5)** |
| Hybrid recall | ✗ | partial | **✓** |
| Confidence decay | ✗ | ✗ | **✓** |
| Memory consolidation | partial | ✗ | **✓** |
| Automatic lifecycle | ✗ | ✗ | **✓** |
| Multi-agent ACL | ✗ | ✗ | v0.4 |
| Entity extraction | ✗ | ✓ | v0.2 |
| Knowledge graph | ✗ | ✓ | v0.2 |
| Chunk hierarchy | ✗ | partial | v0.2 |
| Model versioning | ✗ | ✗ | v0.2 |
| On-device embedding | N/A | N/A | v0.2 |

---

## 14. Appendix: Quick-Start Example

```bash
# 1. Download
$ curl -fsSL https://get.mnemo.dev | sh
$ mnemo --version
mnemo 0.1.0

# 2. Start REPL for your agent
$ mnemo --agent-id=my-claude-agent --repl
mnemo> INIT;
<ok> Database initialized at ~/.mnemo/my-claude-agent/memory.db </ok>

# 3. Remember something
mnemo> REMEMBER "User prefers dark mode in all applications"
  WITH importance=0.9, source=user_stated, tags="ui,theme";
<memory id="mem-abc123" status="text-indexed">
  Text indexed (embedding queued)
</memory>

# 4. Recall later
mnemo> RECALL "dark mode" FROM semantic;
<result count="1">
  <memory id="mem-abc123" type="semantic" confidence="0.95" importance="0.90" score="0.92">
    User prefers dark mode in all applications
  </memory>
</result>

# 5. Consolidate after a conversation
mnemo> CONSOLIDATE WORKING TO EPISODIC;
<ok> Consolidated 5 working memories into 1 episodic memory </ok>

# 6. Check health
mnemo> STATUS;
<status>
  Agent: my-claude-agent
  Database: ~/.mnemo/my-claude-agent/memory.db (1.2 MB)
  Working buffer: 0 items
  Episodic memories: 12
  Semantic memories: 45
  ...
</status>

# 7. Inspect with sqlite3 (for power users)
$ sqlite3 ~/.mnemo/my-claude-agent/memory.db
sqlite> SELECT id, content FROM memories WHERE memory_type = 'semantic';
```

---

## 15. Glossary

| Term | Definition |
|------|------------|
| **Working memory** | Short-term, ephemeral conversation buffer stored in RAM |
| **Episodic memory** | Event-based storage of past interactions (time-stamped, decaying) |
| **Semantic memory** | Persistent facts, preferences, and knowledge (long-term) |
| **Procedural memory** | Task patterns and learned procedures (v0.2+) |
| **Consolidation** | Promotion of memories from a volatile to a stable tier |
| **Confidence decay** | Reduction in confidence score over time until forgetting |
| **HNSW** | Hierarchical Navigable Small World — graph-based approximate nearest neighbor index |
| **FTS5** | Full-Text Search v5 — built-in SQLite text indexing virtual table |
| **WAL** | Write-Ahead Logging — SQLite's crash-recovery mechanism |
| **PRAGMA** | SQLite configuration directive, repurposed for mnemo settings |
