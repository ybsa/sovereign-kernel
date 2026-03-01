//! Telegram Adapter.
//!
//! Ports the OpenClaw telegram adapter to a native Rust polling loop.
//! Acts as an ingestion channel for the Sovereign Kernel.

use sk_types::{AgentId, SovereignError};
use std::time::Duration;
use tracing::{debug, info};

/// Configuration for the Telegram connector.
pub struct TelegramConfig {
    pub bot_token: String,
    pub allowed_users: Vec<String>,
}

/// A handle to the Telegram loop.
pub struct TelegramConnector {
    config: TelegramConfig,
}

impl TelegramConnector {
    pub fn new(config: TelegramConfig) -> Self {
        Self { config }
    }

    /// Run the polling loop indefinitely.
    pub async fn run(&self, _agent_id: AgentId) -> Result<(), SovereignError> {
        info!("Starting native Telegram connector loop");

        let client = reqwest::Client::new();
        let mut offset = 0;

        loop {
            // Long-polling updates from Telegram API
            let url = format!(
                "https://api.telegram.org/bot{}/getUpdates?offset={}&timeout=30",
                self.config.bot_token, offset
            );

            match client.get(&url).send().await {
                Ok(resp) => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(arr) = json.get("result").and_then(|r| r.as_array()) {
                            for update in arr {
                                if let Some(update_id) =
                                    update.get("update_id").and_then(|id| id.as_i64())
                                {
                                    offset = update_id + 1;
                                }

                                if let Some(msg) = update.get("message") {
                                    let text =
                                        msg.get("text").and_then(|t| t.as_str()).unwrap_or("");
                                    let chat_id = msg
                                        .get("chat")
                                        .and_then(|c| c.get("id"))
                                        .and_then(|id| id.as_i64())
                                        .unwrap_or(0);

                                    debug!(chat_id, "Received Telegram message: {}", text);

                                    // TODO: Pass into agent_loop and send reply back via sendMessage
                                    // This requires a handle to the Kernel or an EventBus.
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Telegram polling error: {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}
