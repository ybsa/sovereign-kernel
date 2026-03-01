use async_trait::async_trait;
use dashmap::DashMap;
use sk_channels::bridge::ChannelBridgeHandle;
use sk_engine::media::browser::BrowserManager;
use sk_kernel::SovereignKernel;
use sk_types::AgentId;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

pub struct SovereignBridge {
    kernel: Arc<SovereignKernel>,
    sessions: DashMap<AgentId, Arc<Mutex<sk_types::Session>>>,
    safety: Arc<crate::safety::SafetyGate>,
}

impl SovereignBridge {
    pub fn new(kernel: Arc<SovereignKernel>) -> Self {
        // Enable safety by default; disable with SOVEREIGN_UNSAFE=1
        let safety_enabled = std::env::var("SOVEREIGN_UNSAFE")
            .map(|v| v != "1")
            .unwrap_or(true);
        Self {
            kernel,
            sessions: DashMap::new(),
            safety: Arc::new(crate::safety::SafetyGate::new(safety_enabled)),
        }
    }
}

#[async_trait]
impl ChannelBridgeHandle for SovereignBridge {
    async fn send_message(&self, agent_id: AgentId, message: &str) -> Result<String, String> {
        info!("Channel message received for agent {agent_id}: {message}");

        // Initialize LLM Driver from environment (same as chat.rs)
        let anthropic_key = std::env::var("ANTHROPIC_API_KEY").ok();
        let gemini_key = std::env::var("GEMINI_API_KEY").ok();
        let openai_key = std::env::var("OPENAI_API_KEY").ok();
        let github_token = std::env::var("GITHUB_TOKEN").ok();
        let groq_key = std::env::var("GROQ_API_KEY").ok();

        let mut driver: Option<Box<dyn sk_engine::llm_driver::LlmDriver>> = None;
        let mut model_name = String::new();

        if let Some(key) = anthropic_key {
            driver = Some(Box::new(
                sk_engine::drivers::anthropic::AnthropicDriver::new(
                    key,
                    "https://api.anthropic.com".to_string(),
                ),
            ));
            model_name = "claude-3-5-sonnet-20241022".to_string();
        } else if let Some(key) = openai_key {
            driver = Some(Box::new(sk_engine::drivers::openai::OpenAIDriver::new(
                key,
                "https://api.openai.com/v1".to_string(),
            )));
            model_name = "gpt-4o".to_string();
        } else if let Some(key) = github_token {
            driver = Some(Box::new(sk_engine::drivers::copilot::CopilotDriver::new(
                key,
                "".to_string(),
            )));
            model_name = "gpt-4o".to_string();
        } else if let Some(key) = groq_key {
            driver = Some(Box::new(sk_engine::drivers::openai::OpenAIDriver::new(
                key,
                "https://api.groq.com/openai/v1".to_string(),
            )));
            model_name = "llama3-70b-8192".to_string();
        } else if let Some(key) = gemini_key {
            driver = Some(Box::new(sk_engine::drivers::gemini::GeminiDriver::new(
                key,
                "https://generativelanguage.googleapis.com".to_string(),
            )));
            model_name = "gemini-2.0-flash-lite".to_string();
        }

        let driver = driver.ok_or_else(|| "No valid API key found in environment (tried ANTHROPIC, OPENAI, GITHUB_TOKEN, GROQ, GEMINI). Cannot process channel message.".to_string())?;

        // Retrieve or create session in memory cache
        let session_mutex = self
            .sessions
            .entry(agent_id.clone())
            .or_insert_with(|| {
                // Try to load from SQLite database first
                if let Ok(entries) = self.kernel.memory.sessions.list_for_agent(agent_id.clone()) {
                    if let Some((latest_id, _, _)) = entries.first() {
                        if let Ok(Some(loaded_session)) =
                            self.kernel.memory.sessions.load(*latest_id)
                        {
                            return Arc::new(Mutex::new(loaded_session));
                        }
                    }
                }
                // Fallback to new session
                Arc::new(Mutex::new(sk_types::Session::new(agent_id.clone())))
            })
            .clone();

        let mut session = session_mutex.lock().await;

        let user_msg_lower = message.trim().to_lowercase();
        if user_msg_lower == "approve" || user_msg_lower == "yes" {
            if self.safety.approve_last_for_agent(&agent_id) {
                // We inject a system message into the conversation to let the LLM know
                // that the human approved its action, and it should run the tool again.
                // But for now, just let the "approve" message go to the LLM naturally.
                // The LLM will see "approve" and re-try because the gate is open.
            }
        }

        let system_prompt = self.kernel.soul.to_system_prompt_fragment();

        // Create BrowserManager
        let browser_manager = Arc::new(BrowserManager::new(self.kernel.config.browser.clone()));

        let config = crate::tool_executor::create_agent_config(
            driver.as_ref(),
            system_prompt,
            model_name,
            self.kernel.memory.clone(),
            browser_manager.clone(), // Pass browser_manager here
            agent_id.clone(),
            self.safety.enabled,
            Some(self.safety.clone()),
        );

        let result = sk_engine::agent_loop::run_agent_loop(config, &mut session, message).await;

        // Save the updated session back to SQLite
        if let Err(e) = self.kernel.memory.sessions.save(&session) {
            tracing::warn!("Failed to save session for agent {agent_id}: {e}");
        }

        match result {
            Ok(result) => Ok(result.response),
            Err(e) => {
                error!("Agent loop failed: {e}");
                Err(e.to_string())
            }
        }
    }

    async fn find_agent_by_name(&self, name: &str) -> Result<Option<AgentId>, String> {
        // Create a deterministic UUID so the same agent name always gets the same ID and session
        let hash = md5::compute(name);
        Ok(Some(AgentId(uuid::Uuid::from_bytes(*hash))))
    }

    async fn list_agents(&self) -> Result<Vec<(AgentId, String)>, String> {
        let name = "Sovereign Default";
        let hash = md5::compute(name);
        Ok(vec![(
            AgentId(uuid::Uuid::from_bytes(*hash)),
            name.to_string(),
        )])
    }

    async fn spawn_agent_by_name(&self, manifest_name: &str) -> Result<AgentId, String> {
        let hash = md5::compute(manifest_name);
        Ok(AgentId(uuid::Uuid::from_bytes(*hash)))
    }
}
