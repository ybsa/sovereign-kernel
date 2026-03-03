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

        Ok(Self {
            config,
            soul,
            memory,
            mcp,
        })
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
