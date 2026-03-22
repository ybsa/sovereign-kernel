//! Slack Socket Mode channel adapter.

use crate::types::{
    ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser, LifecycleReaction,
};
use async_trait::async_trait;
use futures::{SinkExt, Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

/// Response from apps.connections.open
#[derive(Deserialize, Debug)]
struct ConnectionsOpenResponse {
    ok: bool,
    url: Option<String>,
    error: Option<String>,
}

/// Socket Mode envelope wrapper
#[derive(Deserialize, Debug)]
struct SlackEnvelope {
    envelope_id: String,
    #[serde(rename = "type")]
    msg_type: String,
    payload: Option<serde_json::Value>,
}

/// Acknowledgment to send back to Slack
#[derive(Serialize)]
struct SlackAck {
    envelope_id: String,
}

pub struct SlackAdapter {
    app_token: String,
    bot_token: String,
    allowed_channels: Vec<String>,
    client: Client,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
}

impl SlackAdapter {
    pub fn new(app_token: String, bot_token: String, allowed_channels: Vec<String>) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            app_token,
            bot_token,
            allowed_channels,
            client: Client::new(),
            shutdown_tx,
            shutdown_rx,
        }
    }

    async fn get_ws_url(&self) -> Result<String, Box<dyn std::error::Error>> {
        let res: ConnectionsOpenResponse = self
            .client
            .post("https://slack.com/api/apps.connections.open")
            .header("Authorization", format!("Bearer {}", self.app_token))
            .send()
            .await?
            .json()
            .await?;

        if res.ok {
            if let Some(url) = res.url {
                return Ok(url);
            }
        }
        Err(format!("Failed to get Slack WS URL. {:?}", res.error).into())
    }
}

#[async_trait]
impl ChannelAdapter for SlackAdapter {
    fn name(&self) -> &str {
        "Slack Socket Mode"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Slack
    }

    async fn start(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel(100);
        let rx_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        
        let ws_url = self.get_ws_url().await?;
        info!("Connecting to Slack Socket Mode...");

        let (ws_stream, _) = connect_async(&ws_url).await?;
        info!("Connected to Slack WebSocket.");

        let (mut write_half, mut read_half) = ws_stream.split();
        let mut shutdown_rx = self.shutdown_rx.clone();
        let allowed_channels = self.allowed_channels.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = read_half.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(envelope) = serde_json::from_str::<SlackEnvelope>(&text) {
                                    // Acknowledge the envelope
                                    let ack = SlackAck {
                                        envelope_id: envelope.envelope_id.clone(),
                                    };
                                    if let Ok(ack_json) = serde_json::to_string(&ack) {
                                        let _ = write_half.send(Message::Text(ack_json)).await;
                                    }

                                    if envelope.msg_type == "events_api" {
                                        if let Some(payload) = envelope.payload {
                                            if let Some(event) = payload.get("event") {
                                                // Handle `message` event
                                                if event.get("type").and_then(|v| v.as_str()) == Some("message") {
                                                    // Ignore bot messages to prevent loops
                                                    if event.get("bot_id").is_some() || event.get("subtype").and_then(|v| v.as_str()) == Some("bot_message") {
                                                        continue;
                                                    }

                                                    let text = event.get("text").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                                    let channel = event.get("channel").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                                    let user = event.get("user").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                                    let ts = event.get("ts").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                                    let thread_ts = event.get("thread_ts").and_then(|v| v.as_str()).map(|s| s.to_string());
                                                    let channel_type = event.get("channel_type").and_then(|v| v.as_str()).unwrap_or_default();

                                                    if !allowed_channels.is_empty() && !allowed_channels.contains(&channel) {
                                                        debug!("Ignored message from unallowed channel: {}", channel);
                                                        continue;
                                                    }

                                                    // We can fetch user info to get display_name, but for now we'll just use the ID
                                                    let display_name = user.clone();
                                                    let is_group = channel_type == "channel" || channel_type == "group";

                                                    // Determine if it was threaded
                                                    let mut metadata = HashMap::new();
                                                    metadata.insert("channel".to_string(), serde_json::json!(channel));

                                                    let msg = ChannelMessage {
                                                        channel: ChannelType::Slack,
                                                        platform_message_id: ts,
                                                        sender: ChannelUser {
                                                            platform_id: user,
                                                            display_name,
                                                            sk_user: None,
                                                        },
                                                        content: ChannelContent::Text(text),
                                                        target_agent: None,
                                                        timestamp: chrono::Utc::now(),
                                                        is_group,
                                                        thread_id: thread_ts,
                                                        metadata,
                                                    };

                                                    if let Err(e) = tx.send(msg).await {
                                                        error!("Error routing Slack message: {}", e);
                                                    }
                                                }
                                            }
                                        }
                                    } else if envelope.msg_type == "disconnect" {
                                        warn!("Slack requested a disconnect. Adapter should reconnect.");
                                        break; // Simplistic approach: breaking loop ends the adapter context
                                    }
                                }
                            }
                            Some(Ok(Message::Ping(data))) => {
                                let _ = write_half.send(Message::Pong(data)).await;
                            }
                            Some(Err(e)) => {
                                error!("Slack WebSocket error: {}", e);
                                break;
                            }
                            None => {
                                warn!("Slack WebSocket stream ended unexpectedly.");
                                break;
                            }
                            _ => {} // Ignore Pong, Binary, Close without error
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        info!("Slack adapter shutting down gracefully...");
                        let _ = write_half.close().await;
                        break;
                    }
                }
            }
        });

        Ok(Box::pin(rx_stream))
    }

    async fn send(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // user.platform_id is the user ID. But for Slack we usually reply to the channel ID.
        // We'll require that `send_in_thread` or standard metadata tells us the channel.
        // If not, we will attempt to DM the user.
        self.post_message(&user.platform_id, content, None).await
    }

    async fn send_in_thread(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
        thread_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.post_message(&user.platform_id, content, Some(thread_id)).await
    }

    async fn send_typing(&self, _user: &ChannelUser) -> Result<(), Box<dyn std::error::Error>> {
        // Typing indicator via Web API is restricted (requires classic apps or RTM)
        // Since we are using Socket Mode, we cannot easily send typing indicators without extra APIs
        Ok(())
    }

    async fn send_reaction(
        &self,
        user: &ChannelUser,
        message_id: &str,
        reaction: &LifecycleReaction,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let endpoint = if reaction.remove_previous {
            "https://slack.com/api/reactions.remove"
        } else {
            "https://slack.com/api/reactions.add"
        };
        
        let mut clean_emoji = reaction.emoji.clone();
        // Remove unicode variation selectors or find Slack alias
        // For simplicity, we assume we use valid aliases here or just pass what we get.
        // In reality, Slack expects things like "thumbsup", not literal emojis.
        // We'll leave it as string literal but Slack may reject unicode.
        match clean_emoji.as_str() {
            "\u{1F914}" => clean_emoji = "thinking_face".into(),
            "\u{2699}\u{FE0F}" => clean_emoji = "gear".into(),
            "\u{270D}\u{FE0F}" => clean_emoji = "writing_hand".into(),
            "\u{2705}" => clean_emoji = "white_check_mark".into(),
            "\u{274C}" => clean_emoji = "x".into(),
            "\u{23F3}" => clean_emoji = "hourglass_flowing_sand".into(),
            "\u{1F504}" => clean_emoji = "arrows_counterclockwise".into(),
            "\u{1F440}" => clean_emoji = "eyes".into(),
            _ => {}
        }

        let payload = serde_json::json!({
            "channel": user.platform_id, // If it's a DM or the platform ID acts as the channel here (it's tricky in Slack; usually channel ID is different from user ID).
            "timestamp": message_id,
            "name": clean_emoji
        });

        let _res = self
            .client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&payload)
            .send()
            .await?;

        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.shutdown_tx.send(true);
        Ok(())
    }
}

impl SlackAdapter {
    async fn post_message(
        &self,
        recipient: &str,
        content: ChannelContent,
        thread_id: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let text = match content {
            ChannelContent::Text(t) => t,
            _ => return Err("Only text supported for Slack right now".into()),
        };

        let mut payload = serde_json::json!({
            "channel": recipient,
            "text": text
        });

        if let Some(ts) = thread_id {
            payload["thread_ts"] = serde_json::json!(ts);
        }

        let res = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            error!("Slack chat.postMessage failed: {}", body);
            return Err("Slack API error".into());
        }

        Ok(())
    }
}
