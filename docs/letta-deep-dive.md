# Letta Deep Dive

**Version:** 0.16.7 | **License:** Apache | **Python:** >=3.11, <3.14 | **Repo:** github.com/letta-ai/letta

---

## Overview

Letta (formerly MemGPT) is an open-source platform for building stateful LLM agents with persistent memory, tool execution, and multi-agent orchestration. The system provides:

- A **REST API server** (FastAPI) for managing agents, tools, messages, and data sources
- An **agent execution loop** supporting multiple agent types (chat, voice, sleeptime, workflow, react)
- A **multi-tenant data layer** using PostgreSQL + pgvector with SQLAlchemy ORM
- **LLM integration** with 20+ providers (OpenAI, Anthropic, Google, Azure, Bedrock, Groq, Ollama, vLLM, etc.)
- **MCP (Model Context Protocol)** integration for external tool servers
- **Summarization/compaction** to manage context windows
- **Sandboxed tool execution** (local, E2B, Modal)
- **Observability** via OpenTelemetry, ClickHouse traces, custom metrics

### Tech Stack Summary

| Category | Technology |
|---|---|
| Language | Python 3.11+ |
| Web Framework | FastAPI + Uvicorn/Granian |
| ORM | SQLAlchemy 2.0 (async) + SQLModel |
| Database | PostgreSQL 15+ with pgvector |
| Caching/Locking | Redis |
| Schema Validation | Pydantic v2 |
| LLM SDKs | OpenAI, Anthropic, Google Genai, Mistral |
| Embeddings | OpenAI, LlamaIndex |
| Observability | OpenTelemetry, ClickHouse (optional) |
| Migrations | Alembic |
| Containerization | Docker + Docker Compose + Nginx |
| Job Scheduling | APScheduler |
| MCP | fastmcp, mcp[cli] |
| Tracing | ddtrace (Datadog), OpenTelemetry |

---

## Layer 1: Infrastructure

### 1.1 Deployment

**Key files:** `Dockerfile`, `compose.yaml`, `dev-compose.yaml`, `docker-compose-vllm.yaml`, `nginx.conf`

#### Docker Compose Services (`compose.yaml`)

| Service | Image | Purpose |
|---|---|---|
| `letta_db` | `ankane/pgvector:v0.5.1` | PostgreSQL 15 with pgvector extension |
| `letta_server` | `letta/letta:latest` | Letta application server, depends on DB health check |
| `letta_nginx` | `nginx:stable-alpine` | Reverse proxy |

The server exposes ports 8083 (REST) and 8283 (REST alt). Environment variables configure all major LLM provider API keys.

#### Dockerfile (Multi-stage Build)

| Stage | Base | Adds |
|---|---|---|
| Builder | pgvector base | Python 3.11, uv, dependency sync |
| Runtime | pgvector base | Node.js 22, OTEL Collector, Redis, application code |

### 1.2 Database

**Key files:** `init.sql`, `letta/server/db.py`

- `init.sql` - PostgreSQL initialization (extensions, roles)
- `letta/server/db.py` - `DatabaseRegistry` provides `async_session()` context manager with asyncpg driver
  - Configurable pool size, SSL, prepared statements
  - Supports both PostgreSQL (with pgvector) and SQLite (with sqlite-vec for local dev)

### 1.3 Migrations

**Key files:** `alembic.ini`, `alembic/env.py`, `alembic/versions/`

Alembic handles schema migrations. `alembic/env.py` supports async SQLAlchemy. Migration version files are in `alembic/versions/`.

### 1.4 Observability

**Key files:** `letta/otel/`, `otel/`

#### In-Code OTEL (`letta/otel/`)

| File | Purpose |
|---|---|
| `tracing.py` | Core: `tracer`, `trace_method` decorator, `log_event()`, OTLP exporter setup |
| `context.py` | Trace context propagation, `get_ctx_attributes()` |
| `resource.py` | OTEL resource construction |
| `events.py` | Event logging |
| `metrics.py` | Custom metrics |
| `metric_registry.py` | `MetricRegistry` - centralized counters (DB pool checkout, agent step timing) |
| `db_pool_monitoring.py` | SQLAlchemy connection pool monitoring |
| `sqlalchemy_instrumentation.py` | SQLAlchemy query instrumentation |
| `sqlalchemy_instrumentation_integration.py` | Integration with OTEL SQLAlchemy instrumentation |

#### OTEL Collector Configs (`otel/`)

Configuration files for multiple backends:
- File-based export (`otel-collector-config-file.yaml`)
- ClickHouse export (`otel-collector-config-clickhouse.yaml`) with dev/prod variants
- SignOz export (`otel-collector-config-signoz.yaml`)
- Startup script (`start-otel-collector.sh`)

#### Provider Trace Backends (`letta/services/provider_trace_backends/`)

| File | Backend |
|---|---|
| `base.py` | `ProviderTraceBackend` ABC |
| `socket.py` | Socket-based trace export |
| `postgres.py` | PostgreSQL trace storage |
| `clickhouse.py` | ClickHouse trace storage |
| `factory.py` | Backend factory |

---

## Layer 2: Data (ORM)

**Key directory:** `letta/orm/`

### 2.1 Base Classes

| File | Class | Purpose |
|---|---|---|
| `base.py` | `Base` (DeclarativeBase) | SQLAlchemy declarative base |
| `base.py` | `CommonSqlalchemyMetaMixins` | `created_at`, `updated_at`, `is_deleted`, `created_by_id` columns |
| `sqlalchemy_base.py` | `SqlalchemyBase` (~1003 lines) | Full CRUD async access layer with retry logic (deadlock handling), optimistic concurrency, soft delete, access type control, pagination, relationship loading |
| `mixins.py` | `OrganizationMixin`, `ProjectMixin`, `TemplateMixin`, `TemplateEntityMixin` | Domain-specific mixins |
| `custom_columns.py` | Custom column types | `CompactionSettingsColumn`, `EmbeddingConfigColumn`, `LLMConfigColumn`, `ResponseFormatColumn`, `ToolRulesColumn` |
| `errors.py` | ORM errors | `NoResultFound`, `UniqueConstraintViolationError`, `DatabaseDeadlockError` |
| `sqlite_functions.py` | SQLite vector search | Compatibility layer for local dev |

### 2.2 Core ORM Models

| File | Model | Key Fields/Relationships |
|---|---|---|
| `agent.py` | `Agent` | id, name, agent_type, system, message_ids, llm_config, embedding_config, memory (blocks), tools, groups, sources, tags, identities |
| `message.py` | `Message` | id, role, content (JSON), tool_calls, tool_call_id, agent_id, step_id, group_id, model |
| `block.py` | `Block` | id, label, value, limit, read_only, description |
| `conversation.py` | `Conversation` | id, name, created_by |
| `source.py` | `Source` | id, name, embedding_config |
| `passage.py` | `BasePassage`, `ArchivalPassage`, `SourcePassage` | id, content, embedding (pgvector), metadata |
| `tool.py` | `Tool` | id, name, source_code, json_schema, tool_type, tags |
| `user.py` | `User` | id, name, organization_id |
| `organization.py` | `Organization` | id, name |
| `provider.py` | `Provider` | id, provider_name, provider_type, api_key (encrypted) |
| `provider_model.py` | `ProviderModel` | Model specifications per provider |
| `group.py` | `Group` | id, agent_ids, manager_type (sleeptime/voice) |
| `job.py` | `Job` | id, status, metadata |
| `run.py` | `Run` | id, status, agent_id, usage |
| `step.py` | `Step` | id, agent_id, provider, model, usage, status |
| `file.py` | `FileMetadata` | id, file_name, source_id, content |
| `identity.py` | `Identity` | id, identifier_key, identity_type |
| `archive.py` | `Archive` | id, name |
| `mcp_server.py` | `MCPServer` | id, server_name, server_type, config |
| `mcp_oauth.py` | `MCPOAuth` | OAuth session data |
| `sandbox_config.py` | `SandboxConfig`, `AgentEnvironmentVariable` | Sandbox execution config |
| `provider_trace.py` | `ProviderTrace` | LLM call traces |
| `prompt.py` | `Prompt` | Stored system prompts |
| `llm_batch_job.py` | `LLMBatchJob`, `LLMBatchItem` | OpenAI batch API tracking |
| `run_metrics.py` | `RunMetrics` | Run-level metrics |
| `step_metrics.py` | `StepMetrics` | Step-level metrics |

### 2.3 Junction/Association Tables

| Table | Connects |
|---|---|
| `agents_tags` | Agent <-> Tag |
| `archives_agents` | Archive <-> Agent |
| `blocks_agents` | Block <-> Agent |
| `blocks_conversations` | Block <-> Conversation |
| `blocks_tags` | Block <-> Tag |
| `conversation_messages` | Conversation <-> Message |
| `files_agents` | File <-> Agent |
| `groups_agents` | Group <-> Agent |
| `groups_blocks` | Group <-> Block |
| `identities_agents` | Identity <-> Agent |
| `identities_blocks` | Identity <-> Block |
| `passage_tag` | Passage <-> Tag |
| `sources_agents` | Source <-> Agent |
| `tools_agents` | Tool <-> Agent |

### 2.4 Key ORM Design Patterns

- **Soft Delete**: All models support `is_deleted` flag via `CommonSqlalchemyMetaMixins`; queries filter deleted records
- **Optimistic Concurrency**: `SqlalchemyBase` implements version-based concurrent update detection with retry on deadlock
- **Custom Column Types**: Complex configs (LLMConfig, EmbeddingConfig, ToolRules) stored as JSON columns with custom SQLAlchemy type adapters
- **Async All The Way**: All ORM access is async using SQLAlchemy 2.0 async sessions with asyncpg

---

## Layer 3: Schemas & Types

**Key directories:** `letta/schemas/`, `letta/types/`

### 3.1 Core Schemas

| File | Key Models | Purpose |
|---|---|---|
| `agent.py` | `AgentState`, `CreateAgent`, `UpdateAgent`, `AgentStepResponse` | Agent state and CRUD contracts |
| `message.py` | `Message` (~2710 lines), `MessageCreate`, `MessageUpdate`, `ToolReturn` | The most complex schema; handles all message types, tool calls, content |
| `memory.py` | `Memory`, `ContextWindowOverview` | Core/in-context memory with blocks |
| `block.py` | `Block`, `FileBlock`, `CreateBlock`, `BlockUpdate` | Memory block schemas |
| `tool.py` | `Tool`, `BaseTool`, `ToolCreate`, `ToolUpdate`, `ToolSearchRequest` | Tool schemas |
| `llm_config.py` | `LLMConfig` | Model, endpoint, context_window, temperature, etc. |
| `embedding_config.py` | `EmbeddingConfig` | Embedding model configuration |
| `enums.py` | `PrimitiveType`, `ProviderType`, `AgentType`, `MessageRole`, `JobStatus`, `ToolType`, `RunStatus` | All enums |

### 3.2 Message Type System

| File | Types | Purpose |
|---|---|---|
| `letta_message.py` | `LettaMessage` union: `AssistantMessage`, `ToolCallMessage`, `ToolReturnMessage`, `ReasoningMessage`, `SummaryMessage`, `UserMessage`, `SystemMessage`, `ApprovalRequestMessage` | Discriminated union of all message types |
| `letta_message_content.py` | `TextContent`, `ImageContent`, `ReasoningContent`, `RedactedReasoningContent`, `ToolCallContent`, `ToolReturnContent` | Content type variants |

### 3.3 Request/Response Types

| File | Key Types | Purpose |
|---|---|---|
| `letta_request.py` | `LettaRequest`, `LettaAsyncRequest`, `LettaStreamingRequest` | Agent request schemas |
| `letta_response.py` | `LettaResponse`, `LettaStreamingResponse` | Agent response schemas |
| `letta_stop_reason.py` | `LettaStopReason` | `end_turn`, `max_steps`, `tool_call`, `error` |
| `usage.py` | `LettaUsageStatistics` | Token usage tracking |

### 3.4 Entity Schemas

| File | Key Types |
|---|---|
| `source.py` | `Source` (data source for RAG) |
| `passage.py` | `Passage` (embedded text chunk) |
| `job.py` | `Job`, `JobUpdate` |
| `run.py` | `Run` |
| `step.py` | `Step`, `StepProgression` |
| `group.py` | `Group`, `GroupCreate`, `SleeptimeManager`, `VoiceSleeptimeManager` |
| `conversation.py` | `Conversation` |
| `organization.py` | `Organization` |
| `user.py` | `User` |
| `provider.py` | Provider type schemas |
| `providers/` | Per-provider configuration schemas |
| `mcp_server.py` | MCP server configuration |
| `sandbox_config.py` | `SandboxConfig`, `LocalSandboxConfig` |
| `identity.py` | `Identity` |
| `archive.py` | `Archive` |
| `file.py` | `FileMetadata` |
| `secret.py` | `Secret` |

### 3.5 Tool & Execution Schemas

| File | Key Types |
|---|---|
| `tool_rule.py` | `ToolRule`, `TerminalToolRule`, `ContinueToolRule`, `RequiresApprovalToolRule` |
| `tool_execution_result.py` | `ToolExecutionResult` |
| `response_format.py` | `ResponseFormatUnion` |
| `model.py` | `ModelSettingsUnion` |
| `llm_batch_job.py` | Batch job schemas |
| `llm_trace.py` | LLM trace schemas |
| `provider_trace.py` | `ProviderTrace`, `BillingContext` |

### 3.6 Base Classes

| File | Class | Purpose |
|---|---|---|
| `letta_base.py` | `LettaBase`, `OrmMetadataBase` | Base classes with ID generation (prefixed IDs like `agent-xxx`) |
| `openai/` | OpenAI-compatible schemas | Interoperability with OpenAI API |

### 3.7 Serialization Schemas

**Key directory:** `letta/serialize_schemas/`

| File | Purpose |
|---|---|
| `marshmallow_agent.py` | Agent marshmallow schema |
| `marshmallow_tool.py` | Tool marshmallow schema |
| `marshmallow_message.py` | Message marshmallow schema |
| `marshmallow_block.py` | Block marshmallow schema |
| `marshmallow_tag.py` | Tag marshmallow schema |
| `marshmallow_custom_fields.py` | Custom field schemas |
| `marshmallow_base.py` | Base marshmallow schema |
| `marshmallow_agent_environment_variable.py` | Agent env var schema |
| `pydantic_agent_schema.py` | Pydantic agent schema for API |

---

## Layer 4: Services

**Key directory:** `letta/services/`

### 4.1 Entity Managers

Each domain entity has a dedicated manager class that encapsulates all ORM access and provides high-level business operations.

| Manager | File | Size | Purpose |
|---|---|---|---|
| `AgentManager` | `agent_manager.py` | ~3600 lines | Agent CRUD, tool/block/source assignment, system prompt generation, in-context message management, context window calculation |
| `MessageManager` | `message_manager.py` | | Message CRUD, search, in-context management |
| `BlockManager` | `block_manager.py` | | Block CRUD, value validation |
| `BlockManager` (Git) | `block_manager_git.py` | | Git-backed memory blocks with version control |
| `PassageManager` | `passage_manager.py` | | Archival/source passage CRUD, vector search |
| `SourceManager` | `source_manager.py` | | Data source CRUD, passage loading |
| `ToolManager` | `tool_manager.py` | | Tool CRUD, schema generation, validation |
| `ProviderManager` | `provider_manager.py` | | LLM provider CRUD |
| `UserManager` | `user_manager.py` | | User CRUD |
| `OrganizationManager` | `organization_manager.py` | | Organization CRUD |
| `GroupManager` | `group_manager.py` | | Multi-agent group CRUD |
| `ConversationManager` | `conversation_manager.py` | | Conversation CRUD, conversation-level locking |
| `JobManager` | `job_manager.py` | | Async job tracking |
| `RunManager` | `run_manager.py` | | Agent run tracking |
| `StepManager` | `step_manager.py` | | Agent step (turn) tracking |
| `FileManager` | `file_manager.py` | | File CRUD for agent file system |
| `FileAgentManager` | `files_agents_manager.py` | | File-agent association |
| `IdentityManager` | `identity_manager.py` | | Identity management |
| `ArchiveManager` | `archive_manager.py` | | Archive CRUD |
| `SandboxConfigManager` | `sandbox_config_manager.py` | | Sandbox configuration |
| `MCPServerManager` | `mcp_server_manager.py` | | MCP server CRUD |
| `AgentSerializationManager` | `agent_serialization_manager.py` | | Agent export/import |

### 4.2 Specialized Services

| Service | File | Purpose |
|---|---|---|
| `MCPManager` | `mcp_manager.py` | MCP tool execution orchestration |
| `StreamingService` | `streaming_service.py` | Server-sent event streaming |
| `TelemetryManager` | `telemetry_manager.py` | Telemetry data collection |
| `WebhookService` | `webhook_service.py` | Step completion webhook notifications |
| `CreditVerificationService` | `credit_verification_service.py` | Credit balance checking |
| `LLMBatchManager` | `llm_batch_manager.py` | OpenAI batch API management |
| `LLMTraceReader` | `llm_trace_reader.py` | Read LLM traces |
| `LLMTraceWriter` | `llm_trace_writer.py` | Write LLM traces |
| `SandboxCredentialsService` | `sandbox_credentials_service.py` | Sandbox credentials management |
| `AgentGenerateCompletionManager` | `agent_generate_completion_manager.py` | Direct LLM generation without agent loop |

### 4.3 Tool Execution System

**Key directory:** `letta/services/tool_executor/`

The tool execution system uses a **Strategy Pattern** where each tool type has a dedicated executor.

```
ToolExecutionManager
  -> ToolExecutorFactory.get_executor(tool_type)
    -> LettaCoreToolExecutor      (memory, send_message, conversation_search, archival_memory)
    -> LettaBuiltinToolExecutor    (run_code, web_search, fetch_webpage)
    -> LettaFileToolExecutor       (open_files, grep_files, semantic_search_files)
    -> ExternalMCPToolExecutor     (external MCP tool calls)
    -> SandboxToolExecutor         (custom user tools in sandbox)
    -> ComposioToolExecutor        (Composio integration tools)
```

| File | Executor | Covers |
|---|---|---|
| `tool_execution_manager.py` | `ToolExecutionManager` | Routes to appropriate executor |
| `tool_executor_base.py` | `ToolExecutor` | ABC with `execute()` method |
| `core_tool_executor.py` | `LettaCoreToolExecutor` | Core memory tools |
| `builtin_tool_executor.py` | `LettaBuiltinToolExecutor` | Built-in tools |
| `files_tool_executor.py` | `LettaFileToolExecutor` | File tools |
| `mcp_tool_executor.py` | `ExternalMCPToolExecutor` | External MCP tool calls |
| `sandbox_tool_executor.py` | `SandboxToolExecutor` | Custom user tools in sandbox |
| `composio_tool_executor.py` | `ComposioToolExecutor` | Composio integration |

### 4.4 Summarizer / Compaction

**Key directory:** `letta/services/summarizer/`

Manages context window overflow by summarizing/compacting messages when tokens exceed thresholds.

| File | Purpose |
|---|---|
| `summarizer.py` | `Summarizer` class - orchestrates message summarization based on mode |
| `enums.py` | `SummarizationMode`: STATIC_MESSAGE_BUFFER, PARTIAL_EVICT_MESSAGE_BUFFER, SLIDING_WINDOW, etc. |
| `summarizer_all.py` | Summarize ALL messages in context |
| `summarizer_sliding_window.py` | Sliding window summarization |
| `self_summarizer.py` | Self-summarization (agent summarizes its own context) |
| `compact.py` | `compact_messages()` - LLM-based compaction of messages |
| `summarizer_config.py` | `CompactionSettings` configuration |
| `thresholds.py` | Token-based thresholds for triggering compaction |
| `constants.py` | Summarizer constants |

### 4.5 Tool Sandbox

**Key directory:** `letta/services/tool_sandbox/`

| File | Sandbox | Purpose |
|---|---|---|
| `base.py` | `BaseSandbox` ABC | Abstract sandbox interface |
| `local_sandbox.py` | `LocalSandbox` | Local process execution |
| `e2b_sandbox.py` | `E2BSandbox` | E2B cloud sandbox |
| `modal_sandbox.py` | `ModalSandbox` | Modal cloud execution |
| `modal_sandbox_v2.py` | `ModalSandboxV2` | V2 Modal sandbox |
| `modal_deployment_manager.py` | - | Modal deployment management |
| `modal_version_manager.py` | - | Modal version tracking |
| `typescript_generator.py` | - | TypeScript tool code generation for sandbox |
| `safe_pickle.py` | - | Safe pickle deserialization |
| `tool_schema_generator.py` | - | Tool JSON schema generation from source code |

### 4.6 File Processor

**Key directory:** `letta/services/file_processor/`

| Component | Purpose |
|---|---|
| `file_processor.py` | Main file processing orchestrator |
| `parser/` | File parsers: MarkItDown, Mistral |
| `chunker/` | Text chunkers: line-based, LlamaIndex-based |
| `embedder/` | Embedding providers: OpenAI, Pinecone, TurboPuffer |
| `types.py` | File processor types |

### 4.7 MCP Client

**Key directory:** `letta/services/mcp/`

| File | Client | Transport |
|---|---|---|
| `base_client.py` | `AsyncBaseMCPClient` | Base |
| `fastmcp_client.py` | `AsyncFastMCPSSEClient` | FastMCP SSE |
| `sse_client.py` | - | SSE MCP transport |
| `streamable_http_client.py` | - | Streamable HTTP transport |
| `stdio_client.py` | `AsyncStdioMCPClient` | Stdio transport |
| `oauth_utils.py` | - | OAuth flow for MCP servers |
| `server_side_oauth.py` | - | Server-side OAuth handling |

### 4.8 Other Service Subpackages

| Subpackage | Purpose |
|---|---|
| `helpers/` | Agent manager helpers, tool parser helpers, run manager helpers |
| `context_window_calculator/` | Context window estimation and token counting |
| `memory_repo/` | Git-backed memory filesystem: block-to-markdown conversion, git operations, memfs client |
| `lettuce/` | `LettuceClient` - external service client |
| `llm_router/` | `LLMRouterClientBase` - LLM routing for multi-provider setups |

### 4.9 SyncServer Orchestrator

**Key file:** `letta/server/server.py` (~1900 lines)

`SyncServer` is the central orchestrator that connects all services. It:
- Manages agents, tools, sources, blocks, MCP servers, jobs, users, organizations, providers, runs, conversations
- Delegates to service managers
- Coordinates multi-step operations (e.g., agent creation = create agent + assign tools + assign blocks + set up memory)

---

## Layer 5: Agent

**Key directories:** `letta/agents/`, `letta/agent.py`, `letta/groups/`

### 5.1 Agent Types

Defined in `letta/schemas/enums.py` `AgentType` enum:

| Type | Description |
|---|---|
| `memgpt_agent` | Original MemGPT with heartbeat chain |
| `memgpt_v2_agent` | Refreshed MemGPT tools |
| `letta_v1_agent` | Simplified loop, no heartbeats |
| `react_agent` | Basic ReAct loop, no memory tools |
| `workflow_agent` | Workflow with auto-clearing buffer |
| `split_thread_agent` | Split conversation thread |
| `sleeptime_agent` | Background memory consolidation |
| `voice_convo_agent` | Real-time voice conversation |
| `voice_sleeptime_agent` | Voice + sleeptime |

### 5.2 Agent Hierarchy

There are **two parallel agent hierarchies**:

#### Legacy Agent (`letta/agent.py`)

```
BaseAgent (ABC) -> Agent (synchronous)
  step() -> inner_step() -> _get_ai_reply() -> _handle_ai_response() -> execute_tool_and_persist_state()
```

Supports context overflow recovery via `summarize_messages_inplace()`.

#### V2+ Agent Architecture (`letta/agents/`)

| File | Class | Purpose |
|---|---|---|
| `base_agent.py` | `BaseAgent` | V1 async base with `step()` returning `LettaResponse` |
| `base_agent_v2.py` | `BaseAgentV2` | V2 abstract async base with `step()` and `build_request()` |
| `letta_agent.py` | `LettaAgent` | Full async agent loop with streaming, summarizer, tool execution manager, structured output support |
| `letta_agent_v2.py` | `LettaAgentV2` | Refined V2 with adapters (LettaLLMAdapter), credit verification, run management |
| `letta_agent_v3.py` | `LettaAgentV3` | Latest V3: parallel tool calling, SGLang native adapter, compact summarization, LLM routing |
| `letta_agent_batch.py` | `LettaAgentBatch` | Batch processing agent |
| `voice_agent.py` | `VoiceAgent` | Streaming voice agent using OpenAI realtime API |
| `voice_sleeptime_agent.py` | `VoiceSleeptimeAgent` | Sleeptime variant for voice agents |
| `ephemeral_agent.py` | `EphemeralAgent` | Stateless thin wrapper around OpenAI |
| `ephemeral_summary_agent.py` | `EphemeralSummaryAgent` | Stateless summarization agent |

### 5.3 Agent Loop Factory

**Key file:** `letta/agents/agent_loop.py`

`AgentLoop.load(agent_state, actor)` is the factory that instantiates the correct agent type based on `AgentState.agent_type`. This is the entry point the server uses to create an agent for message processing.

### 5.4 Multi-Agent / Groups

**Key directory:** `letta/groups/`

| File | Class | Purpose |
|---|---|---|
| `sleeptime_multi_agent.py` | `SleeptimeMultiAgent` | V1: main + background sleeptime agent that consolidates memories |
| `sleeptime_multi_agent_v2.py` | - | V2 sleeptime with improved scheduling |
| `sleeptime_multi_agent_v3.py` | `SleeptimeMultiAgentV3` | V3 using `BaseAgentV2` interface |
| `sleeptime_multi_agent_v4.py` | `SleeptimeMultiAgentV4` | V4 latest: full async, run tracking |
| `dynamic_multi_agent.py` | - | Dynamic multi-agent orchestration |
| `round_robin_multi_agent.py` | - | Round-robin message distribution |
| `supervisor_multi_agent.py` | - | Supervisor-based multi-agent |

### 5.5 Prompts System

**Key directory:** `letta/prompts/`

| File | Purpose |
|---|---|
| `prompt_generator.py` | `PromptGenerator` - compiles system prompts by merging memory blocks, metadata, and tool rules |
| `gpt_system.py` | `get_system_text()` - loads system prompt template by name |
| `gpt_summarize.py` | Summarization prompt templates |
| `summarizer_prompt.py` | Summarizer-specific prompt construction |

#### System Prompt Templates (`letta/prompts/system_prompts/`)

| File | Agent Type |
|---|---|
| `memgpt_chat.py` | Original MemGPT chat |
| `memgpt_v2_chat.py` | V2 MemGPT |
| `letta_v1.py` | Letta V1 agent |
| `react.py` | ReAct agent |
| `voice_chat.py` | Voice conversation |
| `voice_sleeptime.py` | Voice sleeptime |
| `sleeptime_v2.py` | Sleeptime V2 |
| `sleeptime_doc_ingest.py` | Sleeptime document ingestion |
| `workflow.py` | Workflow agent |
| `memgpt_generate_tool.py` | Tool generation |
| `summary_system_prompt.py` | Summary generation |

### 5.6 Tool Rule Solver

**Key file:** `letta/helpers/toolrule_solver.py`

`ToolRulesSolver` enforces tool usage rules:
- **Terminal tools**: Tools that end the agent loop (e.g., `send_message`)
- **Init tools**: Tools that must be called first in a conversation
- **Continue tools**: Tools that always chain to another step
- **Child tools**: Tools that can only be called after a parent tool
- **Requires approval tools**: Tools that need user approval before execution

### 5.7 Interfaces (Observer Pattern)

| File | Interface | Purpose |
|---|---|---|
| `letta/interface.py` | `AgentInterface` ABC | Base: `user_message()`, `internal_monologue()`, `assistant_message()`, `function_message()` |
| `letta/interface.py` | `CLIInterface` | CLI colorized output |
| `letta/streaming_interface.py` | `AgentChunkStreamingInterface` | Streaming token chunks |
| `letta/interfaces/openai_streaming_interface.py` | `OpenAIStreamingInterface` | OpenAI streaming |
| `letta/interfaces/anthropic_streaming_interface.py` | `AnthropicStreamingInterface` | Anthropic streaming |
| `letta/interfaces/gemini_streaming_interface.py` | `GeminiStreamingInterface` | Gemini streaming |

---

## Layer 6: API

**Key directory:** `letta/server/rest_api/`

### 6.1 FastAPI Application

**Key file:** `letta/server/rest_api/app.py` (~987 lines)

- Configures CORS, exception handlers (50+ custom error types), middleware
- Lifespan handler: Pinecone setup, scheduler start
- Mounts routers under `/v1` and `/openai` prefixes
- Exception hierarchy from `letta/errors.py`

### 6.2 V1 Routers

**Key directory:** `letta/server/rest_api/routers/v1/`

35 router modules:

| Router | Key Endpoints |
|---|---|
| `agents.py` | CRUD agents, send messages, archival/recall memory, export/import, context window, approval |
| `messages.py` | List/search messages, batch operations |
| `runs.py` | List/cancel runs, stream retrieval |
| `tools.py` | CRUD tools, execute tools, MCP server management, tool generation |
| `chat_completions.py` | OpenAI-compatible chat completion endpoint |
| `conversations.py` | Conversation management |
| `blocks.py` | Memory block CRUD |
| `sources.py` | Data source CRUD, passage management |
| `passages.py` | Passage (embedded text chunk) CRUD |
| `providers.py` | LLM provider management |
| `organizations.py` | Organization management |
| `users.py` | User management |
| `groups.py` | Agent group management (multi-agent) |
| `jobs.py` | Job management (async operations) |
| `steps.py` | Step tracking |
| `mcp_servers.py` | MCP server CRUD |
| `health.py` | Health check |
| `embeddings.py` | Embedding generation |
| `sandbox_configs.py` | Sandbox configuration |
| `identities.py` | Identity management |
| `archives.py` | Archive management |
| `tags.py` | Tag management |
| `telemetry.py` | Telemetry data |
| `voice.py` | Voice agent endpoints |
| `llms.py` | LLM model listing |
| `folders.py` | Folder/source organization |
| `voice.py` | Voice endpoints |
| `git_http.py` | Git HTTP for memory repos |
| `zai.py` | ZAI-specific |
| `anthropic.py` | Anthropic proxy |
| `internal_*.py` | Admin endpoints |

### 6.3 OpenAI-Compatible API

**Key directory:** `letta/server/rest_api/routers/openai/chat_completions/`

Provides an OpenAI-compatible API surface for interoperability. Allows existing OpenAI SDK clients to talk to Letta agents.

### 6.4 Streaming / SSE

| File | Purpose |
|---|---|
| `letta/server/rest_api/interface.py` | `QueuingInterface`, `StreamingServerInterface` - bridge between agent events and HTTP SSE streaming (~1391 lines) |
| `letta/server/rest_api/redis_stream_manager.py` | Redis-based SSE stream management for concurrent requests |
| `letta/server/rest_api/streaming_response.py` | Custom `StreamingResponseWithStatusCode`, keepalive generation |
| `letta/server/rest_api/json_parser.py` | Optimistic JSON parser for streaming |

### 6.5 Authentication & Middleware

| Directory/File | Purpose |
|---|---|
| `letta/server/rest_api/auth/` | Authentication middleware |
| `letta/server/rest_api/middleware/` | Password check, logging, request ID middleware |
| `letta/server/rest_api/dependencies.py` | FastAPI dependency injection |
| `letta/server/rest_api/auth_token.py` | Auth token handling |
| `letta/server/global_exception_handler.py` | Global exception handling |

### 6.6 WebSocket API (Deprecated)

**Key directory:** `letta/server/ws_api/`

Previously supported WebSocket interface. Contains protocol and server stubs. Deprecated in favor of SSE streaming.

### 6.7 Jobs / Scheduler

**Key directory:** `letta/jobs/`

| File | Purpose |
|---|---|
| `scheduler.py` | APScheduler-based scheduler with PostgreSQL advisory lock leader election. Polls LLM batch jobs at intervals |
| `llm_batch_job_polling.py` | OpenAI batch job polling |
| `helpers.py` | Job helper utilities |

---

## Layer 7: LLM Integration

**Key directories:** `letta/llm_api/`, `letta/local_llm/`, `letta/adapters/`

### 7.1 LLM Client Factory

**Key file:** `letta/llm_api/llm_client.py`

`LLMClient.create()` routes to the correct provider-specific client based on `ProviderType`.

### 7.2 Provider Clients

| File | Client | Provider |
|---|---|---|
| `llm_client_base.py` | `LLMClientBase` | Abstract base: `send_llm_request()`, `send_llm_request_stream()`, telemetry |
| `openai_client.py` | `OpenAIClient` | OpenAI |
| `anthropic_client.py` | `AnthropicClient` | Anthropic |
| `google_ai_client.py` | `GoogleAIClient` | Google AI (Gemini) |
| `google_vertex_client.py` | `GoogleVertexClient` | Google Vertex AI |
| `azure_client.py` | `AzureClient` | Azure OpenAI |
| `bedrock_client.py` | `BedrockClient` | AWS Bedrock |
| `groq_client.py` | `GroqClient` | Groq |
| `deepseek_client.py` | `DeepSeekClient` | DeepSeek |
| `together_client.py` | `TogetherClient` | Together AI |
| `xai_client.py` | `XAIClient` | xAI (Grok) |
| `zai_client.py` | `ZAIClient` | Z.AI |
| `baseten_client.py` | `BasetenClient` | Baseten |
| `fireworks_client.py` | `FireworksClient` | Fireworks AI |
| `minimax_client.py` | `MiniMaxClient` | MiniMax |
| `mistral.py` | - | Mistral AI |
| `chatgpt_oauth_client.py` | - | ChatGPT OAuth |

### 7.3 Helper/Utility Files

| File | Purpose |
|---|---|
| `helpers.py` | Token counting, context overflow detection, summarizer cutoff calculation |
| `error_utils.py` | LLM error classification and retry logic |
| `openai_ws_session.py` | OpenAI WebSocket session for realtime |
| `llm_api_tools.py` | Legacy LLM API call function |
| `openai.py` | Legacy OpenAI functions |

### 7.4 Adapters

**Key directory:** `letta/adapters/`

Adapters abstract away provider-specific differences.

| File | Class | Purpose |
|---|---|---|
| `letta_llm_adapter.py` | `LettaLLMAdapter` | Abstract adapter for blocking + streaming LLM calls, captures response metadata |
| `letta_llm_request_adapter.py` | `LettaLLMRequestAdapter` | Builds request payloads for LLM calls |
| `letta_llm_stream_adapter.py` | `LettaLLMStreamAdapter` | Handles streaming LLM responses |
| `simple_llm_request_adapter.py` | `SimpleLLMRequestAdapter` | Simplified request building |
| `simple_llm_stream_adapter.py` | `SimpleLLMStreamAdapter` | Simplified streaming |
| `sglang_native_adapter.py` | `SGLangNativeAdapter` | SGLang native endpoint (for RL training) |

### 7.5 Local LLM Support

**Key directory:** `letta/local_llm/`

| Dir/File | Purpose |
|---|---|
| `chat_completion_proxy.py` | Proxy for local LLM chat completions |
| `constants.py` | Inner thought kwargs, CLI symbols |
| `function_parser.py` | Parse function calls from local model output |
| `json_parser.py` | JSON parsing for local model output |
| `utils.py` | Token counting for local models |
| `llm_chat_completion_wrappers/` | Wrappers for different chat formats (ChatML, etc.) |
| `grammars/` | Grammar definitions for constrained generation |
| `koboldcpp/` | KoboldCPP support |
| `llamacpp/` | llama.cpp support |
| `lmstudio/` | LM Studio support |
| `ollama/` | Ollama support |
| `vllm/` | vLLM support |
| `webui/` | Text generation web UI support |

---

## Cross-Cutting: Built-in Tool Functions

**Key directory:** `letta/functions/function_sets/`

| File | Tools Defined |
|---|---|
| `base.py` | Core: `memory()`, `send_message()`, `conversation_search()`, `archival_memory_insert()`, `archival_memory_search()`, `core_memory_append()`, `core_memory_replace()` |
| `builtin.py` | Built-in: `run_code()`, `run_code_with_tools()`, `web_search()`, `fetch_webpage()` |
| `files.py` | File: `open_files()`, `grep_files()`, `semantic_search_files()` |
| `multi_agent.py` | Multi-agent: `send_message_to_agent_and_wait_for_reply()`, `send_message_to_agents_matching_tags()`, `send_message_to_agent_async()` |
| `voice.py` | Voice: `store_memories()`, `rethink_user_memory()`, `finish_rethinking_memory()`, `search_memory()` |

### Functions / Schema System

**Key directory:** `letta/functions/`

| File | Purpose |
|---|---|
| `functions.py` | `get_function_from_module()`, `get_json_schema_from_module()`, `derive_openai_json_schema()` |
| `ast_parsers.py` | AST parsing for function annotations |
| `schema_generator.py` | `generate_tool_schema_for_mcp()` |
| `schema_validator.py` | `validate_complete_json_schema()` |
| `typescript_parser.py` | TypeScript source code parsing |
| `composio_helpers.py` | Composio integration |
| `mcp_client/types.py` | `MCPTool`, `BaseServerConfig`, `SSEServerConfig`, `StdioServerConfig` |

---

## Cross-Cutting: Data Flows

### A. User Message -> Agent Response

```
HTTP Request
  -> FastAPI (app.py)
    -> Router (v1/agents.py: create_agent_message)
      -> SyncServer (server.py)
        -> ConversationManager (locking: Redis distributed lock)
        -> AgentLoop.load(agent_state, actor)  -- Factory selects agent type
          -> LettaAgentV3.step()
            -> Summarizer.check_compaction_needed()
            -> _prepare_in_context_messages_no_persist_async()
            -> LettaLLMAdapter (builds request)
              -> LettaLLMRequestAdapter (constructs payload)
                -> LLMClient.create() (routes to provider)
                  -> Provider API (OpenAI/Anthropic/Google/etc.)
            -> Parse response (tool calls vs message)
            -> If tool call:
              -> ToolExecutionManager.execute_tool_call()
                -> ToolExecutorFactory routes to executor
                  -> Core/Builtin/File/MCP/Sandbox executor
              -> Persist tool result as Message
              -> Check ToolRulesSolver for chaining rules
              -> If continue: loop back to LettaLLMAdapter
            -> If message (terminal tool or end_turn):
              -> Persist assistant Message
              -> Return LettaResponse
      -> StreamingServerInterface -> SSE stream -> HTTP Response
```

### B. Agent Creation Flow

```
POST /v1/agents
  -> agents.py router
    -> SyncServer.create_agent()
      -> AgentManager.create()
        -> Create Agent ORM record
        -> Assign memory blocks (human, persona defaults)
        -> Assign tools (core + requested)
        -> Generate system prompt (PromptGenerator)
        -> Calculate initial context window
      -> Return AgentState
```

### C. Data Ingestion Flow

```
POST /v1/sources/{id}/upload
  -> sources.py router
    -> SyncServer.load_data()
      -> SourceManager.load_file()
        -> FileProcessor
          -> Parser (MarkItDown/Mistral)
          -> Chunker (line-based/LlamaIndex)
          -> Embedder (OpenAI/Pinecone/TurboPuffer)
        -> PassageManager
          -> Create Passage ORM records with pgvector embeddings
        -> Return Job (async tracking)
```

### D. Summarization / Compaction Flow

```
Agent step detects context overflow
  -> Summarizer.check_compaction_needed()
    -> Calculate token usage vs threshold
    -> If exceeded:
      -> SummarizationMode determines strategy
        -> STATIC_MESSAGE_BUFFER: Summarize oldest messages, move to summary block
        -> SLIDING_WINDOW: Keep recent N messages, summarize the rest
        -> compact_messages(): LLM-based compaction (replace verbose exchanges with summaries)
      -> Update in-context message list
      -> Persist new summary Message
```

### E. Sleeptime Agent Memory Consolidation

```
SleeptimeMultiAgent
  -> Main agent processes user messages (normal loop)
  -> Background sleeptime agent:
    -> Scheduled activation (interval-based)
    -> Reviews recent conversations
    -> Extracts important information
    -> Updates memory blocks (core_memory_replace, core_memory_append)
    -> Stores in archival memory for retrieval
```

---

## Cross-Cutting: Key Design Patterns

| Pattern | Usage |
|---|---|
| **Factory** | `AgentLoop.load()`, `LLMClient.create()`, `ToolExecutorFactory.get_executor()`, `ProviderTraceBackend` factory |
| **Observer** | `AgentInterface` and `AgentChunkStreamingInterface` for event-driven output to CLI/SSE |
| **Service Layer** | Every domain entity has a manager class encapsulating all ORM access |
| **Adapter** | `LettaLLMAdapter`, `LettaLLMRequestAdapter`, `LettaLLMStreamAdapter` abstract LLM provider differences |
| **Strategy** | Summarizer modes, sandbox types, tool executor types, MCP transport types |
| **Optimistic Concurrency** | `SqlalchemyBase` version-based concurrent update detection with retry on deadlock |
| **Conversation Locking** | Redis-based distributed locks prevent concurrent message sends to same conversation |
| **Soft Delete** | All ORM models support `is_deleted` flag with filtered queries |

---

## Key Module Reference

### Core Files

| File | Key Exports |
|---|---|
| `letta/__init__.py` | Version, `AgentState`, `Memory`, `LLMConfig`, `Tool` |
| `letta/constants.py` | Tool names, memory limits, context windows, provider ordering, embedding defaults, Redis/lock prefixes |
| `letta/system.py` | `get_heartbeat()`, `package_user_message()`, `package_function_response()`, `package_summarize_message()` |
| `letta/settings.py` | `ToolSettings`, `SummarizerSettings`, `ModelSettings`, global `settings` |
| `letta/errors.py` | `LettaError`, `LLMError`, `ContextWindowExceededError`, `ConversationBusyError`, `PendingApprovalError` |
| `letta/utils.py` | Token counting, JSON parsing, tool call ID generation, file validation |

### Client SDK

| File | Purpose |
|---|---|
| `letta/client/streaming.py` | SSE-based streaming: `_sse_post()` yields `LettaStreamingResponse` chunks |
| `letta/client/utils.py` | `pprint()` for Jupyter, function name extraction |

Note: The primary client SDK is the separate `letta-client` PyPI package (imported as `letta_client`).

---

## Test Suite

**Key directory:** `tests/` (~119 test files)

| Category | Examples |
|---|---|
| Unit tests | `test_memory.py`, `test_tool_rule_solver.py`, `test_utils.py`, `test_schema_validator.py`, `test_mcp_encryption.py` |
| Integration | `integration_test_send_message.py`, `integration_test_mcp.py`, `integration_test_multi_agent.py`, `integration_test_builtin_tools.py`, `integration_test_sleeptime_agent.py` |
| SDK tests | `sdk/` directory |
| Manager tests | `managers/` directory |
| Performance | `performance_tests/` |
| Config | `conftest.py`, `config.py`, `constants.py` |
| Test data | `data/`, `test_agent_files/`, `test_tool_schema_parsing_files/` |

---

## API Documentation

**Key directory:** `fern/`

| File | Purpose |
|---|---|
| `openapi.json` | Full OpenAPI 3.0 specification for the Letta REST API |
| `openapi-overrides.yml` | Override rules for API doc generation (Fern) |