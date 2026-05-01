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
