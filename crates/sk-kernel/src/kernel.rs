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
    /// Global approval manager for interactive permission requests.
    pub approval: Arc<crate::approval::ApprovalManager>,
    /// Global embedding driver.
    pub embedding: Arc<dyn sk_memory::embedding::EmbeddingDriver>,
    /// Global LLM driver.
    pub driver: Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync>,
    /// Global LLM model name.
    pub model_name: String,
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
            mcp_servers_map.insert(server.name.clone(), create_mcp_entry(server));
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

        let mut metering = crate::metering::MeteringEngine::new();
        metering.set_persist_path(config.data_dir.join("metering.json"));
        metering.load().await.ok(); // Ignore errors if file doesn't exist yet
        let metering = Arc::new(metering);

        // Initialize Approval Manager with default policy
        let approval = Arc::new(crate::approval::ApprovalManager::new(
            sk_types::approval::ApprovalPolicy::default(),
        ));

        // Initialize Embedding Driver from configuration
        let embedding = match init_embedding_driver(&config).await {
            Ok(driver) => driver,
            Err(e) => {
                warn!("Failed to initialize embedding driver: {e}. Falling back to default.");
                Arc::new(sk_memory::embedding::OpenAIEmbeddingDriver::new(
                    "".into(),
                    "https://api.openai.com/v1".into(),
                    "text-embedding-3-small".into(),
                    1536,
                ))
            }
        };

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
            driver,
            model_name,
            skills,
            bus,
            agents,
            event_bus,
            cron,
            supervisor,
            delivery_handler: std::sync::RwLock::new(None),
            metering,
            approval,
            embedding,
            hands,
            active_loops: Arc::new(DashMap::new()),
        })
    }

    /// Set the global delivery handler for cron jobs.
    pub async fn set_delivery_handler(
        &self,
        handler: Arc<dyn sk_types::scheduler::CronDeliveryHandler>,
    ) {
        let mut lock = wlock!(self.delivery_handler);
        *lock = Some(handler);
    }

    /// Run a complete agent loop for a given session and user input.
    pub async fn run_agent(
        self: &Arc<Self>,
        session: &mut sk_types::Session,
        input: &str,
        stream_handler: Option<sk_engine::agent_loop::StreamHandler>,
    ) -> SovereignResult<sk_engine::agent_loop::AgentLoopResult> {
        let system_prompt = self.soul.to_system_prompt_fragment();

        let agent_config = crate::executor::create_agent_config(
            self.clone(),
            self.driver.clone(),
            system_prompt,
            self.model_name.clone(),
            session.agent_id,
            self.skills.clone(),
            stream_handler,
            None,
        );

        let aid = session.agent_id;
        let token = CancellationToken::new();
        self.active_loops.insert(aid, token.clone());
        self.event_bus
            .publish(crate::event_bus::KernelEvent::AgentStarted {
                agent_id: aid.to_string(),
            });

        let result = tokio::select! {
            res = sk_engine::agent_loop::run_agent_loop(agent_config, session, input) => res,
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
            self.skills.clone(),
            None,
            None,
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
        let _data_dir = self.config.read().expect("lock poisoned").data_dir.clone();
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

                                match k.run_agent(&mut s, &msg, None).await {
                                    Ok(res) => {
                                        k.cron.record_success(job_id);
                                        // Execute delivery
                                        if let Some(handler) = k
                                            .delivery_handler
                                            .read()
                                            .expect("lock poisoned")
                                            .as_ref()
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
                    let _config = self.config.read().expect("lock poisoned");
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
                    let mut lock = self.skills.write().expect("lock poisoned");
                    *lock = new_registry;
                    info!("Skills registry hot-reloaded.");
                }
                UpdateCronConfig => {
                    // CronScheduler config updates handled on next invocation
                    let config = self.config.read().expect("lock poisoned");
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
                        let config = self.config.read().expect("lock poisoned");
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
                                    sk_types::config::McpTransportEntry::Stdio {
                                        command, ..
                                    } => Some(command.clone()),
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
        Ok(())
    }

    /// Shut down the kernel gracefully.
    pub async fn shutdown(&self) -> SovereignResult<()> {
        info!("Sovereign Kernel shutting down...");
        // MCP connections are dropped automatically
        Ok(())
    }

    /// Compile a Rust skill in a Docker-isolated sandbox.
    pub async fn compile_skill(
        &self,
        skill_name: &str,
        description: &str,
        code: &str,
        dependencies_toml: &str,
        instructions: &str,
    ) -> SovereignResult<String> {
        let temp_id = uuid::Uuid::new_v4().to_string();
        let data_dir = self.config.read().expect("lock poisoned").data_dir.clone();

        let temp_abs_dir = data_dir.join("temp_compile").join(&temp_id);
        std::fs::create_dir_all(&temp_abs_dir)
            .map_err(|e| SovereignError::Internal(format!("Failed to create temp dir: {}", e)))?;

        // 1. Create Cargo.toml
        let cargo_toml = format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
{}
"#,
            skill_name, dependencies_toml
        );
        std::fs::write(temp_abs_dir.join("Cargo.toml"), cargo_toml)
            .map_err(|e| SovereignError::Internal(format!("Failed to write Cargo.toml: {}", e)))?;

        // 2. Create src/main.rs
        let src_dir = temp_abs_dir.join("src");
        std::fs::create_dir_all(&src_dir)
            .map_err(|e| SovereignError::Internal(format!("Failed to create src dir: {}", e)))?;
        std::fs::write(src_dir.join("main.rs"), code)
            .map_err(|e| SovereignError::Internal(format!("Failed to write main.rs: {}", e)))?;

        // 3. Compile via local cargo
        info!(skill = %skill_name, "Compiling Otto skill locally...");

        let output = tokio::process::Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&temp_abs_dir)
            .output()
            .await
            .map_err(|e| SovereignError::Internal(format!("Failed to run cargo: {}", e)))?;

        if !output.status.success() {
            return Err(SovereignError::Internal(format!(
                "Compilation failed:\nSTDOUT: {}\nSTDERR: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // 4. Move binary to skills directory
        let binary_name = if cfg!(windows) {
            format!("{}.exe", skill_name)
        } else {
            skill_name.to_string()
        };

        let binary_src = temp_abs_dir
            .join("target")
            .join("release")
            .join(&binary_name);

        let skills_dir = self.skills.read().expect("lock poisoned").dir.clone();

        let target_skill_dir = skills_dir.join(skill_name);
        std::fs::create_dir_all(&target_skill_dir).map_err(|e| {
            SovereignError::Internal(format!("Failed to create target skill dir: {}", e))
        })?;

        let binary_dst = target_skill_dir.join(&binary_name);
        std::fs::copy(&binary_src, &binary_dst)
            .map_err(|e| SovereignError::Internal(format!("Failed to copy binary: {}", e)))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(mut perms) = std::fs::metadata(&binary_dst).map(|m| m.permissions()) {
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&binary_dst, perms);
            }
        }

        // 5. Write SKILL.md
        let skill_md_content = format!(
            "---\nname: {}\ndescription: {}\nmetadata:\n  compiled: true\n---\n{}",
            skill_name, description, instructions
        );
        std::fs::write(target_skill_dir.join("SKILL.md"), skill_md_content)
            .map_err(|e| SovereignError::Internal(format!("Failed to write SKILL.md: {}", e)))?;

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_abs_dir);

        // 6. Reload Registry
        let _lock = self.skills.write().expect("lock poisoned");

        Ok(format!("Successfully compiled skill '{}'", skill_name))
    }
}

async fn init_llm_driver(
    config: &sk_types::config::KernelConfig,
) -> SovereignResult<(
    Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync>,
    String,
)> {
    tracing::debug!("Initializing LLM drivers...");

    // 1. Resolve providers (the 'llm' vector now handles legacy formats via Serde)
    let providers = config.llm.clone();
    if providers.is_empty() {
        return Err(SovereignError::Config(
            "No LLM providers configured. Please add an [[llm]] block to your config.toml."
                .to_string(),
        ));
    }

    let mut entries = Vec::new();
    let mut primary_model = String::new();

    for (i, spec) in providers.iter().enumerate() {
        let api_key = match spec.resolve_api_key() {
            Ok(key) => key,
            Err(e) => {
                if i == 0 {
                    // Fail early if the primary provider has no key
                    return Err(e);
                } else {
                    tracing::debug!("Skipping fallback provider '{}': {}", spec.provider, e);
                    continue;
                }
            }
        };

        let model = spec.model.as_deref().unwrap_or("default");
        match create_driver(&spec.provider, model, &api_key, spec.base_url.as_deref()) {
            Ok((driver, model_name)) => {
                if i == 0 {
                    primary_model = model_name.clone();
                }
                entries.push((model_name, driver));
            }
            Err(e) => {
                if i == 0 {
                    return Err(e);
                }
                warn!(
                    "Failed to initialize fallback provider '{}': {}",
                    spec.provider, e
                );
            }
        }
    }

    // 2. Auto-detect additional fallbacks if only one (the primary) was explicitly configured
    if entries.len() == 1 {
        let primary_provider = providers[0].provider.to_lowercase();
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
            // Skip the primary provider to avoid duplicates
            if provider == primary_provider {
                continue;
            }
            if let Ok(api_key) = std::env::var(env_var) {
                if !api_key.is_empty() {
                    if let Ok((driver, model_name)) = create_driver(provider, model, &api_key, None)
                    {
                        tracing::debug!("Auto-detected fallback provider: {}", provider);
                        entries.push((model_name, driver));
                    }
                }
            }
        }
    }

    info!(
        providers = entries.len(),
        "Sentinel initialized with {} active provider(s)",
        entries.len()
    );

    let sentinel: Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync> =
        Arc::new(sk_engine::sentinel::SentinelDriver::new(entries));

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
        let driver: Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync> =
            Arc::new(sk_engine::drivers::openai::OpenAIDriver::new(
                api_key.to_string(),
                url.to_string(),
                provider.to_string(),
            ));
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

/// Initialize embedding driver from config.
async fn init_embedding_driver(
    config: &KernelConfig,
) -> SovereignResult<Arc<dyn sk_memory::embedding::EmbeddingDriver>> {
    let model = if config.memory.embedding_model.is_empty() {
        "text-embedding-3-small".to_string()
    } else {
        config.memory.embedding_model.clone()
    };

    // Find a provider that matches or default to OpenAI
    let provider = config
        .llm
        .iter()
        .find(|p| p.provider == "openai" || p.base_url.is_some())
        .cloned()
        .unwrap_or_default();

    let key = provider
        .api_key
        .clone()
        .or_else(|| {
            provider
                .api_key_env
                .as_ref()
                .and_then(|var| std::env::var(var).ok())
        })
        .unwrap_or_default();

    let base = provider
        .base_url
        .clone()
        .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

    // Currently only OpenAI/compatible embedding drivers are implemented in sk-memory
    Ok(Arc::new(sk_memory::embedding::OpenAIEmbeddingDriver::new(
        key, base, model, 1536,
    )))
}

/// Helper to convert config server into MCP entry.
fn create_mcp_entry(
    server: &sk_types::config::McpServerConfigEntry,
) -> sk_types::config::McpServerEntry {
    sk_types::config::McpServerEntry {
        transport: match &server.transport {
            sk_types::config::McpTransportEntry::Stdio { .. } => "stdio".to_string(),
            sk_types::config::McpTransportEntry::Sse { .. } => "sse".to_string(),
        },
        command: match &server.transport {
            sk_types::config::McpTransportEntry::Stdio { command, .. } => Some(command.clone()),
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
    }
}
