use super::commands::Command;
use std::error::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error: {}", self.message)
    }
}

impl Error for ParseError {}

pub fn parse_command(input: &str) -> Result<Command, Box<dyn Error>> {
    let trimmed = input.trim().trim_end_matches(';').trim();
    if trimmed.is_empty() {
        return Err(Box::new(ParseError {
            message: "Empty command".to_string(),
        }));
    }

    let tokens = tokenize(trimmed)?;
    let verb = tokens.first().unwrap_or(&"".to_string()).to_uppercase();

    match verb.as_str() {
        "INIT" => Ok(Command::Init),
        "REMEMBER" => parse_remember(&tokens),
        "RECALL" => parse_recall(&tokens),
        "FORGET" => parse_forget(&tokens),
        "CONSOLIDATE" => parse_consolidate(&tokens),
        "REFLECT" => parse_reflect(&tokens),
        "STATUS" => Ok(Command::Status),
        "EXTRACT" => parse_extract(&tokens),
        "PRAGMA" => parse_pragma(&tokens),
        "BIND" => parse_bind(&tokens),
        _ => Err(Box::new(ParseError {
            message: format!("Unknown command: {}", verb),
        })),
    }
}

fn tokenize(input: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in input.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
            current.push(c);
        } else if c.is_whitespace() && !in_quotes {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(c);
        }
    }

    if in_quotes {
        return Err(Box::new(ParseError {
            message: "Unclosed quote".to_string(),
        }));
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

fn parse_remember(tokens: &[String]) -> Result<Command, Box<dyn Error>> {
    let content = extract_quoted_string(tokens)?;
    let memory_type = extract_after_keyword(tokens, "AS")?;
    let metadata = if tokens.iter().any(|t| t == "WITH") {
        extract_metadata(tokens, "WITH")?
    } else {
        vec![]
    };

    Ok(Command::Remember {
        content,
        memory_type,
        metadata,
    })
}

fn parse_recall(tokens: &[String]) -> Result<Command, Box<dyn Error>> {
    let query = extract_quoted_string(tokens)?;
    let memory_types = if tokens.iter().any(|t| t == "FROM") {
        extract_list_after(tokens, "FROM", &["LIMIT", "WHERE", "ORDER"])?
    } else {
        vec![]
    };
    let limit = if tokens.iter().any(|t| t == "LIMIT") {
        extract_after_keyword(tokens, "LIMIT")?.parse::<usize>()?
    } else {
        10
    };

    Ok(Command::Recall {
        query,
        memory_types,
        conditions: vec![],
        limit,
    })
}

fn parse_forget(tokens: &[String]) -> Result<Command, Box<dyn Error>> {
    if tokens.get(1).map(|s| s.as_str()) == Some("id(")
        && tokens.get(3).map(|s| s.as_str()) == Some(")")
    {
        let id = tokens.get(2).unwrap_or(&"".to_string()).to_string();
        return Ok(Command::Forget {
            id: Some(id),
            conditions: vec![],
        });
    }
    Err(Box::new(ParseError {
        message: "FORGET parsing not fully implemented".to_string(),
    }))
}

fn parse_consolidate(tokens: &[String]) -> Result<Command, Box<dyn Error>> {
    if tokens.len() >= 4 && tokens.get(2).map(|s| s.as_str()) == Some("TO") {
        let from = tokens.get(1).unwrap_or(&"".to_string()).to_string();
        let to = tokens.get(3).unwrap_or(&"".to_string()).to_string();
        return Ok(Command::Consolidate {
            from,
            to,
            conditions: vec![],
        });
    }
    Err(Box::new(ParseError {
        message: "CONSOLIDATE parsing not fully implemented".to_string(),
    }))
}

fn parse_reflect(tokens: &[String]) -> Result<Command, Box<dyn Error>> {
    if tokens.len() == 1 {
        Ok(Command::Reflect {
            memory_type: None,
            conditions: vec![],
        })
    } else {
        Err(Box::new(ParseError {
            message: "REFLECT parsing not fully implemented".to_string(),
        }))
    }
}

fn parse_pragma(tokens: &[String]) -> Result<Command, Box<dyn Error>> {
    if tokens.len() == 1 {
        return Ok(Command::Pragma {
            key: None,
            value: None,
        });
    } else if tokens.get(2).map(|s| s.as_str()) == Some("=") {
        let key = tokens.get(1).unwrap_or(&"".to_string()).to_string();
        let value = tokens.get(3).unwrap_or(&"".to_string()).to_string();
        return Ok(Command::Pragma {
            key: Some(key),
            value: Some(value),
        });
    }
    Err(Box::new(ParseError {
        message: "PRAGMA parsing not fully implemented".to_string(),
    }))
}

fn parse_extract(tokens: &[String]) -> Result<Command, Box<dyn Error>> {
    let text = extract_quoted_string(tokens)?;
    Ok(Command::Extract { text })
}

fn parse_bind(tokens: &[String]) -> Result<Command, Box<dyn Error>> {
    let text = extract_quoted_string(tokens)?;
    Ok(Command::Bind { text })
}

// --- Helper functions ---

fn extract_quoted_string(tokens: &[String]) -> Result<String, Box<dyn Error>> {
    tokens
        .iter()
        .find(|t| t.starts_with('"') && t.ends_with('"'))
        .map(|t| t[1..t.len() - 1].to_string())
        .ok_or_else(|| {
            Box::new(ParseError {
                message: "Expected quoted string".to_string(),
            }) as Box<dyn Error>
        })
}

fn extract_after_keyword(tokens: &[String], keyword: &str) -> Result<String, Box<dyn Error>> {
    tokens
        .iter()
        .position(|t| t == keyword)
        .and_then(|idx| tokens.get(idx + 1))
        .map(|t| t.trim_end_matches(',').to_string())
        .ok_or_else(|| {
            Box::new(ParseError {
                message: format!("Expected value after {}", keyword),
            }) as Box<dyn Error>
        })
}

fn extract_metadata(
    tokens: &[String],
    keyword: &str,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let mut result = Vec::new();
    if let Some(idx) = tokens.iter().position(|t| t == keyword) {
        for token in &tokens[idx + 1..] {
            if token == ";" {
                break;
            }
            if let Some(pos) = token.find('=') {
                let key = token[..pos].trim_end_matches(',').to_string();
                let value = token[pos + 1..].trim_end_matches(',').to_string();
                result.push((key, value));
            }
        }
    }
    Ok(result)
}

fn extract_list_after(
    tokens: &[String],
    keyword: &str,
    stop_words: &[&str],
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut result = Vec::new();
    if let Some(idx) = tokens.iter().position(|t| t == keyword) {
        for token in &tokens[idx + 1..] {
            if stop_words.contains(&token.as_str()) {
                break;
            }
            result.push(token.trim_end_matches(',').to_string());
        }
    }
    Ok(result)
}
