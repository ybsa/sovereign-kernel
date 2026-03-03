//! Safety & Approval System for Sovereign Kernel.
//!
//! Inspired by OpenClaw's exec approval system. Classifies tool calls into
//! risk levels and blocks dangerous operations unless explicitly approved.
//!
//! Risk levels:
//! - Safe: read_file, list_dir, recall, web_search, web_fetch
//! - Moderate: write_file, remember, forget
//! - Dangerous: shell_exec (especially destructive commands)

#![allow(dead_code)]

use sk_types::AgentId;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// Risk level for a tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    /// No risk — read-only operations (web_search, read_file, list_dir, recall)
    Safe,
    /// Moderate risk — writes data but not destructive (write_file, remember)
    Moderate,
    /// Dangerous — can delete data or execute arbitrary commands (shell_exec, forget)
    Dangerous,
}

/// Classify a tool call by risk level.
pub fn classify_tool(tool_name: &str, args: &serde_json::Value) -> RiskLevel {
    match tool_name {
        // Always safe — read-only
        "web_search" | "web_fetch" | "read_file" | "list_dir" | "recall" => RiskLevel::Safe,

        // Moderate — writes but not destructive
        "remember" => RiskLevel::Moderate,
        "write_file" => RiskLevel::Moderate,

        // Dangerous — arbitrary execution
        "shell_exec" => {
            // Check if the command is a known destructive pattern
            let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            if is_destructive_command(command) {
                RiskLevel::Dangerous
            } else {
                RiskLevel::Moderate
            }
        }

        // Forget deletes memories
        "forget" => RiskLevel::Dangerous,

        // Unknown tools are dangerous by default
        _ => RiskLevel::Dangerous,
    }
}

/// Check if a shell command matches known destructive patterns.
fn is_destructive_command(command: &str) -> bool {
    let cmd_lower = command.to_lowercase();

    // Windows destructive commands
    let dangerous_patterns = [
        "remove-item",
        "del ",
        "rd ",
        "rmdir",
        "format ",
        "clear-recyclebin",
        "stop-process",
        "kill ",
        "taskkill",
        "shutdown",
        "restart-computer",
        "reg delete",
        "reg add",
        "net user",
        "net stop",
        "diskpart",
        "cipher /w",
        "sfc ",
        "dism ",
        "bcdedit",
        "wmic",
        // Unix destructive commands (for cross-platform safety)
        "rm -rf",
        "rm -r",
        "mkfs",
        "dd if=",
        "chmod 777",
        "chown",
        "kill -9",
        "pkill",
        "reboot",
        "halt",
        "poweroff",
        "> /dev/",
        "curl | sh",
        "wget | sh",
    ];

    dangerous_patterns
        .iter()
        .any(|pattern| cmd_lower.contains(pattern))
}

/// Safety gate that tracks approved operations.
pub struct SafetyGate {
    /// Set of approved tool call signatures (tool_name + args hash).
    approved: Mutex<HashSet<String>>,
    /// Tracks the last blocked tool call per agent, so they can say "approve" to allow it.
    last_blocked: Mutex<HashMap<AgentId, String>>,
    /// Whether safety is enabled (can be disabled for trusted environments).
    pub enabled: bool,
}

impl SafetyGate {
    pub fn new(enabled: bool) -> Self {
        Self {
            approved: Mutex::new(HashSet::new()),
            last_blocked: Mutex::new(HashMap::new()),
            enabled,
        }
    }

    /// Check if a tool call should be allowed.
    /// Returns Ok(()) if allowed, Err(message) with a human-readable block reason.
    pub fn check(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        agent_id: Option<&AgentId>,
    ) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        let risk = classify_tool(tool_name, args);

        match risk {
            RiskLevel::Safe => Ok(()),
            RiskLevel::Moderate => Ok(()), // Allow moderate by default
            RiskLevel::Dangerous => {
                let sig = make_signature(tool_name, args);

                // Check if already approved
                let approved = self.approved.lock().unwrap();
                if approved.contains(&sig) {
                    return Ok(());
                }
                drop(approved); // Release lock

                if let Some(agent_id) = agent_id {
                    let mut blocked = self.last_blocked.lock().unwrap();
                    blocked.insert(agent_id.clone(), sig);
                }

                // Block with explanation
                let detail = match tool_name {
                    "shell_exec" => {
                        let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("?");
                        format!(
                            "🛡️ **Safety Block**: The agent wants to run a potentially dangerous command:\n\
                             ```\n{}\n```\n\
                             This command could modify or delete data on your computer.\n\
                             Reply with **'approve'** to allow this specific command, or **'deny'** to block it.",
                            cmd
                        )
                    }
                    "forget" => "🛡️ **Safety Block**: The agent wants to delete a memory entry.\n\
                         Reply with **'approve'** to allow, or **'deny'** to block."
                        .to_string(),
                    _ => {
                        format!(
                            "🛡️ **Safety Block**: The agent wants to use tool '{}' which is classified as dangerous.\n\
                             Reply with **'approve'** to allow, or **'deny'** to block.",
                            tool_name
                        )
                    }
                };

                Err(detail)
            }
        }
    }

    /// Approve a specific tool call signature.
    pub fn approve_signature(&self, sig: String) {
        let mut approved = self.approved.lock().unwrap();
        approved.insert(sig);
    }

    /// Approve the last blocked action for a specific agent.
    pub fn approve_last_for_agent(&self, agent_id: &AgentId) -> bool {
        let mut blocked = self.last_blocked.lock().unwrap();
        if let Some(sig) = blocked.remove(agent_id) {
            self.approve_signature(sig);
            true
        } else {
            false
        }
    }

    /// Deny (clear) the last blocked action for a specific agent.
    pub fn deny_last_for_agent(&self, agent_id: &AgentId) -> bool {
        let mut blocked = self.last_blocked.lock().unwrap();
        blocked.remove(agent_id).is_some()
    }

    /// Check if there is a pending action waiting for human approval.
    pub fn has_pending(&self, agent_id: &AgentId) -> bool {
        let blocked = self.last_blocked.lock().unwrap();
        blocked.contains_key(agent_id)
    }

    /// Approve a specific tool call.
    pub fn approve(&self, tool_name: &str, args: &serde_json::Value) {
        let sig = make_signature(tool_name, args);
        self.approve_signature(sig);
    }

    /// Approve all operations for the current session (trust mode).
    pub fn approve_all(&self) {
        // Disable safety for this session
        // (We can't mutate `enabled` through &self, so we use a special marker)
        let mut approved = self.approved.lock().unwrap();
        approved.insert("__TRUST_ALL__".to_string());
    }

    /// Check if trust-all mode is active.
    pub fn is_trust_all(&self) -> bool {
        let approved = self.approved.lock().unwrap();
        approved.contains("__TRUST_ALL__")
    }
}

/// Create a unique signature for a tool call.
fn make_signature(tool_name: &str, args: &serde_json::Value) -> String {
    format!("{}:{}", tool_name, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_tools() {
        let args = serde_json::json!({});
        assert_eq!(classify_tool("web_search", &args), RiskLevel::Safe);
        assert_eq!(classify_tool("read_file", &args), RiskLevel::Safe);
        assert_eq!(classify_tool("list_dir", &args), RiskLevel::Safe);
        assert_eq!(classify_tool("recall", &args), RiskLevel::Safe);
    }

    #[test]
    fn test_moderate_tools() {
        let args = serde_json::json!({});
        assert_eq!(classify_tool("write_file", &args), RiskLevel::Moderate);
        assert_eq!(classify_tool("remember", &args), RiskLevel::Moderate);
    }

    #[test]
    fn test_dangerous_shell_commands() {
        let args = serde_json::json!({"command": "Remove-Item C:\\Users -Recurse"});
        assert_eq!(classify_tool("shell_exec", &args), RiskLevel::Dangerous);

        let args = serde_json::json!({"command": "Clear-RecycleBin -Force"});
        assert_eq!(classify_tool("shell_exec", &args), RiskLevel::Dangerous);

        let args = serde_json::json!({"command": "Stop-Process -Name chrome"});
        assert_eq!(classify_tool("shell_exec", &args), RiskLevel::Dangerous);

        let args = serde_json::json!({"command": "shutdown /s /t 0"});
        assert_eq!(classify_tool("shell_exec", &args), RiskLevel::Dangerous);
    }

    #[test]
    fn test_safe_shell_commands() {
        let args = serde_json::json!({"command": "Get-ChildItem C:\\Users"});
        assert_eq!(classify_tool("shell_exec", &args), RiskLevel::Moderate);

        let args = serde_json::json!({"command": "echo hello"});
        assert_eq!(classify_tool("shell_exec", &args), RiskLevel::Moderate);

        let args = serde_json::json!({"command": "Get-PSDrive C"});
        assert_eq!(classify_tool("shell_exec", &args), RiskLevel::Moderate);
    }

    #[test]
    fn test_safety_gate_blocks_dangerous() {
        let gate = SafetyGate::new(true);
        let args = serde_json::json!({"command": "Remove-Item C:\\test -Recurse"});
        assert!(gate.check("shell_exec", &args, None).is_err());
    }

    #[test]
    fn test_safety_gate_allows_after_approve() {
        let gate = SafetyGate::new(true);
        let args = serde_json::json!({"command": "Remove-Item C:\\test -Recurse"});
        assert!(gate.check("shell_exec", &args, None).is_err());

        gate.approve("shell_exec", &args);
        assert!(gate.check("shell_exec", &args, None).is_ok());
    }

    #[test]
    fn test_safety_gate_disabled() {
        let gate = SafetyGate::new(false);
        let args = serde_json::json!({"command": "rm -rf /"});
        assert!(gate.check("shell_exec", &args, None).is_ok());
    }
}
