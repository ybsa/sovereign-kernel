//! Cryptographic audit trail.
//!
//! Provides a tamper-evident Merkle hash chain of all agent actions
//! (tool calls, approvals, rejections) stored safely in SQLite.

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use sk_types::{AgentId, SovereignError, SovereignResult};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tracing::info;

/// A single entry in the cryptographic audit log.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditLogEntry {
    /// The unique sequence number (1-indexed).
    pub id: i64,
    /// The agent that performed the action.
    pub agent_id: AgentId,
    /// The active execution mode (e.g., "Sandbox", "Unrestricted").
    pub execution_mode: String,
    /// The type of action (e.g., "tool_call", "approval", "rejection").
    pub action_type: String,
    /// The actual details of the action (JSON serialized).
    pub action_data: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// The SHA-256 hash of this entry.
    pub hash: String,
    /// The SHA-256 hash of the *previous* entry, creating the chain.
    pub previous_hash: String,
}

/// Store for writing and verifying Merkle-chained audit logs.
pub struct AuditStore {
    conn: Arc<Mutex<Connection>>,
}

impl AuditStore {
    /// Create a new audit store using the shared database connection.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Append a new action to the audit log, automatically calculating its hash
    /// based on the previous chain state.
    pub fn append_log(
        &self,
        agent_id: &AgentId,
        execution_mode: &str,
        action_type: &str,
        action_data: &serde_json::Value,
    ) -> SovereignResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        // 1. Get the last entry's hash to link the chain
        let prev_hash: String = conn
            .query_row(
                "SELECT hash FROM audit_logs ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| SovereignError::Memory(e.to_string()))?
            .unwrap_or_else(|| {
                "0000000000000000000000000000000000000000000000000000000000000000".to_string()
            }); // Genesis hash

        // 2. Prepare the new data
        let timestamp = chrono::Utc::now().to_rfc3339();
        let action_data_str = action_data.to_string();

        // 3. Calculate the Merkle hash for this new entry
        // hash = SHA256(prev_hash + agent_id + mode + type + data + timestamp)
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(agent_id.to_string().as_bytes());
        hasher.update(execution_mode.as_bytes());
        hasher.update(action_type.as_bytes());
        hasher.update(action_data_str.as_bytes());
        hasher.update(timestamp.as_bytes());
        let current_hash = format!("{:x}", hasher.finalize());

        // 4. Insert into the database
        conn.execute(
            "INSERT INTO audit_logs (agent_id, execution_mode, action_type, action_data, timestamp, hash, previous_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                agent_id.to_string(),
                execution_mode,
                action_type,
                action_data_str,
                timestamp,
                current_hash,
                prev_hash
            ],
        )
        .map_err(|e| SovereignError::Memory(format!("Failed to insert audit log: {e}")))?;

        info!(
            agent = %agent_id,
            action = action_type,
            "Appended cryptographic audit log"
        );

        Ok(())
    }

    /// Retrieve the most recent N audit logs.
    pub fn get_recent_logs(&self, limit: i64) -> SovereignResult<Vec<AuditLogEntry>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT id, agent_id, execution_mode, action_type, action_data, timestamp, hash, previous_hash FROM audit_logs ORDER BY id DESC LIMIT ?1")
            .map_err(|e| SovereignError::Memory(e.to_string()))?;

        let iter = stmt
            .query_map(params![limit], |row| {
                let agent_id_str: String = row.get(1)?;
                Ok(AuditLogEntry {
                    id: row.get(0)?,
                    agent_id: AgentId::from_str(&agent_id_str).unwrap_or_else(|_| AgentId::new()),
                    execution_mode: row.get(2)?,
                    action_type: row.get(3)?,
                    action_data: row.get(4)?,
                    timestamp: row.get(5)?,
                    hash: row.get(6)?,
                    previous_hash: row.get(7)?,
                })
            })
            .map_err(|e| SovereignError::Memory(e.to_string()))?;

        let mut entries = Vec::new();
        for item in iter {
            entries.push(item.map_err(|e| SovereignError::Memory(e.to_string()))?);
        }

        Ok(entries)
    }

    /// Verify the integrity of the entire Merkle chain.
    /// Returns Ok(()) if the chain is unbroken and untouched.
    /// Returns Err if tampered with.
    pub fn verify_chain(&self) -> SovereignResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock poisoned: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT id, agent_id, execution_mode, action_type, action_data, timestamp, hash, previous_hash FROM audit_logs ORDER BY id ASC")
            .map_err(|e| SovereignError::Memory(e.to_string()))?;

        let iter = stmt
            .query_map([], |row| {
                let agent_id_str: String = row.get(1)?;
                Ok(AuditLogEntry {
                    id: row.get(0)?,
                    agent_id: AgentId::from_str(&agent_id_str).unwrap_or_else(|_| AgentId::new()),
                    execution_mode: row.get(2)?,
                    action_type: row.get(3)?,
                    action_data: row.get(4)?,
                    timestamp: row.get(5)?,
                    hash: row.get(6)?,
                    previous_hash: row.get(7)?,
                })
            })
            .map_err(|e| SovereignError::Memory(e.to_string()))?;

        let mut expected_prev_hash =
            "0000000000000000000000000000000000000000000000000000000000000000".to_string();

        for item in iter {
            let entry = item.map_err(|e| SovereignError::Memory(e.to_string()))?;

            // 1. Check strict link
            if entry.previous_hash != expected_prev_hash {
                return Err(SovereignError::Memory(format!(
                    "Audit Chain broken at row #{}! Expected previous hash {} but got {}.",
                    entry.id, expected_prev_hash, entry.previous_hash
                )));
            }

            // 2. Re-calculate hash to ensure data wasn't modified
            let mut hasher = Sha256::new();
            hasher.update(entry.previous_hash.as_bytes());
            hasher.update(entry.agent_id.to_string().as_bytes());
            hasher.update(entry.execution_mode.as_bytes());
            hasher.update(entry.action_type.as_bytes());
            hasher.update(entry.action_data.as_bytes());
            hasher.update(entry.timestamp.as_bytes());
            let recalculated_hash = format!("{:x}", hasher.finalize());

            if entry.hash != recalculated_hash {
                return Err(SovereignError::Memory(format!(
                    "Audit Data tampered at row #{}! Database hash {} does not match recalculated hash {}.",
                    entry.id, entry.hash, recalculated_hash
                )));
            }

            // Advance the chain expectation
            expected_prev_hash = entry.hash;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn setup_store() -> AuditStore {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE audit_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_id TEXT NOT NULL,
                execution_mode TEXT NOT NULL,
                action_type TEXT NOT NULL,
                action_data TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                hash TEXT NOT NULL,
                previous_hash TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        AuditStore::new(Arc::new(Mutex::new(conn)))
    }

    #[test]
    fn test_audit_chain_integrity() {
        let store = setup_store();
        let agent_id = AgentId::new();

        // 1. Append valid log
        store
            .append_log(&agent_id, "Sandbox", "tool_call", &json!({"tool": "ls"}))
            .unwrap();

        // 2. Verify chain
        store.verify_chain().expect("Chain should be valid");
    }

    #[test]
    fn test_audit_genesis() {
        let store = setup_store();
        let agent_id = AgentId::new();

        store
            .append_log(&agent_id, "Sandbox", "init", &json!({}))
            .unwrap();

        let logs = store.get_recent_logs(1).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(
            logs[0].previous_hash,
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
        store.verify_chain().unwrap();
    }
}
