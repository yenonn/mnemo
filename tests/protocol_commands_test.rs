use mnemo::protocol::Command;

#[test]
fn test_remember_command_display() {
    let cmd = Command::Remember {
        content: "User prefers dark mode".to_string(),
        memory_type: "semantic".to_string(),
        metadata: vec![("importance".to_string(), "0.9".to_string())],
    };
    assert_eq!(
        format!("{}", cmd),
        "REMEMBER \"User prefers dark mode\" AS semantic WITH importance=0.9"
    );
}

#[test]
fn test_remember_no_metadata_display() {
    let cmd = Command::Remember {
        content: "hello".to_string(),
        memory_type: "working".to_string(),
        metadata: vec![],
    };
    assert_eq!(format!("{}", cmd), "REMEMBER \"hello\" AS working");
}

#[test]
fn test_init_command_display() {
    let cmd = Command::Init;
    assert_eq!(format!("{}", cmd), "INIT");
}

#[test]
fn test_recall_command_display() {
    let cmd = Command::Recall {
        query: "dark mode".to_string(),
        memory_types: vec!["semantic".to_string()],
        conditions: vec![],
        limit: 5,
    };
    assert_eq!(
        format!("{}", cmd),
        "RECALL \"dark mode\" FROM semantic LIMIT 5"
    );
}

#[test]
fn test_recall_no_types_display() {
    let cmd = Command::Recall {
        query: "test".to_string(),
        memory_types: vec![],
        conditions: vec![],
        limit: 10,
    };
    assert_eq!(format!("{}", cmd), "RECALL \"test\" LIMIT 10");
}

#[test]
fn test_forget_with_id_display() {
    let cmd = Command::Forget {
        id: Some("abc123".to_string()),
        conditions: vec![],
    };
    assert_eq!(format!("{}", cmd), "FORGET id(abc123)");
}

#[test]
fn test_forget_without_id_display() {
    let cmd = Command::Forget {
        id: None,
        conditions: vec![],
    };
    assert_eq!(format!("{}", cmd), "FORGET WHERE ...");
}

#[test]
fn test_consolidate_command_display() {
    let cmd = Command::Consolidate {
        from: "working".to_string(),
        to: "episodic".to_string(),
        conditions: vec![],
    };
    assert_eq!(format!("{}", cmd), "CONSOLIDATE working TO episodic");
}

#[test]
fn test_reflect_command_display() {
    let cmd = Command::Reflect {
        memory_type: Some("episodic".to_string()),
        conditions: vec![],
    };
    assert_eq!(format!("{}", cmd), "REFLECT");
}

#[test]
fn test_status_command_display() {
    let cmd = Command::Status;
    assert_eq!(format!("{}", cmd), "STATUS");
}

#[test]
fn test_pragma_set_display() {
    let cmd = Command::Pragma {
        key: Some("max_memories".to_string()),
        value: Some("100".to_string()),
    };
    assert_eq!(format!("{}", cmd), "PRAGMA max_memories = 100");
}

#[test]
fn test_pragma_get_display() {
    let cmd = Command::Pragma {
        key: Some("max_memories".to_string()),
        value: None,
    };
    assert_eq!(format!("{}", cmd), "PRAGMA");
}

#[test]
fn test_extract_command_display() {
    let cmd = Command::Extract {
        text: "I prefer dark mode".to_string(),
    };
    assert_eq!(format!("{}", cmd), "EXTRACT \"I prefer dark mode\"");
}

#[test]
fn test_recall_with_conditions_display() {
    let cmd = Command::Recall {
        query: "query".to_string(),
        memory_types: vec!["semantic".to_string()],
        conditions: vec![("field".to_string(), "=".to_string(), "value".to_string())],
        limit: 10,
    };
    assert_eq!(
        format!("{}", cmd),
        "RECALL \"query\" FROM semantic WHERE field = value LIMIT 10"
    );
}
