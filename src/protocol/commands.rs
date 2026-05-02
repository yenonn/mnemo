use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Init,
    Remember {
        content: String,
        memory_type: String,
        metadata: Vec<(String, String)>,
    },
    Recall {
        query: String,
        memory_types: Vec<String>,
        conditions: Vec<(String, String, String)>, // (field, op, value)
        limit: usize,
    },
    Forget {
        id: Option<String>,
        conditions: Vec<(String, String, String)>,
    },
    Consolidate {
        from: String,
        to: String,
        conditions: Vec<(String, String, String)>,
    },
    Reflect {
        memory_type: Option<String>,
        conditions: Vec<(String, String, String)>,
    },
    Status,
    Extract {
        text: String,
    },
    Bind {
        text: String,
    },
    Pragma {
        key: Option<String>,
        value: Option<String>,
    },
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Init => write!(f, "INIT"),
            Command::Remember { content, memory_type, metadata } => {
                write!(f, "REMEMBER \"{}\" AS {}", content, memory_type)?;
                if !metadata.is_empty() {
                    let meta_str: Vec<String> = metadata
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect();
                    write!(f, " WITH {}", meta_str.join(", "))?;
                }
                Ok(())
            },
            Command::Recall { query, memory_types, conditions, limit } => {
                write!(f, "RECALL \"{}\"", query)?;
                if !memory_types.is_empty() {
                    write!(f, " FROM {}", memory_types.join(", "))?;
                }
                if !conditions.is_empty() {
                    let cond_str: Vec<String> = conditions
                        .iter()
                        .map(|(field, op, value)| format!("{} {} {}", field, op, value))
                        .collect();
                    write!(f, " WHERE {}", cond_str.join(" AND "))?;
                }
                write!(f, " LIMIT {}", limit)
            },
            Command::Forget { id, .. } => match id {
                Some(i) => write!(f, "FORGET id({})", i),
                None => write!(f, "FORGET WHERE ..."),
            },
            Command::Consolidate { from, to, .. } => write!(f, "CONSOLIDATE {} TO {}", from, to),
            Command::Reflect { .. } => write!(f, "REFLECT"),
            Command::Status => write!(f, "STATUS"),
            Command::Extract { text } => {
                write!(f, "EXTRACT \"{}\"", text)
            },
            Command::Bind { text } => {
                write!(f, "BIND \"{}\"", text)
            },
            Command::Pragma { key, value } => match (key, value) {
                (Some(k), Some(v)) => write!(f, "PRAGMA {} = {}", k, v),
                _ => write!(f, "PRAGMA"),
            },
        }
    }
}