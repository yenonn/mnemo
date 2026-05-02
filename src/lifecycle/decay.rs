use rusqlite::Connection;

pub fn decay_episodic(_conn: &Connection, _decay_rate: f64) -> rusqlite::Result<usize> {
    Ok(0)
}