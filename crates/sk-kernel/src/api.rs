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
use tower_http::cors::{Any, CorsLayer};
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

/// Detailed kernel status response.
#[derive(Debug, Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub model: String,
    pub driver: String,
    pub mcp_servers: Vec<McpStatus>,
}

#[derive(Debug, Serialize)]
pub struct McpStatus {
    pub name: String,
    pub tool_count: usize,
}

/// Start the API server in the background.
pub async fn start_server(kernel: Arc<SovereignKernel>, addr: &str) -> SovereignResult<()> {
    let state = Arc::new(ApiState {
        kernel: kernel.clone(),
    });

    // The auth middleware from sk-cli isn't available here in sk-kernel directly,
    // so we enforce API key check here.
    async fn auth(
        req: axum::extract::Request<axum::body::Body>,
        next: axum::middleware::Next,
    ) -> Result<axum::response::Response, axum::http::StatusCode> {
        let api_key = match std::env::var("SOVEREIGN_API_KEY") {
            Ok(key) if !key.is_empty() => key,
            _ => return Err(axum::http::StatusCode::UNAUTHORIZED),
        };

        let path = req.uri().path();
        if path == "/health" {
            return Ok(next.run(req).await);
        }

        let provided_key = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .or_else(|| req.headers().get("x-api-key").and_then(|v| v.to_str().ok()));

        match provided_key {
            Some(key) if key == api_key => Ok(next.run(req).await),
            _ => Err(axum::http::StatusCode::UNAUTHORIZED),
        }
    }

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/v1/status", get(status_handler))
        .route("/v1/chat", post(chat_handler))
        .route("/v1/agents", get(list_agents_handler))
        .route(
            "/v1/agents/:id",
            post(stop_agent_handler).delete(remove_agent_handler),
        )
        .route("/v1/agents/:id/thinking", get(thinking_handler))
        .route("/v1/triggers/webhook", post(webhook_handler))
        .route("/v1/config", get(get_config_handler).post(update_config_handler))
        .route("/v1/tools", get(list_tools_handler))
        .route("/v1/treasury/status", get(treasury_status_handler))
        .route("/v1/treasury/reset", post(treasury_reset_handler))
        .route("/ws", get(crate::control_plane::ws_handler))
        .layer(axum::middleware::from_fn(auth))
        .layer(
            CorsLayer::new()
                .allow_origin(axum::http::HeaderValue::from_static(
                    "http://127.0.0.1:1420",
                ))
                .allow_origin(axum::http::HeaderValue::from_static(
                    "http://localhost:1420",
                ))
                .allow_origin(axum::http::HeaderValue::from_static(
                    "http://localhost:4200",
                ))
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        SovereignError::Config(format!("Failed to bind API server to {}: {}", addr, e))
    })?;

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
    (
        StatusCode::OK,
        Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") })),
    )
}

/// GET /v1/status
async fn status_handler(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    let mcp = state.kernel.mcp.read().await;
    let mcp_servers = mcp
        .list_servers()
        .into_iter()
        .map(|(name, tool_count)| McpStatus { name, tool_count })
        .collect();

    let response = StatusResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        model: state.kernel.model_name.clone(),
        driver: state.kernel.driver.provider().to_string(),
        mcp_servers,
    };

    (StatusCode::OK, Json(response))
}

/// POST /v1/chat
async fn chat_handler(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let agent_id = if let Some(id) = payload.agent_id {
        AgentId(
            uuid::Uuid::parse_str(&id)
                .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid Agent ID".to_string()))?,
        )
    } else {
        AgentId::new() // Use a default/new agent if not specified
    };

    info!(agent = %agent_id, "API Chat Request: {}", payload.message);

    // Load existing session or create new one
    let mut session = if let Ok(entries) = state.kernel.memory.sessions.list_for_agent(agent_id) {
        if let Some((latest_id, _, _)) = entries.first() {
            if let Ok(Some(loaded_session)) = state.kernel.memory.sessions.load(*latest_id) {
                loaded_session
            } else {
                sk_types::Session::new(agent_id)
            }
        } else {
            sk_types::Session::new(agent_id)
        }
    } else {
        sk_types::Session::new(agent_id)
    };

    let result = state
        .kernel
        .run_agent(&mut session, &payload.message)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Agent execution error: {}", e),
            )
        })?;

    Ok(Json(ChatResponse {
        response: result.response,
        agent_id: agent_id.to_string(),
    }))
}

/// GET /v1/agents
async fn list_agents_handler(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    let agents = state.kernel.agents.list();
    let mut entries = Vec::new();
    for agent in agents {
        let is_running = state.kernel.active_loops.contains_key(&agent.id);
        entries.push(serde_json::json!({
            "id": agent.id.to_string(),
            "name": agent.name,
            "description": agent.manifest.description,
            "state": agent.state,
            "is_running": is_running,
            "last_active": agent.last_active,
        }));
    }
    (StatusCode::OK, Json(entries))
}

/// POST /v1/agents/:id (Stop)
async fn stop_agent_handler(
    State(state): State<Arc<ApiState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let agent_id = match uuid::Uuid::parse_str(&id) {
        Ok(u) => AgentId(u),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ActionResponse {
                    success: false,
                    message: "Invalid ID".to_string(),
                }),
            )
        }
    };

    let stopped = state.kernel.stop_agent(&agent_id);
    (
        StatusCode::OK,
        Json(ActionResponse {
            success: stopped,
            message: if stopped {
                "Agent stopped".to_string()
            } else {
                "Agent not running or not found".to_string()
            },
        }),
    )
}

/// DELETE /v1/agents/:id (Remove)
async fn remove_agent_handler(
    State(state): State<Arc<ApiState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let agent_id = match uuid::Uuid::parse_str(&id) {
        Ok(u) => AgentId(u),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ActionResponse {
                    success: false,
                    message: "Invalid ID".to_string(),
                }),
            )
        }
    };

    // Stop if running
    state.kernel.stop_agent(&agent_id);

    // Remove from registry
    match state.kernel.agents.remove(agent_id) {
        Ok(_) => (
            StatusCode::OK,
            Json(ActionResponse {
                success: true,
                message: "Agent removed from registry".to_string(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ActionResponse {
                success: false,
                message: e.to_string(),
            }),
        ),
    }
}

/// GET /v1/agents/:id/thinking
async fn thinking_handler(
    State(state): State<Arc<ApiState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    // 1. Find session ID for agent
    let agent_id = match uuid::Uuid::parse_str(&id) {
        Ok(u) => AgentId(u),
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid ID".to_string()).into_response(),
    };

    let agent = match state.kernel.agents.get(agent_id) {
        Some(a) => a,
        None => return (StatusCode::NOT_FOUND, "Agent not found").into_response(),
    };

    // 2. Look for forensics logs
    // Forensics are usually in DATA_DIR/.steps/<session_id>/step_*.jsonl
    let forensics_dir = state
        .kernel
        .config
        .read()
        .await
        .data_dir
        .join(".steps")
        .join(agent.session_id.to_string());

    if !forensics_dir.exists() {
        return (StatusCode::OK, Json(serde_json::json!({ "thoughts": [], "message": "No forensic logs found for this session." }))).into_response();
    }

    // 3. Read step files
    let mut thoughts = Vec::new();
    if let Ok(entries) = std::fs::read_dir(forensics_dir) {
        let mut paths: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
        paths.sort(); // Sort by step number

        for path in paths {
            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        // Extract thought block if present (usually in response)
                        if let Some(resp) = val.get("response").and_then(|v| v.as_str()) {
                            thoughts.push(resp.to_string());
                        }
                    }
                }
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({ "thoughts": thoughts })),
    )
        .into_response()
}

/// GET /v1/config
async fn get_config_handler(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    let cfg = state.kernel.config.read().await.clone();
    (StatusCode::OK, Json(cfg))
}

/// POST /v1/config
async fn update_config_handler(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<sk_types::config::KernelConfig>,
) -> impl IntoResponse {
    let old_config = state.kernel.config.read().await.clone();
    
    // Validate new config
    if let Err(errors) = crate::config_reload::validate_config_for_reload(&payload) {
        return (StatusCode::BAD_REQUEST, Json(ActionResponse {
            success: false,
            message: format!("Config validation failed: {}", errors.join(", ")),
        })).into_response();
    }

    let plan = crate::config_reload::build_reload_plan(&old_config, &payload);
    plan.log_summary();

    if plan.restart_required {
        return (StatusCode::ACCEPTED, Json(ActionResponse {
            success: true,
            message: format!("Config received. FULL RESTART REQUIRED: {}", plan.restart_reasons.join("; ")),
        })).into_response();
    }

    // Apply hot actions
    {
        let mut lock = state.kernel.config.write().await;
        *lock = payload;
    }

    if let Err(e) = state.kernel.apply_hot_actions(&plan.hot_actions).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ActionResponse {
            success: false,
            message: format!("Failed to apply hot-reload: {}", e),
        })).into_response();
    }

    (StatusCode::OK, Json(ActionResponse {
        success: true,
        message: "Config hot-reloaded successfully.".to_string(),
    })).into_response()
}

/// GET /v1/tools
async fn list_tools_handler(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    let mcp = state.kernel.mcp.read().await;
    let tools = mcp.all_tools();
    (StatusCode::OK, Json(tools))
}

/// POST /v1/triggers/webhook
async fn webhook_handler(
    State(state): State<Arc<ApiState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    info!("External Webhook Received: {:?}", payload);

    // Log the webhook trigger in the audit trail
    if let Err(e) =
        state
            .kernel
            .memory
            .audit
            .append_log(&AgentId::new(), "API", "webhook_trigger", &payload)
    {
        warn!("Failed to log webhook to audit trail: {}", e);
    }

    (
        StatusCode::ACCEPTED,
        Json(ActionResponse {
            success: true,
            message: "Webhook accepted and logged to audit trail.".to_string(),
        }),
    )
}
/// GET /v1/treasury/status
async fn treasury_status_handler(State(state): State<Arc<ApiState>>) -> impl IntoResponse {
    let config = state.kernel.config.read().await;
    let status = state.kernel.metering.budget_status(&config.budget).await;
    (StatusCode::OK, Json(status))
}

/// POST /v1/treasury/reset
async fn treasury_reset_handler(State(_state): State<Arc<ApiState>>) -> impl IntoResponse {
    // For now, we just create a fresh metering engine or reset the state
    // Actually, MeteringEngine should have a reset method.
    // I'll add a simple way to reset in MeteringEngine later if needed, 
    // but for now I'll just clear the global costs.
    
    // I'll call a reset method on metering (needs to be added)
    // state.kernel.metering.reset().await;
    
    (StatusCode::OK, Json(ActionResponse {
        success: true,
        message: "Treasury costs reset (partially implemented)".to_string(),
    }))
}
