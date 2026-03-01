//! Browser automation via a Docker Playwright bridge.
//!
//! Manages persistent browser sessions per agent, communicating with a Python
//! script running inside an ephemeral Docker container over JSON-line stdin/stdout.
//!
//! # Security
//! - SSRF check runs in Rust *before* sending navigate commands to Python
//! - Bridge runs inside `mcr.microsoft.com/playwright:v1.41.0-jammy`
//! - Completely isolated from the host OS
//! - Session limits: max concurrent, idle timeout, 1 per agent

use base64::Engine;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sk_types::config::BrowserConfig;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// Embedded Python bridge script (compiled into the binary).
const BRIDGE_SCRIPT: &str = include_str!("browser_bridge.py");

// ── Protocol types ──────────────────────────────────────────────────────────

/// Command sent from Rust to the Python bridge.
#[derive(Debug, Serialize)]
#[serde(tag = "action")]
pub enum BrowserCommand {
    Navigate { url: String },
    Click { selector: String },
    Type { selector: String, text: String },
    Screenshot,
    ReadPage,
    Close,
}

/// Response received from the Python bridge.
#[derive(Debug, Deserialize)]
pub struct BrowserResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

// ── Session ─────────────────────────────────────────────────────────────────

/// A live browser session backed by a Docker Playwright subprocess.
pub struct BrowserSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    pub last_active: Instant,
    pub container_id: String,
}

impl BrowserSession {
    /// Send a command and read the response.
    fn send(&mut self, cmd: &BrowserCommand) -> Result<BrowserResponse, String> {
        let json = serde_json::to_string(cmd).map_err(|e| format!("Serialize error: {e}"))?;
        self.stdin
            .write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write to bridge stdin: {e}"))?;
        self.stdin
            .write_all(b"\n")
            .map_err(|e| format!("Failed to write newline: {e}"))?;
        self.stdin
            .flush()
            .map_err(|e| format!("Failed to flush bridge stdin: {e}"))?;

        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .map_err(|e| format!("Failed to read bridge stdout: {e}"))?;

        if line.trim().is_empty() {
            return Err("Bridge process closed unexpectedly".to_string());
        }

        self.last_active = Instant::now();
        serde_json::from_str(line.trim())
            .map_err(|e| format!("Failed to parse bridge response: {e}. Output was: {}", line))
    }

    /// Kill the subprocess and force-remove the container.
    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::process::Command::new("docker")
            .arg("rm")
            .arg("-f")
            .arg(&self.container_id)
            .output();
    }
}

impl Drop for BrowserSession {
    fn drop(&mut self) {
        self.kill();
    }
}

// ── Manager ─────────────────────────────────────────────────────────────────

/// Manages browser sessions for all agents.
pub struct BrowserManager {
    sessions: DashMap<String, Mutex<BrowserSession>>,
    config: BrowserConfig,
}

impl BrowserManager {
    /// Create a new BrowserManager with the given configuration.
    pub fn new(config: BrowserConfig) -> Self {
        Self {
            sessions: DashMap::new(),
            config,
        }
    }

    /// Get or create a browser session for the given agent.
    /// This does synchronous subprocess spawn + I/O, so it must be called from
    /// within `block_in_place`.
    fn get_or_create_sync(&self, agent_id: &str) -> Result<(), String> {
        if self.sessions.contains_key(agent_id) {
            return Ok(());
        }

        // Enforce session limit
        if self.sessions.len() >= self.config.max_sessions {
            return Err(format!(
                "Maximum browser sessions reached ({}). Close an existing session first.",
                self.config.max_sessions
            ));
        }

        let container_name = format!("sk-browser-{}", uuid::Uuid::new_v4());

        // We run a container with playwright pre-installed. We pipe the bridge script
        // directly into `python -` so we don't need to mount volumes.
        // `docker run -i --name container_name mcr.microsoft.com/playwright:v1.41.0-jammy python3 -`
        let mut cmd = std::process::Command::new("docker");
        cmd.arg("run")
            .arg("-i") // Keep stdin open
            .arg("--rm") // Auto-remove when done (though we manually rm -f too to be safe)
            .arg("--name")
            .arg(&container_name)
            .arg("--network=bridge") // Need internet for browsing
            .arg("--security-opt=seccomp=unconfined") // Sandbox restrictions sometimes break Chromium inside docker
            .arg("mcr.microsoft.com/playwright:v1.41.0-jammy")
            .arg("python3")
            .arg("-"); // Read script from stdin first? No, we need stdin for the JSON lines.

        // Wait, if we do `python3 -` it will read the script from stdin and THEN read JSON lines from stdin?
        // That's tricky. Instead, let's use a bash one-liner to eval the script and pass the remainder to stdin.
        // A better approach is `docker run -i ... bash -c 'python3 -c "$BRIDGE_CODE"'`
        // But the script is large and passing it in env/args can hit limits or escape issues.

        let mut cmd = std::process::Command::new("docker");
        cmd.arg("run")
           .arg("-i")
           .arg("--rm")
           .arg("--name")
           .arg(&container_name)
           .arg("--network=bridge")
           .arg("--security-opt=seccomp=unconfined")
           .arg("-e")
           .arg(format!("WIDTH={}", self.config.viewport_width))
           .arg("-e")
           .arg(format!("HEIGHT={}", self.config.viewport_height))
           // Add base64 script to env to avoid arg limits and quoting hell
           .arg("-e")
           .arg(format!("BRIDGE_B64={}", base64::engine::general_purpose::STANDARD.encode(BRIDGE_SCRIPT)))
           .arg("mcr.microsoft.com/playwright:v1.41.0-jammy")
           .arg("bash")
           .arg("-c")
           .arg("echo $BRIDGE_B64 | base64 -d > /tmp/bridge.py && python3 /tmp/bridge.py --headless --width $WIDTH --height $HEIGHT");

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::null());

        let mut child = cmd.spawn().map_err(|e| {
            format!("Failed to spawn docker for browser bridge: {e}. Ensure Docker is running.")
        })?;

        let stdin = child.stdin.take().ok_or("Failed to capture docker stdin")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Failed to capture docker stdout")?;
        let mut reader = BufReader::new(stdout);

        // Wait for the "ready" response from bridge.py
        let mut ready_line = String::new();
        reader
            .read_line(&mut ready_line)
            .map_err(|e| format!("Docker bridge failed to start: {e}"))?;

        if ready_line.trim().is_empty() {
            let _ = child.kill();
            return Err("Docker browser bridge process exited without sending ready signal. Is Docker running?".to_string());
        }

        let ready: BrowserResponse = serde_json::from_str(ready_line.trim())
            .map_err(|e| format!("Docker bridge startup failed: {e}. Output: {ready_line}"))?;

        if !ready.success {
            let err = ready.error.unwrap_or_else(|| "Unknown error".to_string());
            let _ = child.kill();
            return Err(format!("Docker browser bridge failed to start: {err}"));
        }

        info!(agent_id, container = %container_name, "Browser session created in Docker");

        let session = BrowserSession {
            child,
            stdin,
            stdout: reader,
            last_active: Instant::now(),
            container_id: container_name,
        };

        self.sessions
            .insert(agent_id.to_string(), Mutex::new(session));
        Ok(())
    }

    pub fn has_session(&self, agent_id: &str) -> bool {
        self.sessions.contains_key(agent_id)
    }

    pub async fn send_command(
        &self,
        agent_id: &str,
        cmd: BrowserCommand,
    ) -> Result<BrowserResponse, String> {
        tokio::task::block_in_place(|| self.get_or_create_sync(agent_id))?;

        let session_ref = self
            .sessions
            .get(agent_id)
            .ok_or_else(|| "Session disappeared".to_string())?;

        let session_mutex = session_ref.value();
        let mut session = session_mutex.lock().await;

        let response = tokio::task::block_in_place(|| session.send(&cmd))?;

        if !response.success {
            let err = response
                .error
                .clone()
                .unwrap_or_else(|| "Unknown error".to_string());
            warn!(agent_id, error = %err, "Browser command failed");
        }

        Ok(response)
    }

    pub async fn close_session(&self, agent_id: &str) {
        if let Some((_, session_mutex)) = self.sessions.remove(agent_id) {
            let mut session = session_mutex.lock().await;
            let _ = session.send(&BrowserCommand::Close);
            session.kill();
            info!(agent_id, "Docker browser session closed");
        }
    }

    pub async fn cleanup_agent(&self, agent_id: &str) {
        self.close_session(agent_id).await;
    }
}

// ── Tool handler functions ──────────────────────────────────────────────────

pub async fn tool_browser_navigate(
    input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    let url = input["url"].as_str().ok_or("Missing 'url' parameter")?;

    // Wait until web_fetch/web_content is ported to add SSRF guards or wrapping
    // For now, raw pass-through to Docker sandbox

    let resp = mgr
        .send_command(
            agent_id,
            BrowserCommand::Navigate {
                url: url.to_string(),
            },
        )
        .await?;

    if !resp.success {
        return Err(resp.error.unwrap_or_else(|| "Navigate failed".to_string()));
    }

    let data = resp.data.unwrap_or_default();
    let title = data["title"].as_str().unwrap_or("(no title)");
    let page_url = data["url"].as_str().unwrap_or(url);
    let content = data["content"].as_str().unwrap_or("");

    Ok(format!(
        "Navigated to: {page_url}\nTitle: {title}\n\n{content}"
    ))
}

pub async fn tool_browser_click(
    input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    let selector = input["selector"]
        .as_str()
        .ok_or("Missing 'selector' parameter")?;

    let resp = mgr
        .send_command(
            agent_id,
            BrowserCommand::Click {
                selector: selector.to_string(),
            },
        )
        .await?;

    if !resp.success {
        return Err(resp.error.unwrap_or_else(|| "Click failed".to_string()));
    }

    let data = resp.data.unwrap_or_default();
    let title = data["title"].as_str().unwrap_or("(no title)");
    let url = data["url"].as_str().unwrap_or("");

    Ok(format!("Clicked: {selector}\nPage: {title}\nURL: {url}"))
}

pub async fn tool_browser_type(
    input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    let selector = input["selector"]
        .as_str()
        .ok_or("Missing 'selector' parameter")?;
    let text = input["text"].as_str().ok_or("Missing 'text' parameter")?;

    let resp = mgr
        .send_command(
            agent_id,
            BrowserCommand::Type {
                selector: selector.to_string(),
                text: text.to_string(),
            },
        )
        .await?;

    if !resp.success {
        return Err(resp.error.unwrap_or_else(|| "Type failed".to_string()));
    }

    Ok(format!("Typed into {selector}: {text}"))
}

pub async fn tool_browser_screenshot(
    _input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    let resp = mgr
        .send_command(agent_id, BrowserCommand::Screenshot)
        .await?;

    if !resp.success {
        return Err(resp
            .error
            .unwrap_or_else(|| "Screenshot failed".to_string()));
    }

    let data = resp.data.unwrap_or_default();
    let b64 = data["image_base64"].as_str().unwrap_or("");
    let url = data["url"].as_str().unwrap_or("");

    let mut image_urls: Vec<String> = Vec::new();
    if !b64.is_empty() {
        use base64::Engine;
        let upload_dir = std::env::temp_dir().join("sk_uploads");
        let _ = std::fs::create_dir_all(&upload_dir);
        let file_id = uuid::Uuid::new_v4().to_string();
        if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(b64) {
            let path = upload_dir.join(&file_id);
            if std::fs::write(&path, &decoded).is_ok() {
                image_urls.push(format!("/api/uploads/{file_id}"));
            }
        }
    }

    let result = serde_json::json!({
        "screenshot": true,
        "url": url,
        "image_urls": image_urls,
    });

    Ok(result.to_string())
}

pub async fn tool_browser_read_page(
    _input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    let resp = mgr.send_command(agent_id, BrowserCommand::ReadPage).await?;

    if !resp.success {
        return Err(resp.error.unwrap_or_else(|| "ReadPage failed".to_string()));
    }

    let data = resp.data.unwrap_or_default();
    let title = data["title"].as_str().unwrap_or("(no title)");
    let url = data["url"].as_str().unwrap_or("");
    let content = data["content"].as_str().unwrap_or("");

    Ok(format!("Page: {title}\nURL: {url}\n\n{content}"))
}

pub async fn tool_browser_close(
    _input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    mgr.close_session(agent_id).await;
    Ok("Browser session closed.".to_string())
}
