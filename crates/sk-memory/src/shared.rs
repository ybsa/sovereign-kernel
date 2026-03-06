//! Shared Semantic Memory — global knowledge graph accessible to all permitted agents.

use rusqlite::Connection;
use sk_types::{AgentId, SovereignError, SovereignResult};
use std::sync::{Arc, Mutex};

/// Global knowledge store — not partitioned by AgentId.
pub struct SharedMemoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl SharedMemoryStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Store a piece of shared knowledge.
    pub fn store(&self, author_id: AgentId, content: &str, topic: &str) -> SovereignResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        // Simple insertion
        conn.execute(
            "INSERT INTO global_knowledge (author_id, content, topic, created_at)
             VALUES (?1, ?2, ?3, datetime('now'))",
            rusqlite::params![author_id.to_string(), content, topic],
        )
        .map_err(|e| SovereignError::Memory(e.to_string()))?;
        Ok(())
    }

    /// Recall shared knowledge by topic or keyword (basic LIKE search for now).
    pub fn recall(&self, query: &str) -> SovereignResult<Vec<(String, String, String)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        let pattern = format!("%{}%", query);
        let mut stmt = conn
            .prepare("SELECT author_id, content, created_at FROM global_knowledge WHERE topic LIKE ?1 OR content LIKE ?1 ORDER BY created_at DESC LIMIT 50")
            .map_err(|e| SovereignError::Memory(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![pattern], |row| {
                let author_id: String = row.get(0)?;
                let content: String = row.get(1)?;
                let created_at: String = row.get(2)?;
                Ok((author_id, content, created_at))
            })
            .map_err(|e| SovereignError::Memory(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let (author_id, content, created_at) =
                row.map_err(|e| SovereignError::Memory(e.to_string()))?;
            results.push((author_id, content, created_at));
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_memory_store_and_recall() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE global_knowledge (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                author_id TEXT NOT NULL,
                content TEXT NOT NULL,
                topic TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
        ",
        )
        .unwrap();
        let conn = Arc::new(Mutex::new(conn));
        let store = SharedMemoryStore::new(conn);

        let agent_a = AgentId::new();

        // Agent A stores something
        store
            .store(agent_a.clone(), "The password is 'banana'", "secrets")
            .unwrap();

        // Recall by content
        let results = store.recall("banana").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, agent_a.to_string());
        assert_eq!(results[0].1, "The password is 'banana'");

        // Recall by topic
        let results_topic = store.recall("secrets").unwrap();
        assert_eq!(results_topic.len(), 1);
        assert_eq!(results_topic[0].1, "The password is 'banana'");
    }
}
