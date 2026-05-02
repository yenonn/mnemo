//! Vector search using sqlite-vec (optional).
//!
//! sqlite-vec is a SQLite extension that adds a `vec0` virtual table
//! for dense vector storage and approximate nearest-neighbor search.
//!
//! It is **optional**. If not installed, `memory_vectors` is never
//! created and all `VectorStore` operations are no-ops.

use rusqlite::{Connection, Result as SqliteResult};

/// Encapsulates sqlite-vec `memory_vectors` CRUD and KNN search.
///
/// If `vec0` is unavailable, `available()` returns `false` and every
/// method succeeds without touching the database.
pub struct VectorStore<'a> {
    conn: &'a Connection,
    available: bool,
}

impl<'a> VectorStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        let available = Self::detect_vec0(conn);
        if available {
            // Best-effort: if creation fails we silently ignore it
            // (e.g. dimension mismatch from a previous run).
            let _ = conn.execute(
                "CREATE VIRTUAL TABLE IF NOT EXISTS memory_vectors USING vec0(
                    memory_id TEXT PRIMARY KEY,
                    embedding FLOAT[1536]
                )",
                [],
            );
        }
        VectorStore { conn, available }
    }

    /// True when the `vec0` extension is loaded and usable.
    pub fn available(&self) -> bool {
        self.available
    }

    /// Insert an embedding vector for a memory.
    ///
    /// No-op when `vec0` is unavailable.
    pub fn insert(
        &self,
        memory_id: &str,
        embedding: &[f32],
    ) -> SqliteResult<()> {
        if !self.available {
            return Ok(());
        }
        let vec_json = vec_to_json(embedding);
        self.conn.execute(
            "INSERT INTO memory_vectors (memory_id, embedding) VALUES (?, vec_from_json(?))",
            rusqlite::params![memory_id, vec_json],
        )?;
        Ok(())
    }

    /// KNN search: returns `(memory_id, distance)` ordered by
    /// ascending distance (lower = closer).
    ///
    /// Empty vector when `vec0` is unavailable.
    pub fn search(
        &self,
        query_vec: &[f32],
        limit: usize,
    ) -> SqliteResult<Vec<(String, f64)>> {
        if !self.available {
            return Ok(Vec::new());
        }
        let vec_json = vec_to_json(query_vec);
        let limit_i64 = limit as i64;
        let mut stmt = self.conn.prepare(
            "SELECT memory_id, distance FROM memory_vectors
             WHERE embedding MATCH vec_from_json(?)
             ORDER BY distance
             LIMIT ?"
        )?;
        let rows = stmt.query_map(
            rusqlite::params![&vec_json, &limit_i64
            ],
            |row| {
                let id: String = row.get(0)?;
                let dist: f64 = row.get(1)?;
                Ok((id, dist))
            },
        )?;
        rows.collect()
    }

    /// Delete a vector by memory_id.
    ///
    /// No-op when `vec0` is unavailable.
    pub fn delete(&self, memory_id: &str) -> SqliteResult<()> {
        if !self.available {
            return Ok(());
        }
        self.conn.execute(
            "DELETE FROM memory_vectors WHERE memory_id = ?",
            [memory_id],
        )?;
        Ok(())
    }

    /// Count indexed vectors.
    ///
    /// Returns `0` when `vec0` is unavailable.
    pub fn count(&self) -> SqliteResult<usize> {
        if !self.available {
            return Ok(0);
        }
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM memory_vectors")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Try to create a temporary `vec0` table. If it succeeds,
    /// drop it and return `true`. Otherwise return `false`.
    fn detect_vec0(conn: &Connection) -> bool {
        let result = conn.execute(
            "CREATE VIRTUAL TABLE _mnemo_vec_probe USING vec0(x FLOAT[1])",
            [],
        );
        if result.is_ok() {
            let _ = conn.execute("DROP TABLE _mnemo_vec_probe", []);
        }
        result.is_ok()
    }
}

/// Serialize a `Vec<f32>` as a JSON array string for
/// `vec_from_json()`.
fn vec_to_json(v: &[f32]) -> String {
    let mut buf = String::with_capacity(v.len() * 8);
    buf.push('[');
    for (i, val) in v.iter().enumerate() {
        if i > 0 {
            buf.push(',');
        }
        buf.push_str(&val.to_string());
    }
    buf.push(']');
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_vec_to_json() {
        assert_eq!(vec_to_json(&[0.1f32, 0.2f32]), "[0.1,0.2]");
    }

    #[test]
    fn test_vec_store_available_when_vec0_absent() {
        let conn = Connection::open_in_memory().unwrap();
        let vstore = VectorStore::new(&conn);
        assert!(!vstore.available());
        assert_eq!(vstore.count().unwrap(), 0);
        assert_eq!(vstore.search(&[0.0f32; 1536], 5).unwrap().len(), 0);
        assert!(vstore.insert("mem-abc", &[0.0f32; 1536]).is_ok());
    }
}
