//! Knowledge graph — entity-relation triples in SQLite.

use rusqlite::Connection;
use sk_types::{AgentId, SovereignError, SovereignResult};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// A knowledge entity.
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: String,
    pub agent_id: AgentId,
    pub name: String,
    pub entity_type: String,
    pub properties: serde_json::Value,
}

/// A relation between two entities.
#[derive(Debug, Clone)]
pub struct Relation {
    pub id: String,
    pub from_entity: String,
    pub relation: String,
    pub to_entity: String,
    pub weight: f64,
}

/// Knowledge graph store backed by SQLite.
pub struct KnowledgeStore {
    conn: Arc<Mutex<Connection>>,
}

impl KnowledgeStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Add an entity to the knowledge graph.
    pub fn add_entity(
        &self,
        agent_id: AgentId,
        name: &str,
        entity_type: &str,
        properties: serde_json::Value,
    ) -> SovereignResult<String> {
        let id = Uuid::new_v4().to_string();
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::MemoryError(format!("Lock: {e}")))?;
        conn.execute(
            "INSERT INTO knowledge_entities (id, agent_id, name, entity_type, properties) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, agent_id.to_string(), name, entity_type, properties.to_string()],
        ).map_err(|e| SovereignError::MemoryError(e.to_string()))?;
        Ok(id)
    }

    /// Add a relation between two entities.
    pub fn add_relation(
        &self,
        agent_id: AgentId,
        from_entity: &str,
        relation: &str,
        to_entity: &str,
        weight: f64,
    ) -> SovereignResult<String> {
        let id = Uuid::new_v4().to_string();
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::MemoryError(format!("Lock: {e}")))?;
        conn.execute(
            "INSERT INTO knowledge_relations (id, agent_id, from_entity, relation, to_entity, weight) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![id, agent_id.to_string(), from_entity, relation, to_entity, weight],
        ).map_err(|e| SovereignError::MemoryError(e.to_string()))?;
        Ok(id)
    }

    /// Find entities by name pattern.
    pub fn find_entities(&self, agent_id: AgentId, pattern: &str) -> SovereignResult<Vec<Entity>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::MemoryError(format!("Lock: {e}")))?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, name, entity_type, properties FROM knowledge_entities WHERE agent_id = ?1 AND name LIKE ?2"
        ).map_err(|e| SovereignError::MemoryError(e.to_string()))?;

        let pattern = format!("%{pattern}%");
        let rows = stmt
            .query_map(rusqlite::params![agent_id.to_string(), pattern], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(2)?;
                let entity_type: String = row.get(3)?;
                let props_str: String = row.get(4)?;
                Ok((id, name, entity_type, props_str))
            })
            .map_err(|e| SovereignError::MemoryError(e.to_string()))?;

        let mut entities = Vec::new();
        for row in rows {
            let (id, name, entity_type, props_str) =
                row.map_err(|e| SovereignError::MemoryError(e.to_string()))?;
            let properties: serde_json::Value =
                serde_json::from_str(&props_str).unwrap_or_default();
            entities.push(Entity {
                id,
                agent_id,
                name,
                entity_type,
                properties,
            });
        }
        Ok(entities)
    }

    /// Get all relations for an entity.
    pub fn get_relations(&self, entity_id: &str) -> SovereignResult<Vec<Relation>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::MemoryError(format!("Lock: {e}")))?;
        let mut stmt = conn.prepare(
            "SELECT id, from_entity, relation, to_entity, weight FROM knowledge_relations WHERE from_entity = ?1 OR to_entity = ?1"
        ).map_err(|e| SovereignError::MemoryError(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![entity_id], |row| {
                Ok(Relation {
                    id: row.get(0)?,
                    from_entity: row.get(1)?,
                    relation: row.get(2)?,
                    to_entity: row.get(3)?,
                    weight: row.get(4)?,
                })
            })
            .map_err(|e| SovereignError::MemoryError(e.to_string()))?;

        let mut relations = Vec::new();
        for row in rows {
            relations.push(row.map_err(|e| SovereignError::MemoryError(e.to_string()))?);
        }
        Ok(relations)
    }
}
