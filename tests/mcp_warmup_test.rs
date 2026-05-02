use tempfile::TempDir;

#[test]
fn test_mcp_serve_stdio_eager_warmup() {
    let dir = TempDir::new().unwrap();
    let agent_id = "test-mcp-warmup";

    // Pre-seed a semantic memory so auto_recall has something to bring back
    let db_path = dir.path().join(".mnemo").join(agent_id).join("memory.db");
    std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
    // We create the DB manually first so MnemoDb::new in serve_stdio
    // finds an existing DB with a semantic memory already stored.
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS memories (
            id TEXT PRIMARY KEY,
            memory_type TEXT NOT NULL CHECK (memory_type IN ('working', 'episodic', 'semantic')),
            content TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            accessed_at INTEGER,
            expires_at INTEGER,
            confidence REAL DEFAULT 1.0,
            importance REAL DEFAULT 0.5,
            source_type TEXT,
            source_turn_id TEXT,
            version INTEGER DEFAULT 1,
            superseded_by TEXT,
            is_indexed INTEGER DEFAULT 0,
            tags TEXT
        );
        CREATE TABLE IF NOT EXISTS _mnemo_meta (
            key TEXT PRIMARY KEY,
            value TEXT
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
            content,
            content='memories',
            content_rowid='rowid'
        );
    "#,
    )
    .unwrap();

    // Insert a semantic memory with high importance so auto_recall picks it up
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "INSERT INTO memories (id, memory_type, content, created_at, importance) VALUES (?1, 'semantic', ?2, ?3, 0.8)",
        rusqlite::params!["mem-seed-001", "User prefers dark mode", now],
    )
    .unwrap();

    // Seed lifecycle defaults with last_activity = 0 (first run)
    conn.execute(
        "INSERT OR REPLACE INTO _mnemo_meta (key, value) VALUES ('lifecycle_enabled', 'true')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO _mnemo_meta (key, value) VALUES ('lifecycle_last_activity', '0')",
        [],
    )
    .unwrap();
    drop(conn);

    // Now start the MCP server via serve_stdio by sending an initialize request.
    // Because serve_stdio eagerly opens the DB and runs check_and_fire,
    // auto_recall should fire and insert a [context-recall] entry.
    let mut proc = std::process::Command::new(
        std::env::var("CARGO_BIN_EXE_mnemo")
            .as_deref()
            .unwrap_or("./target/release/mnemo"),
    )
    .env("HOME", dir.path())
    .arg("--mcp")
    .arg("--agent-id")
    .arg(agent_id)
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .spawn()
    .expect("Failed to spawn mnemo --mcp");

    let mut stdin = proc.stdin.take().unwrap();
    let stdout = proc.stdout.take().unwrap();
    use std::io::{BufRead, Write};
    let mut stdout_buf = std::io::BufReader::new(stdout);

    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    writeln!(stdin, "{}", init_req).unwrap();
    stdin.flush().unwrap();

    let mut line = String::new();
    stdout_buf.read_line(&mut line).unwrap();

    // After initialize response, verify DB has [context-recall] working memories
    // (warmup ran before the first request was processed)
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories WHERE memory_type = 'working'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    drop(conn);

    assert!(
        count > 0,
        "Expected auto_recall to insert working memories on eager warmup, got {}",
        count
    );

    let _ = proc.kill();
}
