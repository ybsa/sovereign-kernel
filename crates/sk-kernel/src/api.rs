//! HTTP API Bridge for the Sovereign Kernel.
//!
//! Provides an axum-based server that exposes chat, health, and trigger
//! endpoints for external integration.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sk_types::{AgentId, SovereignError, SovereignResult};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

use crate::SovereignKernel;

/// Shared state for the API server.
pub struct ApiState {
    pub kernel: Arc<SovereignKernel>,
}

/// Request for a chat message via API.
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    /// The message to send to the agent.
    pub message: String,
    /// The target agent ID (optional).
    pub agent_id: Option<String>,
}

/// Response for a chat message via API.
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    /// The agent's response text.
    pub response: String,
    /// The agent's ID.
    pub agent_id: String,
}

/// Generic success/error response.
#[derive(Debug, Serialize)]
pub struct ActionResponse {
    pub success: bool,
    pub message: String,
}

/// Start the API server in the background.
pub async fn start_server(kernel: Arc<SovereignKernel>, port: u16) -> SovereignResult<()> {
    let state = Arc::new(ApiState { kernel: kernel.clone() });

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/v1/chat", post(chat_handler))
        .route("/v1/triggers/webhook", post(webhook_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await
        .map_err(|e| SovereignError::Config(format!("Failed to bind API server to {}: {}", addr, e)))?;

    info!(addr = %addr, "API Bridge server starting...");

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            warn!("API server error: {}", e);
        }
    });

    Ok(())
}

/// GET /health
async fn health_handler(State(_state): State<Arc<ApiState>>) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") })))
}

/// POST /v1/chat
async fn chat_handler(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let agent_id = if let Some(id) = payload.agent_id {
        AgentId(uuid::Uuid::parse_str(&id).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Agent ID".to_string()))?)
    } else {
        AgentId::new() // Use a default/new agent if not specified
    };

    info!(agent = %agent_id, "API Chat Request: {}", payload.message);

    // Load existing session or create new one
    let mut session = if let Ok(entries) = state.kernel.memory.sessions.list_for_agent(agent_id.clone()) {
        if let Some((latest_id, _, _)) = entries.first() {
            if let Ok(Some(loaded_session)) = state.kernel.memory.sessions.load(*latest_id) {
                loaded_session
            } else {
                sk_types::Session::new(agent_id.clone())
            }
        } else {
            sk_types::Session::new(agent_id.clone())
        }
    } else {
        sk_types::Session::new(agent_id.clone())
    };

    let result = state.kernel.run_agent(&mut session, &payload.message)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Agent execution error: {}", e)))?;

    Ok(Json(ChatResponse {
        response: result.response,
        agent_id: agent_id.to_string(),
    }))
}

/// POST /v1/triggers/webhook
async fn webhook_handler(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    info!("External Webhook Received: {:?}", payload);
    
    // Log the webhook trigger in the audit trail
    if let Err(e) = state.kernel.memory.audit.append_log(
        &AgentId::new(), 
        "API", 
        "webhook_trigger", 
        &payload
    ) {
        warn!("Failed to log webhook to audit trail: {}", e);
    }

    (StatusCode::ACCEPTED, Json(ActionResponse {
        success: true,
        message: "Webhook accepted and logged to audit trail.".to_string(),
    }))
}
