use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub struct WorkingEntry {
    pub id: String,
    pub content: String,
    pub timestamp: i64,
}

pub struct WorkingBuffer {
    entries: VecDeque<WorkingEntry>,
    capacity: usize,
}

impl WorkingBuffer {
    pub fn with_capacity(capacity: usize) -> Self {
        WorkingBuffer {
            entries: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, id: &str, content: &str) {
        let entry = WorkingEntry {
            id: id.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        };

        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    pub fn entries(&self) -> &VecDeque<WorkingEntry> {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Drain all entries for consolidation, leaving buffer empty
    pub fn drain(&mut self) -> Vec<WorkingEntry> {
        self.entries.drain(..).collect()
    }
}
