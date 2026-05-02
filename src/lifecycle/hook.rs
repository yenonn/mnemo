use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum HookResult {
    SessionEnd { consolidated_count: usize, new_episodic_id: Option<String> },
    SessionStart { recalled_count: usize },
    Overflow { consolidated_count: usize },
    Decay { affected_count: usize },
    None,
}

impl fmt::Display for HookResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HookResult::SessionEnd { consolidated_count, new_episodic_id } => {
                write!(f, "[session-end] Consolidated {} memories", consolidated_count)?;
                if let Some(id) = new_episodic_id {
                    write!(f, " to {}", id)?;
                }
                Ok(())
            }
            HookResult::SessionStart { recalled_count } => {
                write!(f, "[session-start] Recalled {} memories", recalled_count)
            }
            HookResult::Overflow { consolidated_count } => {
                write!(f, "[overflow] Consolidated {} memories (buffer full)", consolidated_count)
            }
            HookResult::Decay { affected_count } => {
                write!(f, "[decay] {} memories decayed", affected_count)
            }
            HookResult::None => write!(f, "[lifecycle] No action"),
        }
    }
}