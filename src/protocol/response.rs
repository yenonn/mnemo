use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Ok { message: String },
    Error { code: String, message: String },
    Memory {
        id: String,
        memory_type: String,
        confidence: f64,
        importance: f64,
        score: Option<f64>,
        content: String,
        status: Option<String>,
    },
    ResultSet { count: usize, memories: Vec<Response> },
    Status {
        agent_id: String,
        db_path: String,
        db_size_kb: u64,
        working_count: usize,
        episodic_count: usize,
        semantic_count: usize,
        vector_indexed: usize,
        pending_embeddings: usize,
    },
    Config { entries: Vec<(String, String)> },
    Reflect {
        total_episodic: usize,
        total_semantic: usize,
        low_confidence: usize,
        contradictions: Vec<String>,
        stale: usize,
    },
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Response::Ok { message } => {
                writeln!(f, "<ok>")?;
                writeln!(f, "  {}", message)?;
                writeln!(f, "</ok>")
            }
            Response::Error { code, message } => {
                writeln!(f, "<error code=\"{}\">", code)?;
                writeln!(f, "  {}", message)?;
                writeln!(f, "</error>")
            }
            Response::Memory { id, memory_type, confidence, importance, score, content, status } => {
                write!(f, "<memory id=\"{}\" type=\"{}\" confidence=\"{}\" importance=\"{}\"",
                       id, memory_type, confidence, importance)?;
                if let Some(s) = score {
                    write!(f, " score=\"{}\"", s)?;
                }
                if let Some(s) = status {
                    write!(f, " status=\"{}\"", s)?;
                }
                writeln!(f, ">")?;
                writeln!(f, "  {}", content)?;
                writeln!(f, "</memory>")
            }
            Response::ResultSet { count, memories } => {
                writeln!(f, "<result count=\"{}\">", count)?;
                for mem in memories {
                    write!(f, "  {}", indent(mem.to_string()))?;
                }
                writeln!(f, "</result>")
            }
            Response::Status { agent_id, db_path, db_size_kb, working_count, episodic_count, semantic_count, vector_indexed, pending_embeddings } => {
                writeln!(f, "<status>")?;
                writeln!(f, "  Agent: {}", agent_id)?;
                writeln!(f, "  Database: {} ({} KB)", db_path, db_size_kb)?;
                writeln!(f, "  Working buffer: {}", working_count)?;
                writeln!(f, "  Episodic memories: {}", episodic_count)?;
                writeln!(f, "  Semantic memories: {}", semantic_count)?;
                writeln!(f, "  Vector indexed: {} ({} pending)", vector_indexed, pending_embeddings)?;
                writeln!(f, "</status>")
            }
            Response::Config { entries } => {
                writeln!(f, "<config>")?;
                for (k, v) in entries {
                    writeln!(f, "  {} = {}", k, v)?;
                }
                writeln!(f, "</config>")
            }
            Response::Reflect { total_episodic, total_semantic, low_confidence, contradictions, stale } => {
                writeln!(f, "<analysis>")?;
                writeln!(f, "  Memory count: {} episodic, {} semantic", total_episodic, total_semantic)?;
                writeln!(f, "  Low-confidence items: {}", low_confidence)?;
                if !contradictions.is_empty() {
                    writeln!(f, "  Potential contradictions: {}", contradictions.len())?;
                    for c in contradictions {
                        writeln!(f, "    {}", c)?;
                    }
                }
                writeln!(f, "  Stale memories: {}", stale)?;
                writeln!(f, "</analysis>")
            }
        }
    }
}

fn indent(s: String) -> String {
    s.lines().map(|line| format!("  {}", line)).collect::<Vec<_>>().join("\n")
}
