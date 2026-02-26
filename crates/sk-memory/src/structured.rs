//! Structured key-value store backed by SQLite.

use rusqlite::Connection;
use sk_types::{AgentId, SovereignError, SovereignResult};
use std::sync::{Arc, Mutex};

/// Structured KV store — persistent key-value pairs per agent.
pub struct StructuredStore {
    conn: Arc<Mutex<Connection>>,
}

impl StructuredStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Get a value by key for an agent.
    pub fn get(&self, agent_id: AgentId, key: &str) -> SovereignResult<Option<serde_json::Value>> {
        let conn = self.conn.lock().map_err(|e| {
            SovereignError::MemoryError(format!("Lock poisoned: {e}"))
        })?;
        let mut stmt = conn
            .prepare("SELECT value FROM kv_store WHERE agent_id = ?1 AND key = ?2")
            .map_err(|e| SovereignError::MemoryError(e.to_string()))?;
        let result = stmt
            .query_row(rusqlite::params![agent_id.to_string(), key], |row| {
                let val: String = row.get(0)?;
                Ok(val)
            })
            .optional()
            .map_err(|e| SovereignError::MemoryError(e.to_string()))?;

        match result {
            Some(val) => Ok(Some(serde_json::from_str(&val)?)),
            None => Ok(None),
        }
    }

    /// Set a key-value pair for an agent.
    pub fn set(
        &self,
        agent_id: AgentId,
        key: &str,
        value: serde_json::Value,
    ) -> SovereignResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            SovereignError::MemoryError(format!("Lock poisoned: {e}"))
        })?;
        let val_str = serde_json::to_string(&value)?;
        conn.execute(
            "INSERT OR REPLACE INTO kv_store (agent_id, key, value, updated_at) VALUES (?1, ?2, ?3, datetime('now'))",
            rusqlite::params![agent_id.to_string(), key, val_str],
        )
        .map_err(|e| SovereignError::MemoryError(e.to_string()))?;
        Ok(())
    }

    /// Delete a key for an agent.
    pub fn delete(&self, agent_id: AgentId, key: &str) -> SovereignResult<()> {
        let conn = self.conn.lock().map_err(|e| {
            SovereignError::MemoryError(format!("Lock poisoned: {e}"))
        })?;
        conn.execute(
            "DELETE FROM kv_store WHERE agent_id = ?1 AND key = ?2",
            rusqlite::params![agent_id.to_string(), key],
        )
        .map_err(|e| SovereignError::MemoryError(e.to_string()))?;
        Ok(())
    }

    /// List all KV pairs for an agent.
    pub fn list(&self, agent_id: AgentId) -> SovereignResult<Vec<(String, serde_json::Value)>> {
        let conn = self.conn.lock().map_err(|e| {
            SovereignError::MemoryError(format!("Lock poisoned: {e}"))
        })?;
        let mut stmt = conn
            .prepare("SELECT key, value FROM kv_store WHERE agent_id = ?1 ORDER BY key")
            .map_err(|e| SovereignError::MemoryError(e.to_string()))?;
        let rows = stmt
            .query_map(rusqlite::params![agent_id.to_string()], |row| {
                let key: String = row.get(0)?;
                let val: String = row.get(1)?;
                Ok((key, val))
            })
            .map_err(|e| SovereignError::MemoryError(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            let (key, val) = row.map_err(|e| SovereignError::MemoryError(e.to_string()))?;
            let parsed: serde_json::Value = serde_json::from_str(&val)?;
            result.push((key, parsed));
        }
        Ok(result)
    }
}

/// Extension trait for optional query results.
trait OptionalExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
