use mnemo::store::{MemoryStore, MnemoDb};
use tempfile::TempDir;

#[test]
fn test_memory_crud() {
    let dir = TempDir::new().unwrap();
    let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
    let store = MemoryStore::new(db.conn());

    // Insert
    let id = store
        .insert(
            "semantic",
            "User likes blue",
            0.9,
            "user_stated",
            &["ui", "preferences"],
        )
        .unwrap();
    assert!(id.starts_with("mem-"));

    // Search by content via FTS5
    let memories = store
        .search_content("blue", &["semantic".to_string()], 10)
        .unwrap();
    assert_eq!(memories.len(), 1);
    assert!(memories[0].content.contains("blue"));

    // Get
    let mem = store.get(&id).unwrap();
    assert!(mem.is_some());
    let mem = mem.unwrap();
    assert_eq!(mem.memory_type, "semantic");

    // Delete
    store.delete(&id).unwrap();
    let mem = store.get(&id).unwrap();
    assert!(mem.is_none());
}
