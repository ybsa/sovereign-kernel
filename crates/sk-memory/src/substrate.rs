//! Unified memory substrate — the "brain" of the Sovereign Kernel.
//!
//! Composes structured, semantic, knowledge, and session stores behind
//! a single API backed by a shared SQLite connection.

use crate::audit::AuditStore;
use crate::bm25::Bm25Index;
use crate::knowledge::KnowledgeStore;
use crate::semantic::SemanticStore;
use crate::session::SessionStore;
use crate::shared::SharedMemoryStore;
use crate::structured::StructuredStore;
use async_trait::async_trait;
use rusqlite::Connection;
use sk_types::memory::*;
use sk_types::{AgentId, SovereignError, SovereignResult};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{info, warn};
use uuid::Uuid;

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
    /// Shared semantic memory across agents.
    pub shared: SharedMemoryStore,
    /// Session persistence.
    pub sessions: SessionStore,
    /// Cryptographic audit trail.
    pub audit: AuditStore,
    /// BM25 full-text search index.
    pub bm25: Bm25Index,
    /// State checkpoints for crash recovery (Resurrector).
    pub checkpoint: crate::checkpoint::CheckpointStore,
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
        let shared = SharedMemoryStore::new(Arc::clone(&conn));
        let sessions = SessionStore::new(Arc::clone(&conn));
        let audit = AuditStore::new(Arc::clone(&conn));
        let bm25 = Bm25Index::new(Arc::clone(&conn));
        let checkpoint = crate::checkpoint::CheckpointStore::new(Arc::clone(&conn));

        let substrate = Self {
            conn,
            structured,
            semantic,
            knowledge,
            shared,
            sessions,
            audit,
            bm25,
            checkpoint,
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
        let shared = SharedMemoryStore::new(Arc::clone(&conn));
        let sessions = SessionStore::new(Arc::clone(&conn));
        let audit = AuditStore::new(Arc::clone(&conn));
        let bm25 = Bm25Index::new(Arc::clone(&conn));
        let checkpoint = crate::checkpoint::CheckpointStore::new(Arc::clone(&conn));

        let substrate = Self {
            conn,
            structured,
            semantic,
            knowledge,
            shared,
            sessions,
            audit,
            bm25,
            checkpoint,
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
                summary TEXT,
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

            -- Shared global knowledge graph
            CREATE TABLE IF NOT EXISTS global_knowledge (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                author_id TEXT NOT NULL,
                content TEXT NOT NULL,
                topic TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_global_topic ON global_knowledge(topic);

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
 
             -- State Checkpoints (for Resurrector crash recovery)
             CREATE TABLE IF NOT EXISTS checkpoints (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 agent_id TEXT NOT NULL,
                 session_id TEXT NOT NULL,
                 agent_config TEXT NOT NULL DEFAULT '{}',
                 tool_state TEXT DEFAULT '{}',
                 status TEXT NOT NULL DEFAULT 'active',
                 created_at TEXT NOT NULL DEFAULT (datetime('now'))
             );
             CREATE INDEX IF NOT EXISTS idx_checkpoints_agent ON checkpoints(agent_id);
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

    /// List all agents registered in the system.
    pub fn list_agents(&self) -> SovereignResult<Vec<sk_types::AgentEntry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, manifest, created_at, last_active FROM agents ORDER BY last_active DESC"
        ).map_err(|e| SovereignError::Memory(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let manifest_json: String = row.get(1)?;
                let created_at: String = row.get(2)?;
                let last_active: String = row.get(3)?;

                Ok((id_str, manifest_json, created_at, last_active))
            })
            .map_err(|e| SovereignError::Memory(e.to_string()))?;

        let mut agents = Vec::new();
        for row in rows {
            let (id_str, manifest_json, created_at, last_active) =
                row.map_err(|e| SovereignError::Memory(e.to_string()))?;

            let id = id_str
                .parse()
                .map_err(|_| SovereignError::Internal("Invalid AgentID in DB".into()))?;
            let manifest: sk_types::AgentManifest = serde_json::from_str(&manifest_json)
                .map_err(|e| SovereignError::Internal(e.to_string()))?;

            agents.push(sk_types::AgentEntry {
                id,
                name: manifest.name.clone(),
                manifest: manifest.clone(),
                state: sk_types::AgentState::Created,
                mode: sk_types::AgentMode::Full,
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                last_active: chrono::DateTime::parse_from_rfc3339(&last_active)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                parent: None,
                children: vec![],
                session_id: sk_types::agent::SessionId::new(),
                tags: manifest.tags.clone(),
                identity: Default::default(),
                onboarding_completed: true,
                onboarding_completed_at: None,
            });
        }
        Ok(agents)
    }

    async fn export_raw(&self) -> SovereignResult<MemoryRawData> {
        let (structured, semantic, knowledge_entities, knowledge_relations) = {
            let conn = self
                .conn
                .lock()
                .map_err(|e| SovereignError::Memory(e.to_string()))?;

            // 1. Structured
            let mut structured = HashMap::new();
            let mut stmt = conn
                .prepare("SELECT agent_id, key, value FROM kv_store")
                .map_err(|e| SovereignError::Memory(e.to_string()))?;
            let mut rows = stmt
                .query([])
                .map_err(|e| SovereignError::Memory(e.to_string()))?;
            while let Some(row) = rows
                .next()
                .map_err(|e| SovereignError::Memory(e.to_string()))?
            {
                let agent_id: String = row
                    .get(0)
                    .map_err(|e| SovereignError::Memory(e.to_string()))?;
                let key: String = row
                    .get(1)
                    .map_err(|e| SovereignError::Memory(e.to_string()))?;
                let val_str: String = row
                    .get(2)
                    .map_err(|e| SovereignError::Memory(e.to_string()))?;
                let value: serde_json::Value = serde_json::from_str(&val_str)
                    .map_err(|e| SovereignError::Memory(e.to_string()))?;

                structured
                    .entry(agent_id)
                    .or_insert_with(HashMap::new)
                    .insert(key, value);
            }

            // 2. Semantic
            let mut semantic = Vec::new();
            let mut stmt = conn.prepare("SELECT id, agent_id, content, source, created_at, accessed_at, access_count FROM semantic_memories")
                .map_err(|e| SovereignError::Memory(e.to_string()))?;
            let mut rows = stmt
                .query([])
                .map_err(|e| SovereignError::Memory(e.to_string()))?;
            while let Some(row) = rows
                .next()
                .map_err(|e| SovereignError::Memory(e.to_string()))?
            {
                let id_raw: String = row
                    .get(0)
                    .map_err(|e| SovereignError::Memory(e.to_string()))?;
                let aid_raw: String = row
                    .get(1)
                    .map_err(|e| SovereignError::Memory(e.to_string()))?;
                semantic.push(MemoryFragment {
                    id: MemoryId(Uuid::parse_str(&id_raw).unwrap_or_else(|_| Uuid::new_v4())),
                    agent_id: AgentId(Uuid::parse_str(&aid_raw).unwrap_or_else(|_| Uuid::new_v4())),
                    content: row
                        .get::<usize, String>(2)
                        .map_err(|e| SovereignError::Memory(e.to_string()))?,
                    embedding: None,
                    metadata: HashMap::new(),
                    source: MemorySource::Observation,
                    confidence: 1.0,
                    created_at: row
                        .get::<usize, String>(4)
                        .map_err(|e| SovereignError::Memory(e.to_string()))
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or_default(),
                    accessed_at: row
                        .get::<usize, String>(5)
                        .map_err(|e| SovereignError::Memory(e.to_string()))
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or_default(),
                    access_count: row
                        .get::<usize, i64>(6)
                        .map_err(|e| SovereignError::Memory(e.to_string()))?
                        as u64,
                    scope: "default".into(),
                });
            }

            // 3. Knowledge
            let mut knowledge_entities = Vec::new();
            let mut stmt = conn
                .prepare(
                    "SELECT id, name, entity_type, properties, created_at FROM knowledge_entities",
                )
                .map_err(|e| SovereignError::Memory(e.to_string()))?;
            let mut rows = stmt
                .query([])
                .map_err(|e| SovereignError::Memory(e.to_string()))?;
            while let Some(row) = rows
                .next()
                .map_err(|e| SovereignError::Memory(e.to_string()))?
            {
                knowledge_entities.push(Entity {
                    id: row
                        .get(0)
                        .map_err(|e| SovereignError::Memory(e.to_string()))?,
                    name: row
                        .get(1)
                        .map_err(|e| SovereignError::Memory(e.to_string()))?,
                    entity_type: EntityType::Concept,
                    properties: serde_json::from_str(
                        &row.get::<usize, String>(3)
                            .unwrap_or_else(|_| "{}".to_string()),
                    )
                    .unwrap_or_default(),
                    created_at: row
                        .get::<usize, String>(4)
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or_default(),
                    updated_at: row
                        .get::<usize, String>(4)
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or_default(),
                });
            }

            (structured, semantic, knowledge_entities, vec![])
        };

        Ok(MemoryRawData {
            structured,
            semantic,
            knowledge_entities,
            knowledge_relations,
        })
    }
}

#[async_trait]
impl Memory for MemorySubstrate {
    async fn get(
        &self,
        agent_id: AgentId,
        key: &str,
    ) -> SovereignResult<Option<serde_json::Value>> {
        self.structured.get(agent_id, key)
    }

    async fn set(
        &self,
        agent_id: AgentId,
        key: &str,
        value: serde_json::Value,
    ) -> SovereignResult<()> {
        self.structured.set(agent_id, key, value)
    }

    async fn delete(&self, agent_id: AgentId, key: &str) -> SovereignResult<()> {
        self.structured.delete(agent_id, key)
    }

    async fn remember(
        &self,
        agent_id: AgentId,
        content: &str,
        source: MemorySource,
        _scope: &str,
        _metadata: HashMap<String, serde_json::Value>,
    ) -> SovereignResult<MemoryId> {
        let mem_id = self
            .semantic
            .store(agent_id, content, &[], &format!("{:?}", source))?;
        self.bm25.index(agent_id, &mem_id, content)?;
        Ok(MemoryId(
            Uuid::parse_str(&mem_id).unwrap_or_else(|_| Uuid::new_v4()),
        ))
    }

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        filter: Option<MemoryFilter>,
    ) -> SovereignResult<Vec<MemoryFragment>> {
        let agent_id = filter.as_ref().and_then(|f| f.agent_id);
        let results = self.bm25.search(agent_id, query, limit)?;

        let mut fragments = Vec::new();
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(e.to_string()))?;

        for res in results {
            let mut stmt = conn.prepare("SELECT id, agent_id, content, source, created_at, accessed_at, access_count FROM semantic_memories WHERE id = ?1")
                .map_err(|e| SovereignError::Memory(e.to_string()))?;
            let mut rows = stmt
                .query(rusqlite::params![res.memory_id])
                .map_err(|e| SovereignError::Memory(e.to_string()))?;

            if let Some(row) = rows
                .next()
                .map_err(|e| SovereignError::Memory(e.to_string()))?
            {
                let id_raw: String = row
                    .get(0)
                    .map_err(|e| SovereignError::Memory(e.to_string()))?;
                let aid_raw: String = row
                    .get(1)
                    .map_err(|e| SovereignError::Memory(e.to_string()))?;
                fragments.push(MemoryFragment {
                    id: MemoryId(Uuid::parse_str(&id_raw).unwrap_or_else(|_| Uuid::new_v4())),
                    agent_id: AgentId(Uuid::parse_str(&aid_raw).unwrap_or_else(|_| Uuid::new_v4())),
                    content: row
                        .get::<usize, String>(2)
                        .map_err(|e| SovereignError::Memory(e.to_string()))?,
                    embedding: None,
                    metadata: HashMap::new(),
                    source: MemorySource::Observation,
                    confidence: 1.0,
                    created_at: row
                        .get::<usize, String>(4)
                        .map_err(|e| SovereignError::Memory(e.to_string()))
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or_default(),
                    accessed_at: row
                        .get::<usize, String>(5)
                        .map_err(|e| SovereignError::Memory(e.to_string()))
                        .unwrap_or_default()
                        .parse()
                        .unwrap_or_default(),
                    access_count: row
                        .get::<usize, i64>(6)
                        .map_err(|e| SovereignError::Memory(e.to_string()))?
                        as u64,
                    scope: "default".into(),
                });
            }
        }

        Ok(fragments)
    }

    async fn forget(&self, id: MemoryId) -> SovereignResult<()> {
        let id_str = id.to_string();
        self.semantic.delete(&id_str)?;
        self.bm25.remove(&id_str)?;
        Ok(())
    }

    async fn add_entity(&self, entity: Entity) -> SovereignResult<String> {
        let agent_id = AgentId::new();
        let properties = serde_json::to_value(&entity.properties)
            .unwrap_or(serde_json::Value::Object(Default::default()));
        self.knowledge.add_entity(
            agent_id,
            &entity.name,
            &format!("{:?}", entity.entity_type),
            properties,
        )
    }

    async fn add_relation(&self, relation: Relation) -> SovereignResult<String> {
        let agent_id = AgentId::new();
        self.knowledge.add_relation(
            agent_id,
            &relation.source,
            &format!("{:?}", relation.relation),
            &relation.target,
            1.0,
        )
    }

    async fn query_graph(&self, _pattern: GraphPattern) -> SovereignResult<Vec<GraphMatch>> {
        Ok(vec![])
    }

    async fn consolidate(&self) -> SovereignResult<ConsolidationReport> {
        Ok(ConsolidationReport {
            memories_merged: 0,
            memories_decayed: 0,
            duration_ms: 0,
        })
    }

    async fn export(&self, format: ExportFormat) -> SovereignResult<Vec<u8>> {
        match format {
            ExportFormat::Json => {
                let data = self.export_raw().await?;
                serde_json::to_vec_pretty(&data)
                    .map_err(|e| SovereignError::Memory(format!("JSON export failed: {e}")))
            }
            ExportFormat::Markdown => {
                let data = self.export_raw().await?;
                let mut md = String::new();
                md.push_str("# Sovereign Kernel Memory Export\n\n");

                md.push_str("## Structured Memory (Key-Value)\n");
                for (agent_id, kv) in data.structured {
                    md.push_str(&format!("### Agent: {}\n", agent_id));
                    for (key, value) in kv {
                        md.push_str(&format!("- **{}**: {}\n", key, value));
                    }
                }

                md.push_str("\n## Semantic Memories\n");
                for mem in data.semantic {
                    md.push_str(&format!(
                        "### [{}] - Agent: {}\n",
                        mem.created_at, mem.agent_id
                    ));
                    md.push_str(&format!("> {}\n\n", mem.content));
                }

                md.push_str("\n## Knowledge Entities\n");
                for entity in data.knowledge_entities {
                    md.push_str(&format!("- **{}** ({})\n", entity.name, entity.id));
                }

                Ok(md.into_bytes())
            }
            ExportFormat::MessagePack => Err(SovereignError::Memory(
                "MessagePack export not implemented yet".into(),
            )),
        }
    }

    async fn import(&self, data: &[u8], format: ExportFormat) -> SovereignResult<ImportReport> {
        match format {
            ExportFormat::Json => {
                let raw: MemoryRawData = serde_json::from_slice(data)
                    .map_err(|e| SovereignError::Memory(format!("JSON import failed: {e}")))?;

                let mut report = ImportReport::default();

                for (agent_id_str, kv) in raw.structured {
                    let agent_id = AgentId::parse(&agent_id_str).unwrap_or_default();
                    for (key, value) in kv {
                        if let Err(e) = self.structured.set(agent_id, &key, value) {
                            report
                                .errors
                                .push(format!("KV import failed for {key}: {e}"));
                        } else {
                            report.kv_imported += 1;
                        }
                    }
                }

                for mem in raw.semantic {
                    if let Err(e) = self
                        .remember(
                            mem.agent_id,
                            &mem.content,
                            mem.source,
                            &mem.scope,
                            mem.metadata,
                        )
                        .await
                    {
                        report.errors.push(format!("Memory import failed: {e}"));
                    } else {
                        report.memories_imported += 1;
                    }
                }

                for entity in raw.knowledge_entities {
                    if let Err(e) = self.add_entity(entity).await {
                        report.errors.push(format!("Entity import failed: {e}"));
                    } else {
                        report.entities_imported += 1;
                    }
                }

                Ok(report)
            }
            ExportFormat::Markdown => {
                warn!("Markdown import is lossy and only supports semantic memories currently");
                let content = String::from_utf8_lossy(data);
                let mut report = ImportReport::default();

                // Very simple regex-based parser for our export format
                let mut current_agent = AgentId::new();
                for line in content.lines() {
                    if line.starts_with("### [") && line.contains("] - Agent: ") {
                        if let Some(aid_part) = line.split("Agent: ").nth(1) {
                            current_agent =
                                AgentId::parse(aid_part.trim()).unwrap_or(current_agent);
                        }
                    } else if let Some(text) = line.strip_prefix("> ") {
                        self.remember(
                            current_agent,
                            text,
                            MemorySource::Observation,
                            "imported",
                            HashMap::new(),
                        )
                        .await?;
                        report.memories_imported += 1;
                    }
                }

                Ok(report)
            }
            _ => Err(SovereignError::Memory("Import format not supported".into())),
        }
    }
}

/// Raw data structure for memory export/import
#[derive(serde::Serialize, serde::Deserialize)]
struct MemoryRawData {
    structured: HashMap<String, HashMap<String, serde_json::Value>>,
    semantic: Vec<MemoryFragment>,
    knowledge_entities: Vec<Entity>,
    knowledge_relations: Vec<Relation>,
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
