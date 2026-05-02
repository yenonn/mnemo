use rusqlite::{params, Connection};

/// Get a lifecycle config key, returning the fallback `default` if not present.
pub fn get_bool(conn: &Connection, key: &str, default: bool) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare("SELECT value FROM _mnemo_meta WHERE key = ?")?;
    let mut rows = stmt.query([key])?;
    if let Some(row) = rows.next()? {
        let val: String = row.get(0)?;
        Ok(val == "true")
    } else {
        Ok(default)
    }
}

/// Get a lifecycle config key as i64, returning the fallback `default` if not present.
pub fn get_i64(conn: &Connection, key: &str, default: i64) -> rusqlite::Result<i64> {
    let mut stmt = conn.prepare("SELECT value FROM _mnemo_meta WHERE key = ?")?;
    let mut rows = stmt.query([key])?;
    if let Some(row) = rows.next()? {
        let val: String = row.get(0)?;
        match val.parse::<i64>() {
            Ok(v) => Ok(v),
            Err(_) => Ok(default),
        }
    } else {
        Ok(default)
    }
}

/// Get a lifecycle config key as f64, returning the fallback `default` if not present.
pub fn get_f64(conn: &Connection, key: &str, default: f64) -> rusqlite::Result<f64> {
    let mut stmt = conn.prepare("SELECT value FROM _mnemo_meta WHERE key = ?")?;
    let mut rows = stmt.query([key])?;
    if let Some(row) = rows.next()? {
        let val: String = row.get(0)?;
        match val.parse::<f64>() {
            Ok(v) => Ok(v),
            Err(_) => Ok(default),
        }
    } else {
        Ok(default)
    }
}

/// Set a lifecycle config key. Persists to `_mnemo_meta`.
pub fn set(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO _mnemo_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Seed all default lifecycle config keys on first access.
/// Call once when a new database is initialized.
pub fn seed_defaults(conn: &Connection) -> rusqlite::Result<()> {
    let defaults: Vec<(&str, &str)> = vec![
        ("lifecycle_enabled", "true"),
        ("lifecycle_idle_threshold", "60"),
        ("lifecycle_decay_rate", "0.1"),
        ("lifecycle_consolidate_on_flush", "true"),
    ];
    for (key, value) in defaults {
        // Only insert if not already present
        conn.execute(
            "INSERT OR IGNORE INTO _mnemo_meta (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
    }
    // Only seed last_activity if not present — never overwrite an existing value
    conn.execute(
        "INSERT OR IGNORE INTO _mnemo_meta (key, value) VALUES ('lifecycle_last_activity', '0')",
        [],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_get_bool_missing_returns_default() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(r#"
            CREATE TABLE _mnemo_meta (key TEXT PRIMARY KEY, value TEXT);
        "#).unwrap();
        let val = get_bool(&conn, "missing", true).unwrap();
        assert!(val);
    }

    #[test]
    fn test_get_i64_missing_returns_default() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(r#"
            CREATE TABLE _mnemo_meta (key TEXT PRIMARY KEY, value TEXT);
        "#).unwrap();
        let val = get_i64(&conn, "missing", 60).unwrap();
        assert_eq!(val, 60);
    }

    #[test]
    fn test_get_f64_missing_returns_default() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(r#"
            CREATE TABLE _mnemo_meta (key TEXT PRIMARY KEY, value TEXT);
        "#).unwrap();
        let val = get_f64(&conn, "missing", 0.1).unwrap();
        assert!((val - 0.1).abs() < f64::EPSILON);
    }

    #[test]
    fn test_seed_defaults_inserts_all() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(r#"
            CREATE TABLE _mnemo_meta (key TEXT PRIMARY KEY, value TEXT);
        "#).unwrap();
        seed_defaults(&conn).unwrap();

        assert!(get_bool(&conn, "lifecycle_enabled", false).unwrap());
        assert_eq!(get_i64(&conn, "lifecycle_idle_threshold", 0).unwrap(), 60);
        assert!(get_f64(&conn, "lifecycle_decay_rate", 0.0).unwrap() - 0.1 < f64::EPSILON);
        assert!(get_bool(&conn, "lifecycle_consolidate_on_flush", false).unwrap());
        assert_eq!(get_i64(&conn, "lifecycle_last_activity", -1).unwrap(), 0);
    }

    #[test]
    fn test_seed_defaults_does_not_overwrite() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(r#"
            CREATE TABLE _mnemo_meta (key TEXT PRIMARY KEY, value TEXT);
        "#).unwrap();
        set(&conn, "lifecycle_enabled", "false").unwrap();
        seed_defaults(&conn).unwrap();
        assert!(!get_bool(&conn, "lifecycle_enabled", true).unwrap());
    }
}
