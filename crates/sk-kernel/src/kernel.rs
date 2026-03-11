//! Main kernel struct — lifecycle management for the Sovereign Kernel.

use sk_mcp::McpRegistry;
use sk_memory::MemorySubstrate;
use sk_soul::SoulIdentity;
use sk_types::config::KernelConfig;
use sk_types::{SovereignError, SovereignResult};
use std::sync::Arc;
use tracing::info;

/// The Sovereign Kernel — top-level king.
pub struct SovereignKernel {
    /// Global configuration.
    pub config: KernelConfig,
    /// Soul identity.
    pub soul: SoulIdentity,
    /// Memory substrate.
    pub memory: Arc<MemorySubstrate>,
    /// MCP server registry.
    pub mcp: Arc<tokio::sync::RwLock<McpRegistry>>,
    /// Conversational safety gate.
    pub safety: Arc<crate::approval::SafetyGate>,
    /// Global LLM driver.
    pub driver: Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync>,
    /// Global LLM model name.
    pub model_name: String,
    /// Browser session manager.
    pub browser: Arc<sk_engine::runtime::browser::BrowserManager>,
    /// Skill registry.
    pub skills: Arc<tokio::sync::RwLock<sk_tools::skills::SkillRegistry>>,
    /// Agent-to-Agent message bus.
    pub bus: Arc<crate::bus::InterAgentBus>,
    /// Global agent registry.
    pub agents: Arc<crate::registry::AgentRegistry>,
    /// Scheduled background job king.
    pub cron: Arc<crate::cron::CronScheduler>,
    /// Process supervisor.
    pub supervisor: Arc<crate::supervisor::Supervisor>,
    /// Handler to send back responses to channels like Telegram/Discord.
    pub delivery_handler:
        tokio::sync::RwLock<Option<Arc<dyn sk_types::scheduler::CronDeliveryHandler>>>,
    /// Docker sandbox container pool.
    pub sandbox_pool: Arc<sk_engine::runtime::docker_sandbox::ContainerPool>,
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
        let mcp = Arc::new(tokio::sync::RwLock::new(mcp));

        // Initialize LLM Driver from environment
        // model_name is built up through if-else branches then moved into the struct.
        #[allow(unused_assignments)]
        let mut model_name = String::new();
        let driver: Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync> =
            if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
                model_name = "claude-3-5-sonnet-20241022".to_string();
                Arc::new(sk_engine::drivers::anthropic::AnthropicDriver::new(
                    key,
                    "https://api.anthropic.com".to_string(),
                ))
            } else if let Ok(key) = std::env::var("OPENAI_API_KEY") {
                model_name = "gpt-4o".to_string();
                Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                    key,
                    "https://api.openai.com/v1".to_string(),
                ))
            } else if let Ok(key) = std::env::var("GITHUB_TOKEN") {
                model_name = "gpt-4o".to_string();
                Arc::new(sk_engine::drivers::copilot::CopilotDriver::new(
                    key,
                    "".to_string(),
                ))
            } else if let Ok(key) = std::env::var("GROQ_API_KEY") {
                model_name = "llama3-70b-8192".to_string();
                Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                    key,
                    "https://api.groq.com/openai/v1".to_string(),
                ))
            } else if let Ok(key) = std::env::var("GEMINI_API_KEY") {
                model_name = "gemini-2.0-flash-lite".to_string();
                Arc::new(sk_engine::drivers::gemini::GeminiDriver::new(
                    key,
                    "https://generativelanguage.googleapis.com".to_string(),
                ))
            } else {
                model_name = "claude-3-5-sonnet-20241022".to_string();
                // Default to Anthropic if no keys found (will fail at runtime, but kernel can still boot)
                Arc::new(sk_engine::drivers::anthropic::AnthropicDriver::new(
                    "".to_string(),
                    "https://api.anthropic.com".to_string(),
                ))
            };

        // Initialize Browser Manager
        let browser = Arc::new(sk_engine::runtime::browser::BrowserManager::new(
            config.browser.clone(),
        ));

        // Initialize Skills Registry
        let mut skills_path = std::env::current_dir()
            .unwrap_or_default()
            .join("crates")
            .join("sk-tools")
            .join("skills");

        if !skills_path.exists() {
            if let Ok(exe) = std::env::current_exe() {
                if let Some(parent) = exe.parent() {
                    skills_path = parent.join("skills");
                }
            }
        }
        let skills = Arc::new(tokio::sync::RwLock::new(
            sk_tools::skills::SkillRegistry::load_from_dir(skills_path),
        ));

        let bus = Arc::new(crate::bus::InterAgentBus::new(memory.clone()));

        let cron = Arc::new(crate::cron::CronScheduler::new(&config.data_dir, 100));
        if let Err(e) = cron.load() {
            tracing::warn!("Failed to load persisted cron jobs: {}", e);
        }

        let supervisor = Arc::new(crate::supervisor::Supervisor::new());
        let agents = Arc::new(crate::registry::AgentRegistry::new());
        let sandbox_pool = Arc::new(sk_engine::runtime::docker_sandbox::ContainerPool::new());

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
            bus,
            agents,
            cron,
            supervisor,
            delivery_handler: tokio::sync::RwLock::new(None),
            sandbox_pool,
        })
    }

    /// Set the global delivery handler for cron jobs.
    pub async fn set_delivery_handler(
        &self,
        handler: Arc<dyn sk_types::scheduler::CronDeliveryHandler>,
    ) {
        let mut lock = self.delivery_handler.write().await;
        *lock = Some(handler);
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
            self.driver.clone(),
            system_prompt,
            self.model_name.clone(),
            session.agent_id,
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

    /// Start background services, including the cron job executor.
    pub async fn start_background_services(self: &Arc<Self>) {
        let kernel = self.clone();
        tokio::spawn(async move {
            tracing::info!("Starting background cron scheduler...");
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;

                // --- Heartbeat & Supervisor Check ---
                let db_agents = kernel.agents.list();
                let mut hb_info = Vec::new();
                for agent in db_agents {
                    if agent.state == sk_types::agent::AgentState::Running {
                        hb_info.push(crate::heartbeat::AgentHeartbeatInfo {
                            id: agent.id,
                            name: agent.name.clone(),
                            state: agent.state,
                            last_active: agent.last_active,
                            heartbeat_interval_secs: agent
                                .manifest
                                .autonomous
                                .map(|a| a.heartbeat_interval_secs),
                        });
                    }
                }

                let hb_config = crate::heartbeat::HeartbeatConfig::default();
                let statuses = crate::heartbeat::check_agents(&hb_info, &hb_config);

                for status in statuses {
                    if status.unresponsive {
                        tracing::warn!(
                            "Agent {} is unresponsive. Initiating recovery...",
                            status.name
                        );
                        if kernel
                            .supervisor
                            .record_agent_restart(status.agent_id, 3)
                            .is_ok()
                        {
                            tracing::info!(
                                "Agent {} marked for restart by supervisor.",
                                status.name
                            );
                            // A real system would enqueue a wake-up or reset the loop state here
                        } else {
                            tracing::error!(
                                "Agent {} exceeded max restarts. Suspending.",
                                status.name
                            );
                            if let Some(mut agent) = kernel.agents.get(status.agent_id) {
                                agent.state = sk_types::agent::AgentState::Suspended;
                                let _ = kernel.agents.set_state(
                                    status.agent_id,
                                    sk_types::agent::AgentState::Suspended,
                                );
                            }
                        }
                    }
                }
                // --- End Heartbeat Check ---

                let due_jobs = kernel.cron.due_jobs();
                for job in due_jobs {
                    tracing::info!(
                        agent_id = %job.agent_id,
                        job_id = %job.id,
                        name = %job.name,
                        "Executing scheduled background job"
                    );

                    match &job.action {
                        sk_types::scheduler::CronAction::AgentTurn { message, .. } => {
                            let aid = job.agent_id;
                            let k = kernel.clone();
                            let msg = message.clone();
                            let job_id = job.id;
                            let delivery = job.delivery.clone();

                            tokio::spawn(async move {
                                let session = match k.memory.sessions.list_for_agent(aid) {
                                    Ok(sessions) if !sessions.is_empty() => {
                                        k.memory.sessions.load(sessions[0].0).unwrap_or(None)
                                    }
                                    _ => None,
                                };

                                let mut s = session.unwrap_or_else(|| {
                                    let mut new_s = sk_types::Session::new(aid);
                                    new_s.push_message(sk_types::Message::user("System Notification: You have been woken up by your scheduled background job. Please use logs and context to fulfill the task."));
                                    new_s
                                });

                                match k.run_agent(&mut s, &msg).await {
                                    Ok(res) => {
                                        k.cron.record_success(job_id);
                                        // Execute delivery
                                        if let Some(handler) =
                                            k.delivery_handler.read().await.as_ref()
                                        {
                                            let response_text = res.response.clone();
                                            let h_clone = handler.clone();
                                            tokio::spawn(async move {
                                                if let Err(e) =
                                                    h_clone.deliver(&delivery, &response_text).await
                                                {
                                                    tracing::error!(
                                                        "Delivery failed for job {job_id}: {e}"
                                                    );
                                                }
                                            });
                                        }
                                    }
                                    Err(e) => k.cron.record_failure(job_id, &e.to_string()),
                                }
                                let _ = k.cron.persist();
                            });
                        }
                        sk_types::scheduler::CronAction::SystemEvent { text } => {
                            let payload =
                                serde_json::json!({ "job_id": job.id.to_string(), "text": text });
                            if let Err(e) = kernel.memory.audit.append_log(
                                &job.agent_id,
                                "System",
                                "cron_event",
                                &payload,
                            ) {
                                kernel.cron.record_failure(job.id, &e.to_string());
                            } else {
                                kernel.cron.record_success(job.id);
                            }
                            let _ = kernel.cron.persist();
                        }
                    }
                }
            }
        });
    }

    /// Start the API bridge server if enabled in configuration.
    pub async fn start_api_server(self: Arc<Self>) -> SovereignResult<()> {
        let addr = self.config.api_listen.clone();
        crate::api::start_server(self, &addr).await
    }

    /// Shut down the kernel gracefully.
    pub async fn shutdown(&self) -> SovereignResult<()> {
        info!("Sovereign Kernel shutting down...");
        // MCP connections are dropped automatically
        Ok(())
    }
}
