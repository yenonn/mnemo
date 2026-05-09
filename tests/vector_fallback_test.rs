use mnemo::store::{MemoryStore, MnemoDb};
use mnemo::tier::TierManager;
use tempfile::TempDir;

/// Tests that the system gracefully falls back to FTS5 when no embedding
/// provider is configured or sqlite-vec is unavailable.
#[test]
fn test_fallback_when_no_provider() {
    // Clear any provider env vars
    let _ = std::env::remove_var("MNEMO_OPENAI_API_KEY");
    let _ = std::env::remove_var("MNEMO_OLLAMA_ENDPOINT");

    let dir = TempDir::new().unwrap();
    let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
    let store = MemoryStore::new(db.conn());

    // Store a memory
    store
        .insert("semantic", "User prefers dark mode", 0.8, "test", &[])
        .unwrap();

    // Recall via TierManager (uses expanded FTS5 path when no provider)
    let manager = TierManager::new(db.conn(), 100).unwrap();
    let results = manager
        .recall_expanded(
            &["dark".to_string(), "mode".to_string()],
            &["semantic".to_string()],
            10,
        )
        .unwrap();

    assert!(!results.is_empty(), "Should find memory via FTS5 fallback");
    assert_eq!(results[0].content, "User prefers dark mode");
}

/// Tests that default build (without vec feature) works identically.
#[cfg(not(feature = "vec"))]
#[test]
fn test_recall_without_vec_feature() {
    let dir = TempDir::new().unwrap();
    let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
    let store = MemoryStore::new(db.conn());

    store
        .insert("semantic", "User prefers dark mode", 0.8, "test", &[])
        .unwrap();

    let manager = TierManager::new(db.conn(), 100).unwrap();
    let results = manager
        .recall("dark mode", &["semantic".to_string()], 10)
        .unwrap();

    assert!(!results.is_empty());
}
