//! WebSocket Control Plane for Sovereign Kernel.
//!
//! Provides a real-time bi-directional bridge between the Kernel and clients
//! (Dashboard, CLI, etc.) for session tracking, presence, and live logs.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use sk_types::AgentId;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::SovereignKernel;

/// WebSocket command from client to kernel.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlCommand {
    /// Ping to keep connection alive.
    Ping,
    /// Subscribe to logs for a specific agent or "all".
    Subscribe { agent_id: String },
    /// Unsubscribe from logs.
    Unsubscribe { agent_id: String },
    /// Chat message.
    Chat { agent_id: String, message: String },
}

/// WebSocket event from kernel to client.
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlEvent {
    /// Pong response.
    Pong,
    /// Live log entry.
    Log { agent_id: String, text: String, level: String },
    /// Agent status update.
    Status { agent_id: String, state: String },
    /// Presence update for the village.
    Presence { active_agents: Vec<String> },
}

/// Handler for the /ws endpoint.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<crate::api::ApiState>>,
) -> impl IntoResponse {
    let kernel = state.kernel.clone();
    ws.on_upgrade(|socket| handle_socket(socket, kernel))
}

/// Handle an upgraded WebSocket connection.
async fn handle_socket(socket: WebSocket, kernel: Arc<SovereignKernel>) {
    let (mut sender, mut receiver) = socket.split();
    
    // Subscribe to internal kernel events
    let mut kernel_events = kernel.event_bus.subscribe();
    
    // Channel for sending events to this specific client
    let (tx, mut rx) = tokio::sync::mpsc::channel::<ControlEvent>(100);
    
    // Event loop for sending to client
    let mut send_task = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let msg = match serde_json::to_string(&event) {
                Ok(s) => Message::Text(s),
                Err(e) => {
                    error!("Failed to serialize control event: {}", e);
                    continue;
                }
            };
            if let Err(e) = sender.send(msg).await {
                debug!("WS client disconnected during send: {}", e);
                break;
            }
        }
    });

    // Event loop for receiving from client
    let kernel_clone = kernel.clone();
    let tx_clone = tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(cmd) = serde_json::from_str::<ControlCommand>(&text) {
                        match cmd {
                            ControlCommand::Ping => {
                                let _ = tx_clone.send(ControlEvent::Pong).await;
                            }
                            ControlCommand::Chat { agent_id, message } => {
                                // Delegate to kernel.run_agent (async)
                                let _k = kernel_clone.clone();
                                let aid_str = agent_id.clone();
                                tokio::spawn(async move {
                                    if let Ok(aid) = uuid::Uuid::parse_str(&aid_str) {
                                        let agent_id = AgentId(aid);
                                        // Simplified chat handling for WS
                                        info!(agent = %agent_id, "WS Chat Request: {}", message);
                                        // TODO: Implement full session handling here
                                    }
                                });
                            }
                            _ => {
                                // TODO: Handle subscriptions
                            }
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Event loop for piping kernel events to client
    let tx_clone = tx.clone();
    let mut pipe_task = tokio::spawn(async move {
        while let Ok(event) = kernel_events.recv().await {
            use crate::event_bus::KernelEvent;
            let control_event = match event {
                KernelEvent::AgentStarted { agent_id } => Some(ControlEvent::Status { 
                    agent_id, 
                    state: "running".to_string() 
                }),
                KernelEvent::AgentStopped { agent_id } => Some(ControlEvent::Status { 
                    agent_id, 
                    state: "stopped".to_string() 
                }),
                KernelEvent::Presence { active_agents } => Some(ControlEvent::Presence { 
                    active_agents 
                }),
                KernelEvent::Error { message } => Some(ControlEvent::Log {
                    agent_id: "system".to_string(),
                    text: message,
                    level: "error".to_string(), // Reverted to original as the change was syntactically incorrect
                }),
                _ => None,
            };

            if let Some(ce) = control_event {
                if tx_clone.send(ce).await.is_err() {
                    break;
                }
            }
        }
    });

    // Wait for any task to finish (meaning disconnect)
    tokio::select! {
        _ = (&mut send_task) => { recv_task.abort(); pipe_task.abort(); }
        _ = (&mut recv_task) => { send_task.abort(); pipe_task.abort(); }
        _ = (&mut pipe_task) => { send_task.abort(); recv_task.abort(); }
    }
    
    info!("WebSocket control plane connection closed");
}
