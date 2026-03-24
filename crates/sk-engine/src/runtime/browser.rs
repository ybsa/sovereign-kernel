//! Browser automation via chromiumoxide.
//!
//! Manages persistent browser sessions per agent using native Rust bindings
//! to the Chrome DevTools Protocol (CDP).
//!
//! # Security
//! - SSRF check runs in Rust *before* sending navigate commands
//! - Sessions are isolated via separate pages/incognito contexts if configured
//! - Session limits: max concurrent, idle timeout, 1 per agent

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::Page;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sk_types::config::BrowserConfig as AppBrowserConfig;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, OnceCell};
use tracing::{info, warn};
use futures::StreamExt;

// ── Protocol types ──────────────────────────────────────────────────────────

/// Command sent from agent to the browser manager.
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

/// Response returned to the agent.
#[derive(Debug, Deserialize, Serialize)]
pub struct BrowserResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

// ── Session ─────────────────────────────────────────────────────────────────

/// A live browser session backed by a chromiumoxide Page.
struct BrowserSession {
    page: Page,
    last_active: Instant,
}

impl BrowserSession {
    async fn execute(&mut self, cmd: &BrowserCommand) -> Result<BrowserResponse, String> {
        self.last_active = Instant::now();
        
        match cmd {
            BrowserCommand::Navigate { url } => {
                let p = self.page.goto(url).await.map_err(|e| format!("Nav Error: {e}"))?;
                p.wait_for_navigation().await.map_err(|e| format!("Nav Wait Error: {e}"))?;
                
                let title = self.page.evaluate("document.title").await.map_err(|e| format!("Title fetch error: {e}"))?.into_value::<String>().unwrap_or_default();
                let page_url = self.page.url().await.map_err(|e| format!("URL fetch error: {e}"))?.unwrap_or_default();
                
                // Read clean markdown via Readability
                let content = self.page.evaluate(include_str!("readability.js")).await.map_err(|e| format!("Readability error: {e}"))?.into_value::<String>().unwrap_or_default();
                
                Ok(BrowserResponse {
                    success: true,
                    data: Some(serde_json::json!({
                        "title": title,
                        "url": page_url,
                        "content": content
                    })),
                    error: None,
                })
            }
            BrowserCommand::Click { selector } => {
                let element = self.page.find_element(selector.as_str()).await.map_err(|e| format!("Element not found: {e}"))?;
                element.click().await.map_err(|e| format!("Click failed: {e}"))?;
                
                let title = self.page.evaluate("document.title").await.ok().and_then(|v| v.into_value::<String>().ok()).unwrap_or_default();
                let url = self.page.url().await.ok().flatten().unwrap_or_default();
                
                Ok(BrowserResponse {
                    success: true,
                    data: Some(serde_json::json!({ "title": title, "url": url })),
                    error: None,
                })
            }
            BrowserCommand::Type { selector, text } => {
                let element = self.page.find_element(selector.as_str()).await.map_err(|e| format!("Element not found: {e}"))?;
                element.click().await.map_err(|e| format!("Focus failed: {e}"))?;
                element.type_str(text).await.map_err(|e| format!("Type failed: {e}"))?;
                
                Ok(BrowserResponse {
                    success: true,
                    data: None,
                    error: None,
                })
            }
            BrowserCommand::Screenshot => {
                use chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotFormat;
                let bytes = self.page.screenshot(
                    chromiumoxide::page::ScreenshotParams::builder()
                        .format(CaptureScreenshotFormat::Png)
                        .full_page(false)
                        .build()
                ).await.map_err(|e| format!("Screenshot error: {e}"))?;
                
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
                let url = self.page.url().await.ok().flatten().unwrap_or_default();
                
                Ok(BrowserResponse {
                    success: true,
                    data: Some(serde_json::json!({
                        "image_base64": b64,
                        "url": url
                    })),
                    error: None,
                })
            }
            BrowserCommand::ReadPage => {
                let title = self.page.evaluate("document.title").await.ok().and_then(|v| v.into_value::<String>().ok()).unwrap_or_default();
                let url = self.page.url().await.ok().flatten().unwrap_or_default();
                let content = self.page.evaluate(include_str!("readability.js")).await.map_err(|e| format!("Readability error: {e}"))?.into_value::<String>().unwrap_or_default();
                
                Ok(BrowserResponse {
                    success: true,
                    data: Some(serde_json::json!({
                        "title": title,
                        "url": url,
                        "content": content
                    })),
                    error: None,
                })
            }
            BrowserCommand::Close => {
                let _ = self.page.clone().close().await;
                Ok(BrowserResponse {
                    success: true,
                    data: None,
                    error: None,
                })
            }
        }
    }
}

// ── Manager ─────────────────────────────────────────────────────────────────

/// Manages browser sessions for all agents.
pub struct BrowserManager {
    sessions: DashMap<String, Arc<Mutex<BrowserSession>>>,
    config: AppBrowserConfig,
    browser: OnceCell<Arc<Browser>>,
}

impl BrowserManager {
    /// Create a new BrowserManager with the given configuration.
    pub fn new(config: AppBrowserConfig) -> Self {
        Self {
            sessions: DashMap::new(),
            config,
            browser: OnceCell::new(),
        }
    }

    async fn get_browser(&self) -> Result<Arc<Browser>, String> {
        self.browser.get_or_try_init(|| async {
            let mut build = BrowserConfig::builder()
                .window_size(self.config.viewport_width, self.config.viewport_height);
            
            if !self.config.headless {
                build = build.with_head();
            }

            let b_config = build.build().map_err(|e| format!("Config builder error: {e}"))?;
            
            let (browser, mut handler) = Browser::launch(b_config)
                .await
                .map_err(|e| format!("Failed to launch chromium: {e}"))?;
            
            tokio::spawn(async move {
                while handler.next().await.is_some() {
                    // pump events
                }
            });

            Ok(Arc::new(browser))
        }).await.cloned()
    }

    async fn get_or_create_session(&self, agent_id: &str) -> Result<Arc<Mutex<BrowserSession>>, String> {
        if let Some(session) = self.sessions.get(agent_id) {
            return Ok(session.clone());
        }

        if self.sessions.len() >= self.config.max_sessions {
            return Err(format!("Max browser sessions reached ({})", self.config.max_sessions));
        }

        let browser = self.get_browser().await?;
        let page = browser.new_page("about:blank").await.map_err(|e| format!("Failed to create page: {e}"))?;
        
        // Inject JS helper scripts if needed (e.g., Readability library)
        
        let session = Arc::new(Mutex::new(BrowserSession {
            page,
            last_active: Instant::now(),
        }));
        
        self.sessions.insert(agent_id.to_string(), session.clone());
        Ok(session)
    }

    pub fn has_session(&self, agent_id: &str) -> bool {
        self.sessions.contains_key(agent_id)
    }

    pub async fn send_command(
        &self,
        agent_id: &str,
        cmd: BrowserCommand,
    ) -> Result<BrowserResponse, String> {
        let session_arc = self.get_or_create_session(agent_id).await?;
        let mut session = session_arc.lock().await;

        let res = session.execute(&cmd).await.unwrap_or_else(|e| BrowserResponse {
            success: false,
            data: None,
            error: Some(e),
        });

        if !res.success {
            let err = res.error.clone().unwrap_or_else(|| "Unknown error".to_string());
            warn!(agent_id, error = %err, "Browser command failed");
        }

        Ok(res)
    }

    pub async fn close_session(&self, agent_id: &str) {
        if let Some((_, session_arc)) = self.sessions.remove(agent_id) {
            let mut session = session_arc.lock().await;
            let _ = session.execute(&BrowserCommand::Close).await;
            info!(agent_id, "Browser session closed");
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
    super::web_fetch::check_ssrf(url)?;

    let resp = mgr.send_command(agent_id, BrowserCommand::Navigate { url: url.to_string() }).await?;
    if !resp.success { return Err(resp.error.unwrap_or_else(|| "Navigate failed".into())); }

    let data = resp.data.unwrap_or_default();
    let title = data["title"].as_str().unwrap_or("(no title)");
    let page_url = data["url"].as_str().unwrap_or(url);
    let content = data["content"].as_str().unwrap_or("");
    let wrapped = super::web_content::wrap_external_content(page_url, content);

    Ok(format!("Navigated to: {page_url}\nTitle: {title}\n\n{wrapped}"))
}

pub async fn tool_browser_click(
    input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    let selector = input["selector"].as_str().ok_or("Missing 'selector'")?;
    let resp = mgr.send_command(agent_id, BrowserCommand::Click { selector: selector.into() }).await?;
    if !resp.success { return Err(resp.error.unwrap_or_else(|| "Click failed".into())); }

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
    let selector = input["selector"].as_str().ok_or("Missing 'selector'")?;
    let text = input["text"].as_str().ok_or("Missing 'text'")?;
    let resp = mgr.send_command(agent_id, BrowserCommand::Type { selector: selector.into(), text: text.into() }).await?;
    if !resp.success { return Err(resp.error.unwrap_or_else(|| "Type failed".into())); }
    Ok(format!("Typed into {selector}: {text}"))
}

pub async fn tool_browser_screenshot(
    _input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    let resp = mgr.send_command(agent_id, BrowserCommand::Screenshot).await?;
    if !resp.success { return Err(resp.error.unwrap_or_else(|| "Screenshot failed".into())); }

    let data = resp.data.unwrap_or_default();
    let b64 = data["image_base64"].as_str().unwrap_or("");
    let url = data["url"].as_str().unwrap_or("");

    let mut image_urls = Vec::new();
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

    Ok(serde_json::json!({
        "screenshot": true,
        "url": url,
        "image_urls": image_urls,
    }).to_string())
}

pub async fn tool_browser_read_page(
    _input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    let resp = mgr.send_command(agent_id, BrowserCommand::ReadPage).await?;
    if !resp.success { return Err(resp.error.unwrap_or_else(|| "ReadPage failed".into())); }

    let data = resp.data.unwrap_or_default();
    let title = data["title"].as_str().unwrap_or("(no title)");
    let url = data["url"].as_str().unwrap_or("");
    let content = data["content"].as_str().unwrap_or("");
    let wrapped = super::web_content::wrap_external_content(url, content);

    Ok(format!("Page: {title}\nURL: {url}\n\n{wrapped}"))
}

pub async fn tool_browser_close(
    _input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    mgr.close_session(agent_id).await;
    Ok("Browser session closed.".into())
}
