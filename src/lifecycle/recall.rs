use chrono::Utc;
use rusqlite::Connection;

pub fn auto_recall(conn: &Connection) -> rusqlite::Result<usize> {
    let now = Utc::now().timestamp_millis();
    let mut count = 0;
    
    // Query top 5 episodic memories by created_at desc, confidence > 0.3
    let mut stmt = conn.prepare(
        "SELECT content FROM memories
         WHERE memory_type = 'episodic' AND confidence > 0.3
         ORDER BY created_at DESC LIMIT 5"
    )?;
    
    let rows = stmt.query_map([], |row| {
        let content: String = row.get(0)?;
        Ok(content)
    })?;
    
    for content in rows {
        let content = content?;
        let new_id = format!("mem-recall-{}", nanoid::nanoid!(8));
        conn.execute(
            "INSERT INTO memories (id, memory_type, content, created_at, importance, source_type)
             VALUES (?1, 'working', ?2, ?3, 0.5, 'lifecycle_recall')",
            rusqlite::params![new_id, format!("[context-recall] {}", content), now],
        )?;
        count += 1;
    }
    
    // Query top 5 semantic memories by importance desc
    let mut stmt = conn.prepare(
        "SELECT content FROM memories
         WHERE memory_type = 'semantic'
         ORDER BY importance DESC LIMIT 5"
    )?;
    
    let rows = stmt.query_map([], |row| {
        let content: String = row.get(0)?;
        Ok(content)
    })?;
    
    for content in rows {
        let content = content?;
        let new_id = format!("mem-recall-{}", nanoid::nanoid!(8));
        conn.execute(
            "INSERT INTO memories (id, memory_type, content, created_at, importance, source_type)
             VALUES (?1, 'working', ?2, ?3, 0.7, 'lifecycle_recall')",
            rusqlite::params![new_id, format!("[context-recall] {}", content), now],
        )?;
        count += 1;
    }
    
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_auto_recall_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(r#"
            CREATE TABLE memories (
                id TEXT PRIMARY KEY,
                memory_type TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                confidence REAL DEFAULT 1.0,
                importance REAL DEFAULT 0.5,
                source_type TEXT
            );
        "#).unwrap();
        let count = auto_recall(&conn).unwrap();
        assert_eq!(count, 0);
    }
}