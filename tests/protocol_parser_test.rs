use mnemo::protocol::{parse_command, Command};

#[test]
fn test_parse_init() {
    let cmd = parse_command("INIT;").unwrap();
    assert_eq!(cmd, Command::Init);
}

#[test]
fn test_parse_remember() {
    let cmd = parse_command("REMEMBER \"hello world\" AS semantic WITH importance=0.9, tags=ui;")
        .unwrap();
    assert_eq!(
        cmd,
        Command::Remember {
            content: "hello world".to_string(),
            memory_type: "semantic".to_string(),
            metadata: vec![
                ("importance".to_string(), "0.9".to_string()),
                ("tags".to_string(), "ui".to_string()),
            ],
        }
    );
}

#[test]
fn test_parse_recall() {
    let cmd = parse_command("RECALL \"dark mode\" FROM semantic LIMIT 5;").unwrap();
    assert_eq!(
        cmd,
        Command::Recall {
            query: "dark mode".to_string(),
            memory_types: vec!["semantic".to_string()],
            conditions: vec![],
            limit: 5,
        }
    );
}

#[test]
fn test_parse_status() {
    let cmd = parse_command("STATUS;").unwrap();
    assert_eq!(cmd, Command::Status);
}

#[test]
fn test_invalid_command() {
    let result = parse_command("INVALID;");
    assert!(result.is_err());
}

#[test]
fn test_parse_empty_input() {
    let result = parse_command("   ");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Empty command"));
}

#[test]
fn test_parse_unclosed_quotes() {
    let result = parse_command("REMEMBER \"hello world AS semantic;");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Unclosed quote"));
}

#[test]
fn test_parse_remember_without_with() {
    let cmd = parse_command("REMEMBER \"hello\" AS working;").unwrap();
    assert_eq!(
        cmd,
        Command::Remember {
            content: "hello".to_string(),
            memory_type: "working".to_string(),
            metadata: vec![],
        }
    );
}

#[test]
fn test_parse_forget_with_id() {
    let cmd = parse_command("FORGET id( abc123 );").unwrap();
    assert_eq!(
        cmd,
        Command::Forget {
            id: Some("abc123".to_string()),
            conditions: vec![],
        }
    );
}

#[test]
fn test_parse_forget_invalid() {
    let result = parse_command("FORGET;");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("FORGET parsing not fully implemented"));
}

#[test]
fn test_parse_consolidate() {
    let cmd = parse_command("CONSOLIDATE working TO episodic;").unwrap();
    assert_eq!(
        cmd,
        Command::Consolidate {
            from: "working".to_string(),
            to: "episodic".to_string(),
            conditions: vec![],
        }
    );
}

#[test]
fn test_parse_consolidate_invalid() {
    let result = parse_command("CONSOLIDATE;");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("CONSOLIDATE parsing not fully implemented"));
}

#[test]
fn test_parse_reflect() {
    let cmd = parse_command("REFLECT;").unwrap();
    assert_eq!(
        cmd,
        Command::Reflect {
            memory_type: None,
            conditions: vec![],
        }
    );
}

#[test]
fn test_parse_reflect_invalid() {
    let result = parse_command("REFLECT episodic;");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("REFLECT parsing not fully implemented"));
}

#[test]
fn test_parse_pragma_set() {
    let cmd = parse_command("PRAGMA max_memories = 100;").unwrap();
    assert_eq!(
        cmd,
        Command::Pragma {
            key: Some("max_memories".to_string()),
            value: Some("100".to_string()),
        }
    );
}

#[test]
fn test_parse_pragma_get_all() {
    let cmd = parse_command("PRAGMA;").unwrap();
    assert_eq!(
        cmd,
        Command::Pragma {
            key: None,
            value: None
        }
    );
}

#[test]
fn test_parse_pragma_invalid() {
    let result = parse_command("PRAGMA key_only;");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("PRAGMA parsing not fully implemented"));
}

#[test]
fn test_parse_extract() {
    let cmd = parse_command("EXTRACT \"I prefer dark mode\";").unwrap();
    assert_eq!(
        cmd,
        Command::Extract {
            text: "I prefer dark mode".to_string(),
        }
    );
}

#[test]
fn test_parse_recall_without_types() {
    let cmd = parse_command("RECALL \"query\" LIMIT 10;").unwrap();
    assert_eq!(
        cmd,
        Command::Recall {
            query: "query".to_string(),
            memory_types: vec![],
            conditions: vec![],
            limit: 10,
        }
    );
}

#[test]
fn test_parse_remember_no_quotes_error() {
    let result = parse_command("REMEMBER hello AS semantic;");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Expected quoted string"));
}

#[test]
fn test_parse_recall_no_quotes_error() {
    let result = parse_command("RECALL hello LIMIT 5;");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Expected quoted string"));
}

#[test]
fn test_parse_remember_no_as_error() {
    let result = parse_command("REMEMBER \"hello\" semantic;");
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Expected value after AS"));
}
