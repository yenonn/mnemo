use mnemo::protocol::Command;

#[test]
fn test_remember_command_display() {
    let cmd = Command::Remember {
        content: "User prefers dark mode".to_string(),
        memory_type: "semantic".to_string(),
        metadata: vec![("importance".to_string(), "0.9".to_string())],
    };
    assert_eq!(format!("{}", cmd), "REMEMBER \"User prefers dark mode\" AS semantic WITH importance=0.9");
}
