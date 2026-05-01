use mnemo::store::MnemoDb;
use mnemo::tier::TierManager;
use tempfile::TempDir;

#[test]
fn test_tier_manager_lifecycle() {
    let dir = TempDir::new().unwrap();
    let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
    let mut manager = TierManager::new(db.conn(), 100).unwrap();

    // Remember in working
    let id = manager.remember_working("User said hello").unwrap();
    assert!(id.starts_with("mem-"));
    assert_eq!(manager.working_count(), 1);

    // Consolidate working → episodic
    let new_id = manager.consolidate_working_to_episodic().unwrap();
    assert!(new_id.is_some());
    assert_eq!(manager.working_count(), 0);
    assert_eq!(manager.episodic_count().unwrap(), 1);

    // Remember in episodic
    let _ = manager.remember_episodic("User likes tea", 0.5).unwrap();
    assert_eq!(manager.episodic_count().unwrap(), 2);
}

#[test]
fn test_consolidate_episodic_to_semantic() {
    let dir = TempDir::new().unwrap();
    let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
    let manager = TierManager::new(db.conn(), 100).unwrap();

    // Create episodic memory first so there's something to consolidate
    let _ = manager.remember_episodic("User likes coffee", 0.7).unwrap();
    assert_eq!(manager.episodic_count().unwrap(), 1);

    // Consolidate episodic → semantic
    let result = manager.consolidate_episodic_to_semantic("coffee").unwrap();
    assert!(result.is_some());

    assert_eq!(manager.semantic_count().unwrap(), 1);
}

#[test]
fn test_consolidate_empty_episodic() {
    let dir = TempDir::new().unwrap();
    let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
    let manager = TierManager::new(db.conn(), 100).unwrap();

    // No episodic memories exist
    let result = manager.consolidate_episodic_to_semantic("query").unwrap();
    assert!(result.is_none());

    assert_eq!(manager.semantic_count().unwrap(), 0);
}
