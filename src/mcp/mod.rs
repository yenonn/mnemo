use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

impl McpResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Value>, code: i32, message: String) -> Self {
        McpResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(McpError {
                code,
                message,
                data: None,
            }),
        }
    }
}

/// Handle a single MCP JSON-RPC request.
pub fn handle_request(req: McpRequest, agent_id: &str) -> McpResponse {
    match req.method.as_str() {
        "initialize" => handle_initialize(req),
        "tools/list" => handle_tools_list(req),
        "tools/call" => handle_tools_call(req, agent_id),
        _ => McpResponse::error(
            req.id,
            -32601,
            format!("Method not found: {}", req.method),
        ),
    }
}

fn handle_initialize(req: McpRequest) -> McpResponse {
    McpResponse::success(
        req.id,
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "mnemo",
                "version": "0.1.0"
            }
        }),
    )
}

fn handle_tools_list(req: McpRequest) -> McpResponse {
    McpResponse::success(
        req.id,
        json!({
            "tools": [
                {
                    "name": "remember",
                    "description": "Store a memory explicitly. Use this when the user states a clear fact or preference.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "The factual memory to store"
                            },
                            "memory_type": {
                                "type": "string",
                                "enum": ["working", "episodic", "semantic"],
                                "description": "Working = seconds-minutes, Episodic = hours-months, Semantic = permanent"
                            },
                            "importance": {
                                "type": "number",
                                "description": "0.0-1.0 importance score"
                            }
                        },
                        "required": ["content", "memory_type"]
                    }
                },
                {
                    "name": "recall",
                    "description": "Retrieve memories matching a query. Call this before responding to load relevant context.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "What to search for, e.g. 'dark mode'"
                            },
                            "memory_type": {
                                "type": "string",
                                "description": "Optional filter: working, episodic, semantic"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Max results to return"
                            }
                        },
                        "required": ["query"]
                    }
                },
                {
                    "name": "extract",
                    "description": "Automatically extract memories from natural language text. Use this when the user writes prose and you want to infer what to remember.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "text": {
                                "type": "string",
                                "description": "Natural language text to extract memories from"
                            }
                        },
                        "required": ["text"]
                    }
                },
                {
                    "name": "status",
                    "description": "Show how many memories are stored per tier.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "forget",
                    "description": "Delete a memory by its ID.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "The memory ID to delete, e.g. mem-abc123"
                            }
                        },
                        "required": ["id"]
                    }
                },
                {
                    "name": "bind",
                    "description": "Process a natural language message and automatically retrieve relevant memories or store new ones based on intent detection.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "text": {
                                "type": "string",
                                "description": "Natural language text to process"
                            }
                        },
                        "required": ["text"]
                    }
                }
            ]
        }),
    )
}

fn handle_tools_call(req: McpRequest, agent_id: &str) -> McpResponse {
    let params = match req.params {
        Some(p) => p,
        None => {
            return McpResponse::error(req.id, -32602, "Missing params".to_string());
        }
    };

    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("");

    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or(json!({}));

    match name {
        "remember" => handle_remember(req.id, args, agent_id),
        "recall" => handle_recall(req.id, args, agent_id),
        "extract" => handle_extract(req.id, args, agent_id),
        "bind" => handle_bind(req.id, args, agent_id),
        "status" => handle_status(req.id, agent_id),
        "forget" => handle_forget(req.id, args, agent_id),
        _ => McpResponse::error(
            req.id,
            -32602,
            format!("Unknown tool: {}", name),
        ),
    }
}

fn get_db_path(agent_id: &str) -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".mnemo")
        .join(agent_id)
        .join("memory.db")
}

fn ensure_db_dir(db_path: &std::path::Path) {
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}

fn handle_remember(
    id: Option<serde_json::Value>,
    args: serde_json::Value,
    agent_id: &str,
) -> McpResponse {
    let content = args
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("");
    let memory_type = args
        .get("memory_type")
        .and_then(|m| m.as_str())
        .unwrap_or("semantic");
    let importance = args
        .get("importance")
        .and_then(|i| i.as_f64())
        .unwrap_or(0.5);

    if content.is_empty() {
        return McpResponse::error(id, -32602, "Missing content".to_string());
    }

    let db_path = get_db_path(agent_id);
    ensure_db_dir(&db_path);

    match crate::store::MnemoDb::new(&db_path) {
        Ok(db) => {
            let mut manager = match crate::tier::TierManager::new(db.conn(), 100) {
                Ok(m) => m,
                Err(e) => {
                    return McpResponse::error(id, -32603, format!("DB error: {}", e));
                }
            };

            let result = match memory_type {
                "working" => manager.remember_working(content),
                "episodic" => manager.remember_episodic(content, importance),
                "semantic" | _ => manager.remember_semantic(content, importance, &[]),
            };

            match result {
                Ok(mem_id) => McpResponse::success(
                    id,
                    json!({
                        "content": [{"type": "text", "text": format!("Stored memory: {}", mem_id)}]
                    }),
                ),
                Err(e) => McpResponse::error(id, -32603, format!("Store error: {}", e)),
            }
        }
        Err(e) => McpResponse::error(id, -32603, format!("DB init error: {}", e)),
    }
}

fn handle_recall(
    id: Option<serde_json::Value>,
    args: serde_json::Value,
    agent_id: &str,
) -> McpResponse {
    let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
    let memory_type = args.get("memory_type").and_then(|m| m.as_str());
    let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

    if query.is_empty() {
        return McpResponse::error(id, -32602, "Missing query".to_string());
    }

    let db_path = get_db_path(agent_id);
    ensure_db_dir(&db_path);

    match crate::store::MnemoDb::new(&db_path) {
        Ok(db) => {
            let manager = match crate::tier::TierManager::new(db.conn(), 100) {
                Ok(m) => m,
                Err(e) => {
                    return McpResponse::error(id, -32603, format!("DB error: {}", e));
                }
            };

            let types_to_search: Vec<String> = match memory_type {
                Some(t) => vec![t.to_string()],
                None => vec!["episodic".to_string(), "semantic".to_string()],
            };

            match manager.recall(query, &types_to_search, limit) {
                Ok(memories) => {
                    let memory_texts: Vec<String> = memories
                        .iter()
                        .map(|m| {
                            format!(
                                "[{}] {} (confidence: {})",
                                m.memory_type, m.content, m.confidence
                            )
                        })
                        .collect();

                    let text = if memory_texts.is_empty() {
                        "No memories found.".to_string()
                    } else {
                        format!("Found {} memories:\n{}", memory_texts.len(), memory_texts.join("\n"))
                    };

                    McpResponse::success(
                        id,
                        json!({
                            "content": [{"type": "text", "text": text}]
                        }),
                    )
                }
                Err(e) => McpResponse::error(id, -32603, format!("Recall error: {}", e)),
            }
        }
        Err(e) => McpResponse::error(id, -32603, format!("DB init error: {}", e)),
    }
}

fn handle_extract(
    id: Option<serde_json::Value>,
    args: serde_json::Value,
    agent_id: &str,
) -> McpResponse {
    let text = args.get("text").and_then(|t| t.as_str()).unwrap_or("");
    if text.is_empty() {
        return McpResponse::error(id, -32602, "Missing text".to_string());
    }

    let config = crate::extract::OpenAiConfig::from_env();
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => return McpResponse::error(id, -32603, format!("Runtime error: {}", e)),
    };

    let results = match rt.block_on(crate::extract::extract_memories(text, config.as_ref())) {
        Ok(r) => r,
        Err(e) => {
            return McpResponse::error(
                id,
                -32603,
                format!("Extraction error: {}", e),
            );
        }
    };

    let db_path = get_db_path(agent_id);
    ensure_db_dir(&db_path);

    match crate::store::MnemoDb::new(&db_path) {
        Ok(db) => {
            let mut manager = match crate::tier::TierManager::new(db.conn(), 100) {
                Ok(m) => m,
                Err(e) => {
                    return McpResponse::error(id, -32603, format!("DB error: {}", e));
                }
            };

            let mut stored_ids = Vec::new();
            for result in results {
                let store_result = match result.tier.as_str() {
                    "working" => manager.remember_working(&result.content),
                    "episodic" => manager.remember_episodic(&result.content, result.importance),
                    "semantic" | _ => {
                        manager.remember_semantic(&result.content,
                            result.importance,
                            &[],
                        )
                    }
                };

                if let Ok(mem_id) = store_result {
                    stored_ids.push(mem_id);
                }
            }

            let text = format!(
                "Extracted and stored {} memories: {}",
                stored_ids.len(),
                stored_ids.join(", ")
            );
            McpResponse::success(
                id,
                json!({
                    "content": [{"type": "text", "text": text}]
                }),
            )
        }
        Err(e) => McpResponse::error(id, -32603, format!("DB init error: {}", e)),
    }
}

fn handle_status(id: Option<serde_json::Value>, agent_id: &str) -> McpResponse {
    let db_path = get_db_path(agent_id);
    ensure_db_dir(&db_path);

    match crate::store::MnemoDb::new(&db_path) {
        Ok(db) => {
            let manager = match crate::tier::TierManager::new(db.conn(), 100) {
                Ok(m) => m,
                Err(e) => {
                    return McpResponse::error(id, -32603, format!("DB error: {}", e));
                }
            };

            let text = format!(
                "Working: {} | Episodic: {} | Semantic: {}",
                manager.working_count(),
                manager.episodic_count().unwrap_or(0),
                manager.semantic_count().unwrap_or(0)
            );

            McpResponse::success(
                id,
                json!({
                    "content": [{"type": "text", "text": text}]
                }),
            )
        }
        Err(e) => McpResponse::error(id, -32603, format!("DB init error: {}", e)),
    }
}

fn handle_bind(
    id: Option<serde_json::Value>,
    args: serde_json::Value,
    agent_id: &str,
) -> McpResponse {
    let text = args.get("text").and_then(|t| t.as_str()).unwrap_or("");
    if text.is_empty() {
        return McpResponse::error(id, -32602, "Missing text".to_string());
    }

    let db_path = get_db_path(agent_id);
    ensure_db_dir(&db_path);

    match crate::store::MnemoDb::new(&db_path) {
        Ok(db) => {
            use crate::context::{analyze_intent, build_query};
            use crate::tier::TierManager;

            if let Some(intent) = analyze_intent(text) {
                let query = build_query(&intent);
                let mut manager = match TierManager::new(db.conn(), 100) {
                    Ok(m) => m,
                    Err(e) => return McpResponse::error(id, -32603, format!("DB error: {}", e)),
                };

                let types_to_search = vec!["working".to_string(), "episodic".to_string(), "semantic".to_string()];

                match manager.recall(&query, &types_to_search, 20) {
                    Ok(memories) => {
                        let memory_texts: Vec<String> = memories
                            .iter()
                            .map(|m| format!("[{}] {}", m.memory_type, m.content))
                            .collect();

                        let retrieved = if memory_texts.is_empty() {
                            "No matching memories found.".to_string()
                        } else {
                            format!("Found {} memories:\n{}", memory_texts.len(), memory_texts.join("\n"))
                        };

                        let _ = manager.remember_working(&format!("User asked: {}", text));

                        let response_text = format!(
                            "{}\n\nQuery intent: {:?} (confidence: {:.2})",
                            retrieved, intent.intent_type, intent.confidence
                        );

                        McpResponse::success(
                            id,
                            json!({
                                "content": [{"type": "text", "text": response_text}]
                            }),
                        )
                    }
                    Err(e) => McpResponse::error(id, -32603, format!("Recall error: {}", e)),
                }
            } else {
                McpResponse::success(
                    id,
                    json!({
                        "content": [{"type": "text", "text": "No clear retrieval intent detected. Message stored in working memory."}]
                    }),
                )
            }
        }
        Err(e) => McpResponse::error(id, -32603, format!("DB init error: {}", e)),
    }
}

fn handle_forget(
    id: Option<serde_json::Value>,
    args: serde_json::Value,
    agent_id: &str,
) -> McpResponse {
    let mem_id = args.get("id").and_then(|i| i.as_str()).unwrap_or("");
    if mem_id.is_empty() {
        return McpResponse::error(id, -32602, "Missing id".to_string());
    }

    let db_path = get_db_path(agent_id);
    ensure_db_dir(&db_path);

    match crate::store::MnemoDb::new(&db_path) {
        Ok(db) => {
            let store = crate::store::MemoryStore::new(db.conn());
            match store.delete(mem_id) {
                Ok(()) => McpResponse::success(
                    id,
                    json!({
                        "content": [{"type": "text", "text": format!("Deleted memory {}", mem_id)}]
                    }),
                ),
                Err(e) => McpResponse::error(id, -32603, format!("Delete error: {}", e)),
            }
        }
        Err(e) => McpResponse::error(id, -32603, format!("DB init error: {}", e)),
    }
}

/// Run the MCP server reading from stdin and writing to stdout.
/// This is the standard MCP stdio transport.
pub fn serve_stdio(agent_id: &str) {
    use std::io::{self, BufRead, Write};

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }

                match serde_json::from_str::<McpRequest>(&line) {
                    Ok(req) => {
                        let resp = handle_request(req, agent_id);
                        if let Ok(json) = serde_json::to_string(&resp) {
                            let _ = writeln!(stdout_lock, "{}", json);
                            let _ = stdout_lock.flush();
                        }
                    }
                    Err(e) => {
                        let resp = McpResponse::error(
                            None,
                            -32700,
                            format!("Parse error: {}", e),
                        );
                        if let Ok(json) = serde_json::to_string(&resp) {
                            let _ = writeln!(stdout_lock, "{}", json);
                            let _ = stdout_lock.flush();
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }
}
