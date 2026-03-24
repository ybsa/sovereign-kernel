//! WebChat channel adapter.
//!
//! Provides a dedicated WebSocket server for a standalone chat widget.
//! Converts incoming JSON messages into `ChannelMessage` events and
//! streams agent responses back to the same WebSocket.

use crate::types::{ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser};
use async_trait::async_trait;
use axum::{
    extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use chrono::Utc;
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use tracing::{error, info};

/// WebChat adapter hosting its own WebSocket server.
#[allow(dead_code)]
pub struct WebChatAdapter {
    /// Port to listen on.
    port: u16,
    /// Default agent to route to if not specified.
    default_agent: Option<String>,
    /// Shutdown signal.
    shutdown_tx: Arc<watch::Sender<bool>>,
    shutdown_rx: watch::Receiver<bool>,
    /// Channel for sending outgoing messages back to the WebSocket client.
    /// Key: platform_id (client_id), Value: mpsc::Sender<String>
    clients: Arc<dashmap::DashMap<String, mpsc::Sender<String>>>,
}

impl WebChatAdapter {
    /// Create a new WebChat adapter.
    pub fn new(port: u16, default_agent: Option<String>) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            port,
            default_agent,
            shutdown_tx: Arc::new(shutdown_tx),
            shutdown_rx,
            clients: Arc::new(dashmap::DashMap::new()),
        }
    }
}

#[async_trait]
impl ChannelAdapter for WebChatAdapter {
    fn name(&self) -> &str {
        "webchat"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::WebChat
    }

    async fn start(
        &self,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = ChannelMessage> + Send>>, Box<dyn std::error::Error>>
    {
        let (tx, rx) = mpsc::channel::<ChannelMessage>(256);
        let port = self.port;
        let clients = self.clients.clone();
        let mut shutdown_rx = self.shutdown_rx.clone();

        info!("Starting WebChat adapter (WebSocket server on port {})", port);

        // Spawn the Axum server
        tokio::spawn(async move {
            let app = Router::new()
                .route("/ws", get(ws_handler))
                .layer(Extension(tx))
                .layer(Extension(clients));

            let addr = SocketAddr::from(([0, 0, 0, 0], port));
            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    error!("WebChat failed to bind to {}: {}", addr, e);
                    return;
                }
            };

            info!("WebChat WebSocket server listening on {}", addr);

            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.changed().await;
                })
                .await
                .unwrap_or_else(|e| error!("WebChat server error: {}", e));
        });

        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn send(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(sender) = self.clients.get(&user.platform_id) {
            let text = match content {
                ChannelContent::Text(t) => t,
                _ => "(Unsupported content type)".to_string(),
            };

            // Push the message to the client's WebSocket task
            if let Err(e) = sender.send(text).await {
                error!("Failed to send to WebChat client {}: {}", user.platform_id, e);
                self.clients.remove(&user.platform_id);
            }
        }
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.shutdown_tx.send(true);
        Ok(())
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(tx): Extension<mpsc::Sender<ChannelMessage>>,
    Extension(clients): Extension<Arc<dashmap::DashMap<String, mpsc::Sender<String>>>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, tx, clients))
}

async fn handle_socket(
    socket: WebSocket,
    tx: mpsc::Sender<ChannelMessage>,
    clients: Arc<dashmap::DashMap<String, mpsc::Sender<String>>>,
) {
    let (mut sink, mut stream): (SplitSink<WebSocket, WsMessage>, SplitStream<WebSocket>) =
        socket.split();
    let client_id = uuid::Uuid::new_v4().to_string();
    let (back_tx, mut back_rx) = mpsc::channel::<String>(100);

    // Register this client for outgoing messages
    clients.insert(client_id.clone(), back_tx);

    info!("WebChat client connected: {}", client_id);

    // Task for forwarding outgoing messages from the kernel to the WebSocket
    let client_id_clone = client_id.clone();
    let forward_task = tokio::spawn(async move {
        while let Some(msg) = back_rx.recv().await {
            let response = serde_json::json!({
                "type": "message",
                "text": msg,
                "timestamp": Utc::now().to_rfc3339(),
            });

            if sink
                .send(WsMessage::Text(response.to_string()))
                .await
                .is_err()
            {
                break;
            }
        }
        info!(
            "WebChat client disconnected (outgoing task): {}",
            client_id_clone
        );
    });

    // Handle incoming messages from the WebSocket
    while let Some(Ok(msg)) = stream.next().await {
        if let WsMessage::Text(text) = msg {
            // Expecting JSON: { "text": "hello" }
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(msg_text) = value["text"].as_str() {
                    let channel_msg = ChannelMessage {
                        channel: ChannelType::WebChat,
                        platform_message_id: uuid::Uuid::new_v4().to_string(),
                        sender: ChannelUser {
                            platform_id: client_id.clone(),
                            display_name: "Web User".to_string(),
                            sk_user: None,
                        },
                        content: ChannelContent::Text(msg_text.to_string()),
                        target_agent: None,
                        timestamp: Utc::now(),
                        is_group: false,
                        thread_id: None,
                        metadata: HashMap::new(),
                    };

                    if tx.send(channel_msg).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    info!("WebChat client disconnected (incoming task): {}", client_id);
    clients.remove(&client_id);
    forward_task.abort();
}
