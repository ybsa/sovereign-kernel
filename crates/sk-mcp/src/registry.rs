//! MCP server registry — manage multiple MCP server connections.

use crate::client::McpClient;
use crate::transport::McpTransport;
use sk_types::config::McpServerEntry;
use sk_types::{SovereignError, ToolDefinition};
use std::collections::HashMap;
use tracing::{info, warn};

/// Registry of active MCP server connections.
pub struct McpRegistry {
    clients: HashMap<String, McpClient>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    /// Connect to all configured MCP servers.
    pub async fn connect_all(
        &mut self,
        servers: &HashMap<String, McpServerEntry>,
    ) -> Result<(), SovereignError> {
        for (name, entry) in servers {
            match self.connect_one(name, entry).await {
                Ok(_) => info!(server = %name, "MCP server connected"),
                Err(e) => warn!(server = %name, error = %e, "Failed to connect MCP server"),
            }
        }
        Ok(())
    }

    /// Connect to a single MCP server.
    async fn connect_one(
        &mut self,
        name: &str,
        entry: &McpServerEntry,
    ) -> Result<(), SovereignError> {
        let transport = match entry.transport.as_str() {
            "stdio" => {
                let command = entry.command.as_deref().ok_or_else(|| {
                    SovereignError::McpError(format!("MCP server '{name}': missing command"))
                })?;
                McpTransport::spawn_stdio(command, &entry.args, &entry.env).await?
            }
            "sse" => {
                let url = entry.url.as_deref().ok_or_else(|| {
                    SovereignError::McpError(format!("MCP server '{name}': missing url"))
                })?;
                McpTransport::sse(url)
            }
            other => {
                return Err(SovereignError::McpError(format!(
                    "Unknown MCP transport type: {other}"
                )));
            }
        };

        let client = McpClient::connect(name, transport).await?;
        self.clients.insert(name.to_string(), client);
        Ok(())
    }

    /// Get all tool definitions from all connected servers.
    pub fn all_tools(&self) -> Vec<ToolDefinition> {
        self.clients
            .values()
            .flat_map(|c| c.tools().iter().cloned())
            .collect()
    }

    /// Call a tool by its namespaced name (mcp_{server}_{tool}).
    pub async fn call_tool(
        &mut self,
        namespaced_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<String, SovereignError> {
        // Find which server owns this tool
        let server_name = self
            .clients
            .iter()
            .find(|(_, client)| client.tools().iter().any(|t| t.name == namespaced_name))
            .map(|(name, _)| name.clone())
            .ok_or_else(|| {
                SovereignError::McpError(format!("No MCP server owns tool: {namespaced_name}"))
            })?;

        let client = self.clients.get_mut(&server_name).ok_or_else(|| {
            SovereignError::McpError(format!("Server not connected: {server_name}"))
        })?;

        client.call_tool(namespaced_name, arguments).await
    }

    /// Check if a tool name belongs to an MCP server.
    pub fn is_mcp_tool(&self, tool_name: &str) -> bool {
        tool_name.starts_with("mcp_")
    }

    /// Get the number of connected servers.
    pub fn server_count(&self) -> usize {
        self.clients.len()
    }

    /// Get the total number of available MCP tools.
    pub fn tool_count(&self) -> usize {
        self.clients.values().map(|c| c.tools().len()).sum()
    }

    /// List all connected servers and their tool counts.
    pub fn list_servers(&self) -> Vec<(String, usize)> {
        self.clients
            .iter()
            .map(|(name, client)| (name.clone(), client.tools().len()))
            .collect()
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}
