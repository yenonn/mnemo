use crate::store::{Memory, MemoryStore};
use crate::tier::WorkingBuffer;
use rusqlite::Connection;

pub struct TierManager<'a> {
    working: WorkingBuffer,
    store: MemoryStore<'a>,
}

impl<'a> TierManager<'a> {
    pub fn new(conn: &'a Connection, working_capacity: usize) -> rusqlite::Result<Self> {
        let mut working = WorkingBuffer::with_capacity(working_capacity);
        let store = MemoryStore::new(conn);
        // Hydrate working buffer from existing DB entries
        let mut stmt = conn.prepare(
            "SELECT id, content FROM memories WHERE memory_type = 'working' ORDER BY created_at DESC LIMIT ?"
        )?;
        let rows = stmt.query_map([working_capacity as i64], |row| {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            Ok((id, content))
        })?;
        // Collect then reverse so buffer ends up in chronological order
        let mut loaded: Vec<(String, String)> = rows.collect::<Result<_, _>>()?;
        loaded.reverse();
        for (id, content) in loaded {
            working.push(&id, &content);
        }
        Ok(TierManager { working, store })
    }

    pub fn remember_working(&mut self, content: &str) -> rusqlite::Result<String> {
        let id = self
            .store
            .insert("working", content, 0.5, "observation", &[])?;
        self.working.push(&id, content);
        Ok(id)
    }

    pub fn remember_episodic(&self, content: &str, importance: f64) -> rusqlite::Result<String> {
        self.store
            .insert("episodic", content, importance, "observation", &[])
    }

    pub fn remember_semantic(
        &self,
        content: &str,
        importance: f64,
        tags: &[&str],
    ) -> rusqlite::Result<String> {
        self.store
            .insert("semantic", content, importance, "user_stated", tags)
    }

    pub fn recall(
        &self,
        query: &str,
        memory_types: &[String],
        limit: usize,
    ) -> rusqlite::Result<Vec<Memory>> {
        self.store.search_content(query, memory_types, limit)
    }

    pub fn recall_expanded(
        &self,
        expanded_terms: &[String],
        memory_types: &[String],
        limit: usize,
    ) -> rusqlite::Result<Vec<Memory>> {
        self.store
            .search_content_expanded(expanded_terms, memory_types, limit)
    }

    #[cfg(feature = "vec")]
    pub fn recall_hybrid(
        &self,
        query_text: &str,
        expanded_terms: &[String],
        memory_types: &[String],
        limit: usize,
        vstore: &crate::store::VectorStore,
        gateway: &crate::embed::EmbeddingGateway,
    ) -> rusqlite::Result<Vec<Memory>> {
        self.store.search_hybrid(
            query_text,
            expanded_terms,
            memory_types,
            limit,
            vstore,
            gateway,
        )
    }

    pub fn consolidate_working_to_episodic(&mut self) -> rusqlite::Result<Option<String>> {
        let entries = self.working.drain();
        if entries.is_empty() {
            return Ok(None);
        }

        // Delete DB working rows
        self.store
            .conn()
            .execute("DELETE FROM memories WHERE memory_type = 'working'", [])?;

        let contents: Vec<String> = entries.iter().map(|e| e.content.clone()).collect();
        let summary = format!("[Consolidated] {}", contents.join("; "));

        let id = self
            .store
            .insert("episodic", &summary, 0.5, "consolidation", &[])?;
        Ok(Some(id))
    }

    pub fn clear_working(&mut self) {
        self.working.clear();
        let _ = self
            .store
            .conn()
            .execute("DELETE FROM memories WHERE memory_type = 'working'", []);
    }

    pub fn consolidate_episodic_to_semantic(
        &self,
        query: &str,
    ) -> rusqlite::Result<Option<String>> {
        let episodes = self
            .store
            .search_content(query, &["episodic".to_string()], 10)?;
        if episodes.is_empty() {
            return Ok(None);
        }

        let content = format!("[Extracted] {}", episodes[0].content);
        let id = self
            .store
            .insert("semantic", &content, 0.7, "consolidation", &[])?;
        Ok(Some(id))
    }

    pub fn working_count(&self) -> usize {
        self.working.len()
    }

    pub fn episodic_count(&self) -> rusqlite::Result<usize> {
        let mut stmt = self
            .store
            .conn()
            .prepare("SELECT COUNT(*) FROM memories WHERE memory_type = 'episodic'")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as usize)
    }

    pub fn semantic_count(&self) -> rusqlite::Result<usize> {
        let mut stmt = self
            .store
            .conn()
            .prepare("SELECT COUNT(*) FROM memories WHERE memory_type = 'semantic'")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as usize)
    }
}
