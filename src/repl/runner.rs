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

        match manager.recall(&query, &types_to_search, limit) {
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
        use crate::extract::{OpenAiConfig, extract_memories, ExtractResult};
        use crate::tier::TierManager;

        let config = OpenAiConfig::from_env();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let results: Vec<ExtractResult> = match rt.block_on(extract_memories(&text, config.as_ref())) {
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
            message: format!("Extracted and stored {} memories: {}", stored_ids.len(), stored_ids.join(", ")),
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
}
