//! Session persistence — save/load conversation sessions in SQLite.

use rusqlite::Connection;
use sk_types::{AgentId, Message, Session, SessionId, SovereignError, SovereignResult};
use std::sync::{Arc, Mutex};

/// Session persistence store.
pub struct SessionStore {
    conn: Arc<Mutex<Connection>>,
}

impl SessionStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Save a session (insert or update).
    pub fn save(&self, session: &Session) -> SovereignResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock: {e}")))?;
        let messages_json = serde_json::to_string(&session.messages)?;
        conn.execute(
            "INSERT OR REPLACE INTO sessions (id, agent_id, messages, label, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                session.id.to_string(),
                session.agent_id.to_string(),
                messages_json,
                session.label,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
            ],
        ).map_err(|e| SovereignError::Memory(e.to_string()))?;
        Ok(())
    }

    /// Load a session by ID.
    pub fn load(&self, session_id: SessionId) -> SovereignResult<Option<Session>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock: {e}")))?;
        let result = conn.query_row(
            "SELECT id, agent_id, messages, label, created_at, updated_at FROM sessions WHERE id = ?1",
            rusqlite::params![session_id.to_string()],
            |row| {
                let id: String = row.get(0)?;
                let agent_id: String = row.get(1)?;
                let messages_json: String = row.get(2)?;
                let label: Option<String> = row.get(3)?;
                let created_at: String = row.get(4)?;
                let updated_at: String = row.get(5)?;
                Ok((id, agent_id, messages_json, label, created_at, updated_at))
            },
        );

        match result {
            Ok((id, agent_id, messages_json, label, created_at, updated_at)) => {
                let messages: Vec<Message> = serde_json::from_str(&messages_json)?;
                Ok(Some(Session {
                    id: SessionId::parse(&id).map_err(|e| SovereignError::Memory(e.to_string()))?,
                    agent_id: AgentId::parse(&agent_id)
                        .map_err(|e| SovereignError::Memory(e.to_string()))?,
                    messages,
                    label,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(SovereignError::Memory(e.to_string())),
        }
    }

    /// Create a new empty session for an agent.
    pub fn create(&self, agent_id: AgentId) -> SovereignResult<Session> {
        let session = Session::new(agent_id);
        self.save(&session)?;
        Ok(session)
    }

    /// List all sessions for an agent.
    pub fn list_for_agent(
        &self,
        agent_id: AgentId,
    ) -> SovereignResult<Vec<(SessionId, Option<String>, String)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock: {e}")))?;
        let mut stmt = conn.prepare(
            "SELECT id, label, updated_at FROM sessions WHERE agent_id = ?1 ORDER BY updated_at DESC"
        ).map_err(|e| SovereignError::Memory(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params![agent_id.to_string()], |row| {
                let id: String = row.get(0)?;
                let label: Option<String> = row.get(1)?;
                let updated: String = row.get(2)?;
                Ok((id, label, updated))
            })
            .map_err(|e| SovereignError::Memory(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            let (id, label, updated) = row.map_err(|e| SovereignError::Memory(e.to_string()))?;
            let session_id =
                SessionId::parse(&id).map_err(|e| SovereignError::Memory(e.to_string()))?;
            results.push((session_id, label, updated));
        }
        Ok(results)
    }

    /// Delete a session by ID.
    pub fn delete(&self, session_id: SessionId) -> SovereignResult<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SovereignError::Memory(format!("Lock: {e}")))?;
        conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            rusqlite::params![session_id.to_string()],
        )
        .map_err(|e| SovereignError::Memory(e.to_string()))?;
        Ok(())
    }
}
