use chrono::Utc;
use rusqlite::{params, Connection, Result as SqliteResult, Row, ToSql};

#[derive(Debug, Clone)]
pub struct Memory {
    pub id: String,
    pub memory_type: String,
    pub content: String,
    pub created_at: i64,
    pub accessed_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub confidence: f64,
    pub importance: f64,
    pub source_type: Option<String>,
    pub source_turn_id: Option<String>,
    pub version: i32,
    pub superseded_by: Option<String>,
    pub is_indexed: i32,
    pub tags: Option<String>,
}

impl Memory {
    pub fn from_row(row: &Row) -> SqliteResult<Self> {
        Ok(Memory {
            id: row.get(0)?,
            memory_type: row.get(1)?,
            content: row.get(2)?,
            created_at: row.get(3)?,
            accessed_at: row.get(4)?,
            expires_at: row.get(5)?,
            confidence: row.get(6)?,
            importance: row.get(7)?,
            source_type: row.get(8)?,
            source_turn_id: row.get(9)?,
            version: row.get(10)?,
            superseded_by: row.get(11)?,
            is_indexed: row.get(12)?,
            tags: row.get(13)?,
        })
    }
}

pub struct MemoryStore<'a> {
    conn: &'a Connection,
}

impl<'a> MemoryStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        MemoryStore { conn }
    }

    pub fn conn(&self) -> &'a Connection {
        self.conn
    }

    pub fn insert(
        &self,
        memory_type: &str,
        content: &str,
        importance: f64,
        source_type: &str,
        tags: &[&str],
    ) -> SqliteResult<String> {
        let id = format!("mem-{}", nanoid::nanoid!(10));
        let now = Utc::now().timestamp_millis();
        let tags_str = if tags.is_empty() {
            None
        } else {
            Some(tags.join(","))
        };

        self.conn.execute(
            "INSERT INTO memories (id, memory_type, content, created_at, importance, source_type, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![&id, memory_type, content, now, importance, source_type, tags_str],
        )?;

        Ok(id)
    }

    pub fn get(&self, id: &str) -> SqliteResult<Option<Memory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, memory_type, content, created_at, accessed_at, expires_at,
                    confidence, importance, source_type, source_turn_id,
                    version, superseded_by, is_indexed, tags
             FROM memories WHERE id = ?",
        )?;

        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Memory::from_row(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn search_content(
        &self,
        query: &str,
        memory_types: &[String],
        limit: usize,
    ) -> SqliteResult<Vec<Memory>> {
        let mut sql = String::from(
            "SELECT id, memory_type, content, created_at, accessed_at, expires_at,
                    confidence, importance, source_type, source_turn_id,
                    version, superseded_by, is_indexed, tags
             FROM memories
             WHERE rowid IN (SELECT rowid FROM memories_fts WHERE content MATCH ?)",
        );

        if !memory_types.is_empty() {
            let placeholders = memory_types
                .iter()
                .map(|_| "?".to_string())
                .collect::<Vec<_>>()
                .join(",");
            sql.push_str(&format!(" AND memory_type IN ({})", placeholders));
        }

        sql.push_str(" LIMIT ?");

        let mut stmt = self.conn.prepare(&sql)?;
        let mut params: Vec<&dyn ToSql> = vec![&query];
        for t in memory_types {
            params.push(t);
        }
        let limit_i64 = limit as i64;
        params.push(&limit_i64);

        let rows = stmt.query_map(params.as_slice(), Memory::from_row)?;
        rows.collect()
    }

    pub fn delete(&self, id: &str) -> SqliteResult<()> {
        self.conn
            .execute("DELETE FROM memories WHERE id = ?", [id])?;
        Ok(())
    }

    /// Search memories using an expanded query with OR-joined synonyms.
    ///
    /// Each term in `expanded_terms` is joined with `OR` so a match on
    /// any synonym returns the memory, casting a wider net than exact
    /// lexical match alone.
    pub fn search_content_expanded(
        &self,
        expanded_terms: &[String],
        memory_types: &[String],
        limit: usize,
    ) -> SqliteResult<Vec<Memory>> {
        if expanded_terms.is_empty() {
            return Ok(Vec::new());
        }

        let fts_query = expanded_terms.join(" OR ");
        self.search_content(&fts_query, memory_types, limit)
    }
}
