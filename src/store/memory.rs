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

    #[cfg(feature = "vec")]
    /// Search memories using both FTS5 (expanded) and vector (HNSW) together.
    ///
    /// When sqlite-vec is available and an embedding provider is configured,
    /// the query text is embedded and searched against `memory_vectors`.
    ///
    /// A weighted merge (`0.4 * BM25_norm + 0.6 * vector_norm`) combines
    /// both sources. FTS5 alone is returned when vectors are unavailable.
    pub fn search_hybrid(
        &self,
        query_text: &str,
        expanded_terms: &[String],
        memory_types: &[String],
        limit: usize,
        vstore: &super::VectorStore,
        gateway: &crate::embed::EmbeddingGateway,
    ) -> SqliteResult<Vec<Memory>> {
        use std::collections::HashMap;

        // 1. FTS5 search
        let fts_results =
            self.search_content_expanded(expanded_terms, memory_types, limit * 2)?;

        // 2. Vector search (if available)
        let vec_results: Vec<(String, f64)> = if vstore.available() {
            match gateway.embed(query_text) {
                Ok(vec) => {
                    match vstore.search(&vec, limit * 2) {
                        Ok(rows) => rows,
                        Err(_) => Vec::new(),
                    }
                }
                Err(_) => Vec::new(),
            }
        } else {
            Vec::new()
        };

        // No vectors → return FTS5 results only
        if vec_results.is_empty() {
            return Ok(fts_results.into_iter().take(limit).collect());
        }

        // 3. Score merging
        let fts_count = fts_results.len();
        let mut score_map: HashMap<String, f64> = HashMap::new();

        // BM25 proxy: descending by position (row 0 = highest)
        for (i, mem) in fts_results.into_iter().enumerate() {
            let bm25_norm = if fts_count > 1 {
                1.0 - (i as f64 / (fts_count.saturating_sub(1) as f64))
            } else {
                1.0
            };
            score_map.insert(mem.id, 0.4 * bm25_norm);
        }

        // Vector normalization: (max_dist - dist) / range
        if !vec_results.is_empty() {
            let min_dist = vec_results
                .iter()
                .map(|(_, d)| *d)
                .fold(f64::INFINITY, f64::min);
            let max_dist = vec_results
                .iter()
                .map(|(_, d)| *d)
                .fold(f64::NEG_INFINITY, f64::max);
            let dist_range = max_dist - min_dist;

            for (id, dist) in &vec_results {
                let cos_norm = if dist_range > 0.0 {
                    ((max_dist - dist) / dist_range).max(0.0)
                } else {
                    1.0
                };
                score_map
                    .entry(id.clone())
                    .and_modify(|s| *s += 0.6 * cos_norm)
                    .or_insert(0.6 * cos_norm);
            }
        }

        // 4. Rank descending, trim to limit, hydrate full rows
        let mut ranked: Vec<(String, f64)> = score_map.into_iter().collect();
        ranked
            .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut output = Vec::with_capacity(limit.min(ranked.len()));
        for (id, _) in ranked.iter().take(limit) {
            if let Ok(Some(mem)) = self.get(id) {
                output.push(mem);
            }
        }
        Ok(output)
    }
}
