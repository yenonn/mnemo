use mnemo::protocol::{Command, parse_command};

#[test]
fn test_parse_init() {
    let cmd = parse_command("INIT;").unwrap();
    assert_eq!(cmd, Command::Init);
}

#[test]
fn test_parse_remember() {
    let cmd = parse_command("REMEMBER \"hello world\" AS semantic WITH importance=0.9, tags=ui;").unwrap();
    assert_eq!(cmd, Command::Remember {
        content: "hello world".to_string(),
        memory_type: "semantic".to_string(),
        metadata: vec![
            ("importance".to_string(), "0.9".to_string()),
            ("tags".to_string(), "ui".to_string()),
        ],
    });
}

#[test]
fn test_parse_recall() {
    let cmd = parse_command("RECALL \"dark mode\" FROM semantic LIMIT 5;").unwrap();
    assert_eq!(cmd, Command::Recall {
        query: "dark mode".to_string(),
        memory_types: vec!["semantic".to_string()],
        conditions: vec![],
        limit: 5,
    });
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
