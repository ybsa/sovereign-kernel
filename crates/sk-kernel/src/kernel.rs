//! Main kernel struct — lifecycle management for the Sovereign Kernel.

use dashmap::DashMap;
use sk_mcp::McpRegistry;
use sk_memory::MemorySubstrate;
use sk_soul::SoulIdentity;
use sk_types::config::KernelConfig;
use sk_types::{AgentId, SovereignError, SovereignResult};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

/// The Sovereign Kernel — top-level king.
pub struct SovereignKernel {
    /// Global configuration.
    pub config: Arc<std::sync::RwLock<KernelConfig>>,
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
    pub skills: Arc<std::sync::RwLock<sk_tools::skills::SkillRegistry>>,
    /// Agent-to-Agent message bus.
    pub bus: Arc<crate::bus::InterAgentBus>,
    /// Global agent registry.
    pub agents: Arc<crate::registry::AgentRegistry>,
    /// Global event bus for kernel-wide notifications.
    pub event_bus: Arc<crate::event_bus::EventBus>,
    /// Scheduled background job king.
    pub cron: Arc<crate::cron::CronScheduler>,
    /// Process supervisor.
    pub supervisor: Arc<crate::supervisor::Supervisor>,
    /// Handler to send back responses to channels like Telegram/Discord.
    pub delivery_handler:
        std::sync::RwLock<Option<Arc<dyn sk_types::scheduler::CronDeliveryHandler>>>,
    /// Docker sandbox container pool.
    pub sandbox_pool: Arc<sk_engine::runtime::docker_sandbox::ContainerPool>,
    /// Global metering engine for cost tracking and budget enforcement.
    pub metering: Arc<crate::metering::MeteringEngine>,
    /// Global hand registry for managing capability packages.
    pub hands: Arc<std::sync::RwLock<sk_hands::registry::HandRegistry>>,
    /// Track active agent loops for cancellation/inspection.
    pub active_loops: Arc<DashMap<AgentId, CancellationToken>>,
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
                env: match &server.env {
                    sk_types::config::McpEnv::List(nodes) => nodes
                        .iter()
                        .map(|k| (k.clone(), std::env::var(k).unwrap_or_default()))
                        .collect(),
                    sk_types::config::McpEnv::Map(map) => map
                        .iter()
                        .map(|(k, v)| {
                            let val = if let Some(env_name) = v.strip_prefix('$') {
                                std::env::var(env_name).unwrap_or_default()
                            } else {
                                v.clone()
                            };
                            (k.clone(), val)
                        })
                        .collect(),
                },
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

        // Initialize LLM Driver from configuration or environment
        let (driver, model) = match init_llm_driver(&config).await {
            Ok(res) => res,
            Err(e) => {
                warn!("Failed to initialize LLM driver from config: {e}. Falling back to default.");
                // Fallback to Anthropic placeholder (current behavior)
                let fallback_driver: Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync> =
                    Arc::new(sk_engine::drivers::anthropic::AnthropicDriver::new(
                        "".to_string(),
                        "https://api.anthropic.com".to_string(),
                    ));
                (fallback_driver, "claude-3-5-sonnet-20241022".to_string())
            }
        };
        let model_name = model;

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
        let skills = Arc::new(std::sync::RwLock::new(
            sk_tools::skills::SkillRegistry::load_from_dir(skills_path),
        ));

        let bus = Arc::new(crate::bus::InterAgentBus::new(memory.clone()));
        let event_bus = Arc::new(crate::event_bus::EventBus::new(1024));

        let cron = Arc::new(crate::cron::CronScheduler::new(&config.data_dir, 100));
        if let Err(e) = cron.load() {
            tracing::warn!("Failed to load persisted cron jobs: {}", e);
        }

        let supervisor = Arc::new(crate::supervisor::Supervisor::new());
        let agents = Arc::new(crate::registry::AgentRegistry::new());
        let sandbox_pool = Arc::new(sk_engine::runtime::docker_sandbox::ContainerPool::new());

        let mut metering = crate::metering::MeteringEngine::new();
        metering.set_persist_path(config.data_dir.join("metering.json"));
        metering.load().await.ok(); // Ignore errors if file doesn't exist yet
        let metering = Arc::new(metering);

        // Periodically save metering status
        let m_save = metering.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(300)).await;
                if let Err(e) = m_save.save().await {
                    tracing::error!("Failed to save metering state: {}", e);
                }
            }
        });

        // Initialize Hands Registry
        let mut hand_registry = sk_hands::registry::HandRegistry::new();
        hand_registry.load_bundled();

        let custom_hands_path = config.data_dir.join("hands");
        if !custom_hands_path.exists() {
            let _ = std::fs::create_dir_all(&custom_hands_path);
        }
        hand_registry.load_custom_hands(&custom_hands_path);
        let hands = Arc::new(std::sync::RwLock::new(hand_registry));
 
        Ok(Self {
            config: Arc::new(std::sync::RwLock::new(config)),
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
            event_bus,
            cron,
            supervisor,
            delivery_handler: std::sync::RwLock::new(None),
            sandbox_pool,
            metering,
            hands,
            active_loops: Arc::new(DashMap::new()),
        })
    }

    /// Set the global delivery handler for cron jobs.
    pub async fn set_delivery_handler(
        &self,
        handler: Arc<dyn sk_types::scheduler::CronDeliveryHandler>,
    ) {
        let mut lock = self.delivery_handler.write().unwrap();
        *lock = Some(handler);
    }

    /// Run a complete agent loop for a given session and user input.
    pub async fn run_agent(
        self: &Arc<Self>,
        session: &mut sk_types::Session,
        input: &str,
    ) -> SovereignResult<sk_engine::agent_loop::AgentLoopResult> {
        let system_prompt = self.soul.to_system_prompt_fragment();

        let mut agent_config = crate::executor::create_agent_config(
            self.clone(),
            self.driver.clone(),
            system_prompt,
            self.model_name.clone(),
            session.agent_id,
            self.browser.clone(),
            self.skills.clone(),
        );

        let k = self.clone();
        let aid = session.agent_id;
        let sid = session.id;
        // AgentLoopConfig contains non-serializable fields (Box<dyn...>), so we pass Null for now.
        // In a future version, we would save the original AgentManifest.
        let config_value = serde_json::Value::Null;

        agent_config.checkpoint_handler = Some(Box::new(move |_sess| {
            k.memory.checkpoint.save(
                &aid,
                &sid.0,
                &config_value,
                &serde_json::Value::Null,
                "active",
            )
        }));

        // 1. Create and store cancellation token
        let token = CancellationToken::new();
        self.active_loops.insert(aid, token.clone());
        self.event_bus
            .publish(crate::event_bus::KernelEvent::AgentStarted {
                agent_id: aid.to_string(),
            });

        // 2. Wrap the loop in a select! to handle cancellation
        let result = tokio::select! {
            res = sk_engine::agent_loop::run_agent_loop(agent_config, session, input) => {
                res.map_err(|e| sk_types::error::SovereignError::Internal(e.to_string()))
            }
            _ = token.cancelled() => {
                info!(agent_id = %aid, "Agent loop cancelled externally.");
                Err(sk_types::error::SovereignError::Internal("Agent loop cancelled".to_string()))
            }
        };

        // 3. Remove token after completion
        self.active_loops.remove(&aid);
        self.event_bus
            .publish(crate::event_bus::KernelEvent::AgentStopped {
                agent_id: aid.to_string(),
            });

        // Save terminal checkpoint status
        let terminal_status = if result.is_ok() { "completed" } else { "error" };
        let _ = self.memory.checkpoint.save(
            &session.agent_id,
            &session.id.0,
            &serde_json::Value::Null,
            &serde_json::Value::Null,
            terminal_status,
        );

        let result = result?;

        // Save after every turn
        if let Err(e) = self.memory.sessions.save(session) {
            tracing::warn!("Failed to save session across run_agent: {e}");
        }

        Ok(result)
    }

    /// Stop a running agent loop.
    pub fn stop_agent(&self, id: &AgentId) -> bool {
        if let Some((_, token)) = self.active_loops.remove(id) {
            token.cancel();
            self.event_bus
                .publish(crate::event_bus::KernelEvent::AgentStopped {
                    agent_id: id.to_string(),
                });
            true
        } else {
            false
        }
    }

    /// Resurrect an agent from its latest checkpoint.
    pub async fn resurrect_agent(
        self: &Arc<Self>,
        agent_id: sk_types::AgentId,
    ) -> SovereignResult<()> {
        info!(agent_id = %agent_id, "Resurrector: Attempting to resurrect agent");

        // 1. Load latest checkpoint
        let checkpoint = match self.memory.checkpoint.load_latest(&agent_id)? {
            Some(cp) => cp,
            None => {
                warn!(agent_id = %agent_id, "Resurrector: No checkpoint found for agent");
                return Ok(());
            }
        };

        // 2. Restore session
        let mut session = match self
            .memory
            .sessions
            .load(sk_types::SessionId(checkpoint.session_id))?
        {
            Some(s) => s,
            None => {
                warn!(session_id = %checkpoint.session_id, "Resurrector: Session for checkpoint not found. Creating new empty session.");
                sk_types::Session::new(agent_id)
            }
        };

        // 3. Inject resurrection notification
        session.push_message(sk_types::Message::system(
            "[Resurrector] SYSTEM: Critical failure recovered. Auto-restarted from latest checkpoint. Please verify your last state and continue."
        ));

        // 4. Re-run agent loop with restored config
        let system_prompt = self.soul.to_system_prompt_fragment();

        let mut agent_config = crate::executor::create_agent_config(
            self.clone(),
            self.driver.clone(),
            system_prompt,
            self.model_name.clone(),
            agent_id,
            self.browser.clone(),
            self.skills.clone(),
        );

        let k = self.clone();
        let aid = session.agent_id;
        let sid = session.id;
        // In this phase, we use Null for config_value to satisfy type requirements.
        let config_value = serde_json::Value::Null;

        agent_config.checkpoint_handler = Some(Box::new(move |_sess| {
            k.memory.checkpoint.save(
                &aid,
                &sid.0,
                &config_value,
                &serde_json::Value::Null,
                "active",
            )
        }));

        // Trigger loop continuation
        let result = sk_engine::agent_loop::run_agent_loop(
            agent_config,
            &mut session,
            "System: Please continue where you left off.",
        )
        .await;

        let terminal_status = if result.is_ok() { "completed" } else { "error" };
        let _ = self.memory.checkpoint.save(
            &agent_id,
            &session.id.0,
            &serde_json::Value::Null,
            &serde_json::Value::Null,
            terminal_status,
        );

        // Save session after resurrection
        let _ = self.memory.sessions.save(&session);

        let _ = result?;

        info!(agent_id = %agent_id, "Resurrector: Agent successfully resurrected");
        Ok(())
    }

    /// Resurrect all agents that have active checkpoints (i.e. crashed).
    pub async fn resurrect_all_active_agents(self: &Arc<Self>) {
        if let Ok(active_agents) = self.memory.checkpoint.list_active_agents() {
            for agent_id in active_agents {
                let k = self.clone();
                tokio::spawn(async move {
                    if let Err(e) = k.resurrect_agent(agent_id).await {
                        tracing::error!(agent_id = %agent_id, "Failed to resurrect agent: {}", e);
                    }
                });
            }
        }
    }

    /// Start background services, including the cron job executor.
    pub async fn start_background_services(self: &Arc<Self>) {
        let kernel = self.clone();
        let _data_dir = self.config.read().unwrap().data_dir.clone();
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

                // Publish presence info to event bus (The Beacon)
                let active_agents: Vec<String> = statuses
                    .iter()
                    .filter(|s| !s.unresponsive)
                    .map(|s| s.agent_id.to_string())
                    .collect();
                kernel
                    .event_bus
                    .publish(crate::event_bus::KernelEvent::Presence { active_agents });

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
                                "Agent {} marked for restart by supervisor. Resurrecting...",
                                status.name
                            );
                            let k_clone = kernel.clone();
                            let aid = status.agent_id;
                            tokio::spawn(async move {
                                if let Err(e) = k_clone.resurrect_agent(aid).await {
                                    tracing::error!("Resurrection failed for agent {aid}: {e}");
                                }
                            });
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
                                            k.delivery_handler.read().unwrap().as_ref()
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

    /// Apply hot-reload actions from a reload plan.
    pub async fn apply_hot_actions(
        &self,
        actions: &[crate::config_reload::HotAction],
    ) -> SovereignResult<()> {
        for action in actions {
            use crate::config_reload::HotAction::*;
            info!("Applying hot-reload action: {:?}", action);
            match action {
                ReloadSkills => {
                    let _config = self.config.read().unwrap();
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
                    let new_registry = sk_tools::skills::SkillRegistry::load_from_dir(skills_path);
                    let mut lock = self.skills.write().unwrap();
                    *lock = new_registry;
                    info!("Skills registry hot-reloaded.");
                }
                UpdateCronConfig => {
                    // CronScheduler config updates handled on next invocation
                    let config = self.config.read().unwrap();
                    info!(
                        "Cron configuration updated (max_jobs={}).",
                        config.max_cron_jobs
                    );
                }
                UpdateApprovalPolicy => {
                    // Safety gate policy updates handled on next invocation
                    info!("Approval policy hot-reload noted (takes effect on next check).");
                }
                ReloadMcpServers => {
                    let mcp_servers_map = {
                        let config = self.config.read().unwrap();
                        let mut map = std::collections::HashMap::new();
                        for server in &config.mcp_servers {
                            let entry = sk_types::config::McpServerEntry {
                                transport: match &server.transport {
                                    sk_types::config::McpTransportEntry::Stdio { .. } => {
                                        "stdio".to_string()
                                    }
                                    sk_types::config::McpTransportEntry::Sse { .. } => {
                                        "sse".to_string()
                                    }
                                },
                                command: match &server.transport {
                                    sk_types::config::McpTransportEntry::Stdio { command, .. } => {
                                        Some(command.clone())
                                    }
                                    _ => None,
                                },
                                args: match &server.transport {
                                    sk_types::config::McpTransportEntry::Stdio { args, .. } => {
                                        args.clone()
                                    }
                                    _ => Vec::new(),
                                },
                                env: match &server.env {
                                    sk_types::config::McpEnv::List(nodes) => nodes
                                        .iter()
                                        .map(|k| (k.clone(), std::env::var(k).unwrap_or_default()))
                                        .collect(),
                                    sk_types::config::McpEnv::Map(map) => map
                                        .iter()
                                        .map(|(k, v)| {
                                            let val = if let Some(env_name) = v.strip_prefix('$') {
                                                std::env::var(env_name).unwrap_or_default()
                                            } else {
                                                v.clone()
                                            };
                                            (k.clone(), val)
                                        })
                                        .collect(),
                                },
                                url: match &server.transport {
                                    sk_types::config::McpTransportEntry::Sse { url, .. } => {
                                        Some(url.clone())
                                    }
                                    _ => None,
                                },
                            };
                            map.insert(server.name.clone(), entry);
                        }
                        map
                    }; // config guard dropped here

                    let mut mcp = self.mcp.write().await;
                    mcp.connect_all(&mcp_servers_map).await?;
                    info!("MCP servers hot-reloaded.");
                }
                _ => {
                    warn!(
                        "Hot-reload action {:?} is partially implemented or a no-op currently.",
                        action
                    );
                }
            }
        }
        Ok(())
    }

    /// Start the API bridge server if enabled in configuration.
    pub async fn start_api_server(self: Arc<Self>) -> SovereignResult<()> {
        let addr = self.config.read().unwrap().api_listen.clone();
        crate::api::start_server(self, &addr).await
    }

    /// Shut down the kernel gracefully.
    pub async fn shutdown(&self) -> SovereignResult<()> {
        info!("Sovereign Kernel shutting down...");
        // MCP connections are dropped automatically
        Ok(())
    }
}

/// Helper to initialize the LLM driver from configuration.
async fn init_llm_driver(
    config: &sk_types::config::KernelConfig,
) -> SovereignResult<(
    Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync>,
    String,
)> {
    let dm = &config.default_model;

    println!("!!! init_llm_driver start");
    // 1. Initialize primary driver
    let primary_api_key = std::env::var(&dm.api_key_env).unwrap_or_default();
    println!("!!! creating primary driver: provider={}, model={}", dm.provider, dm.model);
    let (primary_driver, primary_model) = create_driver(
        &dm.provider,
        &dm.model,
        &primary_api_key,
        dm.base_url.as_deref(),
    )?;
    println!("!!! primary driver created");

    // 2. Initialize fallbacks
    let mut entries = vec![(primary_model.clone(), primary_driver)];

    for fallback in &config.fallback_providers {
        let fb_api_key = if !fallback.api_key_env.is_empty() {
            std::env::var(&fallback.api_key_env).unwrap_or_default()
        } else {
            String::new()
        };

        if let Ok((driver, model)) = create_driver(
            &fallback.provider,
            &fallback.model,
            &fb_api_key,
            fallback.base_url.as_deref(),
        ) {
            entries.push((model, driver));
        }
    }

    // 3. Auto-detect additional fallbacks if none were explicitly configured
    if entries.len() == 1 {
        let auto_fallbacks = [
            (
                "anthropic",
                "claude-3-5-sonnet-20241022",
                "ANTHROPIC_API_KEY",
            ),
            ("openai", "gpt-4o", "OPENAI_API_KEY"),
            ("gemini", "gemini-1.5-pro", "GEMINI_API_KEY"),
            ("groq", "llama-3.3-70b-versatile", "GROQ_API_KEY"),
        ];

        for (provider, model, env_var) in auto_fallbacks {
            println!("!!! Checking auto fallback: provider={}, env={}", provider, env_var);
            // Skip the primary provider to avoid duplicates
            if provider == dm.provider.to_lowercase() {
                println!("!!! Skipping because it's primary");
                continue;
            }
            if let Ok(api_key) = std::env::var(env_var) {
                if !api_key.is_empty() {
                    println!("!!! Creating fallback driver for provider={}", provider);
                    if let Ok((driver, model_name)) = create_driver(provider, model, &api_key, None)
                    {
                        println!("!!! Fallback driver created for model={}", model_name);
                        entries.push((model_name, driver));
                    } else {
                        println!("!!! create_driver returned error for {}", provider);
                    }
                } else {
                    println!("!!! api key empty for {}", provider);
                }
            } else {
                println!("!!! env var missing: {}", env_var);
            }
        }
    }

    info!(
        providers = entries.len(),
        "Sentinel initialized with failover chain"
    );
    let sentinel: Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync> =
        Arc::new(sk_engine::sentinel::SentinelDriver::new(entries));
    println!("!!! init_llm_driver complete");

    Ok((sentinel, primary_model))
}

/// Create a concrete LLM driver from provider details.
fn create_driver(
    provider: &str,
    model: &str,
    api_key: &str,
    base_url: Option<&str>,
) -> SovereignResult<(
    Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync>,
    String,
)> {
    if let Some(url) = base_url {
        let driver: Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync> = Arc::new(
            sk_engine::drivers::openai::OpenAIDriver::new(
                api_key.to_string(),
                url.to_string(),
                provider.to_string(),
            ),
        );
        return Ok((driver, model.to_string()));
    }

    let (driver, model_name): (
        Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync>,
        String,
    ) = match provider.to_lowercase().as_str() {
        "anthropic" => (
            Arc::new(sk_engine::drivers::anthropic::AnthropicDriver::new(
                api_key.to_string(),
                "https://api.anthropic.com".to_string(),
            )),
            model.to_string(),
        ),
        "openai" => (
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                api_key.to_string(),
                "https://api.openai.com/v1".to_string(),
                "openai".to_string(),
            )),
            model.to_string(),
        ),
        "gemini" => (
            Arc::new(sk_engine::drivers::gemini::GeminiDriver::new(
                api_key.to_string(),
                "https://generativelanguage.googleapis.com".to_string(),
            )),
            model.to_string(),
        ),
        "groq" => (
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                api_key.to_string(),
                "https://api.groq.com/openai/v1".to_string(),
                "groq".to_string(),
            )),
            model.to_string(),
        ),
        "deepseek" => (
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                api_key.to_string(),
                "https://api.deepseek.com".to_string(),
                "deepseek".to_string(),
            )),
            model.to_string(),
        ),
        "xai" | "grok" => (
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                api_key.to_string(),
                "https://api.x.ai/v1".to_string(),
                "xai".to_string(),
            )),
            model.to_string(),
        ),
        "openrouter" => (
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                api_key.to_string(),
                "https://openrouter.ai/api/v1".to_string(),
                "openrouter".to_string(),
            )),
            model.to_string(),
        ),
        "mistral" => (
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                api_key.to_string(),
                "https://api.mistral.ai/v1".to_string(),
                "mistral".to_string(),
            )),
            model.to_string(),
        ),
        "together" => (
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                api_key.to_string(),
                "https://api.together.xyz/v1".to_string(),
                "together".to_string(),
            )),
            model.to_string(),
        ),
        "perplexity" => (
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                api_key.to_string(),
                "https://api.perplexity.ai".to_string(),
                "perplexity".to_string(),
            )),
            model.to_string(),
        ),
        "nvidia" => (
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                api_key.to_string(),
                "https://integrate.api.nvidia.com/v1".to_string(),
                "nvidia".to_string(),
            )),
            model.to_string(),
        ),
        // Ollama default
        "ollama" => (
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                api_key.to_string(),
                "http://localhost:11434/v1".to_string(),
                "ollama".to_string(),
            )),
            model.to_string(),
        ),
        _ => {
            return Err(SovereignError::Config(format!(
                "Unknown LLM provider '{}' (Try setting base_url in config to use a custom OpenAI-compatible provider)",
                provider
            )));
        }
    };

    Ok((driver, model_name))
}
