use chrono::Utc;
use rusqlite::Connection;
use crate::tier::TierManager;
use crate::lifecycle::config;
use crate::lifecycle::hook::HookResult;
use crate::lifecycle::decay;
use crate::lifecycle::recall;

pub struct LifecycleEngine;

impl LifecycleEngine {
    pub fn check_and_fire(conn: &Connection, manager: &mut TierManager) -> Vec<HookResult> {
        let mut results = Vec::new();

        // Guard: disabled
        let enabled = config::get_bool(conn, "lifecycle_enabled", true)
            .unwrap_or(true);
        if !enabled {
            return results;
        }

        let idle_threshold: i64 = config::get_i64(conn, "lifecycle_idle_threshold", 60)
            .unwrap_or(60);
        let decay_rate: f64 = config::get_f64(conn, "lifecycle_decay_rate", 0.1)
            .unwrap_or(0.1);
        let consolidate_on_flush = config::get_bool(conn, "lifecycle_consolidate_on_flush", true)
            .unwrap_or(true);

        let now = Utc::now().timestamp_millis();
        let last_activity = config::get_i64(conn, "lifecycle_last_activity", 0)
            .unwrap_or(0);
        let idle_seconds = (now - last_activity) / 1000;

        // 1. Buffer overflow check
        let mut stmt = match conn.prepare("SELECT COUNT(*) FROM memories WHERE memory_type = 'working'") {
            Ok(s) => s,
            Err(_) => {
                let _ = config::set(conn, "lifecycle_last_activity", &now.to_string());
                return results;
            }
        };
        let working_count_db: i64 = stmt.query_row([], |row| row.get(0)).unwrap_or(0);
        let capacity = 100;
        let overflow_threshold = ((capacity as f64) * 0.8) as i64;

        if working_count_db >= overflow_threshold {
            let consolidated = manager.working_count();
            match manager.consolidate_working_to_episodic() {
                Ok(_) => {
                    results.push(HookResult::Overflow { consolidated_count: consolidated });
                }
                Err(_) => {}
            }
        }

        // 2. Idle session boundary
        if idle_seconds >= idle_threshold && last_activity > 0 {
            let mut consolidated_count = 0;
            let mut new_episodic_id: Option<String> = None;

            if consolidate_on_flush {
                consolidated_count = manager.working_count();
                match manager.consolidate_working_to_episodic() {
                    Ok(Some(id)) => {
                        new_episodic_id = Some(id);
                    }
                    Ok(None) => {}
                    Err(_) => {}
                }
            } else {
                manager.clear_working();
            }

            if consolidated_count > 0 || !consolidate_on_flush {
                results.push(HookResult::SessionEnd { consolidated_count, new_episodic_id });
            }

            // Session start: auto-recall
            match recall::auto_recall(conn) {
                Ok(recalled) => {
                    if recalled > 0 {
                        results.push(HookResult::SessionStart { recalled_count: recalled });
                    }
                }
                Err(_) => {}
            }
        }

        // 3. Decay
        match decay::decay_episodic(conn, decay_rate) {
            Ok(affected) => {
                if affected > 0 {
                    results.push(HookResult::Decay { affected_count: affected });
                }
            }
            Err(_) => {}
        }

        // 4. Update last_activity
        let _ = config::set(conn, "lifecycle_last_activity", &now.to_string());

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use crate::tier::TierManager;
    use crate::store::MemoryStore;

    #[test]
    fn test_check_and_fire_disabled() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(r#"
            CREATE TABLE memories (id TEXT PRIMARY KEY, memory_type TEXT, content TEXT, created_at INTEGER, confidence REAL DEFAULT 1.0, importance REAL DEFAULT 0.5);
            CREATE TABLE _mnemo_meta (key TEXT PRIMARY KEY, value TEXT);
        "#).unwrap();
        crate::lifecycle::config::seed_defaults(&conn).unwrap();
        crate::lifecycle::config::set(&conn, "lifecycle_enabled", "false").unwrap();

        let mut manager = TierManager::new(&conn, 100).unwrap();
        let results = LifecycleEngine::check_and_fire(&conn, &mut manager);
        assert!(results.is_empty());
    }

    #[test]
    fn test_check_and_fire_idle_consolidates() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(r#"
            CREATE TABLE memories (
                id TEXT PRIMARY KEY,
                memory_type TEXT,
                content TEXT,
                created_at INTEGER,
                confidence REAL DEFAULT 1.0,
                importance REAL DEFAULT 0.5,
                source_type TEXT,
                tags TEXT,
                accessed_at INTEGER,
                expires_at INTEGER,
                version INTEGER DEFAULT 1,
                superseded_by TEXT,
                is_indexed INTEGER DEFAULT 0
            );
            CREATE TABLE _mnemo_meta (key TEXT PRIMARY KEY, value TEXT);
        "#).unwrap();
        crate::lifecycle::config::seed_defaults(&conn).unwrap();

        let store = MemoryStore::new(&conn);
        store.insert("working", "Test memory", 0.5, "test", &[]).unwrap();

        // Set last_activity to 2 minutes ago
        let two_mins_ago = Utc::now().timestamp_millis() - (2 * 60 * 1000);
        crate::lifecycle::config::set(&conn, "lifecycle_last_activity", &two_mins_ago.to_string()).unwrap();

        let mut manager = TierManager::new(&conn, 100).unwrap();
        let results = LifecycleEngine::check_and_fire(&conn, &mut manager);

        // Should have SessionEnd + SessionStart (at least)
        assert!(!results.is_empty(), "Should fire hooks on idle boundary");
        let has_session_end = results.iter().any(|r| matches!(r, HookResult::SessionEnd { .. }));
        assert!(has_session_end, "Should fire SessionEnd when idle");
    }
}