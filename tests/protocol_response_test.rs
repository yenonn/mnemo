use mnemo::protocol::response::Response;

#[test]
fn test_ok_response() {
    let resp = Response::Ok { message: "Database initialized".to_string() };
    assert_eq!(format!("{}", resp), "<ok>\n  Database initialized\n</ok>\n");
}

#[test]
fn test_memory_response() {
    let resp = Response::Memory {
        id: "mem-abc123".to_string(),
        memory_type: "semantic".to_string(),
        confidence: 0.95,
        importance: 0.8,
        score: Some(0.87),
        content: "User prefers dark mode".to_string(),
        status: Some("indexed".to_string()),
    };
    let out = format!("{}", resp);
    assert!(out.contains("<memory id=\"mem-abc123\" type=\"semantic\" confidence=\"0.95\" importance=\"0.8\" score=\"0.87\" status=\"indexed\">"));
    assert!(out.contains("User prefers dark mode"));
}

#[test]
fn test_error_response() {
    let resp = Response::Error { code: "NO_MATCH".to_string(), message: "No memories found".to_string() };
    assert_eq!(format!("{}", resp), "<error code=\"NO_MATCH\">\n  No memories found\n</error>\n");
}
