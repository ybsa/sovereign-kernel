//! Config hot-reload — diffs two `KernelConfig` instances and produces a `ReloadPlan`.
//!
//! **Hot-reload safe**: channels, skills, usage footer, web config, browser,
//! approval policy, cron settings, webhook triggers, extensions.
//!
//! **No-op** (informational only): log_level, language, mode.
//!
//! **Restart required**: api_listen, api_key, network, memory, default_model.

use sk_types::config::{KernelConfig, ReloadMode};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// HotAction — what can be changed at runtime without restart
// ---------------------------------------------------------------------------

/// An individual action that can be applied at runtime (hot-reload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotAction {
    /// Channel configuration changed — reload channel bridges.
    ReloadChannels,
    /// Skill configuration changed — reload skill registry.
    ReloadSkills,
    /// Usage footer mode changed.
    UpdateUsageFooter,
    /// Web config changed — rebuild web tools context.
    ReloadWebConfig,
    /// Browser config changed.
    ReloadBrowserConfig,
    /// Approval policy changed.
    UpdateApprovalPolicy,
    /// Cron max jobs changed.
    UpdateCronConfig,
    /// Webhook trigger config changed.
    UpdateWebhookConfig,
    /// Extension config changed.
    ReloadExtensions,
    /// MCP server list changed — reconnect MCP clients.
    ReloadMcpServers,
    /// A2A config changed.
    ReloadA2aConfig,
    /// Fallback provider chain changed.
    ReloadFallbackProviders,
}

// ---------------------------------------------------------------------------
// ReloadPlan — the output of diffing two configs
// ---------------------------------------------------------------------------

/// A categorized plan for applying config changes.
///
/// After building a plan via [`build_reload_plan`], callers inspect
/// `restart_required` to decide whether a full restart is needed or
/// the `hot_actions` can be applied in-place.
#[derive(Debug, Clone)]
pub struct ReloadPlan {
    /// Whether a full restart is needed.
    pub restart_required: bool,
    /// Human-readable reasons why restart is required.
    pub restart_reasons: Vec<String>,
    /// Actions that can be hot-reloaded without restart.
    pub hot_actions: Vec<HotAction>,
    /// Fields that changed but are no-ops (informational only).
    pub noop_changes: Vec<String>,
}

impl ReloadPlan {
    /// Whether any changes were detected at all.
    pub fn has_changes(&self) -> bool {
        self.restart_required || !self.hot_actions.is_empty() || !self.noop_changes.is_empty()
    }

    /// Whether the plan can be applied without restart.
    pub fn is_hot_reloadable(&self) -> bool {
        !self.restart_required
    }

    /// Log a human-readable summary of the plan.
    pub fn log_summary(&self) {
        if !self.has_changes() {
            info!("config reload: no changes detected");
            return;
        }
        if self.restart_required {
            warn!(
                "config reload: restart required — {}",
                self.restart_reasons.join("; ")
            );
        }
        for action in &self.hot_actions {
            info!("config reload: hot-reload action queued — {action:?}");
        }
        for noop in &self.noop_changes {
            info!("config reload: no-op change — {noop}");
        }
    }
}

// ---------------------------------------------------------------------------
// build_reload_plan
// ---------------------------------------------------------------------------

/// Compare JSON-serialized forms of a field. Returns `true` when the
/// serialized representations differ (or if one side fails to serialize).
fn field_changed<T: serde::Serialize>(old: &T, new: &T) -> bool {
    let old_json = serde_json::to_string(old).ok();
    let new_json = serde_json::to_string(new).ok();
    old_json != new_json
}

/// Diff two configurations and produce a reload plan.
pub fn build_reload_plan(old: &KernelConfig, new: &KernelConfig) -> ReloadPlan {
    let mut plan = ReloadPlan {
        restart_required: false,
        restart_reasons: Vec::new(),
        hot_actions: Vec::new(),
        noop_changes: Vec::new(),
    };

    // ----- Restart-required fields -----

    if old.api_listen != new.api_listen {
        plan.restart_required = true;
        plan.restart_reasons.push(format!(
            "api_listen changed: {} -> {}",
            old.api_listen, new.api_listen
        ));
    }

    if old.api_key != new.api_key {
        plan.restart_required = true;
        plan.restart_reasons.push("api_key changed".to_string());
    }

    if old.network_enabled != new.network_enabled {
        plan.restart_required = true;
        plan.restart_reasons
            .push("network_enabled changed".to_string());
    }

    // Network config
    if field_changed(&old.network, &new.network) {
        plan.restart_required = true;
        plan.restart_reasons
            .push("network config changed".to_string());
    }

    // Memory config (requires restarting SQLite connections)
    if field_changed(&old.memory, &new.memory) {
        plan.restart_required = true;
        plan.restart_reasons
            .push("memory config changed".to_string());
    }

    // Default model (driver needs recreation)
    if field_changed(&old.default_model, &new.default_model) {
        plan.restart_required = true;
        plan.restart_reasons
            .push("default_model changed".to_string());
    }

    // Home/data directory changes
    if old.home_dir != new.home_dir {
        plan.restart_required = true;
        plan.restart_reasons.push(format!(
            "home_dir changed: {:?} -> {:?}",
            old.home_dir, new.home_dir
        ));
    }
    if old.data_dir != new.data_dir {
        plan.restart_required = true;
        plan.restart_reasons.push(format!(
            "data_dir changed: {:?} -> {:?}",
            old.data_dir, new.data_dir
        ));
    }

    // Vault config
    if field_changed(&old.vault, &new.vault) {
        plan.restart_required = true;
        plan.restart_reasons
            .push("vault config changed".to_string());
    }

    // ----- Hot-reloadable fields -----

    if field_changed(&old.channels, &new.channels) {
        plan.hot_actions.push(HotAction::ReloadChannels);
    }

    if old.usage_footer != new.usage_footer {
        plan.hot_actions.push(HotAction::UpdateUsageFooter);
    }

    if field_changed(&old.web, &new.web) {
        plan.hot_actions.push(HotAction::ReloadWebConfig);
    }

    if field_changed(&old.browser, &new.browser) {
        plan.hot_actions.push(HotAction::ReloadBrowserConfig);
    }

    if field_changed(&old.approval, &new.approval) {
        plan.hot_actions.push(HotAction::UpdateApprovalPolicy);
    }

    if old.max_cron_jobs != new.max_cron_jobs {
        plan.hot_actions.push(HotAction::UpdateCronConfig);
    }

    if field_changed(&old.webhook_triggers, &new.webhook_triggers) {
        plan.hot_actions.push(HotAction::UpdateWebhookConfig);
    }

    if field_changed(&old.extensions, &new.extensions) {
        plan.hot_actions.push(HotAction::ReloadExtensions);
    }

    if field_changed(&old.mcp_servers, &new.mcp_servers) {
        plan.hot_actions.push(HotAction::ReloadMcpServers);
    }

    if field_changed(&old.a2a, &new.a2a) {
        plan.hot_actions.push(HotAction::ReloadA2aConfig);
    }

    if field_changed(&old.fallback_providers, &new.fallback_providers) {
        plan.hot_actions.push(HotAction::ReloadFallbackProviders);
    }

    // ----- No-op fields -----

    if old.log_level != new.log_level {
        plan.noop_changes
            .push(format!("log_level: {} -> {}", old.log_level, new.log_level));
    }

    if old.language != new.language {
        plan.noop_changes
            .push(format!("language: {} -> {}", old.language, new.language));
    }

    if old.mode != new.mode {
        plan.noop_changes
            .push(format!("mode: {:?} -> {:?}", old.mode, new.mode));
    }

    plan
}

// ---------------------------------------------------------------------------
// validate_config_for_reload
// ---------------------------------------------------------------------------

/// Validate a new config before applying it.
pub fn validate_config_for_reload(config: &KernelConfig) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if config.api_listen.is_empty() {
        errors.push("api_listen cannot be empty".to_string());
    }

    if config.max_cron_jobs > 10_000 {
        errors.push("max_cron_jobs exceeds reasonable limit (10000)".to_string());
    }

    if let Err(e) = config.approval.validate() {
        errors.push(format!("approval policy: {e}"));
    }

    if config.network_enabled && config.network.shared_secret.is_empty() {
        errors.push("network_enabled is true but network.shared_secret is empty".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn should_apply_hot(mode: ReloadMode, plan: &ReloadPlan) -> bool {
    match mode {
        ReloadMode::Off => false,
        ReloadMode::Restart => false,
        ReloadMode::Hot => !plan.hot_actions.is_empty(),
        ReloadMode::Hybrid => !plan.hot_actions.is_empty(),
    }
}
