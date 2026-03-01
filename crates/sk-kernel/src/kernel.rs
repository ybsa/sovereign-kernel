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
        let soul = if let Some(ref soul_path) = config.soul_path {
            SoulIdentity::load(soul_path)?
        } else {
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
        let db_path = config.db_path();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SovereignError::ConfigError(format!("Failed to create data dir: {e}"))
            })?;
        }
        let memory = Arc::new(MemorySubstrate::open(&db_path, config.memory_decay_rate)?);
        info!(path = %db_path.display(), "Memory substrate opened");

        // Connect MCP servers
        let mut mcp = McpRegistry::new();
        mcp.connect_all(&config.mcp_servers).await?;
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

    /// Shut down the kernel gracefully.
    pub async fn shutdown(&self) -> SovereignResult<()> {
        info!("Sovereign Kernel shutting down...");
        // MCP connections are dropped automatically
        Ok(())
    }
}
