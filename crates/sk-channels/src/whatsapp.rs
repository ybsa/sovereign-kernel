//! WhatsApp Cloud API channel adapter.

use crate::types::{
    ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser, LifecycleReaction,
};
use async_trait::async_trait;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use futures::Stream;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tracing::{error, info, warn};

/// Shared state for the Axum webhook handlers.
struct WebhookState {
    verify_token: String,
    tx: mpsc::Sender<ChannelMessage>,
    allowed_users: Vec<String>,
}

pub struct WhatsAppAdapter {
    access_token: String,
    verify_token: String,
    phone_number_id: String,
    webhook_port: u16,
    allowed_users: Vec<String>,
    client: Client,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
}

impl WhatsAppAdapter {
    pub fn new(
        access_token: String,
        verify_token: String,
        phone_number_id: String,
        webhook_port: u16,
        allowed_users: Vec<String>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            access_token,
            verify_token,
            phone_number_id,
            webhook_port,
            allowed_users,
            client: Client::new(),
            shutdown_tx,
            shutdown_rx,
        }
    }
}

#[derive(Deserialize)]
struct VerifyQuery {
    #[serde(rename = "hub.mode")]
    mode: Option<String>,
    #[serde(rename = "hub.challenge")]
    challenge: Option<String>,
    #[serde(rename = "hub.verify_token")]
    verify_token: Option<String>,
}

#[derive(Deserialize, Debug)]
struct WhatsAppPayload {
    object: Option<String>,
    entry: Option<Vec<WhatsAppEntry>>,
}

#[derive(Deserialize, Debug)]
struct WhatsAppEntry {
    changes: Option<Vec<WhatsAppChange>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct WhatsAppChange {
    value: Option<WhatsAppValue>,
    field: Option<String>,
}

#[derive(Deserialize, Debug)]
struct WhatsAppValue {
    messages: Option<Vec<WhatsAppMessage>>,
    contacts: Option<Vec<WhatsAppContact>>,
}

#[derive(Deserialize, Debug)]
struct WhatsAppContact {
    profile: Option<WhatsAppProfile>,
    wa_id: Option<String>,
}

#[derive(Deserialize, Debug)]
struct WhatsAppProfile {
    name: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct WhatsAppMessage {
    from: String,
    id: String,
    timestamp: Option<String>,
    text: Option<WhatsAppText>,
    // other types ignored for now
}

#[derive(Deserialize, Debug)]
struct WhatsAppText {
    body: String,
}

async fn get_webhook(
    State(state): State<Arc<WebhookState>>,
    Query(query): Query<VerifyQuery>,
) -> impl IntoResponse {
    if let (Some(mode), Some(token), Some(challenge)) =
        (query.mode, query.verify_token, query.challenge)
    {
        if mode == "subscribe" && token == state.verify_token {
            info!("WhatsApp Webhook verified successfully.");
            return (StatusCode::OK, challenge).into_response();
        }
    }
    (StatusCode::FORBIDDEN, "Forbidden".to_string()).into_response()
}

async fn post_webhook(
    State(state): State<Arc<WebhookState>>,
    Json(payload): Json<WhatsAppPayload>,
) -> impl IntoResponse {
    if payload.object.as_deref() != Some("whatsapp_business_account") {
        return StatusCode::NOT_FOUND;
    }

    if let Some(entries) = payload.entry {
        for entry in entries {
            if let Some(changes) = entry.changes {
                for change in changes {
                    if let Some(value) = change.value {
                        // Create a map of contacts for faster lookup
                        let mut display_names = HashMap::new();
                        if let Some(contacts) = value.contacts {
                            for contact in contacts {
                                if let (Some(wa_id), Some(profile)) =
                                    (contact.wa_id, contact.profile)
                                {
                                    display_names.insert(wa_id, profile.name);
                                }
                            }
                        }

                        if let Some(messages) = value.messages {
                            for msg in messages {
                                if let Some(text) = msg.text {
                                    let sender_id = msg.from.clone();

                                    // Filter by allowed users
                                    if !state.allowed_users.is_empty()
                                        && !state.allowed_users.contains(&sender_id)
                                    {
                                        warn!(
                                            "Message from unauthorized user {} ignored.",
                                            sender_id
                                        );
                                        continue;
                                    }

                                    let display_name = display_names
                                        .get(&sender_id)
                                        .cloned()
                                        .unwrap_or_else(|| sender_id.clone());

                                    let channel_msg = ChannelMessage {
                                        channel: ChannelType::WhatsApp,
                                        platform_message_id: msg.id,
                                        sender: ChannelUser {
                                            platform_id: sender_id,
                                            display_name,
                                            sk_user: None,
                                        },
                                        content: ChannelContent::Text(text.body),
                                        target_agent: None,
                                        timestamp: chrono::Utc::now(),
                                        is_group: false,
                                        thread_id: None,
                                        metadata: HashMap::new(),
                                    };

                                    if let Err(e) = state.tx.send(channel_msg).await {
                                        error!("Failed to enqueue WhatsApp message: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    StatusCode::OK
}

#[async_trait]
impl ChannelAdapter for WhatsAppAdapter {
    fn name(&self) -> &str {
        "WhatsApp Cloud API"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::WhatsApp
    }

    async fn start(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>, Box<dyn std::error::Error>>
    {
        let (tx, rx) = mpsc::channel(100);
        let rx_stream = tokio_stream::wrappers::ReceiverStream::new(rx);

        let state = Arc::new(WebhookState {
            verify_token: self.verify_token.clone(),
            tx,
            allowed_users: self.allowed_users.clone(),
        });

        let app = Router::new()
            .route("/webhook", get(get_webhook).post(post_webhook))
            .with_state(state);

        let port = self.webhook_port;

        let mut shutdown_rx = self.shutdown_rx.clone();

        tokio::spawn(async move {
            let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
            info!("WhatsApp webhook server listening on {addr}/webhook");

            let listener = tokio::net::TcpListener::bind(&addr).await;
            if let Ok(listener) = listener {
                let server = axum::serve(listener, app);
                tokio::select! {
                    res = server => {
                        if let Err(err) = res {
                            error!("WhatsApp webhook server error: {}", err);
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        info!("WhatsApp webhook server shutting down...");
                    }
                }
            } else {
                error!("Failed to bind WhatsApp webhook port {}", port);
            }
        });

        Ok(Box::pin(rx_stream))
    }

    async fn send(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let text = match content {
            ChannelContent::Text(t) => t,
            _ => return Err("Only text messages are supported for WhatsApp right now".into()),
        };

        let url = format!(
            "https://graph.facebook.com/v17.0/{}/messages",
            self.phone_number_id
        );

        let payload = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": user.platform_id,
            "type": "text",
            "text": {
                "body": text
            }
        });

        let res = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            error!("WhatsApp send failed ({}): {}", status, body);
            return Err(format!("WhatsApp API error: {}", status).into());
        }

        Ok(())
    }

    async fn send_typing(&self, _user: &ChannelUser) -> Result<(), Box<dyn std::error::Error>> {
        // WhatsApp doesn't have a reliable typing indicator endpoint in Cloud API (it's implicit).
        Ok(())
    }

    async fn send_reaction(
        &self,
        user: &ChannelUser,
        message_id: &str,
        reaction: &LifecycleReaction,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!(
            "https://graph.facebook.com/v17.0/{}/messages",
            self.phone_number_id
        );

        let payload = if reaction.remove_previous {
            serde_json::json!({
                "messaging_product": "whatsapp",
                "recipient_type": "individual",
                "to": user.platform_id,
                "type": "reaction",
                "reaction": {
                    "message_id": message_id,
                    "emoji": ""
                }
            })
        } else {
            serde_json::json!({
                "messaging_product": "whatsapp",
                "recipient_type": "individual",
                "to": user.platform_id,
                "type": "reaction",
                "reaction": {
                    "message_id": message_id,
                    "emoji": reaction.emoji
                }
            })
        };

        let res = self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            error!("WhatsApp reaction failed ({}): {}", status, body);
        }

        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.shutdown_tx.send(true);
        Ok(())
    }
}
