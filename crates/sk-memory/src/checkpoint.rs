//! State Checkpoints — persistence for agent state to enable "Resurrection" after crash.
use rusqlite::{params, Connection, Row};
use sk_types::{AgentId, SovereignError, SovereignResult};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// A point-in-time snapshot of an agent's state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Checkpoint {
    pub id: i64,
    pub agent_id: AgentId,
    pub session_id: Uuid,
    pub agent_config: serde_json::Value,
    pub tool_state: serde_json::Value,
    pub status: String,
    pub created_at: String,
}

pub struct CheckpointStore {
    conn: Arc<Mutex<Connection>>,
}

impl CheckpointStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Save a new checkpoint for an agent.
    pub fn save(
        &self,
        agent_id: &AgentId,
        session_id: &Uuid,
        agent_config: &serde_json::Value,
        tool_state: &serde_json::Value,
        status: &str,
    ) -> SovereignResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        let config_json = serde_json::to_string(agent_config).map_err(|e| {
            SovereignError::Internal(format!("Failed to serialize agent_config: {e}"))
        })?;
        let tool_state_json = serde_json::to_string(tool_state).map_err(|e| {
            SovereignError::Internal(format!("Failed to serialize tool_state: {e}"))
        })?;

        conn.execute(
            "INSERT INTO checkpoints (agent_id, session_id, agent_config, tool_state, status, created_at)
             VALUES (?, ?, ?, ?, ?, datetime('now'))",
            params![
                agent_id.to_string(),
                session_id.to_string(),
                config_json,
                tool_state_json,
                status.to_string()
            ],
        )
        .map_err(|e| SovereignError::Memory(format!("Failed to save checkpoint: {e}")))?;

        Ok(())
    }

    /// Load the most recent checkpoint for a given agent.
    pub fn load_latest(&self, agent_id: &AgentId) -> SovereignResult<Option<Checkpoint>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, agent_id, session_id, agent_config, tool_state, status, created_at
             FROM checkpoints
             WHERE agent_id = ?
             ORDER BY created_at DESC
             LIMIT 1",
            )
            .map_err(|e| SovereignError::Memory(format!("Failed to prepare query: {e}")))?;

        let mut rows = stmt
            .query(params![agent_id.to_string()])
            .map_err(|e| SovereignError::Memory(format!("Query failed: {e}")))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| SovereignError::Memory(format!("Row error: {e}")))?
        {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all checkpoints for an agent.
    pub fn list(&self, agent_id: &AgentId) -> SovereignResult<Vec<Checkpoint>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, agent_id, session_id, agent_config, tool_state, status, created_at
             FROM checkpoints
             WHERE agent_id = ?
             ORDER BY created_at DESC",
            )
            .map_err(|e| SovereignError::Memory(format!("Failed to prepare query: {e}")))?;

        let rows = stmt
            .query_map(params![agent_id.to_string()], |row| Ok(Self::map_row(row)))
            .map_err(|e| SovereignError::Memory(format!("Query failed: {e}")))?;

        rows.collect::<Result<Result<Vec<_>, _>, _>>()
            .map_err(|e| SovereignError::Memory(format!("Iterator error: {e}")))?
    }

    /// List all agents that have an active checkpoint (e.g. they crashed while running).
    pub fn list_active_agents(&self) -> SovereignResult<Vec<AgentId>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT DISTINCT agent_id FROM checkpoints WHERE status = 'active'")
            .map_err(|e| SovereignError::Memory(format!("Failed to prepare query: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                let agent_id_str: String = row.get(0)?;
                Ok(agent_id_str)
            })
            .map_err(|e| SovereignError::Memory(format!("Query failed: {e}")))?;

        let mut agents = Vec::new();
        for id_str in rows.flatten() {
            if let Ok(id) = id_str.parse() {
                agents.push(id);
            }
        }
        Ok(agents)
    }

    /// Delete all but the N most recent checkpoints for an agent.
    pub fn prune(&self, agent_id: &AgentId, keep_last: usize) -> SovereignResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        conn.execute(
            "DELETE FROM checkpoints
             WHERE agent_id = ? AND id NOT IN (
                 SELECT id FROM checkpoints
                 WHERE agent_id = ?
                 ORDER BY created_at DESC
                 LIMIT ?
             )",
            params![agent_id.to_string(), agent_id.to_string(), keep_last],
        )
        .map_err(|e| SovereignError::Memory(format!("Prune failed: {e}")))?;

        Ok(())
    }

    fn map_row(row: &Row) -> SovereignResult<Checkpoint> {
        let agent_id_str: String = row
            .get(1)
            .map_err(|e| SovereignError::Memory(e.to_string()))?;
        let session_id_str: String = row
            .get(2)
            .map_err(|e| SovereignError::Memory(e.to_string()))?;
        let config_json: String = row
            .get(3)
            .map_err(|e| SovereignError::Memory(e.to_string()))?;
        let tool_state_json: String = row
            .get(4)
            .map_err(|e| SovereignError::Memory(e.to_string()))?;

        Ok(Checkpoint {
            id: row
                .get(0)
                .map_err(|e| SovereignError::Memory(e.to_string()))?,
            agent_id: agent_id_str
                .parse()
                .map_err(|_| SovereignError::Internal("Invalid AgentID in DB".into()))?,
            session_id: Uuid::parse_str(&session_id_str)
                .map_err(|_| SovereignError::Internal("Invalid SessionID in DB".into()))?,
            agent_config: serde_json::from_str(&config_json)
                .map_err(|e| SovereignError::Internal(e.to_string()))?,
            tool_state: serde_json::from_str(&tool_state_json)
                .map_err(|e| SovereignError::Internal(e.to_string()))?,
            status: row
                .get(5)
                .map_err(|e| SovereignError::Memory(e.to_string()))?,
            created_at: row
                .get(6)
                .map_err(|e| SovereignError::Memory(e.to_string()))?,
        })
    }
}
