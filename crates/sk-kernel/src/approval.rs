//! Execution approval manager — gates dangerous operations behind human approval.
use chrono::Utc;
use dashmap::DashMap;
use sk_types::approval::{
    ApprovalDecision, ApprovalPolicy, ApprovalRequest, ApprovalResponse, RiskLevel,
};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Max pending requests per agent.
const MAX_PENDING_PER_AGENT: usize = 5;

/// Manages approval requests with oneshot channels for blocking resolution.
pub struct ApprovalManager {
    pending: DashMap<Uuid, PendingRequest>,
    /// Track number of pending requests per agent to prevent exhaustion. (Fixes TOCTOU race)
    pending_counts: DashMap<String, usize>,
    policy: std::sync::RwLock<ApprovalPolicy>,
    /// Broadcast channel to notify listeners of new requests.
    notifier: tokio::sync::broadcast::Sender<Uuid>,

    // --- SafetyGate Integration ---
    /// Approved signatures (tool:args) for the current session.
    approved_signatures: std::sync::RwLock<HashSet<String>>,
    /// The last signature that was blocked by the safety gate, per agent.
    last_blocked_signatures: std::sync::RwLock<HashMap<String, String>>,
}

struct PendingRequest {
    request: ApprovalRequest,
    sender: tokio::sync::oneshot::Sender<ApprovalDecision>,
}

impl ApprovalManager {
    pub fn new(policy: ApprovalPolicy) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(32);
        Self {
            pending: DashMap::new(),
            pending_counts: DashMap::new(),
            policy: std::sync::RwLock::new(policy),
            notifier: tx,
            approved_signatures: std::sync::RwLock::new(HashSet::new()),
            last_blocked_signatures: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Check if a tool requires approval based on current policy.
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        let policy = self.policy.read().unwrap_or_else(|e| e.into_inner());
        policy.require_approval.iter().any(|t| t == tool_name)
    }

    /// Submit an approval request. Returns a future that resolves when approved/denied/timed out.
    pub async fn request_approval(&self, req: ApprovalRequest) -> ApprovalDecision {
        let agent_id_str = req.agent_id.clone();
        let request_id = req.id;

        // 1. Atomic check and increment of pending count (Fixed TOCTOU)
        {
            let mut count = self.pending_counts.entry(agent_id_str.clone()).or_insert(0);
            if *count >= MAX_PENDING_PER_AGENT {
                warn!(agent_id = %agent_id_str, "Agent exceeded max pending requests limit.");
                return ApprovalDecision::Denied;
            }
            *count += 1;
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.insert(
            request_id,
            PendingRequest {
                request: req.clone(),
                sender: tx,
            },
        );

        // Notify subscribers
        let _ = self.notifier.send(request_id);

        let timeout_secs = req.timeout_secs;
        debug!(id = %request_id, tool = %req.tool_name, "Approval request submitted");

        let decision = tokio::select! {
            res = rx => res.unwrap_or(ApprovalDecision::TimedOut),
            _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)) => {
                ApprovalDecision::TimedOut
            }
        };

        // 2. Decrement count and clean up
        self.pending.remove(&request_id);
        if let Some(mut count) = self.pending_counts.get_mut(&agent_id_str) {
            if *count > 0 {
                *count -= 1;
            }
        }

        decision
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
                let agent_id = pending.request.agent_id.clone();
                // Decrement counter
                if let Some(mut count) = self.pending_counts.get_mut(&agent_id) {
                    if *count > 0 {
                        *count -= 1;
                    }
                }

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

    // --- SafetyGate Unified API ---

    /// Check if a tool call is safe or already approved.
    pub fn check_safety(
        &self,
        agent_id: &sk_types::AgentId,
        tool_name: &str,
        args: &serde_json::Value,
    ) -> Result<(), String> {
        let sig = format!("{}:{}", tool_name, args);

        // 1. Check if signature exists in approved cache
        {
            let approved = self.approved_signatures.read().unwrap();
            if approved.contains(&sig) || approved.contains("__TRUST_ALL__") {
                return Ok(());
            }
        }

        // 2. Check risk
        let risk = Self::classify_risk(tool_name, Some(args));

        if risk == RiskLevel::Low {
            return Ok(());
        }

        // 3. Block and record
        {
            let mut last_blocked = self.last_blocked_signatures.write().unwrap();
            last_blocked.insert(agent_id.to_string(), sig.clone());
        }

        let detail = match tool_name {
            "shell_exec" => {
                let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("?");
                format!("🛡️ **Safety Block**: The agent wants to execute a command: `{}`.\nReply with **'approve'** to allow, or **'deny'** to block.", cmd)
            }
            "code_exec" => {
                "🛡️ **Safety Block**: The agent wants to execute raw code.\nReply with **'approve'** to allow, or **'deny'** to block.".to_string()
            }
            _ => {
                format!("🛡️ **Safety Block**: The agent wants to use tool '{}' which is risky.\nReply with **'approve'** to allow, or **'deny'** to block.", tool_name)
            }
        };

        Err(detail)
    }

    pub fn approve_signature(&self, sig: String) {
        let mut approved = self.approved_signatures.write().unwrap();
        approved.insert(sig);
    }

    pub fn approve_last_for_agent(&self, agent_id: &sk_types::AgentId) -> bool {
        let mut last_blocked = self.last_blocked_signatures.write().unwrap();
        if let Some(sig) = last_blocked.remove(&agent_id.to_string()) {
            self.approve_signature(sig);
            true
        } else {
            false
        }
    }

    pub fn deny_last_for_agent(&self, agent_id: &sk_types::AgentId) -> bool {
        let mut last_blocked = self.last_blocked_signatures.write().unwrap();
        last_blocked.remove(&agent_id.to_string()).is_some()
    }

    pub fn has_blocked(&self, agent_id: &sk_types::AgentId) -> bool {
        self.last_blocked_signatures
            .read()
            .unwrap()
            .contains_key(&agent_id.to_string())
    }

    pub fn approve_all(&self) {
        let mut approved = self.approved_signatures.write().unwrap();
        approved.insert("__TRUST_ALL__".to_string());
    }

    pub fn is_trust_all(&self) -> bool {
        self.approved_signatures
            .read()
            .unwrap()
            .contains("__TRUST_ALL__")
    }

    // --- Helpers ---

    pub fn list_pending(&self) -> Vec<ApprovalRequest> {
        self.pending
            .iter()
            .map(|r| r.value().request.clone())
            .collect()
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn update_policy(&self, policy: ApprovalPolicy) {
        *self.policy.write().unwrap() = policy;
    }

    pub fn policy(&self) -> ApprovalPolicy {
        self.policy.read().unwrap().clone()
    }

    pub fn classify_risk(tool_name: &str, args: Option<&serde_json::Value>) -> RiskLevel {
        match tool_name {
            "code_exec"
            | "host_desktop_control"
            | "host_system_config"
            | "host_install_app"
            | "host_uninstall_app" => RiskLevel::Critical,
            "shell_exec" => {
                if let Some(cmd) = args.and_then(|a| a.get("command")).and_then(|v| v.as_str()) {
                    let cmd = cmd.to_lowercase();
                    if cmd.contains("rm ")
                        || cmd.contains("del ")
                        || cmd.contains("format ")
                        || cmd.contains(" > ")
                    {
                        return RiskLevel::Critical;
                    }
                }
                RiskLevel::High
            }
            "delete_file" | "move_file" | "forget" => RiskLevel::High,
            "write_file" | "browser_navigate" | "browser_click" | "browser_type" => {
                RiskLevel::Medium
            }
            _ => RiskLevel::Low,
        }
    }
}
