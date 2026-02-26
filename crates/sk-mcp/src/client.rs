//! MCP client — connect to MCP servers, discover tools, call them.
//!
//! Based on OpenFang's mcp.rs, enhanced with proper lifecycle management.

use crate::protocol::*;
use crate::transport::McpTransport;
use sk_types::{SovereignError, ToolDefinition};
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info};

/// An active MCP client connection.
pub struct McpClient {
    name: String,
    transport: McpTransport,
    next_id: AtomicU64,
    tools: Vec<ToolDefinition>,
    server_info: Option<McpServerInfo>,
}

impl McpClient {
    /// Connect to an MCP server, perform handshake, and discover tools.
    pub async fn connect(
        name: impl Into<String>,
        transport: McpTransport,
    ) -> Result<Self, SovereignError> {
        let name = name.into();
        let mut client = Self {
            name: name.clone(),
            transport,
            next_id: AtomicU64::new(1),
            tools: Vec::new(),
            server_info: None,
        };

        // Initialize handshake
        client.initialize().await?;

        // Discover available tools
        client.discover_tools().await?;

        info!(
            server = %name,
            tools = client.tools.len(),
            "MCP server connected"
        );
        Ok(client)
    }

    /// Send the MCP `initialize` handshake.
    async fn initialize(&mut self) -> Result<(), SovereignError> {
        let result = self
            .send_request(
                "initialize",
                Some(serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "sovereign-kernel",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                })),
            )
            .await?;

        if let Some(result) = result {
            if let Ok(init) = serde_json::from_value::<McpInitializeResult>(result) {
                self.server_info = init.server_info;
                debug!(
                    protocol = %init.protocol_version,
                    "MCP initialize handshake complete"
                );
            }
        }

        // Send initialized notification
        self.send_notification("notifications/initialized", None)
            .await?;

        Ok(())
    }

    /// Discover available tools via `tools/list`.
    async fn discover_tools(&mut self) -> Result<(), SovereignError> {
        let result = self.send_request("tools/list", None).await?;

        if let Some(result) = result {
            if let Some(tools_arr) = result.get("tools").and_then(|t| t.as_array()) {
                for tool_val in tools_arr {
                    if let Ok(info) = serde_json::from_value::<McpToolInfo>(tool_val.clone()) {
                        let namespaced = format!("mcp_{}_{}", self.name, info.name);
                        self.tools.push(ToolDefinition {
                            name: namespaced,
                            description: info.description.unwrap_or_default(),
                            parameters: info.input_schema.unwrap_or(serde_json::json!({})),
                            source: format!("mcp:{}", self.name),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Call a tool on the MCP server.
    pub async fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<String, SovereignError> {
        // Strip the mcp_{server}_ prefix to get the original tool name
        let prefix = format!("mcp_{}_", self.name);
        let original_name = tool_name.strip_prefix(&prefix).unwrap_or(tool_name);

        let result = self
            .send_request(
                "tools/call",
                Some(serde_json::json!({
                    "name": original_name,
                    "arguments": arguments,
                })),
            )
            .await?;

        match result {
            Some(value) => {
                // Extract text content from MCP response
                if let Some(content) = value.get("content").and_then(|c| c.as_array()) {
                    let texts: Vec<&str> = content
                        .iter()
                        .filter_map(|item| item.get("text").and_then(|t| t.as_str()))
                        .collect();
                    Ok(texts.join("\n"))
                } else {
                    Ok(value.to_string())
                }
            }
            None => Ok(String::new()),
        }
    }

    /// Get the discovered tool definitions.
    pub fn tools(&self) -> &[ToolDefinition] {
        &self.tools
    }

    /// Get the server name.
    pub fn name(&self) -> &str {
        &self.name
    }

    // --- Internal helpers ---

    async fn send_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<Option<serde_json::Value>, SovereignError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest::new(id, method, params);
        let msg = serde_json::to_string(&request)?;

        self.transport.send(&msg).await?;
        let response_str = self.transport.recv().await?;
        let response: JsonRpcResponse = serde_json::from_str(&response_str)
            .map_err(|e| SovereignError::McpError(format!("Invalid response: {e}")))?;

        if let Some(err) = response.error {
            return Err(SovereignError::McpError(err.to_string()));
        }

        Ok(response.result)
    }

    async fn send_notification(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<(), SovereignError> {
        let notification = JsonRpcRequest::notification(method, params);
        let msg = serde_json::to_string(&notification)?;
        self.transport.send(&msg).await
    }
}
