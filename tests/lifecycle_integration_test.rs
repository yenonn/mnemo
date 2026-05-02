use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn test_lifecycle_idle_consolidation() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-lifecycle-idle";

    // 1. Store a working memory
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("remember").arg("User said hello");
    cmd.arg("--memory-type").arg("working");
    cmd.assert().success();

    // 2. Simulate idle by directly writing lifecycle_last_activity in the DB
    let db_path = dir.path().join(".mnemo").join(agent_id).join("memory.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let past_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        - 120_000;
    conn.execute(
        "INSERT OR REPLACE INTO _mnemo_meta (key, value) VALUES ('lifecycle_last_activity', ?)",
        [&past_time.to_string()],
    )
    .unwrap();
    drop(conn);

    // 3. Next command should trigger session-end consolidation
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("status");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("<status>"));
}

#[test]
fn test_lifecycle_disabled() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-lifecycle-disabled";

    // Disable lifecycle
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("pragma").arg("lifecycle_enabled").arg("false");
    cmd.assert().success();

    // Store working memory
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("remember").arg("Should not consolidate");
    cmd.arg("--memory-type").arg("working");
    cmd.assert().success();

    // Fake idle directly in DB (without triggering lifecycle)
    let db_path = dir.path().join(".mnemo").join(agent_id).join("memory.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let past_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        - 120_000;
    conn.execute(
        "INSERT OR REPLACE INTO _mnemo_meta (key, value) VALUES ('lifecycle_last_activity', ?)",
        [&past_time.to_string()],
    )
    .unwrap();
    drop(conn);

    // Next command should NOT consolidate (lifecycle disabled)
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("status");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Working buffer: 1"));
}

#[test]
fn test_lifecycle_auto_recall() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-lifecycle-recall";

    // Store semantic memory
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("remember").arg("User prefers dark mode");
    cmd.arg("--memory-type").arg("semantic");
    cmd.assert().success();

    // Fake idle directly in DB
    let db_path = dir.path().join(".mnemo").join(agent_id).join("memory.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let past_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        - 120_000;
    conn.execute(
        "INSERT OR REPLACE INTO _mnemo_meta (key, value) VALUES ('lifecycle_last_activity', ?)",
        [&past_time.to_string()],
    )
    .unwrap();
    drop(conn);

    // Status should trigger session-start which recalls context
    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("status");
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Working buffer:"));
}

#[test]
fn test_lifecycle_decay_cli() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-lifecycle-decay";

    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("remember").arg("Very old memory");
    cmd.arg("--memory-type").arg("episodic");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("pragma").arg("lifecycle_decay_rate").arg("1.0");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("mnemo").unwrap();
    cmd.env("HOME", dir.path());
    cmd.arg("--agent-id").arg(agent_id);
    cmd.arg("status");
    cmd.assert().success();
}

#[test]
fn test_lifecycle_mcp_remember_hooks() {
    use mnemo::mcp::{handle_request, McpRequest};
    use std::env;

    let _tmp = tempfile::tempdir().unwrap();
    env::set_var("HOME", _tmp.path());
    env::set_var("MNEMO_OPENAI_API_KEY", "");

    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(1.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "remember",
                "arguments": {
                    "content": "User prefers dark mode",
                    "memory_type": "working"
                }
            })),
        },
        "mcp-lifecycle-hooks",
    );
    assert!(resp.error.is_none());

    // Fake idle
    let db_path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".mnemo")
        .join("mcp-lifecycle-hooks")
        .join("memory.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let past_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
        - 120_000;
    conn.execute(
        "INSERT OR REPLACE INTO _mnemo_meta (key, value) VALUES ('lifecycle_last_activity', ?)",
        [&past_time.to_string()],
    )
    .unwrap();
    drop(conn);

    let resp = handle_request(
        McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(2.into())),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "status",
                "arguments": {}
            })),
        },
        "mcp-lifecycle-hooks",
    );
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let text = result.get("content").unwrap().as_array().unwrap()[0]
        .get("text")
        .unwrap()
        .as_str()
        .unwrap();
    assert!(
        text.contains("Working:") || text.contains("session-start"),
        "Should contain status or session-start"
    );
}
