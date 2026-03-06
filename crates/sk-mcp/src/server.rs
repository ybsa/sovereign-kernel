//! MCP server mode — expose kernel tools to external MCP clients.
//!
//! This allows other applications to use the Sovereign Kernel as an MCP server,
//! accessing its memory, tools, and agent capabilities over stdio.

use crate::protocol::*;
use sk_types::{SovereignError, ToolDefinition};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

/// Trait for executing tools exposed by the MCP server.
#[async_trait::async_trait]
pub trait ToolHandler: Send + Sync {
    async fn execute_tool(
        &self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<String, SovereignError>;
    fn list_tools(&self) -> Vec<ToolDefinition>;
}

/// An active MCP server listening on stdio.
pub struct McpServer {
    handler: Arc<dyn ToolHandler>,
}

impl McpServer {
    /// Create a new MCP server with the given tool handler.
    pub fn new(handler: Arc<dyn ToolHandler>) -> Self {
        Self { handler }
    }

    /// Run the server loop indefinitely on stdin/stdout.
    pub async fn run(&self) -> Result<(), SovereignError> {
        info!("Starting Sovereign MCP Server on stdio");

        let mut stdin = BufReader::new(tokio::io::stdin());
        let mut stdout = tokio::io::stdout();

        loop {
            // Read headers
            let mut content_length: Option<usize> = None;
            loop {
                let mut line = String::new();
                let bytes_read = stdin
                    .read_line(&mut line)
                    .await
                    .map_err(|e| SovereignError::McpError(format!("Failed to read stdin: {e}")))?;

                if bytes_read == 0 {
                    info!("Client disconnected (EOF)");
                    return Ok(());
                }

                let trimmed = line.trim();
                if trimmed.is_empty() {
                    break;
                }

                if let Some(len_str) = trimmed.strip_prefix("Content-Length: ") {
                    content_length = len_str.trim().parse().ok();
                }
            }

            let len = match content_length {
                Some(len) => len,
                None => {
                    error!("Missing Content-Length header");
                    continue;
                }
            };

            // Read payload
            let mut buf = vec![0u8; len];
            tokio::io::AsyncReadExt::read_exact(&mut stdin, &mut buf)
                .await
                .map_err(|e| SovereignError::McpError(format!("Failed to read body: {e}")))?;

            let body = String::from_utf8_lossy(&buf);
            debug!(len, "Received MCP message");

            // Parse request
            if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&body) {
                let response = self.handle_request(req).await;
                if let Some(resp) = response {
                    let resp_str = serde_json::to_string(&resp).unwrap();
                    let payload = format!("Content-Length: {}\r\n\r\n{}", resp_str.len(), resp_str);
                    stdout.write_all(payload.as_bytes()).await.unwrap();
                    stdout.flush().await.unwrap();
                }
            } else {
                error!("Invalid JSON-RPC request: {}", body);
            }
        }
    }

    async fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let result = match req.method.as_str() {
            "initialize" => {
                let init_res = McpInitializeResult {
                    protocol_version: "2024-11-05".into(),
                    capabilities: McpServerCapabilities {
                        tools: Some(serde_json::json!({})),
                        resources: None,
                        prompts: None,
                    },
                    server_info: Some(McpServerInfo {
                        name: "sovereign-kernel".into(),
                        version: Some(env!("CARGO_PKG_VERSION").into()),
                    }),
                };
                Some(serde_json::to_value(init_res).unwrap())
            }
            "tools/list" => {
                let tools = self.handler.list_tools();
                let mcp_tools: Vec<serde_json::Value> = tools
                    .into_iter()
                    .map(|t| {
                        serde_json::json!({
                            "name": t.name,
                            "description": t.description,
                            "inputSchema": t.input_schema,
                        })
                    })
                    .collect();
                Some(serde_json::json!({ "tools": mcp_tools }))
            }
            "tools/call" => {
                if let Some(params) = req.params {
                    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let default_args = serde_json::json!({});
                    let arguments = params.get("arguments").unwrap_or(&default_args);

                    match self.handler.execute_tool(name, arguments).await {
                        Ok(content) => Some(serde_json::json!({
                            "content": [
                                { "type": "text", "text": content }
                            ],
                            "isError": false,
                        })),
                        Err(e) => Some(serde_json::json!({
                            "content": [
                                { "type": "text", "text": e.to_string() }
                            ],
                            "isError": true,
                        })),
                    }
                } else {
                    None
                }
            }
            "notifications/initialized" => {
                // Ignore initialization notification, no response
                return None;
            }
            _ => {
                error!("Unknown method: {}", req.method);
                return None;
            }
        };

        req.id.map(|id| JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: Some(id),
            result,
            error: None,
        })
    }
}
