use rusqlite::Connection;

pub fn auto_recall(_conn: &Connection) -> rusqlite::Result<usize> {
    Ok(0)
}