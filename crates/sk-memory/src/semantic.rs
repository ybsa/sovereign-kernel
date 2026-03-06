//! Vector-based semantic memory search.
//!
//! Stores text with embedding vectors as SQLite BLOBs and performs
//! cosine similarity search in pure Rust — no external vector DB needed.

use rusqlite::Connection;
use sk_types::{AgentId, SovereignError, SovereignResult};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// A semantic memory entry with its embedding.
#[derive(Debug, Clone)]
pub struct SemanticEntry {
    pub id: String,
    pub agent_id: AgentId,
    pub content: String,
    pub source: String,
    pub created_at: String,
}

/// A search result with similarity score.
#[derive(Debug, Clone)]
pub struct SemanticSearchResult {
    pub entry: SemanticEntry,
    pub similarity: f32,
}

/// Vector-based semantic memory store.
pub struct SemanticStore {
    conn: Arc<Mutex<Connection>>,
}

impl SemanticStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Store a memory with its embedding vector.
    pub fn store(
        &self,
        agent_id: AgentId,
        content: &str,
        embedding: &[f32],
        source: &str,
    ) -> SovereignResult<String> {
        let id = Uuid::new_v4().to_string();
        let embedding_bytes = embedding_to_bytes(embedding);
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;
        conn.execute(
            "INSERT INTO semantic_memories (id, agent_id, content, embedding, source) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, agent_id.to_string(), content, embedding_bytes, source],
        )
        .map_err(|e| SovereignError::Memory(e.to_string()))?;
        Ok(id)
    }

    /// Search for similar memories using cosine similarity.
    ///
    /// Returns the top `limit` results sorted by similarity (descending).
    pub fn search(
        &self,
        agent_id: AgentId,
        query_embedding: &[f32],
        limit: usize,
    ) -> SovereignResult<Vec<SemanticSearchResult>> {
        let mut results = Vec::new();

        {
            let conn = self
                .conn
                .lock()
                .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;
            let mut stmt = conn
                .prepare(
                    "SELECT id, agent_id, content, embedding, source, created_at
                     FROM semantic_memories WHERE agent_id = ?1 AND embedding IS NOT NULL",
                )
                .map_err(|e| SovereignError::Memory(e.to_string()))?;

            let rows = stmt
                .query_map(rusqlite::params![agent_id.to_string()], |row| {
                    let id: String = row.get(0)?;
                    let content: String = row.get(2)?;
                    let emb_bytes: Vec<u8> = row.get(3)?;
                    let source: String = row.get(4)?;
                    let created_at: String = row.get(5)?;
                    Ok((id, content, emb_bytes, source, created_at))
                })
                .map_err(|e| SovereignError::Memory(e.to_string()))?;

            for row in rows {
                let (id, content, emb_bytes, source, created_at) =
                    row.map_err(|e| SovereignError::Memory(e.to_string()))?;
                let embedding = embedding_from_bytes(&emb_bytes);
                let similarity = cosine_similarity(query_embedding, &embedding);

                results.push(SemanticSearchResult {
                    entry: SemanticEntry {
                        id,
                        agent_id,
                        content,
                        source,
                        created_at,
                    },
                    similarity,
                });
            }
        } // conn lock released here

        // Sort by similarity descending
        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);

        // Update access timestamps for returned results
        for result in &results {
            self.touch(&result.entry.id)?;
        }

        Ok(results)
    }

    /// Update the access timestamp and count for a memory.
    fn touch(&self, memory_id: &str) -> SovereignResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;
        conn.execute(
            "UPDATE semantic_memories SET accessed_at = datetime('now'), access_count = access_count + 1 WHERE id = ?1",
            rusqlite::params![memory_id],
        )
        .map_err(|e| SovereignError::Memory(e.to_string()))?;
        Ok(())
    }

    /// Delete a specific memory by ID.
    pub fn delete(&self, memory_id: &str) -> SovereignResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;
        conn.execute(
            "DELETE FROM semantic_memories WHERE id = ?1",
            rusqlite::params![memory_id],
        )
        .map_err(|e| SovereignError::Memory(e.to_string()))?;
        Ok(())
    }

    /// Count total memories for an agent.
    pub fn count(&self, agent_id: AgentId) -> SovereignResult<usize> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM semantic_memories WHERE agent_id = ?1",
                rusqlite::params![agent_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|e| SovereignError::Memory(e.to_string()))?;
        Ok(count as usize)
    }
}

// ---------------------------------------------------------------------------
// Vector math utilities (from Sovereign Kernel embedding.rs)
// ---------------------------------------------------------------------------

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for (&x, &y) in a.iter().zip(b.iter()) {
        let x = x as f64;
        let y = y as f64;
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }
    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < 1e-10 {
        0.0
    } else {
        (dot / denom) as f32
    }
}

/// Serialize an embedding vector to bytes (for SQLite BLOB storage).
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// Deserialize an embedding vector from bytes.
pub fn embedding_from_bytes(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical() {
        let v = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-5);
    }

    #[test]
    fn embedding_roundtrip() {
        let original = vec![1.0f32, -2.5, std::f32::consts::PI, 0.0];
        let bytes = embedding_to_bytes(&original);
        let recovered = embedding_from_bytes(&bytes);
        assert_eq!(original, recovered);
    }
}
