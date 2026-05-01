use crate::store::{Memory, MemoryStore};
use crate::tier::WorkingBuffer;
use rusqlite::Connection;

pub struct TierManager<'a> {
    working: WorkingBuffer,
    store: MemoryStore<'a>,
}

impl<'a> TierManager<'a> {
    pub fn new(conn: &'a Connection, working_capacity: usize) -> rusqlite::Result<Self> {
        Ok(TierManager {
            working: WorkingBuffer::with_capacity(working_capacity),
            store: MemoryStore::new(conn),
        })
    }

    pub fn remember_working(&mut self, content: &str) -> rusqlite::Result<String> {
        let id = self.store.insert("working", content, 0.5, "observation", &[])?;
        self.working.push(&id, content);
        Ok(id)
    }

    pub fn remember_episodic(&self, content: &str, importance: f64) -> rusqlite::Result<String> {
        self.store.insert("episodic", content, importance, "observation", &[])
    }

    pub fn remember_semantic(
        &self,
        content: &str,
        importance: f64,
        tags: &[&str],
    ) -> rusqlite::Result<String> {
        self.store.insert("semantic", content, importance, "user_stated", tags)
    }

    pub fn recall(
        &self,
        query: &str,
        memory_types: &[String],
        limit: usize,
    ) -> rusqlite::Result<Vec<Memory>> {
        self.store.search_content(query, memory_types, limit)
    }

    pub fn consolidate_working_to_episodic(
        &mut self,
    ) -> rusqlite::Result<Option<String>> {
        let entries = self.working.drain();
        if entries.is_empty() {
            return Ok(None);
        }

        let contents: Vec<String> = entries.iter().map(|e| e.content.clone()).collect();
        let summary = format!("[Consolidated] {}", contents.join("; "));

        let id = self.store.insert("episodic", &summary, 0.5, "consolidation", &[])?;
        Ok(Some(id))
    }

    pub fn consolidate_episodic_to_semantic(
        &self,
        query: &str,
    ) -> rusqlite::Result<Option<String>> {
        let episodes = self.store.search_content(query, &["episodic".to_string()], 10)?;
        if episodes.is_empty() {
            return Ok(None);
        }

        let content = format!("[Extracted] {}", episodes[0].content);
        let id = self.store.insert("semantic", &content, 0.7, "consolidation", &[])?;
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
