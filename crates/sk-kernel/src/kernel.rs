//! Main kernel struct — lifecycle management for the Sovereign Kernel.

use sk_mcp::McpRegistry;
use sk_memory::MemorySubstrate;
use sk_soul::SoulIdentity;
use sk_types::config::KernelConfig;
use sk_types::{SovereignError, SovereignResult};
use std::sync::Arc;
use tracing::info;

/// The Sovereign Kernel — top-level orchestrator.
pub struct SovereignKernel {
    /// Global configuration.
    pub config: KernelConfig,
    /// Soul identity.
    pub soul: SoulIdentity,
    /// Memory substrate.
    pub memory: Arc<MemorySubstrate>,
    /// MCP server registry.
    pub mcp: McpRegistry,
    /// Conversational safety gate.
    pub safety: Arc<crate::approval::SafetyGate>,
    /// Global LLM driver.
    pub driver: Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync>,
    /// Global LLM model name.
    pub model_name: String,
    /// Browser session manager.
    pub browser: Arc<sk_engine::media::browser::BrowserManager>,
    /// Skill registry.
    pub skills: Arc<sk_tools::skills::SkillRegistry>,
}

impl SovereignKernel {
    /// Initialize the kernel from configuration.
    pub async fn init(config: KernelConfig) -> SovereignResult<Self> {
        info!("Initializing Sovereign Kernel...");

        // Load Soul identity
        let soul = {
            // Auto-discover SOUL.md in common locations
            let auto_paths = vec![
                std::path::PathBuf::from("soul/SOUL.md"),
                std::path::PathBuf::from("SOUL.md"),
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("soul/SOUL.md")))
                    .unwrap_or_default(),
            ];
            let mut found = None;
            for p in &auto_paths {
                if p.exists() {
                    info!(path = %p.display(), "Auto-discovered SOUL.md");
                    found = Some(SoulIdentity::load(p)?);
                    break;
                }
            }
            found.unwrap_or_else(SoulIdentity::empty)
        };
        info!(has_soul = !soul.is_empty(), "Soul loaded");

        // Open memory substrate
        let db_path = config
            .memory
            .sqlite_path
            .clone()
            .unwrap_or_else(|| config.data_dir.join("memory.db"));
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| SovereignError::Config(format!("Failed to create data dir: {e}")))?;
        }
        let memory = Arc::new(MemorySubstrate::open(
            db_path.as_path(),
            config.memory.decay_rate,
        )?);
        info!(path = %db_path.display(), "Memory substrate opened");

        // Connect MCP servers
        let mut mcp = McpRegistry::new();

        let mut mcp_servers_map = std::collections::HashMap::new();
        for server in &config.mcp_servers {
            let entry = sk_types::config::McpServerEntry {
                transport: match &server.transport {
                    sk_types::config::McpTransportEntry::Stdio { .. } => "stdio".to_string(),
                    sk_types::config::McpTransportEntry::Sse { .. } => "sse".to_string(),
                },
                command: match &server.transport {
                    sk_types::config::McpTransportEntry::Stdio { command, .. } => {
                        Some(command.clone())
                    }
                    _ => None,
                },
                args: match &server.transport {
                    sk_types::config::McpTransportEntry::Stdio { args, .. } => args.clone(),
                    _ => Vec::new(),
                },
                env: server
                    .env
                    .iter()
                    .map(|k| (k.clone(), std::env::var(k).unwrap_or_default()))
                    .collect(),
                url: match &server.transport {
                    sk_types::config::McpTransportEntry::Sse { url, .. } => Some(url.clone()),
                    _ => None,
                },
            };
            mcp_servers_map.insert(server.name.clone(), entry);
        }

        mcp.connect_all(&mcp_servers_map).await?;
        info!(
            servers = mcp.server_count(),
            tools = mcp.tool_count(),
            "MCP registry initialized"
        );

        // Initialize LLM Driver from environment
        // model_name is built up through if-else branches then moved into the struct.
        #[allow(unused_assignments)]
        let mut model_name = String::new();
        let driver: Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync> = if let Some(key) = std::env::var("ANTHROPIC_API_KEY").ok() {
            model_name = "claude-3-5-sonnet-20241022".to_string();
            Arc::new(sk_engine::drivers::anthropic::AnthropicDriver::new(key, "https://api.anthropic.com".to_string()))
        } else if let Some(key) = std::env::var("OPENAI_API_KEY").ok() {
            model_name = "gpt-4o".to_string();
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(key, "https://api.openai.com/v1".to_string()))
        } else if let Some(key) = std::env::var("GITHUB_TOKEN").ok() {
            model_name = "gpt-4o".to_string();
            Arc::new(sk_engine::drivers::copilot::CopilotDriver::new(key, "".to_string()))
        } else if let Some(key) = std::env::var("GROQ_API_KEY").ok() {
            model_name = "llama3-70b-8192".to_string();
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(key, "https://api.groq.com/openai/v1".to_string()))
        } else if let Some(key) = std::env::var("GEMINI_API_KEY").ok() {
            model_name = "gemini-2.0-flash-lite".to_string();
            Arc::new(sk_engine::drivers::gemini::GeminiDriver::new(key, "https://generativelanguage.googleapis.com".to_string()))
        } else {
            model_name = "claude-3-5-sonnet-20241022".to_string();
            // Default to Anthropic if no keys found (will fail at runtime, but kernel can still boot)
            Arc::new(sk_engine::drivers::anthropic::AnthropicDriver::new("".to_string(), "https://api.anthropic.com".to_string()))
        };

        // Initialize Browser Manager
        let browser = Arc::new(sk_engine::media::browser::BrowserManager::new(config.browser.clone()));

        // Initialize Skills Registry
        let skills_path = std::env::current_dir()
            .unwrap_or_default()
            .join("crates")
            .join("sk-tools")
            .join("skills");
        let skills = Arc::new(sk_tools::skills::SkillRegistry::load_from_dir(skills_path));

        Ok(Self {
            config,
            soul,
            memory,
            mcp,
            safety: Arc::new(crate::approval::SafetyGate::new(true)),
            driver,
            model_name,
            browser,
            skills,
        })
    }

    /// Run a complete agent loop for a given session and user input.
    pub async fn run_agent(
        self: &Arc<Self>,
        session: &mut sk_types::Session,
        input: &str,
    ) -> SovereignResult<sk_engine::agent_loop::AgentLoopResult> {
        let system_prompt = self.soul.to_system_prompt_fragment();

        let agent_config = crate::executor::create_agent_config(
            self.clone(),
            self.driver.as_ref(), // needs to be &dyn LlmDriver
            system_prompt,
            self.model_name.clone(),
            session.agent_id.clone(),
            self.browser.clone(),
            self.skills.clone(),
        );

        let result = sk_engine::agent_loop::run_agent_loop(agent_config, session, input)
            .await
            .map_err(|e| sk_types::error::SovereignError::Internal(e.to_string()))?;

        // Save after every turn
        if let Err(e) = self.memory.sessions.save(session) {
            tracing::warn!("Failed to save session across run_agent: {e}");
        }

        Ok(result)
    }

    /// Start the API bridge server if enabled in configuration.
    pub async fn start_api_server(self: Arc<Self>) -> SovereignResult<()> {
        let addr = &self.config.api_listen;
        let port = addr.split(':').last().and_then(|p| p.parse().ok()).unwrap_or(3000);
        crate::api::start_server(self, port).await
    }

    /// Shut down the kernel gracefully.
    pub async fn shutdown(&self) -> SovereignResult<()> {
        info!("Sovereign Kernel shutting down...");
        // MCP connections are dropped automatically
        Ok(())
    }
}
