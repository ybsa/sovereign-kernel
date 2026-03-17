use chrono::Utc;
use regex_lite::Regex;
use serde::{Deserialize, Serialize};
use sk_types::{Message, SovereignResult};
use std::fs;
use std::path::{Path, PathBuf};

/// Handles forensic step dumping for an agent session.
#[derive(Debug)]
pub struct StepForensics {
    root_dir: PathBuf,
    session_id: String,
    redactor: Regex,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StepRecord {
    pub timestamp: String,
    pub iteration: u32,
    pub prompt: Vec<Message>,
    pub response: String,
    pub tool_calls: Vec<sk_types::ToolCall>,
    pub usage: crate::llm_driver::TokenUsage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub total_tokens: u32,
    pub iterations: u32,
    pub final_status: String,
    pub completed_at: String,
}

impl StepForensics {
    pub fn new(root: &Path, session_id: &str) -> Self {
        let root_dir = root.join(".steps").join(session_id);

        // Match common API keys, bearer tokens, and secrets
        let redactor = Regex::new(
            r"(?i)(sk-[a-zA-Z0-9]{20,}|bearer\s+[a-zA-Z0-9\-._]{20,}|AIza[a-zA-Z0-9\-_]{35,}|[a-f0-9]{32,}|AWS_ACCESS_KEY_ID=[^\s]+|OPENAI_API_KEY=[^\s]+|ANTHROPIC_API_KEY=[^\s]+|GOOG_API_KEY=[^\s]+)"
        ).unwrap();

        Self {
            root_dir,
            session_id: session_id.to_string(),
            redactor,
        }
    }

    pub fn dump_step(
        &self,
        iteration: u32,
        prompt: &[Message],
        response: &str,
        tool_calls: &[sk_types::ToolCall],
        usage: crate::llm_driver::TokenUsage,
    ) -> SovereignResult<()> {
        if let Err(e) = fs::create_dir_all(&self.root_dir) {
            tracing::warn!(
                "Failed to create forensics directory {}: {}",
                self.root_dir.display(),
                e
            );
            return Ok(()); // Don't crash the loop if disk is full/locked
        }

        let record = StepRecord {
            timestamp: Utc::now().to_rfc3339(),
            iteration,
            prompt: prompt.to_vec(),
            response: response.to_string(),
            tool_calls: tool_calls.to_vec(),
            usage,
        };

        let mut json = serde_json::to_string(&record)?;

        // Redaction
        json = self.redactor.replace_all(&json, "[REDACTED]").to_string();

        let filename = format!("step_{:03}.jsonl", iteration);
        let path = self.root_dir.join(filename);

        fs::write(&path, json)?;
        Ok(())
    }

    pub fn dump_summary(
        &self,
        total_tokens: u32,
        iterations: u32,
        status: &str,
    ) -> SovereignResult<()> {
        if !self.root_dir.exists() {
            return Ok(());
        }

        let summary = SessionSummary {
            session_id: self.session_id.clone(),
            total_tokens,
            iterations,
            final_status: status.to_string(),
            completed_at: Utc::now().to_rfc3339(),
        };

        let path = self.root_dir.join("summary.json");
        let json = serde_json::to_string_pretty(&summary)?;
        fs::write(&path, json)?;
        Ok(())
    }
}
