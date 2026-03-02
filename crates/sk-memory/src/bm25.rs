//! BM25 full-text search via SQLite FTS5.
//!
//! Ported from OpenClaw's QMD memory manager — provides keyword-based
//! retrieval using the Okapi BM25 ranking algorithm built into FTS5.

use rusqlite::Connection;
use sk_types::{AgentId, SovereignError, SovereignResult};
use std::sync::{Arc, Mutex};

/// A BM25 search result.
#[derive(Debug, Clone)]
pub struct Bm25Result {
    /// The matching memory ID.
    pub memory_id: String,
    /// The matching content.
    pub content: String,
    /// BM25 rank score (lower = more relevant in FTS5).
    pub rank: f64,
}

/// BM25 full-text search index backed by SQLite FTS5.
pub struct Bm25Index {
    conn: Arc<Mutex<Connection>>,
}

impl Bm25Index {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Index a memory entry for full-text search.
    pub fn index(&self, agent_id: AgentId, memory_id: &str, content: &str) -> SovereignResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;
        conn.execute(
            "INSERT INTO fts_memories (content, agent_id, memory_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![content, agent_id.to_string(), memory_id],
        )
        .map_err(|e| SovereignError::Memory(e.to_string()))?;
        Ok(())
    }

    /// Search using BM25 ranking.
    pub fn search(
        &self,
        agent_id: AgentId,
        query: &str,
        limit: usize,
    ) -> SovereignResult<Vec<Bm25Result>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        // Sanitize query for FTS5
        let sanitized = sanitize_fts5_query(query);
        if sanitized.is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = conn
            .prepare(
                "SELECT memory_id, content, rank
                 FROM fts_memories
                 WHERE fts_memories MATCH ?1 AND agent_id = ?2
                 ORDER BY rank
                 LIMIT ?3",
            )
            .map_err(|e| SovereignError::Memory(e.to_string()))?;

        let rows = stmt
            .query_map(
                rusqlite::params![sanitized, agent_id.to_string(), limit as i64],
                |row| {
                    Ok(Bm25Result {
                        memory_id: row.get(0)?,
                        content: row.get(1)?,
                        rank: row.get(2)?,
                    })
                },
            )
            .map_err(|e| SovereignError::Memory(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| SovereignError::Memory(e.to_string()))?);
        }
        Ok(results)
    }

    /// Remove a memory from the FTS index.
    pub fn remove(&self, memory_id: &str) -> SovereignResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;
        conn.execute(
            "DELETE FROM fts_memories WHERE memory_id = ?1",
            rusqlite::params![memory_id],
        )
        .map_err(|e| SovereignError::Memory(e.to_string()))?;
        Ok(())
    }
}

/// Sanitize a query string for FTS5 syntax.
///
/// FTS5 has special characters (AND, OR, NOT, *, ", etc.) that need escaping
/// to prevent syntax errors on user input.
fn sanitize_fts5_query(query: &str) -> String {
    // Wrap each word in double quotes to treat as literal terms
    query
        .split_whitespace()
        .filter(|w| !w.is_empty())
        .map(|word| {
            let cleaned: String = word
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();
            if cleaned.is_empty() {
                String::new()
            } else {
                format!("\"{cleaned}\"")
            }
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_simple() {
        assert_eq!(sanitize_fts5_query("hello world"), "\"hello\" \"world\"");
    }

    #[test]
    fn sanitize_special_chars() {
        assert_eq!(sanitize_fts5_query("test*"), "\"test\"");
    }

    #[test]
    fn sanitize_empty() {
        assert_eq!(sanitize_fts5_query(""), "");
    }
}
