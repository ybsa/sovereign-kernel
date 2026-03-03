//! Unified memory substrate — the "brain" of the Sovereign Kernel.
//!
//! Composes structured, semantic, knowledge, and session stores behind
//! a single API backed by a shared SQLite connection.

use crate::audit::AuditStore;
use crate::bm25::Bm25Index;
use crate::knowledge::KnowledgeStore;
use crate::semantic::SemanticStore;
use crate::session::SessionStore;
use crate::structured::StructuredStore;
use rusqlite::Connection;
use sk_types::{SovereignError, SovereignResult};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::info;

/// The unified memory substrate.
///
/// All memory operations go through this struct, which delegates to
/// specialized stores backed by a shared SQLite connection.
pub struct MemorySubstrate {
    /// Shared database connection.
    conn: Arc<Mutex<Connection>>,
    /// Key-value structured store.
    pub structured: StructuredStore,
    /// Vector-based semantic search.
    pub semantic: SemanticStore,
    /// Entity-relation knowledge graph.
    pub knowledge: KnowledgeStore,
    /// Session persistence.
    pub sessions: SessionStore,
    /// Cryptographic audit trail.
    pub audit: AuditStore,
    /// BM25 full-text search index.
    pub bm25: Bm25Index,
    /// Memory decay rate for temporal scoring.
    decay_rate: f32,
}

impl MemorySubstrate {
    /// Open or create a memory substrate at the given database path.
    pub fn open(db_path: &Path, decay_rate: f32) -> SovereignResult<Self> {
        let conn = Connection::open(db_path)
            .map_err(|e| SovereignError::Memory(format!("Failed to open database: {e}")))?;

        // Enable WAL mode for concurrent reads
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| SovereignError::Memory(format!("PRAGMA failed: {e}")))?;

        let conn = Arc::new(Mutex::new(conn));

        let structured = StructuredStore::new(Arc::clone(&conn));
        let semantic = SemanticStore::new(Arc::clone(&conn));
        let knowledge = KnowledgeStore::new(Arc::clone(&conn));
        let sessions = SessionStore::new(Arc::clone(&conn));
        let audit = AuditStore::new(Arc::clone(&conn));
        let bm25 = Bm25Index::new(Arc::clone(&conn));

        let substrate = Self {
            conn,
            structured,
            semantic,
            knowledge,
            sessions,
            audit,
            bm25,
            decay_rate,
        };

        substrate.initialize_schema()?;
        info!(path = %db_path.display(), "Memory substrate opened");
        Ok(substrate)
    }

    /// Create an in-memory substrate (for testing).
    pub fn open_in_memory(decay_rate: f32) -> SovereignResult<Self> {
        let conn = Connection::open_in_memory().map_err(|e| {
            SovereignError::Memory(format!("Failed to open in-memory database: {e}"))
        })?;

        conn.execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(|e| SovereignError::Memory(format!("PRAGMA failed: {e}")))?;

        let conn = Arc::new(Mutex::new(conn));

        let structured = StructuredStore::new(Arc::clone(&conn));
        let semantic = SemanticStore::new(Arc::clone(&conn));
        let knowledge = KnowledgeStore::new(Arc::clone(&conn));
        let sessions = SessionStore::new(Arc::clone(&conn));
        let audit = AuditStore::new(Arc::clone(&conn));
        let bm25 = Bm25Index::new(Arc::clone(&conn));

        let substrate = Self {
            conn,
            structured,
            semantic,
            knowledge,
            sessions,
            audit,
            bm25,
            decay_rate,
        };

        substrate.initialize_schema()?;
        Ok(substrate)
    }

    /// Initialize all database tables.
    fn initialize_schema(&self) -> SovereignResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        conn.execute_batch(
            "
            -- Agents
            CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                manifest TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_active TEXT NOT NULL
            );

            -- Structured KV store
            CREATE TABLE IF NOT EXISTS kv_store (
                agent_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (agent_id, key)
            );

            -- Sessions
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                messages TEXT NOT NULL DEFAULT '[]',
                label TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_agent ON sessions(agent_id);

            -- Semantic memory (vector embeddings stored as BLOBs)
            CREATE TABLE IF NOT EXISTS semantic_memories (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                content TEXT NOT NULL,
                embedding BLOB,
                source TEXT NOT NULL DEFAULT 'conversation',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                accessed_at TEXT NOT NULL DEFAULT (datetime('now')),
                access_count INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_semantic_agent ON semantic_memories(agent_id);

            -- Knowledge graph (entity-relation triples)
            CREATE TABLE IF NOT EXISTS knowledge_entities (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                name TEXT NOT NULL,
                entity_type TEXT NOT NULL DEFAULT 'concept',
                properties TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_entities_agent ON knowledge_entities(agent_id);

            CREATE TABLE IF NOT EXISTS knowledge_relations (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                from_entity TEXT NOT NULL,
                relation TEXT NOT NULL,
                to_entity TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (from_entity) REFERENCES knowledge_entities(id),
                FOREIGN KEY (to_entity) REFERENCES knowledge_entities(id)
            );
            CREATE INDEX IF NOT EXISTS idx_relations_agent ON knowledge_relations(agent_id);

            -- BM25 full-text search
            CREATE VIRTUAL TABLE IF NOT EXISTS fts_memories USING fts5(
                content,
                agent_id UNINDEXED,
                memory_id UNINDEXED,
                tokenize='porter unicode61'
            );

            -- Cryptographic Audit Trail (Merkle Chain)
            CREATE TABLE IF NOT EXISTS audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_id TEXT NOT NULL,
                execution_mode TEXT NOT NULL,
                action_type TEXT NOT NULL,
                action_data TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                hash TEXT NOT NULL,
                previous_hash TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_audit_agent ON audit_logs(agent_id);
            ",
        )
        .map_err(|e| SovereignError::Memory(format!("Schema init failed: {e}")))?;

        Ok(())
    }

    /// Get the memory decay rate.
    pub fn decay_rate(&self) -> f32 {
        self.decay_rate
    }

    /// Get a reference to the shared database connection.
    pub fn conn(&self) -> &Arc<Mutex<Connection>> {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory() {
        let substrate = MemorySubstrate::open_in_memory(0.1).unwrap();
        assert!((substrate.decay_rate() - 0.1).abs() < f32::EPSILON);
    }
}
