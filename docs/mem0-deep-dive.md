# Mem0 — Deep Dive Study Guide

> Architecture overview for developers studying the mem0 intelligent memory layer codebase.

## Table of Contents

1. [What is Mem0](#what-is-mem0)
2. [Repository Structure](#repository-structure)
3. [Two Usage Modes](#two-usage-modes)
4. [Core Python SDK — Memory Engine](#core-python-sdk--memory-engine)
5. [Core Python SDK — Provider Architecture](#core-python-sdk--provider-architecture)
6. [Core Python SDK — Hybrid Search](#core-python-sdk--hybrid-search)
7. [Core Python SDK — Entity Store](#core-python-sdk--entity-store)
8. [Core Python SDK — Hosted Client](#core-python-sdk--hosted-client)
9. [Core Python SDK — OpenAI Proxy](#core-python-sdk--openai-proxy)
10. [TypeScript SDK](#typescript-sdk)
11. [Server — Self-Hosted REST API](#server--self-hosted-rest-api)
12. [OpenMemory — Self-Hosted Platform](#openmemory--self-hosted-platform)
13. [CLIs — Python & Node](#clis--python--node)
14. [Plugin System — MCP & AI Editors](#plugin-system--mcp--ai-editors)
15. [Evaluation Framework](#evaluation-framework)
16. [Key Architectural Patterns](#key-architectural-patterns)
17. [Data Flow: Complete Add Pipeline](#data-flow-complete-add-pipeline)
18. [Data Flow: Complete Search Pipeline](#data-flow-complete-search-pipeline)

---

## What is Mem0

Mem0 ("mem-zero") is an **intelligent memory layer** for AI agents and assistants. It provides persistent, personalized memory that survives across conversations and sessions. Think of it as a database that doesn't just store facts — it extracts, deduplicates, links entities, and retrieves them with hybrid scoring.

**Core problem it solves:** LLMs are stateless. Every conversation starts from scratch. Mem0 gives AI applications a way to remember user preferences, past interactions, extracted facts, and entity relationships — across sessions, users, and agents.

**Key capabilities:**
- Extract structured memories from unstructured conversations
- Deduplicate and merge overlapping memories automatically
- Link named entities to memories for relationship-aware retrieval
- Hybrid search (semantic + keyword + entity boosting)
- History tracking for all memory mutations
- Works self-hosted or as a hosted cloud service

---

## Repository Structure

```
mem0/                    # Core Python SDK (mem0ai on PyPI)
├── memory/              #   Core engine: add/search/delete pipeline
├── llms/                 #   24 LLM providers
├── embeddings/           #   15 embedding providers
├── vector_stores/        #   30 vector store providers
├── reranker/             #   5 reranker providers
├── graphs/               #   4 graph store providers
├── configs/              #   Configuration & prompt templates
├── client/               #   Hosted platform API client
├── utils/                 #   Entity extraction, scoring, factories
├── proxy/                 #   OpenAI-compatible proxy layer
├── exceptions.py         #   Structured exception hierarchy

mem0-ts/                 # TypeScript SDK (mem0ai on npm)
├── src/client/           #   Hosted platform client
├── src/oss/              #   Self-hosted OSS memory
│   ├── src/llms/         #     LLM providers
│   ├── src/embeddings/   #     Embedding providers
│   ├── src/vector_stores/#     Vector store providers
│   ├── src/config/       #     Configuration
│   ├── src/prompts/      #     Prompt templates
│   ├── src/storage/     #     SQLite storage
│   └── src/utils/       #     Factory pattern

server/                  # Self-hosted REST API
├── main.py              #   FastAPI app
├── auth.py              #   JWT + API key auth
├── routers/             #   API route handlers
├── docker-compose.yaml  #   PostgreSQL + Neo4j + API
└── dashboard/           #   Next.js admin UI

openmemory/              # Self-hosted memory platform
├── api/                 #   FastAPI + Alembic + MCP server
│   ├── app/mcp_server.py#     MCP protocol server
│   └── app/routers/     #     memories, apps, stats, config
└── ui/                  #   Next.js 15 + React 19 frontend

cli/python/              # Python CLI (mem0-cli on PyPI)
cli/node/                # Node CLI (@mem0/cli on npm)
mem0-plugin/             # AI editor plugins (Claude Code, Cursor, Codex)
evaluation/              # Benchmarking framework (LOCOMO evals)
embedchain/              # Legacy RAG framework (separate, skip)
docs/                    # Mintlify documentation site
```

---

## Two Usage Modes

Mem0 operates in **two fundamentally different modes**, both in Python and TypeScript:

### Self-Hosted: `Memory` / `AsyncMemory`

```
Your App → Memory.add(messages) → Local LLM + Embedder + Vector Store
```

- Runs entirely on your infrastructure
- You choose LLM provider, embedder, vector store, optional graph store and reranker
- All data stays local (SQLite for history, vector store for memories)
- `Memory` is synchronous; `AsyncMemory` wraps blocking I/O with `asyncio.to_thread()`

### Hosted Platform: `MemoryClient` / `AsyncMemoryClient`

```
Your App → MemoryClient.add(messages) → HTTPS → api.mem0.ai
```

- Thin HTTP client to mem0's cloud service
- Authentication via `MEM0_API_KEY`
- Same API surface (add, search, get, update, delete, history)
- Additional features: webhooks, projects, feedback, summaries, memory exports
- Auto-validates API key on initialization with `/v1/ping/`

**Both modes share the same conceptual API** — `add()`, `search()`, `get()`, `get_all()`, `update()`, `delete()`, `delete_all()`, `history()`.

---

## Core Python SDK — Memory Engine

The heart of mem0. Located in `mem0/memory/main.py` (2895+ lines).

### Key Classes

| Class | Lines | Purpose |
|-------|-------|---------|
| `Memory` | 331–1793 | Synchronous self-hosted memory |
| `AsyncMemory` | 1795–2895+ | Async mirror using `asyncio.to_thread()` |
| `MemoryBase` | `memory/base.py:4-63` | Abstract base with `get()`, `get_all()`, `update()`, `delete()`, `history()` |

### Initialization Flow

```
Memory(config) →
  EmbedderFactory.create()     # e.g., OpenAI embedder
  VectorStoreFactory.create()   # e.g., Qdrant vector store
  LlmFactory.create()          # e.g., OpenAI LLM
  SQLiteManager(history_db_path) # SQLite for history + messages
  [RerankerFactory.create()]   # optional reranker
  [entity_store]               # lazy-initialized on first access
```

The `entity_store` is lazily created on first access — it spins up a **second vector store collection** (`{collection_name}_entities`) to store extracted named entities. For Qdrant, it shares the same client instance to avoid RocksDB lock contention.

### Configuration

`MemoryConfig` (in `configs/base.py`) is a Pydantic BaseModel:

| Field | Default | Purpose |
|-------|---------|---------|
| `vector_store` | Qdrant | Vector database configuration |
| `llm` | OpenAI | LLM provider configuration |
| `embedder` | OpenAI | Embedding provider configuration |
| `history_db_path` | `~/.mem0/history.db` | SQLite path |
| `reranker` | None | Optional reranker |
| `version` | "v1.1" | Pipeline version |
| `custom_instructions` | None | Extra extraction instructions |

### SQLite Storage (`memory/storage.py`)

`SQLiteManager` manages two tables:

- **`history`**: Tracks all memory mutations (adds, updates, deletes) with `actor_id`, `role`, `is_deleted` flag, `old_memory`/`new_memory` for diffs, and timestamps
- **`messages`**: Stores last 10 messages per `session_scope` (auto-evicted). Used as context for new memory extraction

Thread-safe via `threading.Lock()`. All inserts through `batch_add_history()` for bulk efficiency.

### Prompt Templates (`configs/prompts.py`)

The most critical file for understanding mem0's behavior — 1000+ lines of prompts:

| Prompt | Purpose |
|--------|---------|
| `ADDITIVE_EXTRACTION_PROMPT` | **V3 core prompt** — ADD-only extraction with memory linking. UUID-to-int mapping to prevent hallucination |
| `FACT_RETRIEVAL_PROMPT` | Legacy v1 extraction prompt (deprecated) |
| `USER_MEMORY_EXTRACTION_PROMPT` | Extracts facts from user messages only |
| `AGENT_MEMORY_EXTRACTION_PROMPT` | Extracts facts from assistant messages only |
| `DEFAULT_UPDATE_MEMORY_PROMPT` | ADD/UPDATE/DELETE/NONE decision prompt for existing memory updates |
| `PROCEDURAL_MEMORY_SYSTEM_PROMPT` | For agent procedural memory creation |
| `generate_additive_extraction_prompt()` | Dynamic prompt builder: injects existing memories, new messages, last k messages, custom instructions |

The **V3 Additive approach** is a key design decision: instead of asking the LLM to decide whether to ADD, UPDATE, or DELETE (which caused errors), the extraction prompt only produces ADD operations. Deduplication and conflict resolution happen via hash comparison and a separate update decision step.

---

## Core Python SDK — Provider Architecture

Mem0 uses a **plugin architecture** across 5 provider categories. Each follows the same pattern:

```
base.py         → Abstract class defining the interface
<provider>.py   → Concrete implementation
configs.py      → Pydantic configuration models (optional)
__init__.py     → Registration
utils/factory.py → Factory with provider_name → class mapping
```

### LLM Providers (24)

Base class: `llms/base.py:7` — `LLMBase`
- `generate_response(prompt, messages)` — abstract
- `_is_reasoning_model()` — detects o1/o3 models
- `_get_supported_params()` — model-specific parameter allowlists

**Providers:** OpenAI, Anthropic, AWS Bedrock, Azure OpenAI, Gemini, Groq, Ollama, Together, DeepSeek, vLLM, LiteLLM, LM Studio, xAI, and more.

### Embedding Providers (15)

Base class: `embeddings/base.py:7` — `EmbeddingBase`
- `embed(text)` — abstract, returns embedding vector
- `embed_batch(texts)` — default: sequential fallback calling `embed()` one-by-one. Providers override with true batch API calls

**Providers:** OpenAI, Azure OpenAI, Gemini, HuggingFace, FastEmbed, Together, AWS Bedrock, Ollama, Vertex AI, and more.

**Special case:** `upstash_vector` with `enable_embeddings` returns `MockEmbeddings` — the vector store handles embedding internally.

### Vector Store Providers (30)

Base class: `vector_stores/base.py:4` — `VectorStoreBase`
- `create_col()` — create collection
- `insert()` — insert vectors
- `search()` — similarity search
- `delete()`, `update()`, `get()`, `list()` — CRUD
- `keyword_search()` — optional BM25 support
- `search_batch()` — optional batch search

**Providers:** Qdrant, Pinecone, Chroma, Weaviate, Milvus, MongoDB, Redis, Elasticsearch, pgvector, Supabase, Faiss, S3 Vectors, and more.

### Graph Store Providers (4)

For relationship-aware retrieval configured via `graph` section of `MemoryConfig`.

**Providers:** Neo4j, Memgraph, Kuzu, Apache AGE

### Reranker Providers (5)

Base class: `reranker/base.py:4` — `BaseReranker`
- `rerank(query, documents)` — re-ranks results

**Providers:** Cohere, HuggingFace, LLM-based, Sentence Transformer, Zero Entropy

### Factory Pattern (`utils/factory.py`)

All factories (`LlmFactory`, `EmbedderFactory`, `VectorStoreFactory`, `RerankerFactory`) share this structure:
- Map `provider_name` → `(module_path, class_name, config_class)`
- `create()` handles `None` → default config, `dict` → config class instantiation, or passthrough config object
- Uses `importlib.import_module()` for dynamic loading
- `VectorStoreFactory.create()` uses `model_dump()` to serialize config into kwargs

---

## Core Python SDK — Hybrid Search

Mem0's search (`_search_vector_store()`) combines three signals for retrieval:

### The Three Signals

```
Final Score = (semantic_score + bm25_score + entity_boost) / max_possible
```

1. **Semantic Search**: Embed the query → cosine similarity in vector store. Over-fetch at 4x the requested limit for better recall.

2. **Keyword Search (BM25)**: If the vector store supports `keyword_search()` (e.g., Qdrant), run BM25 keyword search. Scores normalized via **query-length-adaptive sigmoid**:
   - `normalize_bm25()` uses logistic sigmoid: `1 / (1 + exp(-k * (x - x0)))`
   - Parameters `k` and `x0` adapt based on query word count (shorter queries need different normalization)
   - Maps raw BM25 scores (unbounded) to [0, 1]

3. **Entity Boost**: If the query contains entities that are also linked to memories in the entity store, boost those memories' scores. Uses **spread-attenuated boosting** — an entity linked to many memories gets a smaller per-memory boost (avoiding popular entity dominance).

### Scoring Algorithm (`utils/scoring.py`)

```
score_and_rank(candidates, limit, threshold):
  1. If semantic_score < threshold → skip candidate
  2. For each candidate:
     - Add semantic score (normalized)
     - Add BM25 score (if available)
     - Add entity boost (if available)
  3. Divide by max_possible = number of non-zero signals
  4. Sort descending, return top `limit`
```

The **threshold gate** is important: if semantic similarity is below a minimum, the candidate is discarded entirely regardless of BM25 or entity signals. This prevents irrelevant keyword matches from dominating.

### Advanced Metadata Filters

`search()` supports a rich filter language:
- **Logical operators**: AND, OR, NOT
- **Comparison**: eq, ne, gt, gte, lt, lte, in, not_in
- **String**: contains, not_contains, wildcard
- Filters are translated into vector-store-specific query syntax

---

## Core Python SDK — Entity Store

The entity store is mem0's **relationship layer on top of vector memory**.

### What It Does

When memories are added, mem0 extracts named entities from the text using spaCy NER and stores them in a **second vector store collection** (`{collection_name}_entities`).

### Entity Types (4)

| Type | What it captures | Example |
|------|------------------|---------|
| PROPER | Proper noun sequences | "John Smith", "New York" |
| QUOTED | Single/double quoted text | 'project "Phoenix"' |
| COMPOUND | Noun compounds | "machine learning model" |
| NOUN | Single nouns (fallback) | "database", "project" |

Extraction happens in `utils/entity_extraction.py` using spaCy's `nlp.pipe()` for batch processing.

### How Entities Link to Memories

Each entity record contains:
- `entity_id`: Unique identifier
- `entity_type`: PROPER / QUOTED / COMPOUND / NOUN
- `entity_name`: The extracted text
- `linked_memory_ids`: Set of memory IDs this entity appears in
- `embedding`: Vector embedding of the entity name

When a memory is added:
1. Extract entities from memory text
2. Embed entity names via `embed_batch()`
3. Search entity store for existing matches (by embedding similarity)
4. If match found → update `linked_memory_ids` to include new memory
5. If no match → insert new entity with the new memory linked

When a memory is deleted:
1. Remove the memory ID from all linked entities
2. If an entity has no remaining links → optionally clean up

### Entity Boosting in Search

When `search()` runs:
1. Preprocess the query (lemmatize, extract entities)
2. For each extracted query entity, search the entity store
3. For each matching entity, get its `linked_memory_ids`
4. Boost the score of those memories in the final ranking
5. Spread-attenuation: if an entity links to N memories, each boost = `base_boost / N`

This means "popular" entities (linked to many memories) don't dominate results — each individual memory gets a smaller boost.

---

## Core Python SDK — Hosted Client

`mem0/client/main.py` — The `MemoryClient` and `AsyncMemoryClient` classes.

### Architecture

Thin HTTP client using `httpx` (sync) or `httpx.AsyncClient` (async). All operations are REST API calls to `api.mem0.ai`.

### Authentication

- `MEM0_API_KEY` environment variable or constructor argument
- Auto-validates on init by calling `GET /v1/ping/`
- Sends key as `Authorization: Token <key>` header

### API Surface

Beyond the shared memory operations, the client exposes platform-specific features:

| Method | Purpose |
|--------|---------|
| `users()` | List users in the project |
| `feedback()` | POSITIVE / NEGATIVE / VERY_NEGATIVE feedback on memories |
| `create_memory_export()` / `get_memory_export()` | Bulk memory export |
| `get_summary()` | AI-generated memory summary |
| Webhooks CRUD | Create/list/update/delete webhooks for events |
| Project management | `get_project()`, `update_project()` |

### Type System

Typed options classes in `client/types.py`:
- `AddMemoryOptions`, `SearchMemoryOptions`, `GetMemoryOptions`
- Enforce required fields (user_id, agent_id, or run_id)
- `_prepare_params()` strips None values for clean API calls

---

## Core Python SDK — OpenAI Proxy

`mem0/proxy/main.py` — A drop-in OpenAI-compatible layer.

### What It Does

Wraps either `Memory` or `MemoryClient` and intercepts LLM calls:

```
User → Chat.Completions.create() → Mem0 Proxy
  → Fetch relevant memories from Mem0
  → Inject memories into LLM prompt
  → Call LLM via litellm.completion()
  → Fire-and-forget add to Mem0 (daemon thread)
  → Return LLM response
```

### Memory Injection

`_fetch_relevant_memories()` searches the last 6 messages as queries, then `_format_query_with_memories()` injects a block like:

```
Relevant Memories/Facts:
- User prefers dark mode
- User's name is Alice

Entities:
- Alice, dark_mode
```

This context is prepended before the user's actual question.

### Fire-and-Forget Storage

`_async_add_to_memory()` runs in a daemon thread — it stores the conversation turn to mem0 without blocking the LLM response. This means the first message of a conversation won't have memories, but subsequent messages will.

---

## TypeScript SDK

`mem0-ts/` — Parallel implementation in TypeScript (version 3.0.2).

### Package Structure

Two entry points via `package.json` exports:
- `"."` → Hosted client (`MemoryClient`)
- `"./oss"` → Self-hosted memory (`Memory`)

### Hosted Client (`src/client/mem0.ts`)

697 lines. Mirrors the Python `MemoryClient`:
- Uses `fetch` API + `axios` for HTTP
- Snake_case ↔ camelCase conversion via `snakeToCamelKeys()` / `camelToSnakeKeys()`
- Same methods: `add()`, `getAll()`, `search()`, `update()`, `delete()`, `deleteAll()`, `history()`
- Project management, webhooks, feedback, memory exports

### Self-Hosted OSS Memory (`src/oss/`)

Port of the Python `Memory` class:
- Providers: OpenAI, Anthropic, Google, Groq, Ollama, LMStudio, Mistral, LangChain (LLMs)
- Embeddings: OpenAI, Ollama, LMStudio, Google, Azure, LangChain
- Vector stores: Memory (in-memory), Qdrant, Redis, Supabase, LangChain, Vectorize, Azure AI Search, pgvector
- Same extraction → dedup → entity linking pipeline

### Key Differences from Python

- No `AsyncMemory` equivalent (TypeScript's async/await model doesn't need the same separation)
- Fewer provider implementations (8 vector stores vs 30 in Python)
- Built with `tsup` (CJS + ESM dual output)
- Tests use `jest` for hosted client, separate test configs for OSS

---

## Server — Self-Hosted REST API

`server/` — A FastAPI application providing REST access to the self-hosted `Memory` class.

### Docker Compose Stack (3 services)

| Service | Image | Port | Purpose |
|---------|-------|------|---------|
| `mem0` | Custom (FastAPI) | 8888 | API server with auto-reload |
| `postgres` | `ankane/pgvector:v0.5.1` | 8432 | Vector storage with pgvector |
| `neo4j` | Neo4j 5.x + APOC | 8474/8687 | Graph store |

### Authentication (3 methods)

1. **JWT tokens** — via `/auth/login` endpoint, validated on each request
2. **API keys** — `X-API-Key` header, per-user keys stored in DB
3. **Admin key** — `ADMIN_API_KEY` env var, bypasses all checks

Set `AUTH_DISABLED=true` for development.

### REST Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/memories` | POST | Add new memory |
| `/memories` | GET | List memories (with filters) |
| `/search` | POST | Search memories |
| `/memories/{id}` | PUT | Update memory |
| `/memories/{id}` | DELETE | Delete memory |
| `/memories/{id}/history` | GET | Get change history |
| `/reset` | POST | Reset all data |
| `/configure` | GET/POST | Get/set configuration |

### Request Logging Middleware

Every request is logged to the DB: method, path, status_code, latency_ms, auth_type. Useful for usage analytics and debugging.

### Dashboard

Next.js admin UI in `server/dashboard/` for managing memories, viewing logs, and configuration.

---

## OpenMemory — Self-Hosted Platform

`openmemory/` — A more complete self-hosted platform with MCP server support and a full web UI.

### Docker Compose Stack (3 services)

| Service | Port | Purpose |
|---------|------|---------|
| `mem0_store` (Qdrant) | 6333 | Vector store |
| `openmemory-mcp` (FastAPI) | 8765 | API + MCP server |
| `openmemory-ui` (Next.js) | 3000 | Web frontend |

### MCP Server (`api/app/mcp_server.py`)

**Model Context Protocol** — the key differentiator from the basic server. Implements the MCP protocol for AI agents to interact with memories:

| MCP Tool | Purpose |
|----------|---------|
| `add_memories(text, infer=True)` | Add memory, update DB state + history |
| `search_memory(query)` | Search with ACL permission checks, log access |
| `list_memories()` | List with ACL filtering, log access |
| `delete_memories(memory_ids)` | Delete with ACL + state updates |
| `delete_all_memories()` | Bulk delete with ACL |

**Transports:**
- SSE: `/mcp/messages/`
- Streamable HTTP: `/mcp/{client_name}/http/{user_id}`

**Access Control:** `check_memory_access_permissions()` enforces per-app memory visibility. Every access is logged to `MemoryAccessLog`.

**State Tracking:** Memories have states (`MemoryState.active` / `MemoryState.deleted`) with full history via `MemoryStatusHistory`.

### Database Models

SQLAlchemy models: `Memory`, `App`, `User`, `MemoryAccessLog`, and more. Migrations via Alembic.

### UI

Next.js 15 + React 19 + Radix UI + Redux Toolkit + TailwindCSS + Recharts. Full management interface for memories, apps, users, and access logs.

---

## CLIs — Python & Node

### Python CLI (`cli/python/`)

- **Framework:** Typer + Rich + httpx
- **Entry point:** `mem0 = "mem0_cli.app:main"`
- **Backend abstraction:** Platform (hosted) or OSS (self-hosted) mode
- **Optional `[oss]` extra:** Installs `mem0ai` for self-hosted mode
- **Python:** 3.10+ (not 3.9)

### Node CLI (`cli/node/`)

- **Framework:** Commander + Chalk + ora + cli-table3
- **Entry point:** `mem0` command
- **Commands:** `init`, `add`, `search`, `get`, `list`, `update`, `delete`, `config`, `entity`, `event`, `status`, `import`, `help`
- **Agent mode:** `--json` / `--agent` flag for machine-readable output. `help --json` outputs `cli-spec.json` for LLM agent consumption
- **ID resolution:** CLI flag → config default → undefined (priority chain)
- **Auto-validation:** API key validated on first use with 5s timeout

### Shared CLI Patterns

Both CLIs:
- Support hosted and self-hosted backends
- Rich formatted output (tables, spinners, colors)
- Config management (show/get/set)
- Entity management (list/delete)
- Telemetry (PostHog)
- Agent-friendly output mode

---

## Plugin System — MCP & AI Editors

`mem0-plugin/` — Integrations for Claude Code, Cursor, and Codex AI editors.

### MCP Tools (9)

| Tool | Purpose |
|------|---------|
| `add_memory` | Store a new memory |
| `search_memories` | Search memories by query |
| `get_memories` | List all memories |
| `get_memory` | Get single memory by ID |
| `update_memory` | Update memory content |
| `delete_memory` | Delete a single memory |
| `delete_all_memories` | Delete all memories |
| `delete_entities` | Delete named entities |
| `list_entities` | List all entities |

### Claude Code Lifecycle Hooks

| Hook | Script | When it fires |
|------|--------|---------------|
| `SessionStart` | `on_session_start.sh` | When Claude Code session starts — loads mem0 context |
| `PreToolUse` (Write/Edit) | `block_memory_write.sh` | Guards direct writes to memory DB |
| `PreCompact` | `on_pre_compact.sh` + `.py` | Before context compaction — saves session state to mem0 |
| `UserPromptSubmit` | `on_user_prompt.sh` | When user submits prompt — searches mem0 for relevant memories (5s timeout) |
| `TaskCompleted` | `on_task_completed.sh` | When a task finishes — captures completion |
| `Stop` | `on_stop.sh` | Session cleanup |

The key insight: **PreCompact** is critical — it ensures session knowledge is persisted to mem0 before Claude Code compacts (summarizes) the context window, preventing information loss.

### Three MCP Surfaces

1. **Remote MCP** — `mcp.mem0.ai` (hosted cloud)
2. **Local MCP** — OpenMemory's FastAPI-based MCP server
3. **Plugin MCP** — 9 tools via `.mcp.json` for direct AI editor access

---

## Evaluation Framework

`evaluation/` — Benchmarks mem0 against other memory/RAG approaches using the LOCOMO evaluation dataset.

### What Gets Evaluated

| Approach | Make target |
|----------|-------------|
| mem0 (vector only) | `run-mem0-add`, `run-mem0-search` |
| mem0+ (vector + graph) | `run-mem0-plus-add`, `run-mem0-plus-search` |
| RAG baseline | `run-rag` |
| Full context baseline | `run-full-context` |
| LangMem comparison | `run-langmem` |
| OpenAI comparison | `run-openai` |

### Metrics

| Metric | How it works |
|--------|-------------|
| **F1** | Token-level F1 between predicted and ground truth |
| **BLEU-1** | N-gram overlap scoring |
| **LLM Judge** | LLM evaluates whether predicted answer captures the ground truth intent |

### Execution

`evals.py` runs evaluations with concurrent `ThreadPoolExecutor` workers. Results saved to JSON. Category 5 is skipped (by design).

---

## Key Architectural Patterns

### 1. Provider Plugin Architecture

Every provider category (LLM, embedding, vector store, graph, reranker) follows:
- `base.py` → abstract class
- `<provider>.py` → concrete implementation inheriting from base
- `configs.py` → Pydantic config (optional)
- `__init__.py` → registry
- `factory.py` → dynamic loading via `importlib.import_module()`

**Adding a provider:** Create the file, inherit from base, add to factory mapping, add dependencies to optional group in `pyproject.toml`.

### 2. V3 Additive Extraction Pipeline

The single most important design decision in mem0. Previous versions asked the LLM to decide between ADD/UPDATE/DELETE for each extracted fact — this caused hallucination and error.

**V3 approach:**
- Single LLM call produces only ADD operations (no decision complexity)
- Hash-based deduplication prevents storing duplicate memories
- Separate step handles updates to existing conflicting memories
- Batch operations throughout (embed, insert, entity linking)

### 3. Hash Deduplication

Each extracted memory gets an **MD5 hash** computed from its text content. Before insertion:
- Check batch-internal duplicates (same hash within extraction results)
- Check existing duplicates (search vector store for same hash)
- If hash matches → skip insertion, potentially update instead

### 4. Session Scoping

Memories are scoped by at least one of: `user_id`, `agent_id`, `run_id`.

The `session_scope` string is built deterministically:
```
agent_id=X&run_id=Y&user_id=Z
```

This is used as the key for SQLite message storage and vector store filtering.

### 5. History Tracking

Every memory mutation is tracked in SQLite `history` table with:
- `old_memory` / `new_memory` for diffs
- `actor_id` and `role` (who made the change)
- `is_deleted` flag
- Timestamps

This enables the `history()` API and audit trails.

### 6. Telemetry

PostHog-based, thread-safe singleton:
- 10% sampling for hot-path events (add/search operations)
- 100% for lifecycle events (initialization, errors)
- MD5-hashed entity IDs for privacy
- Graceful failure — telemetry errors never affect functionality

---

## Data Flow: Complete Add Pipeline

The 8-phase V3 additive pipeline in `Memory._add_to_vector_store()`:

```
Phase 0: Context Gathering
  │  Fetch last 10 messages from SQLite for this session_scope
  │
Phase 1: Existing Memory Retrieval
  │  Embed the new messages → search vector store for top-10 existing memories
  │  These existing memories are injected into the extraction prompt
  │
Phase 2: LLM Extraction
  │  Build ADDITIVE_EXTRACTION_PROMPT with:
  │   - Existing memories (with UUID→int mapping to prevent hallucination)
  │   - New messages
  │   - Last k messages from context
  │   - Custom instructions
  │  Call LLM → get list of extracted facts/memories (ADD only)
  │
Phase 3: Batch Embed
  │  Embed all extracted memory texts via embed_batch()
  │
Phase 4: CPU Processing per Memory
  │  For each extracted memory:
  │   - Clean text, compute metadata
  │   - Compute MD5 hash
  │
Phase 5: Hash Deduplication
  │  - Check for duplicate hashes within the batch
  │  - Check for duplicate hashes against existing memories
  │  - Remove duplicates, flag conflicts for update
  │
Phase 6: Batch Persist
  │  - vector_store.insert() for all new memories
  │  - db.batch_add_history() for all history records
  │
Phase 7: Batch Entity Linking
  │  - extract_entities_batch() via spaCy NER
  │  - Global dedup: search entity store for existing matches
  │  - Batch embed entity names
  │  - Batch search entity store for similarity matches
  │  - Separate into: new entities (insert) vs existing (update linked_memory_ids)
  │  - Batch insert new entities, batch update existing entities
  │
Phase 8: Save & Return
     - Save messages to SQLite (for future context)
     - Return added/updated/deleted memory summaries
```

---

## Data Flow: Complete Search Pipeline

The hybrid search in `Memory._search_vector_store()`:

```
1. Preprocess Query
   │  Lemmatize, extract entities from query text
   │
2. Embed Query
   │  Call embedder.embed(query) → query vector
   │
3. Semantic Search
   │  Search vector store with query vector
   │  Over-fetch at 4x limit for better recall
   │
4. Keyword Search (if supported)
   │  vector_store.keyword_search(query) → BM25 results
   │  Skip if vector store doesn't implement keyword_search
   │
5. Normalize BM25
   │  Apply query-length-adaptive sigmoid normalization
   │  Maps raw BM25 scores → [0, 1]
   │
6. Entity Boost Lookup
   │  Search entity store for query entities
   │  Collect linked_memory_ids for matching entities
   │
7. Score & Rank
   │  For each candidate:
   │   - If semantic_score < threshold → discard
   │   - combined = (semantic + bm25 + entity_boost) / num_signals
   │   - Apply spread-attenuation for entity boosts
   │
8. Return Results
     Sort by combined score descending
     Return top `limit` memories with scores
```

### Filter Application

Advanced metadata filters are translated before search:
```
{"AND": [
  {"user_id": {"eq": "alice"}},
  {"category": {"in": ["work", "personal"]}}
]}
```
→ vector-store-specific filter syntax (varies by provider implementation)

---

## Quick Reference: Key File Locations

| What | File | Lines |
|------|------|-------|
| Memory engine (sync) | `mem0/memory/main.py` | 331–1793 |
| Memory engine (async) | `mem0/memory/main.py` | 1795–2895 |
| Memory base class | `mem0/memory/base.py` | 4–63 |
| SQLite storage | `mem0/memory/storage.py` | 11–347 |
| Prompt templates | `mem0/configs/prompts.py` | 1000+ |
| Memory config | `mem0/configs/base.py` | 29–81 |
| LLM base class | `mem0/llms/base.py` | 7 |
| Embedding base class | `mem0/embeddings/base.py` | 7 |
| Vector store base class | `mem0/vector_stores/base.py` | 4 |
| Reranker base class | `mem0/reranker/base.py` | 4 |
| Factory pattern | `mem0/utils/factory.py` | 1–264 |
| Entity extraction | `mem0/utils/entity_extraction.py` | 1–357 |
| Hybrid scoring | `mem0/utils/scoring.py` | 1–121 |
| Hosted client (sync) | `mem0/client/main.py` | 36–917 |
| Hosted client (async) | `mem0/client/main.py` | 918+ |
| OpenAI proxy | `mem0/proxy/main.py` | 1–189 |
| Exception hierarchy | `mem0/exceptions.py` | 1–485 |
| TS hosted client | `mem0-ts/src/client/mem0.ts` | 70–697 |
| Server FastAPI app | `server/main.py` | 488 |
| OpenMemory MCP server | `openmemory/api/app/mcp_server.py` | 574 |
| Python CLI | `cli/python/src/mem0_cli/app.py` | main entry |
| Node CLI | `cli/node/src/index.ts` | 772 |
| Plugin hooks | `mem0-plugin/hooks/hooks.json` | 78 |
| Evaluation | `evaluation/evals.py` | 81 |