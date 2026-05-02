use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;
use crate::lifecycle::config;

pub struct MnemoDb {
    conn: Connection,
}

impl MnemoDb {
    pub fn new<P: AsRef<Path>>(path: P) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
        ",
        )?;

        let db = MnemoDb { conn };
        db.init_schema()?;
        db.seed_defaults()?;
        Ok(db)
    }

    fn init_schema(&self) -> SqliteResult<()> {
        self.conn.execute_batch(SCHEMA_SQL)?;
        Ok(())
    }

    fn seed_defaults(&self) -> SqliteResult<()> {
        config::seed_defaults(&self.conn)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS memories (
    id          TEXT PRIMARY KEY,
    memory_type TEXT NOT NULL CHECK (memory_type IN ('working', 'episodic', 'semantic')),
    content     TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    accessed_at INTEGER,
    expires_at  INTEGER,
    confidence  REAL DEFAULT 1.0,
    importance  REAL DEFAULT 0.5,
    source_type TEXT,
    source_turn_id TEXT,
    version     INTEGER DEFAULT 1,
    superseded_by TEXT,
    is_indexed  INTEGER DEFAULT 0,
    tags        TEXT
);

CREATE TABLE IF NOT EXISTS memory_links (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    source_id   TEXT NOT NULL REFERENCES memories(id),
    target_id   TEXT NOT NULL REFERENCES memories(id),
    link_type   TEXT NOT NULL,
    confidence  REAL DEFAULT 1.0,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS memory_access_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    memory_id   TEXT NOT NULL REFERENCES memories(id),
    access_type TEXT NOT NULL,
    query_text  TEXT,
    relevance   REAL,
    accessed_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS _mnemo_meta (
    key     TEXT PRIMARY KEY,
    value   TEXT
);

CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    content,
    content='memories',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
END;

CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content) VALUES ('delete', old.rowid, old.content);
    INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
END;
"#;
