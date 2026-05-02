use chrono::Utc;
use rusqlite::Connection;

pub fn decay_episodic(conn: &Connection, decay_rate: f64) -> rusqlite::Result<usize> {
    let now = Utc::now().timestamp_millis();
    let five_min_ago = now - (5 * 60 * 1000);

    // Query episodic memories older than 5 minutes with confidence > 0.1
    let mut stmt = conn.prepare(
        "SELECT id, confidence, created_at FROM memories
         WHERE memory_type = 'episodic'
         AND created_at < ?
         AND confidence > 0.1",
    )?;

    let rows = stmt.query_map(rusqlite::params![five_min_ago], |row| {
        let id: String = row.get(0)?;
        let confidence: f64 = row.get(1)?;
        let created_at: i64 = row.get(2)?;
        Ok((id, confidence, created_at))
    })?;

    let mut affected = 0;
    for result in rows {
        let (id, confidence, created_at) = result?;
        let age_days = ((now - created_at) as f64) / 86400000.0;
        let new_confidence = (confidence * (1.0 - decay_rate).powf(age_days)).max(0.1);
        let new_confidence_rounded = (new_confidence * 100.0).round() / 100.0;

        conn.execute(
            "UPDATE memories SET confidence = ? WHERE id = ?",
            rusqlite::params![new_confidence_rounded, id],
        )?;
        affected += 1;
    }

    Ok(affected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_decay_no_rows() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(r#"
            CREATE TABLE memories (id TEXT, memory_type TEXT, content TEXT, created_at INTEGER, confidence REAL);
            CREATE TABLE _mnemo_meta (key TEXT PRIMARY KEY, value TEXT);
        "#).unwrap();
        let affected = decay_episodic(&conn, 0.1).unwrap();
        assert_eq!(affected, 0);
    }
}
