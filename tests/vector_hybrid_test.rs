#[cfg(feature = "vec")]
mod hybrid_tests {
    use mnemo::embed::EmbeddingGateway;
    use mnemo::store::{MemoryStore, MnemoDb, VectorStore};
    use mnemo::tier::TierManager;
    use tempfile::TempDir;

    fn provider_available() -> bool {
        EmbeddingGateway::from_env().is_some()
    }

    /// Test that hybrid search finds memories even when query words
    /// don't lexically overlap with stored memory text.
    #[test]
    fn test_hybrid_search_finds_paraphrase() {
        if !provider_available() {
            eprintln!("Skipping: no embedding provider configured");
            return;
        }

        let dir = TempDir::new().unwrap();
        let db = MnemoDb::new(dir.path().join("test.db")).unwrap();
        let store = MemoryStore::new(db.conn());

        // Store a semantic memory
        let _id = store
            .insert("semantic", "User prefers dark mode", 0.8, "test", &[])
            .unwrap();

        // Query with a paraphrase that has zero word overlap
        let manager = TierManager::new(db.conn(), 100).unwrap();
        let vstore = VectorStore::new(db.conn());
        let gateway = EmbeddingGateway::from_env_cached().unwrap();

        // The query "visual settings" shares no words with "dark mode"
        let results = manager
            .recall_hybrid(
                "visual settings",
                &["visual".to_string(), "settings".to_string()],
                &["semantic".to_string()],
                10,
                &vstore,
                gateway,
            )
            .unwrap();

        // We expect the hybrid search to find the memory via vector similarity
        // even though FTS5 alone would return nothing
        let found = results
            .iter()
            .any(|m| m.content == "User prefers dark mode");

        assert!(
            found,
            "Hybrid search should find 'User prefers dark mode' from query 'visual settings'. \
             Got {} results: {:?}",
            results.len(),
            results.iter().map(|m| &m.content).collect::<Vec<_>>()
        );
    }
}
