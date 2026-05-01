use mnemo::store::MnemoDb;
use tempfile::TempDir;

#[test]
fn test_db_initialization() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let db = MnemoDb::new(&db_path).unwrap();

    // Verify tables exist by inserting a memory
    db.conn()
        .execute(
            "INSERT INTO memories (id, memory_type, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            ["mem-test", "semantic", "Test content", "0"],
        )
        .unwrap();
}
