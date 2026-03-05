//! Execution approval manager — gates dangerous operations behind human approval.
//!
//! Ported from Sovereign Kernel's Sovereign Kernel-kernel/src/approval.rs — complete with
//! per-agent pending limits, oneshot channels for blocking resolution,
//! hot-reloadable policy, and risk classification.

use chrono::Utc;
use dashmap::DashMap;
use sk_types::approval::{
    ApprovalDecision, ApprovalPolicy, ApprovalRequest, ApprovalResponse, RiskLevel,
};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Max pending requests per agent.
const MAX_PENDING_PER_AGENT: usize = 5;

/// Manages approval requests with oneshot channels for blocking resolution.
pub struct ApprovalManager {
    pending: DashMap<Uuid, PendingRequest>,
    policy: std::sync::RwLock<ApprovalPolicy>,
}

struct PendingRequest {
    request: ApprovalRequest,
    sender: tokio::sync::oneshot::Sender<ApprovalDecision>,
}

impl ApprovalManager {
    pub fn new(policy: ApprovalPolicy) -> Self {
        Self {
            pending: DashMap::new(),
            policy: std::sync::RwLock::new(policy),
        }
    }

    /// Check if a tool requires approval based on current policy.
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        let policy = self.policy.read().unwrap_or_else(|e| e.into_inner());
        policy.require_approval.iter().any(|t| t == tool_name)
    }

    /// Submit an approval request. Returns a future that resolves when approved/denied/timed out.
    pub async fn request_approval(&self, req: ApprovalRequest) -> ApprovalDecision {
        // Check per-agent pending limit
        let agent_id_str = req.agent_id.clone();
        let agent_pending: usize = self
            .pending
            .iter()
            .filter(|r| r.value().request.agent_id == agent_id_str)
            .count();
        if agent_pending >= MAX_PENDING_PER_AGENT {
            warn!(agent_id = %req.agent_id, "Approval request rejected: too many pending");
            return ApprovalDecision::Denied;
        }

        let timeout = std::time::Duration::from_secs(req.timeout_secs);
        let id = req.id;

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.insert(
            id,
            PendingRequest {
                request: req,
                sender: tx,
            },
        );

        info!(request_id = %id, "Approval request submitted, waiting for resolution");

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(decision)) => {
                debug!(request_id = %id, ?decision, "Approval resolved");
                decision
            }
            _ => {
                self.pending.remove(&id);
                warn!(request_id = %id, "Approval request timed out");
                ApprovalDecision::TimedOut
            }
        }
    }

    /// Resolve a pending request (called by API/UI).
    pub fn resolve(
        &self,
        request_id: Uuid,
        decision: ApprovalDecision,
        decided_by: Option<String>,
    ) -> Result<ApprovalResponse, String> {
        match self.pending.remove(&request_id) {
            Some((_, pending)) => {
                let response = ApprovalResponse {
                    request_id,
                    decision,
                    decided_at: Utc::now(),
                    decided_by,
                };
                let _ = pending.sender.send(decision);
                info!(request_id = %request_id, ?decision, "Approval request resolved");
                Ok(response)
            }
            None => Err(format!("No pending approval request with id {request_id}")),
        }
    }

    /// List all pending requests (for API/dashboard display).
    pub fn list_pending(&self) -> Vec<ApprovalRequest> {
        self.pending
            .iter()
            .map(|r| r.value().request.clone())
            .collect::<Vec<_>>()
    }

    /// Number of pending requests.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Update the approval policy (for hot-reload).
    pub fn update_policy(&self, policy: ApprovalPolicy) {
        *self.policy.write().unwrap_or_else(|e| e.into_inner()) = policy;
    }

    /// Get a copy of the current policy.
    pub fn policy(&self) -> ApprovalPolicy {
        self.policy
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Classify the risk level of a tool invocation.
    pub fn classify_risk(tool_name: &str, args: Option<&serde_json::Value>) -> RiskLevel {
        match tool_name {
            // Read-only / safe
            "web_search" | "web_fetch" | "read_file" | "list_dir" | "recall" | "browser_read_page" => RiskLevel::Low,
            
            // Moderate actions
            "write_file" | "copy_file" | "remember" | "browser_navigate" | "browser_click" | "browser_type" | "browser_screenshot" | "browser_close" => RiskLevel::Medium,
            
            // Dangerous execution
            "shell_exec" => {
                let command = args.and_then(|a| a.get("command")).and_then(|v| v.as_str()).unwrap_or("");
                if Self::is_destructive_command(command) {
                    RiskLevel::Critical
                } else {
                    RiskLevel::High // Even non-destructive shell is High risk
                }
            }
            "code_exec" => RiskLevel::Critical,
            "forget" | "delete_file" | "move_file" => RiskLevel::High,
            
            _ => RiskLevel::High,
        }
    }

    /// Check if a shell command matches known destructive patterns.
    fn is_destructive_command(command: &str) -> bool {
        let cmd_lower = command.to_lowercase();
        let dangerous_patterns = [
            "remove-item", "del ", "rd ", "rmdir", "format ", "clear-recyclebin",
            "stop-process", "kill ", "taskkill", "shutdown", "restart-computer",
            "reg delete", "reg add", "net user", "net stop", "diskpart",
            "cipher /w", "sfc ", "dism ", "bcdedit", "wmic",
            "rm -rf", "rm -r", "mkfs", "dd if=", "chmod 777", "chown",
            "kill -9", "pkill", "reboot", "halt", "poweroff", "> /dev/",
            "curl | sh", "wget | sh",
        ];
        dangerous_patterns.iter().any(|pattern| cmd_lower.contains(pattern))
    }
}

use sk_types::AgentId;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

/// Safety gate that tracks approved operations for conversational approval.
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

        let risk = ApprovalManager::classify_risk(tool_name, Some(args));

        match risk {
            RiskLevel::Low => Ok(()),
            RiskLevel::Medium => Ok(()), // Allow moderate by default
            RiskLevel::High | RiskLevel::Critical => {
                let sig = Self::make_signature(tool_name, args);

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
                    "forget" | "file_delete" => "🛡️ **Safety Block**: The agent wants to delete data.\n\
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
        let sig = Self::make_signature(tool_name, args);
        self.approve_signature(sig);
    }

    /// Approve all operations for the current session (trust mode).
    pub fn approve_all(&self) {
        let mut approved = self.approved.lock().unwrap();
        approved.insert("__TRUST_ALL__".to_string());
    }

    /// Check if trust-all mode is active.
    pub fn is_trust_all(&self) -> bool {
        let approved = self.approved.lock().unwrap();
        approved.contains("__TRUST_ALL__")
    }

    /// Create a unique signature for a tool call.
    pub fn make_signature(tool_name: &str, args: &serde_json::Value) -> String {
        format!("{}:{}", tool_name, args)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn default_manager() -> ApprovalManager {
        ApprovalManager::new(ApprovalPolicy::default())
    }

    fn make_request(agent_id: &str, tool_name: &str, timeout_secs: u64) -> ApprovalRequest {
        ApprovalRequest {
            id: Uuid::new_v4(),
            agent_id: agent_id.to_string(),
            tool_name: tool_name.to_string(),
            description: "test operation".to_string(),
            action_summary: "test action".to_string(),
            risk_level: RiskLevel::High,
            requested_at: Utc::now(),
            timeout_secs,
        }
    }

    #[test]
    fn test_requires_approval_default() {
        let mgr = default_manager();
        assert!(mgr.requires_approval("shell_exec"));
        assert!(!mgr.requires_approval("file_read"));
    }

    #[test]
    fn test_requires_approval_custom_policy() {
        let policy = ApprovalPolicy {
            require_approval: vec!["file_write".to_string(), "file_delete".to_string()],
            timeout_secs: 30,
            auto_approve_autonomous: false,
        };
        let mgr = ApprovalManager::new(policy);
        assert!(mgr.requires_approval("file_write"));
        assert!(mgr.requires_approval("file_delete"));
        assert!(!mgr.requires_approval("shell_exec"));
    }

    #[test]
    fn test_classify_risk() {
        // Critical: destructive shell patterns
        assert_eq!(
            ApprovalManager::classify_risk("shell_exec", Some(&serde_json::json!({"command": "rm -rf /"}))),
            RiskLevel::Critical
        );
        // High: non-destructive shell still High
        assert_eq!(
            ApprovalManager::classify_risk("shell_exec", Some(&serde_json::json!({"command": "ls -la"}))),
            RiskLevel::High
        );
        // Critical: raw code execution allows maximum arbitrary control
        assert_eq!(
            ApprovalManager::classify_risk("code_exec", None),
            RiskLevel::Critical
        );
        // Medium: moderate write actions (implementation uses "write_file")
        assert_eq!(
            ApprovalManager::classify_risk("write_file", None),
            RiskLevel::Medium
        );
        // High: forget/delete (implementation uses "forget" | "delete_file" | "move_file")
        assert_eq!(
            ApprovalManager::classify_risk("delete_file", None),
            RiskLevel::High
        );
        // Low: read-only actions (implementation uses "web_fetch")
        assert_eq!(
            ApprovalManager::classify_risk("web_fetch", None),
            RiskLevel::Low
        );
        // Medium: browser actions
        assert_eq!(
            ApprovalManager::classify_risk("browser_navigate", None),
            RiskLevel::Medium
        );
        // Low: read_file (implementation uses "read_file")
        assert_eq!(ApprovalManager::classify_risk("read_file", None), RiskLevel::Low);
    }

    #[test]
    fn test_resolve_nonexistent() {
        let mgr = default_manager();
        let result = mgr.resolve(Uuid::new_v4(), ApprovalDecision::Approved, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No pending approval request"));
    }

    #[test]
    fn test_list_pending_empty() {
        let mgr = default_manager();
        assert!(mgr.list_pending().is_empty());
    }

    #[test]
    fn test_update_policy() {
        let mgr = default_manager();
        assert!(mgr.requires_approval("shell_exec"));
        assert!(!mgr.requires_approval("file_write"));

        let new_policy = ApprovalPolicy {
            require_approval: vec!["file_write".to_string()],
            timeout_secs: 120,
            auto_approve_autonomous: true,
        };
        mgr.update_policy(new_policy);

        assert!(!mgr.requires_approval("shell_exec"));
        assert!(mgr.requires_approval("file_write"));

        let policy = mgr.policy();
        assert_eq!(policy.timeout_secs, 120);
        assert!(policy.auto_approve_autonomous);
    }

    #[test]
    fn test_pending_count() {
        let mgr = default_manager();
        assert_eq!(mgr.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_request_approval_timeout() {
        let mgr = Arc::new(default_manager());
        let req = make_request("agent-1", "shell_exec", 10);
        let decision = mgr.request_approval(req).await;
        assert_eq!(decision, ApprovalDecision::TimedOut);
        assert_eq!(mgr.pending_count(), 0);
    }

    #[tokio::test]
    async fn test_request_approval_approve() {
        let mgr = Arc::new(default_manager());
        let req = make_request("agent-1", "shell_exec", 60);
        let request_id = req.id;

        let mgr2 = Arc::clone(&mgr);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            let result = mgr2.resolve(
                request_id,
                ApprovalDecision::Approved,
                Some("admin".to_string()),
            );
            assert!(result.is_ok());
        });

        let decision = mgr.request_approval(req).await;
        assert_eq!(decision, ApprovalDecision::Approved);
    }

    #[tokio::test]
    async fn test_request_approval_deny() {
        let mgr = Arc::new(default_manager());
        let req = make_request("agent-1", "shell_exec", 60);
        let request_id = req.id;

        let mgr2 = Arc::clone(&mgr);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            let _ = mgr2.resolve(request_id, ApprovalDecision::Denied, None);
        });

        let decision = mgr.request_approval(req).await;
        assert_eq!(decision, ApprovalDecision::Denied);
    }

    #[tokio::test]
    async fn test_max_pending_per_agent() {
        let mgr = Arc::new(default_manager());

        let mut ids = Vec::new();
        for _ in 0..MAX_PENDING_PER_AGENT {
            let req = make_request("agent-1", "shell_exec", 300);
            ids.push(req.id);
            let mgr_clone = Arc::clone(&mgr);
            tokio::spawn(async move {
                mgr_clone.request_approval(req).await;
            });
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert_eq!(mgr.pending_count(), MAX_PENDING_PER_AGENT);

        // 6th request for the same agent should be immediately denied
        let req6 = make_request("agent-1", "shell_exec", 300);
        let decision = mgr.request_approval(req6).await;
        assert_eq!(decision, ApprovalDecision::Denied);

        // Cleanup
        for id in &ids {
            let _ = mgr.resolve(*id, ApprovalDecision::Denied, None);
        }
    }

    #[test]
    fn test_policy_defaults() {
        let mgr = default_manager();
        let policy = mgr.policy();
        assert_eq!(policy.require_approval, vec!["shell_exec".to_string()]);
        assert_eq!(policy.timeout_secs, 60);
        assert!(!policy.auto_approve_autonomous);
    }
}
