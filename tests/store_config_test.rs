use mnemo::store::{ConfigStore, MnemoDb};
use tempfile::TempDir;

#[test]
fn test_config_crud() {
    let dir = TempDir::new().unwrap();
    let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
    let store = ConfigStore::new(db.conn());

    // Insert
    store.set("embedding_provider", "ollama").unwrap();
    assert_eq!(
        store.get("embedding_provider").unwrap(),
        Some("ollama".to_string())
    );

    // Update (should overwrite)
    store.set("embedding_provider", "openai").unwrap();
    assert_eq!(
        store.get("embedding_provider").unwrap(),
        Some("openai".to_string())
    );

    // Get all (should include lifecycle defaults)
    let all = store.get_all().unwrap();
    assert_eq!(all.len(), 6); // 1 user key + 5 lifecycle defaults

    // Delete
    store.delete("embedding_provider").unwrap();
    assert_eq!(store.get("embedding_provider").unwrap(), None);
}
