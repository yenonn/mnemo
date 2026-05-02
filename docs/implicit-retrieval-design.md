# Implicit Memory Retrieval Design

## Problem

Current mnemo requires **explicit API calls** for memory operations. When a user says:

> "What were my todos yesterday?"

The agent must know to call `recall "todos yesterday"`. This breaks natural conversation flow.

## Solution: `BIND` Command

The `BIND` command enables **implicit context retrieval** — natural language messages are automatically analyzed for intent, and relevant memories are retrieved without explicit tool calls.

## Architecture

```
User Message
    │
    ▼
┌─────────────────┐
│  Intent Engine  │  ← Keyword matching + heuristic scoring
│                 │
│  Score: 0.0-1.0 │
└────────┬────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
Retrieve   Store
(RECALL)   (EXTRACT)
    │         │
    ▼         ▼
Inject     Auto-store
memories   in semantic
into LLM   tier
context
```

## Intent Detection

### Retrieval Signals (score +0.2 each)
- Question words: "what", "tell me", "show me", "list"
- Time references: "yesterday", "last time", "recently"
- Possession: "my todos", "my preferences", "i have"
- Question mark at end

### Store Signals
- Statements starting with "I prefer", "I like", "I use"
- Contains "remember", "note"
- Does NOT contain retrieval keywords

### Confidence Thresholds
- > 0.8: Retrieve all relevant memories (auto-RECALL)
- > 0.4: Retrieve by topic
- < 0.3: Store in working memory, return neutral

## Usage Flow

### 1. Agent Configuration (MCP)

Instead of calling separate tools, the agent uses a single `bind` tool for ALL user messages:

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

### 2. Per-Message Flow

```
User: "What were my todos yesterday?"
     │
     ▼
Agent → mnemo.bind("What were my todos yesterday?")
     │
     ▼
m nemo detects:
  - Question pattern (+0.3)
  - "what" keyword (+0.2)
  - "yesterday" time ref (+0.25)
  - "my" possession (+0.3)
  ────────────────────────
  Total: 1.05 → RETRIEVE
     │
     ▼
Auto-calls: recall("yesterday todos")
     │
     ▼
Returns: "Found 3 memories:
          [episodic] ✅ Review PR #123
          [episodic] ✅ Email boss about vacation
          [episodic] ⏳ Fix login bug"
     │
     ▼
Agent LLM uses retrieved context in system prompt
→ Responds: "Yesterday you completed..."
```

### 3. Store Flow (Implicit Storage)

```
User: "I prefer dark mode for everything"
     │
     ▼
Agent → mnemo.bind("I prefer dark mode for everything")
     │
     ▼
m nemo detects:
  - "I prefer" store signal
  - No retrieval keywords
  → STORE intent
     │
     ▼
Auto-extracts: "User prefers dark mode" → semantic
Returns: "Stored memory: mem-abc123"
     │
     ▼
Agent: "Got it, I'll remember that you prefer dark mode"
```

## CLI Usage

```bash
# Explicit (old way)
mnemo recall "todos yesterday"

# Implicit (new way)
mnemo bind "What were my todos yesterday?"

# In REPL
mnemo --repl
mnemo> BIND "My todos from yesterday?";
# → Found 3 memories automatically

mnemo> BIND "I like using vim";
# → Extracted and stored 1 memory
```

## MCP Integration

The `bind` tool is exposed as an MCP tool:

```json
{
  "name": "bind",
  "description": "Process natural language and auto-retrieve/store memories",
  "inputSchema": {
    "type": "object",
    "properties": {
      "text": {
        "type": "string",
        "description": "Natural language text to analyze"
      }
    },
    "required": ["text"]
  }
}
```

## Agent Pattern

The recommended pattern for AI agents:

```rust
// Pseudo-code for agent integration
fn handle_user_message(text: &str) -> String {
    // Step 1: Always call bind before responding
    let context = mnemo_bind(text);
    
    // Step 2: Inject context into LLM prompt
    let prompt = format!(
        "Relevant memories:\n{}\n\nUser: {}\nAssistant:",
        context, text
    );
    
    // Step 3: Generate response with memory context
    llm_generate(prompt)
}
```

## Benefits

1. **Natural conversation**: No explicit "search my memory" prompts
2. **Always-on context**: Every message checked for memory relevance
3. **Dual purpose**: Both retrieval AND storage in one call
4. **Tier awareness**: Automatically promotes context through tiers
5. **Agent-agnostic**: Works with any MCP-compatible agent

## Future Enhancements

- LLM-powered intent classification (vs current heuristics)
- Embedding-based semantic similarity for better recall
- Automatic consolidation scheduling
- Memory decay and refresh strategies
