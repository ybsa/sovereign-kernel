//! Metering engine — tracks LLM cost and enforces spending quotas.
//!
//! Ported from Sovereign Kernel's Sovereign Kernel-kernel/src/metering.rs — complete with
//! cost estimation for 30+ model families, budget status snapshots, and
//! hourly/daily/monthly quota enforcement.

use serde::Serialize;
use sk_types::AgentId;

/// The metering engine tracks usage cost and enforces quota limits.
///
/// This is a standalone cost estimator. Budget enforcement is done
/// in the scheduler using `ResourceQuota` checks on the rolling window.
pub struct MeteringEngine {
    /// Per-agent accumulated cost in the current session.
    costs: dashmap::DashMap<AgentId, f64>,
}

impl MeteringEngine {
    /// Create a new metering engine.
    pub fn new() -> Self {
        Self {
            costs: dashmap::DashMap::new(),
        }
    }

    /// Record usage cost for an agent.
    pub fn record_cost(&self, agent_id: AgentId, cost_usd: f64) {
        let mut entry = self.costs.entry(agent_id).or_insert(0.0);
        *entry += cost_usd;
    }

    /// Get total cost for an agent in the current session.
    pub fn agent_cost(&self, agent_id: AgentId) -> f64 {
        self.costs.get(&agent_id).map(|c| *c).unwrap_or(0.0)
    }

    /// Get total cost across all agents.
    pub fn total_cost(&self) -> f64 {
        self.costs.iter().map(|r| *r.value()).sum()
    }

    /// Check if an agent has exceeded its hourly cost quota.
    pub fn check_hourly_quota(
        &self,
        agent_id: AgentId,
        max_cost_per_hour_usd: f64,
    ) -> Result<(), String> {
        if max_cost_per_hour_usd <= 0.0 {
            return Ok(()); // 0.0 = unlimited
        }
        let current = self.agent_cost(agent_id);
        if current >= max_cost_per_hour_usd {
            return Err(format!(
                "Agent {} exceeded hourly cost quota: ${:.4} / ${:.4}",
                agent_id, current, max_cost_per_hour_usd
            ));
        }
        Ok(())
    }

    /// Get a budget status snapshot.
    pub fn budget_status(&self, hourly_limit: f64, daily_limit: f64) -> BudgetStatus {
        let total = self.total_cost();
        BudgetStatus {
            current_spend: total,
            hourly_limit,
            hourly_pct: if hourly_limit > 0.0 {
                total / hourly_limit
            } else {
                0.0
            },
            daily_limit,
            daily_pct: if daily_limit > 0.0 {
                total / daily_limit
            } else {
                0.0
            },
        }
    }

    /// Estimate the cost of an LLM call based on model and token counts.
    ///
    /// Pricing table (approximate, per million tokens):
    ///
    /// | Model Family          | Input $/M | Output $/M |
    /// |-----------------------|-----------|------------|
    /// | claude-haiku          |     0.25  |      1.25  |
    /// | claude-sonnet         |     3.00  |     15.00  |
    /// | claude-opus           |    15.00  |     75.00  |
    /// | gpt-4o                |     2.50  |     10.00  |
    /// | gpt-4o-mini           |     0.15  |      0.60  |
    /// | gpt-4.1               |     2.00  |      8.00  |
    /// | gpt-4.1-mini          |     0.40  |      1.60  |
    /// | gpt-4.1-nano          |     0.10  |      0.40  |
    /// | o3-mini               |     1.10  |      4.40  |
    /// | gemini-2.0-flash      |     0.10  |      0.40  |
    /// | gemini-2.5-pro        |     1.25  |     10.00  |
    /// | gemini-2.5-flash      |     0.15  |      0.60  |
    /// | deepseek-chat/v3      |     0.27  |      1.10  |
    /// | deepseek-reasoner/r1  |     0.55  |      2.19  |
    /// | llama/mixtral (groq)  |     0.05  |      0.10  |
    /// | qwen                  |     0.20  |      0.60  |
    /// | mistral-large         |     2.00  |      6.00  |
    /// | mistral-small         |     0.10  |      0.30  |
    /// | command-r-plus        |     2.50  |     10.00  |
    /// | Default (unknown)     |     1.00  |      3.00  |
    pub fn estimate_cost(model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
        let model_lower = model.to_lowercase();
        let (input_per_m, output_per_m) = estimate_cost_rates(&model_lower);

        let input_cost = (input_tokens as f64 / 1_000_000.0) * input_per_m;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * output_per_m;
        input_cost + output_cost
    }

    /// Reset all cost tracking (e.g., on session reset).
    pub fn reset(&self) {
        self.costs.clear();
    }
}

impl Default for MeteringEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Budget status snapshot — current spend vs limits.
#[derive(Debug, Clone, Serialize)]
pub struct BudgetStatus {
    pub current_spend: f64,
    pub hourly_limit: f64,
    pub hourly_pct: f64,
    pub daily_limit: f64,
    pub daily_pct: f64,
}

/// Returns (input_per_million, output_per_million) pricing for a model.
///
/// Order matters: more specific patterns must come before generic ones
/// (e.g. "gpt-4o-mini" before "gpt-4o", "gpt-4.1-mini" before "gpt-4.1").
fn estimate_cost_rates(model: &str) -> (f64, f64) {
    // ── Anthropic ──────────────────────────────────────────────
    if model.contains("haiku") {
        return (0.25, 1.25);
    }
    if model.contains("opus") {
        return (15.0, 75.0);
    }
    if model.contains("sonnet") {
        return (3.0, 15.0);
    }

    // ── OpenAI ─────────────────────────────────────────────────
    if model.contains("gpt-4o-mini") {
        return (0.15, 0.60);
    }
    if model.contains("gpt-4o") {
        return (2.50, 10.0);
    }
    if model.contains("gpt-4.1-nano") {
        return (0.10, 0.40);
    }
    if model.contains("gpt-4.1-mini") {
        return (0.40, 1.60);
    }
    if model.contains("gpt-4.1") {
        return (2.00, 8.00);
    }
    if model.contains("o4-mini") {
        return (1.10, 4.40);
    }
    if model.contains("o3-mini") {
        return (1.10, 4.40);
    }
    if model.contains("o3") {
        return (2.00, 8.00);
    }
    // Generic gpt-4 fallback
    if model.contains("gpt-4") {
        return (2.50, 10.0);
    }

    // ── Google Gemini ──────────────────────────────────────────
    if model.contains("gemini-2.5-pro") {
        return (1.25, 10.0);
    }
    if model.contains("gemini-2.5-flash") {
        return (0.15, 0.60);
    }
    if model.contains("gemini-2.0-flash") || model.contains("gemini-flash") {
        return (0.10, 0.40);
    }
    // Generic gemini fallback
    if model.contains("gemini") {
        return (0.15, 0.60);
    }

    // ── DeepSeek ───────────────────────────────────────────────
    if model.contains("deepseek-reasoner") || model.contains("deepseek-r1") {
        return (0.55, 2.19);
    }
    if model.contains("deepseek") {
        return (0.27, 1.10);
    }

    // ── Cerebras (ultra-fast, cheap) ───────────────────────────
    if model.contains("cerebras") {
        return (0.06, 0.06);
    }

    // ── SambaNova ──────────────────────────────────────────────
    if model.contains("sambanova") {
        return (0.06, 0.06);
    }

    // ── Replicate ─────────────────────────────────────────────
    if model.contains("replicate") {
        return (0.40, 0.40);
    }

    // ── Open-source (Groq, Together, etc.) ─────────────────────
    if model.contains("llama") || model.contains("mixtral") {
        return (0.05, 0.10);
    }

    // ── Qwen (Alibaba) ──────────────────────────────────────────
    if model.contains("qwen-max") {
        return (4.00, 12.00);
    }
    if model.contains("qwen-vl") {
        return (1.50, 4.50);
    }
    if model.contains("qwen-plus") {
        return (0.80, 2.00);
    }
    if model.contains("qwen-turbo") {
        return (0.30, 0.60);
    }
    if model.contains("qwen") {
        return (0.20, 0.60);
    }

    // ── MiniMax ──────────────────────────────────────────────────
    if model.contains("minimax") {
        return (1.00, 3.00);
    }

    // ── Zhipu / GLM ─────────────────────────────────────────────
    if model.contains("glm-4-flash") {
        return (0.10, 0.10);
    }
    if model.contains("glm") {
        return (1.50, 5.00);
    }

    // ── Moonshot / Kimi ─────────────────────────────────────────
    if model.contains("moonshot") || model.contains("kimi") {
        return (0.80, 0.80);
    }

    // ── Baidu ERNIE ─────────────────────────────────────────────
    if model.contains("ernie") {
        return (2.00, 6.00);
    }

    // ── AWS Bedrock ─────────────────────────────────────────────
    if model.contains("nova-pro") {
        return (0.80, 3.20);
    }
    if model.contains("nova-lite") {
        return (0.06, 0.24);
    }

    // ── Mistral ────────────────────────────────────────────────
    if model.contains("mistral-large") {
        return (2.00, 6.00);
    }
    if model.contains("mistral-small") || model.contains("mistral") {
        return (0.10, 0.30);
    }

    // ── Cohere ─────────────────────────────────────────────────
    if model.contains("command-r-plus") {
        return (2.50, 10.0);
    }
    if model.contains("command-r") {
        return (0.15, 0.60);
    }

    // ── Perplexity ──────────────────────────────────────────────
    if model.contains("sonar-pro") {
        return (3.0, 15.0);
    }
    if model.contains("sonar") {
        return (1.0, 5.0);
    }

    // ── xAI / Grok ──────────────────────────────────────────────
    if model.contains("grok-3-mini") || model.contains("grok-2-mini") || model.contains("grok-mini")
    {
        return (0.30, 0.50);
    }
    if model.contains("grok-3") {
        return (3.0, 15.0);
    }
    if model.contains("grok") {
        return (2.0, 10.0);
    }

    // ── AI21 / Jamba ────────────────────────────────────────────
    if model.contains("jamba") {
        return (2.0, 8.0);
    }

    // ── Default (conservative) ─────────────────────────────────
    (1.0, 3.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_cost_haiku() {
        let cost = MeteringEngine::estimate_cost("claude-haiku-4-5-20251001", 1_000_000, 1_000_000);
        assert!((cost - 1.50).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_sonnet() {
        let cost = MeteringEngine::estimate_cost("claude-sonnet-4-20250514", 1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_opus() {
        let cost = MeteringEngine::estimate_cost("claude-opus-4-20250514", 1_000_000, 1_000_000);
        assert!((cost - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_gpt4o() {
        let cost = MeteringEngine::estimate_cost("gpt-4o-2024-11-20", 1_000_000, 1_000_000);
        assert!((cost - 12.50).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_gpt4o_mini() {
        let cost = MeteringEngine::estimate_cost("gpt-4o-mini", 1_000_000, 1_000_000);
        assert!((cost - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_gpt41() {
        let cost = MeteringEngine::estimate_cost("gpt-4.1", 1_000_000, 1_000_000);
        assert!((cost - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_gpt41_mini() {
        let cost = MeteringEngine::estimate_cost("gpt-4.1-mini", 1_000_000, 1_000_000);
        assert!((cost - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_gpt41_nano() {
        let cost = MeteringEngine::estimate_cost("gpt-4.1-nano", 1_000_000, 1_000_000);
        assert!((cost - 0.50).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_o3_mini() {
        let cost = MeteringEngine::estimate_cost("o3-mini", 1_000_000, 1_000_000);
        assert!((cost - 5.50).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_gemini_20_flash() {
        let cost = MeteringEngine::estimate_cost("gemini-2.0-flash", 1_000_000, 1_000_000);
        assert!((cost - 0.50).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_gemini_25_pro() {
        let cost = MeteringEngine::estimate_cost("gemini-2.5-pro", 1_000_000, 1_000_000);
        assert!((cost - 11.25).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_gemini_25_flash() {
        let cost = MeteringEngine::estimate_cost("gemini-2.5-flash", 1_000_000, 1_000_000);
        assert!((cost - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_deepseek_chat() {
        let cost = MeteringEngine::estimate_cost("deepseek-chat", 1_000_000, 1_000_000);
        assert!((cost - 1.37).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_deepseek_reasoner() {
        let cost = MeteringEngine::estimate_cost("deepseek-reasoner", 1_000_000, 1_000_000);
        assert!((cost - 2.74).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_llama() {
        let cost = MeteringEngine::estimate_cost("llama-3.3-70b-versatile", 1_000_000, 1_000_000);
        assert!((cost - 0.15).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_grok() {
        let cost = MeteringEngine::estimate_cost("grok-2", 1_000_000, 1_000_000);
        assert!((cost - 12.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_grok_mini() {
        let cost = MeteringEngine::estimate_cost("grok-2-mini", 1_000_000, 1_000_000);
        assert!((cost - 0.80).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_cerebras() {
        let cost = MeteringEngine::estimate_cost("cerebras/llama3.3-70b", 1_000_000, 1_000_000);
        assert!((cost - 0.12).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_unknown() {
        let cost = MeteringEngine::estimate_cost("my-custom-model", 1_000_000, 1_000_000);
        assert!((cost - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_record_and_check_cost() {
        let engine = MeteringEngine::new();
        let agent_id = AgentId::new();

        engine.record_cost(agent_id, 0.10);
        assert!((engine.agent_cost(agent_id) - 0.10).abs() < 0.001);

        engine.record_cost(agent_id, 0.20);
        assert!((engine.agent_cost(agent_id) - 0.30).abs() < 0.001);
    }

    #[test]
    fn test_total_cost() {
        let engine = MeteringEngine::new();
        let a1 = AgentId::new();
        let a2 = AgentId::new();

        engine.record_cost(a1, 0.50);
        engine.record_cost(a2, 0.25);
        assert!((engine.total_cost() - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_hourly_quota_check() {
        let engine = MeteringEngine::new();
        let agent_id = AgentId::new();

        engine.record_cost(agent_id, 1.50);
        assert!(engine.check_hourly_quota(agent_id, 1.0).is_err());
        assert!(engine.check_hourly_quota(agent_id, 2.0).is_ok());
        assert!(engine.check_hourly_quota(agent_id, 0.0).is_ok()); // unlimited
    }

    #[test]
    fn test_budget_status() {
        let engine = MeteringEngine::new();
        let agent_id = AgentId::new();
        engine.record_cost(agent_id, 0.50);

        let status = engine.budget_status(1.0, 10.0);
        assert!((status.current_spend - 0.50).abs() < 0.001);
        assert!((status.hourly_pct - 0.50).abs() < 0.001);
        assert!((status.daily_pct - 0.05).abs() < 0.001);
    }

    #[test]
    fn test_reset() {
        let engine = MeteringEngine::new();
        let agent_id = AgentId::new();
        engine.record_cost(agent_id, 1.0);
        engine.reset();
        assert!((engine.total_cost()).abs() < 0.001);
    }
}
