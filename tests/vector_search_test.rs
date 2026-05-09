#[cfg(feature = "vec")]
mod vector_tests {
    use mnemo::embed::EmbeddingGateway;
    use mnemo::store::{MemoryStore, MnemoDb, VectorStore};
    use tempfile::TempDir;

    fn provider_available() -> bool {
        EmbeddingGateway::from_env().is_some()
    }

    #[test]
    fn test_semantic_memory_gets_embedded() {
        if !provider_available() {
            eprintln!(
                "Skipping test_semantic_memory_gets_embedded: no embedding provider configured"
            );
            return;
        }

        let dir = TempDir::new().unwrap();
        let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
        let store = MemoryStore::new(db.conn());

        let _id = store
            .insert("semantic", "User prefers dark mode", 0.8, "test", &[])
            .unwrap();

        let vstore = VectorStore::new(db.conn());
        let count = vstore.count().unwrap();
        assert!(
            count > 0,
            "Semantic memory should have embedding stored, got count={}",
            count
        );
    }

    #[test]
    fn test_working_memory_skips_embedding() {
        let dir = TempDir::new().unwrap();
        let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
        let store = MemoryStore::new(db.conn());

        let _id = store
            .insert("working", "Transient thought", 0.5, "test", &[])
            .unwrap();

        let vstore = VectorStore::new(db.conn());
        let count = vstore.count().unwrap();
        assert_eq!(
            count, 0,
            "Working memory should NOT have embedding stored, got count={}",
            count
        );
    }

    #[test]
    fn test_episodic_memory_gets_embedded() {
        if !provider_available() {
            eprintln!(
                "Skipping test_episodic_memory_gets_embedded: no embedding provider configured"
            );
            return;
        }

        let dir = TempDir::new().unwrap();
        let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
        let store = MemoryStore::new(db.conn());

        let _id = store
            .insert("episodic", "Met with Alice yesterday", 0.6, "test", &[])
            .unwrap();

        let vstore = VectorStore::new(db.conn());
        let count = vstore.count().unwrap();
        assert!(
            count > 0,
            "Episodic memory should have embedding stored, got count={}",
            count
        );
    }
}
