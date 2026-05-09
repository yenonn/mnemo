use crate::protocol::{parse_command, Command, Response};
use crate::store::{ConfigStore, MnemoDb};
use crate::tier::TierManager;
use std::io::{self, BufRead, Write};

pub struct Repl {
    db: MnemoDb,
    agent_id: String,
}

impl Repl {
    pub fn new(agent_id: &str) -> anyhow::Result<Self> {
        let db_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".mnemo")
            .join(agent_id)
            .join("memory.db");

        std::fs::create_dir_all(db_path.parent().unwrap())?;
        let db = MnemoDb::new(&db_path)?;

        Ok(Repl {
            db,
            agent_id: agent_id.to_string(),
        })
    }

    pub fn run(&mut self) {
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        loop {
            write!(stdout, "mnemo> ").unwrap();
            stdout.flush().unwrap();

            let mut line = String::new();
            if stdin.lock().read_line(&mut line).unwrap() == 0 {
                break;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("--") {
                continue;
            }
            if trimmed == "quit" || trimmed == "exit" {
                break;
            }

            match parse_command(trimmed) {
                Ok(cmd) => {
                    let response = self.execute(cmd);
                    println!("{}", response);
                }
                Err(e) => {
                    println!(
                        "{}",
                        Response::Error {
                            code: "INVALID_SYNTAX".to_string(),
                            message: e.to_string(),
                        }
                    );
                }
            }
        }
    }

    pub fn execute(&mut self, cmd: Command) -> Response {
        use Command::*;

        // Run lifecycle hooks before every command
        let hook_results = if let Ok(mut manager) = TierManager::new(self.db.conn(), 100) {
            crate::lifecycle::LifecycleEngine::check_and_fire(self.db.conn(), &mut manager)
        } else {
            Vec::new()
        };

        // Print lifecycle messages
        for hook in &hook_results {
            println!("<lifecycle>\n  {}\n</lifecycle>", hook);
        }

        match cmd {
            Init => self.cmd_init(),
            Remember {
                content,
                memory_type,
                metadata,
            } => self.cmd_remember(content, memory_type, metadata),
            Recall {
                query,
                memory_types,
                limit,
                ..
            } => self.cmd_recall(query, memory_types, limit),
            Status => self.cmd_status(),
            Forget { id, .. } => self.cmd_forget(id),
            Extract { text } => self.cmd_extract(text),
            Bind { text } => self.cmd_bind(text),
            Pragma { key, value } => self.cmd_pragma(key, value),
            Consolidate { from, to, .. } => self.cmd_consolidate(from, to),
            Reflect { .. } => self.cmd_reflect(),
        }
    }

    fn cmd_init(&self) -> Response {
        Response::Ok {
            message: format!("Database initialized at agent {}", self.agent_id),
        }
    }

    fn cmd_remember(
        &mut self,
        content: String,
        memory_type: String,
        _metadata: Vec<(String, String)>,
    ) -> Response {
        let mut manager = TierManager::new(self.db.conn(), 100).unwrap();

        let result = match memory_type.as_str() {
            "working" => manager.remember_working(&content),
            "episodic" => manager.remember_episodic(&content, 0.5),
            "semantic" => manager.remember_semantic(&content, 0.7, &[]),
            _ => {
                return Response::Error {
                    code: "INVALID_TYPE".to_string(),
                    message: format!("Unknown memory type: {}", memory_type),
                }
            }
        };

        match result {
            Ok(id) => Response::Memory {
                id,
                memory_type,
                confidence: 1.0,
                importance: 0.5,
                score: None,
                content,
                status: Some("text-indexed".to_string()),
            },
            Err(e) => Response::Error {
                code: "DB_ERROR".to_string(),
                message: e.to_string(),
            },
        }
    }

    fn cmd_recall(&self, query: String, memory_types: Vec<String>, limit: usize) -> Response {
        let manager = TierManager::new(self.db.conn(), 100).unwrap();
        let types_to_search = if memory_types.is_empty() {
            vec!["episodic".to_string(), "semantic".to_string()]
        } else {
            memory_types
        };

        // Split query into terms and expand with synonyms + morphology
        let query_terms: Vec<String> = query.split_whitespace().map(|s| s.to_string()).collect();
        let expanded = crate::context::expand_query(&query_terms);

        match manager.recall_expanded(&expanded, &types_to_search, limit) {
            Ok(memories) => {
                let responses: Vec<Response> = memories
                    .into_iter()
                    .map(|m| Response::Memory {
                        id: m.id,
                        memory_type: m.memory_type,
                        confidence: m.confidence,
                        importance: m.importance,
                        score: None,
                        content: m.content,
                        status: Some("indexed".to_string()),
                    })
                    .collect();

                Response::ResultSet {
                    count: responses.len(),
                    memories: responses,
                }
            }
            Err(e) => Response::Error {
                code: "DB_ERROR".to_string(),
                message: e.to_string(),
            },
        }
    }

    fn cmd_status(&self) -> Response {
        let manager = TierManager::new(self.db.conn(), 100).unwrap();

        Response::Status {
            agent_id: self.agent_id.clone(),
            db_path: format!("~/.mnemo/{}/memory.db", self.agent_id),
            db_size_kb: 0,
            working_count: manager.working_count(),
            episodic_count: manager.episodic_count().unwrap_or(0),
            semantic_count: manager.semantic_count().unwrap_or(0),
            vector_indexed: 0,
            pending_embeddings: 0,
        }
    }

    fn cmd_forget(&self, id: Option<String>) -> Response {
        let store = crate::store::MemoryStore::new(self.db.conn());
        match id {
            Some(i) => match store.delete(&i) {
                Ok(()) => Response::Ok {
                    message: format!("Deleted memory {}", i),
                },
                Err(e) => Response::Error {
                    code: "NOT_FOUND".to_string(),
                    message: e.to_string(),
                },
            },
            None => Response::Error {
                code: "INVALID_SYNTAX".to_string(),
                message: "FORGET requires id".to_string(),
            },
        }
    }

    fn cmd_pragma(&self, key: Option<String>, value: Option<String>) -> Response {
        let store = ConfigStore::new(self.db.conn());

        match (key, value) {
            (Some(k), Some(v)) => match store.set(&k, &v) {
                Ok(()) => Response::Ok {
                    message: format!("Set {} = {}", k, v),
                },
                Err(e) => Response::Error {
                    code: "CONFIG_ERROR".to_string(),
                    message: e.to_string(),
                },
            },
            (Some(k), None) => match store.get(&k) {
                Ok(Some(v)) => Response::Config {
                    entries: vec![(k, v)],
                },
                Ok(None) => Response::Error {
                    code: "CONFIG_ERROR".to_string(),
                    message: format!("Key not found: {}", k),
                },
                Err(e) => Response::Error {
                    code: "CONFIG_ERROR".to_string(),
                    message: e.to_string(),
                },
            },
            (None, None) => match store.get_all() {
                Ok(entries) => Response::Config { entries },
                Err(e) => Response::Error {
                    code: "CONFIG_ERROR".to_string(),
                    message: e.to_string(),
                },
            },
            _ => Response::Error {
                code: "INVALID_SYNTAX".to_string(),
                message: "Invalid PRAGMA syntax".to_string(),
            },
        }
    }

    fn cmd_consolidate(&mut self, from: String, to: String) -> Response {
        let mut manager = TierManager::new(self.db.conn(), 100).unwrap();

        let result = match (from.as_str(), to.as_str()) {
            ("working", "episodic") => manager.consolidate_working_to_episodic(),
            _ => {
                return Response::Error {
                    code: "INVALID_SYNTAX".to_string(),
                    message: format!("Cannot consolidate {} to {}", from, to),
                }
            }
        };

        match result {
            Ok(Some(id)) => Response::Ok {
                message: format!("Consolidated {} to {} -> {}", from, to, id),
            },
            Ok(None) => Response::Ok {
                message: format!("Nothing to consolidate from {} to {}", from, to),
            },
            Err(e) => Response::Error {
                code: "DB_ERROR".to_string(),
                message: e.to_string(),
            },
        }
    }

    fn cmd_extract(&mut self, text: String) -> Response {
        use crate::extract::{extract_memories, ExtractResult, OpenAiConfig};
        use crate::tier::TierManager;

        let config = OpenAiConfig::from_env();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let results: Vec<ExtractResult> =
            match rt.block_on(extract_memories(&text, config.as_ref())) {
                Ok(r) => r,
                Err(e) => {
                    return Response::Error {
                        code: "EXTRACT_ERROR".to_string(),
                        message: e.to_string(),
                    };
                }
            };

        let mut manager = TierManager::new(self.db.conn(), 100).unwrap();
        let mut stored_ids = Vec::new();

        for result in results {
            let store_result = match result.tier.as_str() {
                "working" => manager.remember_working(&result.content),
                "episodic" => manager.remember_episodic(&result.content, result.importance),
                "semantic" => manager.remember_semantic(&result.content, result.importance, &[]),
                _ => manager.remember_semantic(&result.content, result.importance, &[]),
            };

            match store_result {
                Ok(id) => stored_ids.push(id),
                Err(e) => {
                    return Response::Error {
                        code: "DB_ERROR".to_string(),
                        message: e.to_string(),
                    };
                }
            }
        }

        Response::Ok {
            message: format!(
                "Extracted and stored {} memories: {}",
                stored_ids.len(),
                stored_ids.join(", ")
            ),
        }
    }

    fn cmd_reflect(&self) -> Response {
        let manager = TierManager::new(self.db.conn(), 100).unwrap();

        Response::Reflect {
            total_episodic: manager.episodic_count().unwrap_or(0),
            total_semantic: manager.semantic_count().unwrap_or(0),
            low_confidence: 0,
            contradictions: vec![],
            stale: 0,
        }
    }

    fn cmd_bind(&mut self, text: String) -> Response {
        use crate::context::{analyze_intent, build_query, expand_query, has_store_intent};

        // Step 1: Analyze intent - does user want to retrieve or store?
        if let Some(intent) = analyze_intent(&text) {
            let query = build_query(&intent);
            let limit = 20;

            // Expand query terms with synonyms before searching
            let query_terms: Vec<String> =
                query.split_whitespace().map(|s| s.to_string()).collect();
            let expanded = expand_query(&query_terms);

            // Try hybrid search if embedding provider and vec0 are available.
            // Otherwise, fall back to simple expanded FTS5.
            let mut manager = TierManager::new(self.db.conn(), 100).unwrap();
            let types_to_search = vec![
                "working".to_string(),
                "episodic".to_string(),
                "semantic".to_string(),
            ];

            #[cfg(feature = "vec")]
            let memories: Vec<crate::store::Memory> = {
                let vstore = crate::store::VectorStore::new(self.db.conn());
                let gateway = crate::embed::EmbeddingGateway::from_env_cached();

                if vstore.available() {
                    if let Some(gw) = gateway {
                        manager
                            .recall_hybrid(&query, &expanded, &types_to_search, limit, &vstore, gw)
                            .unwrap_or_default()
                    } else {
                        manager
                            .recall_expanded(&expanded, &types_to_search, limit)
                            .unwrap_or_default()
                    }
                } else {
                    manager
                        .recall_expanded(&expanded, &types_to_search, limit)
                        .unwrap_or_default()
                }
            };

            #[cfg(not(feature = "vec"))]
            let memories: Vec<crate::store::Memory> = manager
                .recall_expanded(&expanded, &types_to_search, limit)
                .unwrap_or_default();

            let memory_texts: Vec<String> = memories
                .iter()
                .map(|m| format!("[{}] {}", m.memory_type, m.content))
                .collect();

            let retrieved = if memory_texts.is_empty() {
                "No matching memories found.".to_string()
            } else {
                format!(
                    "Found {} memories:\n{}",
                    memory_texts.len(),
                    memory_texts.join("\n")
                )
            };

            // Also store the conversation turn in working memory
            let _ = manager.remember_working(&format!("User asked: {}", text));

            Response::Ok {
                message: format!(
                    "{}\n\nQuery intent: {:?} (confidence: {:.2})",
                    retrieved, intent.intent_type, intent.confidence
                ),
            }
        } else if has_store_intent(&text) {
            // If no retrieval intent but store intent detected, check config
            let store = ConfigStore::new(self.db.conn());
            let confirmation_needed = store
                .get("auto_remember_confirmation")
                .unwrap_or(None)
                .map(|v| v == "true")
                .unwrap_or(false);

            if confirmation_needed {
                // Extract memories but return them for confirmation instead of storing
                use crate::extract::{extract_memories, OpenAiConfig};
                let config = OpenAiConfig::from_env();
                let rt = tokio::runtime::Runtime::new().unwrap();
                match rt.block_on(extract_memories(&text, config.as_ref())) {
                    Ok(results) if !results.is_empty() => {
                        let extracted: Vec<String> = results
                            .iter()
                            .map(|r| format!("[{} | {:.2}] {}", r.tier, r.importance, r.content))
                            .collect();
                        Response::Ok {
                            message: format!(
                                "Confirmation required: The following memories were extracted but NOT stored. Reply with 'remember' to store them, or ignore to discard:\n\n{}",
                                extracted.join("\n")
                            ),
                        }
                    }
                    _ => Response::Ok {
                        message: "Store intent detected but nothing extracted for confirmation."
                            .to_string(),
                    },
                }
            } else {
                // Auto-store if confirmation is not required
                self.cmd_extract(text)
            }
        } else {
            // No clear intent - store as working memory and return neutral
            let mut manager = TierManager::new(self.db.conn(), 100).unwrap();
            let _ = manager.remember_working(&format!("Conversation: {}", text));

            Response::Ok {
                message:
                    "Message stored in working memory. No clear retrieval or store intent detected."
                        .to_string(),
            }
        }
    }
}
