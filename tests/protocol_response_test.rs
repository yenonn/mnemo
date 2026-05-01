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
fn test_memory_response_no_score_no_status() {
    let resp = Response::Memory {
        id: "mem-xyz789".to_string(),
        memory_type: "episodic".to_string(),
        confidence: 0.5,
        importance: 0.3,
        score: None,
        content: "Memory content".to_string(),
        status: None,
    };
    let out = format!("{}", resp);
    assert!(out.contains("<memory id=\"mem-xyz789\" type=\"episodic\" confidence=\"0.5\" importance=\"0.3\">"));
    assert!(!out.contains("score="));
    assert!(!out.contains("status="));
}

#[test]
fn test_error_response() {
    let resp = Response::Error { code: "NO_MATCH".to_string(), message: "No memories found".to_string() };
    assert_eq!(format!("{}", resp), "<error code=\"NO_MATCH\">\n  No memories found\n</error>\n");
}

#[test]
fn test_result_set_response() {
    let resp = Response::ResultSet {
        count: 2,
        memories: vec![
            Response::Memory {
                id: "mem-1".to_string(),
                memory_type: "semantic".to_string(),
                confidence: 0.9,
                importance: 0.8,
                score: None,
                content: "First".to_string(),
                status: None,
            },
            Response::Memory {
                id: "mem-2".to_string(),
                memory_type: "episodic".to_string(),
                confidence: 0.7,
                importance: 0.6,
                score: None,
                content: "Second".to_string(),
                status: None,
            },
        ],
    };
    let out = format!("{}", resp);
    assert!(out.contains("<result count=\"2\">"));
    assert!(out.contains("First"));
    assert!(out.contains("Second"));
}

#[test]
fn test_status_response() {
    let resp = Response::Status {
        agent_id: "agent-42".to_string(),
        db_path: "~/.mnemo/agent-42/memory.db".to_string(),
        db_size_kb: 1024,
        working_count: 5,
        episodic_count: 12,
        semantic_count: 30,
        vector_indexed: 45,
        pending_embeddings: 2,
    };
    let out = format!("{}", resp);
    assert!(out.contains("Agent: agent-42"));
    assert!(out.contains("Database: ~/.mnemo/agent-42/memory.db (1024 KB)"));
    assert!(out.contains("Working buffer: 5"));
    assert!(out.contains("Episodic memories: 12"));
    assert!(out.contains("Semantic memories: 30"));
    assert!(out.contains("Vector indexed: 45 (2 pending)"));
}

#[test]
fn test_config_response() {
    let resp = Response::Config {
        entries: vec![
            ("max_memories".to_string(), "100".to_string()),
            ("version".to_string(), "0.1.0".to_string()),
        ],
    };
    let out = format!("{}", resp);
    assert!(out.contains("max_memories = 100"));
    assert!(out.contains("version = 0.1.0"));
}

#[test]
fn test_reflect_response() {
    let resp = Response::Reflect {
        total_episodic: 10,
        total_semantic: 25,
        low_confidence: 3,
        contradictions: vec!["c1".to_string(), "c2".to_string()],
        stale: 1,
    };
    let out = format!("{}", resp);
    assert!(out.contains("Memory count: 10 episodic, 25 semantic"));
    assert!(out.contains("Low-confidence items: 3"));
    assert!(out.contains("Potential contradictions: 2"));
    assert!(out.contains("c1"));
    assert!(out.contains("c2"));
    assert!(out.contains("Stale memories: 1"));
}

#[test]
fn test_reflect_response_no_contradictions() {
    let resp = Response::Reflect {
        total_episodic: 5,
        total_semantic: 10,
        low_confidence: 0,
        contradictions: vec![],
        stale: 0,
    };
    let out = format!("{}", resp);
    assert!(out.contains("Memory count: 5 episodic, 10 semantic"));
    assert!(!out.contains("Potential contradictions"));
}
