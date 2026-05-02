use chrono::Utc;
use mnemo::store::{MemoryStore, MnemoDb};
use tempfile::TempDir;

#[test]
fn test_auto_recall_inserts_working() {
    let dir = TempDir::new().unwrap();
    let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
    let store = MemoryStore::new(db.conn());

    // Insert 2 semantic memories
    store
        .insert("semantic", "User prefers dark mode", 0.8, "test", &[])
        .unwrap();
    store
        .insert("semantic", "User uses vim", 0.9, "test", &[])
        .unwrap();

    // Insert 3 episodic memories
    store
        .insert("episodic", "Met with Alice yesterday", 0.5, "test", &[])
        .unwrap();
    store
        .insert("episodic", "Deployed release v2.1", 0.6, "test", &[])
        .unwrap();
    store
        .insert("episodic", "Fixed bug #42", 0.4, "test", &[])
        .unwrap();

    // Run auto_recall
    let recalled = mnemo::lifecycle::recall::auto_recall(db.conn()).unwrap();
    assert!(recalled > 0, "Should recall some memories");

    // Query working memories with [context-recall] prefix
    let mut stmt = db
        .conn()
        .prepare("SELECT content FROM memories WHERE memory_type = 'working'")
        .unwrap();
    let rows = stmt
        .query_map([], |row| {
            let content: String = row.get(0)?;
            Ok(content)
        })
        .unwrap();

    let working: Vec<String> = rows.collect::<Result<_, _>>().unwrap();
    assert!(
        !working.is_empty(),
        "Should have working memories after recall"
    );
    assert!(
        working[0].starts_with("[context-recall]"),
        "Should have context-recall prefix"
    );
}

#[test]
fn test_decay_reduces_confidence() {
    let dir = TempDir::new().unwrap();
    let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
    let store = MemoryStore::new(db.conn());

    // Insert episodic memory with confidence 1.0, created 7 days ago
    let seven_days_ago = Utc::now().timestamp_millis() - (7 * 86400 * 1000);
    let id = store
        .insert("episodic", "Old memory", 0.5, "test", &[])
        .unwrap();
    // Manually fix timestamp
    let _ = db.conn().execute(
        "UPDATE memories SET created_at = ? WHERE id = ?",
        rusqlite::params![seven_days_ago, id],
    );

    // Run decay with rate 0.1 (10% per day)
    let affected = mnemo::lifecycle::decay::decay_episodic(db.conn(), 0.1).unwrap();
    assert_eq!(affected, 1);

    // Query updated confidence
    let mem = store.get(&id).unwrap().unwrap();
    // Expected: 1.0 * (1 - 0.1)^7 = ~0.478
    assert!(
        mem.confidence < 1.0,
        "Confidence should decay: got {}",
        mem.confidence
    );
    assert!(
        mem.confidence > 0.1,
        "Confidence should stay above floor: got {}",
        mem.confidence
    );
}

#[test]
fn test_decay_respects_floor() {
    let dir = TempDir::new().unwrap();
    let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
    let store = MemoryStore::new(db.conn());

    // Insert episodic memory, created 365 days ago
    let old_time = Utc::now().timestamp_millis() - (365 * 86400 * 1000);
    let id = store
        .insert("episodic", "Very old memory", 0.5, "test", &[])
        .unwrap();
    let _ = db.conn().execute(
        "UPDATE memories SET created_at = ? WHERE id = ?",
        rusqlite::params![old_time, id],
    );

    mnemo::lifecycle::decay::decay_episodic(db.conn(), 0.1).unwrap();

    let mem = store.get(&id).unwrap().unwrap();
    assert_eq!(mem.confidence, 0.1, "Floor at 0.1 should be applied");
}
