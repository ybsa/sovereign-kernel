//! MCP transport abstractions — stdio and SSE.

use sk_types::SovereignError;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tracing::{debug, warn};

/// Transport handle — abstraction over stdio subprocess or HTTP.
pub enum McpTransport {
    /// Subprocess with JSON-RPC over stdin/stdout.
    Stdio {
        child: Child,
        stdin: ChildStdin,
        stdout: BufReader<ChildStdout>,
    },
    /// HTTP Server-Sent Events.
    Sse {
        client: reqwest::Client,
        url: String,
    },
}

impl McpTransport {
    /// Send a JSON-RPC message over the transport.
    pub async fn send(&mut self, message: &str) -> Result<(), SovereignError> {
        match self {
            McpTransport::Stdio { stdin, .. } => {
                let msg = format!("Content-Length: {}\r\n\r\n{}", message.len(), message);
                stdin.write_all(msg.as_bytes()).await.map_err(|e| {
                    SovereignError::McpError(format!("Failed to write to stdin: {e}"))
                })?;
                stdin
                    .flush()
                    .await
                    .map_err(|e| SovereignError::McpError(format!("Failed to flush stdin: {e}")))?;
                debug!(len = message.len(), "Sent MCP message via stdio");
                Ok(())
            }
            McpTransport::Sse { client, url } => {
                client
                    .post(url.as_str())
                    .header("Content-Type", "application/json")
                    .body(message.to_string())
                    .send()
                    .await
                    .map_err(|e| SovereignError::McpError(format!("SSE POST failed: {e}")))?;
                debug!(len = message.len(), "Sent MCP message via SSE");
                Ok(())
            }
        }
    }

    /// Read a JSON-RPC response from the transport.
    pub async fn recv(&mut self) -> Result<String, SovereignError> {
        match self {
            McpTransport::Stdio { stdout, .. } => {
                // Read headers until empty line
                let mut content_length: Option<usize> = None;
                loop {
                    let mut line = String::new();
                    stdout.read_line(&mut line).await.map_err(|e| {
                        SovereignError::McpError(format!("Failed to read from stdout: {e}"))
                    })?;
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        break;
                    }
                    if let Some(len_str) = trimmed.strip_prefix("Content-Length: ") {
                        content_length = len_str.trim().parse().ok();
                    }
                }

                let len = content_length.ok_or_else(|| {
                    SovereignError::McpError("Missing Content-Length header".into())
                })?;

                let mut buf = vec![0u8; len];
                stdout
                    .read_exact(&mut buf)
                    .await
                    .map_err(|e| SovereignError::McpError(format!("Failed to read body: {e}")))?;

                let body = String::from_utf8(buf).map_err(|e| {
                    SovereignError::McpError(format!("Invalid UTF-8 response: {e}"))
                })?;
                debug!(len, "Received MCP response via stdio");
                Ok(body)
            }
            McpTransport::Sse { .. } => {
                // SSE responses come via the event stream — simplified here
                Err(SovereignError::McpError(
                    "SSE recv not yet implemented — use stdio transport".into(),
                ))
            }
        }
    }

    /// Spawn a stdio subprocess transport.
    pub async fn spawn_stdio(
        command: &str,
        args: &[String],
        env: &std::collections::HashMap<String, String>,
    ) -> Result<Self, SovereignError> {
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());

        // Only pass explicitly declared env vars (security: no leaking secrets)
        cmd.env_clear();
        for (key, value) in env {
            cmd.env(key, value);
        }

        // Inherit PATH and common system vars
        if let Ok(path) = std::env::var("PATH") {
            cmd.env("PATH", path);
        }
        if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            cmd.env("HOME", &home);
            cmd.env("USERPROFILE", &home);
        }

        let mut child = cmd.spawn().map_err(|e| {
            SovereignError::McpError(format!("Failed to spawn MCP server '{command}': {e}"))
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            SovereignError::McpError("Failed to open stdin for MCP server".into())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            SovereignError::McpError("Failed to open stdout for MCP server".into())
        })?;

        Ok(McpTransport::Stdio {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    /// Create an SSE transport.
    pub fn sse(url: impl Into<String>) -> Self {
        McpTransport::Sse {
            client: reqwest::Client::new(),
            url: url.into(),
        }
    }
}

impl Drop for McpTransport {
    fn drop(&mut self) {
        if let McpTransport::Stdio { child, .. } = self {
            // Try to kill the subprocess on drop
            if let Err(e) = child.start_kill() {
                warn!("Failed to kill MCP subprocess: {e}");
            }
        }
    }
}
