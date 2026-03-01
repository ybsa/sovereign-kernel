//! Live Canvas — embedded web dashboard for Sovereign Kernel.
//!
//! Follows the OpenFang pattern: compile-time embedded HTML/CSS/JS via
//! `include_str!()` and `include_bytes!()` for single-binary deployment.
//!
//! Features (from OpenFang):
//! - Alpine.js SPA with hash-based routing
//! - Dark/light theme toggle with system preference detection
//! - Responsive layout with collapsible sidebar
//! - Markdown rendering + syntax highlighting (bundled locally)
//! - WebSocket real-time chat with HTTP fallback
//! - Agent management, channels, hands, workflows, and more

use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use serde::Serialize;
use sk_hands::registry::HandRegistry;
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::CorsLayer;
use tracing::info;

// ─── Compile-time ETag ───────────────────────────────────────────────────────

/// Compile-time ETag based on the crate version.
const ETAG: &str = concat!("\"sovereign-", env!("CARGO_PKG_VERSION"), "\"");

// ─── Embedded static assets ──────────────────────────────────────────────────

/// Embedded logo PNG for single-binary deployment.
const LOGO_PNG: &[u8] = include_bytes!("../static/logo.png");

/// Embedded favicon ICO for browser tabs.
const FAVICON_ICO: &[u8] = include_bytes!("../static/favicon.ico");

/// The full embedded SPA assembled at compile time from organized static files.
/// All vendor libraries (Alpine.js, marked.js, highlight.js) are bundled
/// locally — no CDN dependency. Alpine.js is included LAST because it
/// immediately processes x-data directives and fires alpine:init on load.
const DASHBOARD_HTML: &str = concat!(
    include_str!("../static/index_head.html"),
    "<style>\n",
    include_str!("../static/css/theme.css"),
    "\n",
    include_str!("../static/css/layout.css"),
    "\n",
    include_str!("../static/css/components.css"),
    "\n",
    include_str!("../static/vendor/github-dark.min.css"),
    "\n</style>\n",
    include_str!("../static/index_body.html"),
    // Vendor libs: marked + highlight first (used by app.js)
    "<script>\n",
    include_str!("../static/vendor/marked.min.js"),
    "\n</script>\n",
    "<script>\n",
    include_str!("../static/vendor/highlight.min.js"),
    "\n</script>\n",
    // App code
    "<script>\n",
    include_str!("../static/js/api.js"),
    "\n",
    include_str!("../static/js/app.js"),
    "\n",
    include_str!("../static/js/pages/overview.js"),
    "\n",
    include_str!("../static/js/pages/chat.js"),
    "\n",
    include_str!("../static/js/pages/agents.js"),
    "\n",
    include_str!("../static/js/pages/workflows.js"),
    "\n",
    include_str!("../static/js/pages/workflow-builder.js"),
    "\n",
    include_str!("../static/js/pages/channels.js"),
    "\n",
    include_str!("../static/js/pages/skills.js"),
    "\n",
    include_str!("../static/js/pages/hands.js"),
    "\n",
    include_str!("../static/js/pages/scheduler.js"),
    "\n",
    include_str!("../static/js/pages/settings.js"),
    "\n",
    include_str!("../static/js/pages/usage.js"),
    "\n",
    include_str!("../static/js/pages/sessions.js"),
    "\n",
    include_str!("../static/js/pages/logs.js"),
    "\n",
    include_str!("../static/js/pages/wizard.js"),
    "\n",
    include_str!("../static/js/pages/approvals.js"),
    "\n</script>\n",
    // Alpine.js MUST be last — it processes x-data and fires alpine:init
    "<script>\n",
    include_str!("../static/vendor/alpine.min.js"),
    "\n</script>\n",
    "</body></html>"
);

// ─── Shared state ────────────────────────────────────────────────────────────

/// Shared state accessible by all dashboard API routes.
pub struct AppState {
    pub hand_registry: std::sync::Mutex<HandRegistry>,
    pub started_at: Instant,
    pub telegram_connected: bool,
}

// ─── Static asset handlers ───────────────────────────────────────────────────

/// GET / — Serve the Sovereign Kernel Dashboard SPA.
async fn dashboard_page() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::ETAG, ETAG),
            (
                header::CACHE_CONTROL,
                "public, max-age=3600, must-revalidate",
            ),
        ],
        DASHBOARD_HTML,
    )
}

/// GET /logo.png — Serve the logo.
async fn logo_png() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=86400, immutable"),
        ],
        LOGO_PNG,
    )
}

/// GET /favicon.ico — Serve the favicon.
async fn favicon_ico() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "image/x-icon"),
            (header::CACHE_CONTROL, "public, max-age=86400, immutable"),
        ],
        FAVICON_ICO,
    )
}

// ─── API response types ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct StatusResponse {
    name: &'static str,
    version: &'static str,
    uptime_secs: u64,
    channels_active: u32,
    hands_loaded: usize,
    hands_active: usize,
}

#[derive(Serialize)]
struct AgentInfo {
    id: String,
    name: String,
    status: String,
    model: String,
}

#[derive(Serialize)]
struct HandInfo {
    id: String,
    name: String,
    description: String,
    category: String,
    icon: String,
    tools: Vec<String>,
}

#[derive(Serialize)]
struct HandInstanceInfo {
    instance_id: String,
    hand_id: String,
    status: String,
    agent_name: String,
}

#[derive(Serialize)]
struct ChannelInfo {
    name: String,
    channel_type: String,
    connected: bool,
}

#[derive(Serialize)]
struct VersionInfo {
    version: &'static str,
    platform: &'static str,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

// ─── API route handlers ──────────────────────────────────────────────────────

async fn api_health() -> axum::Json<HealthResponse> {
    axum::Json(HealthResponse { status: "ok" })
}

async fn api_version() -> axum::Json<VersionInfo> {
    axum::Json(VersionInfo {
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
    })
}

async fn api_status(state: axum::extract::State<Arc<AppState>>) -> axum::Json<StatusResponse> {
    let reg = state.hand_registry.lock().unwrap();
    let defs = reg.list_definitions();
    let instances = reg.list_instances();

    axum::Json(StatusResponse {
        name: "Sovereign Kernel",
        version: env!("CARGO_PKG_VERSION"),
        uptime_secs: state.started_at.elapsed().as_secs(),
        channels_active: if state.telegram_connected { 1 } else { 0 },
        hands_loaded: defs.len(),
        hands_active: instances.len(),
    })
}

async fn api_list_agents() -> axum::Json<Vec<AgentInfo>> {
    axum::Json(vec![AgentInfo {
        id: "default".into(),
        name: "Sovereign Agent".into(),
        status: "running".into(),
        model: "claude-sonnet-4-20250514".into(),
    }])
}

async fn api_list_hands(state: axum::extract::State<Arc<AppState>>) -> axum::Json<Vec<HandInfo>> {
    let reg = state.hand_registry.lock().unwrap();
    let defs = reg.list_definitions();
    axum::Json(
        defs.iter()
            .map(|d| HandInfo {
                id: d.id.clone(),
                name: d.name.clone(),
                description: d.description.clone(),
                category: d.category.to_string(),
                icon: d.icon.clone(),
                tools: d.tools.clone(),
            })
            .collect(),
    )
}

async fn api_list_hand_instances(
    state: axum::extract::State<Arc<AppState>>,
) -> axum::Json<Vec<HandInstanceInfo>> {
    let reg = state.hand_registry.lock().unwrap();
    let instances = reg.list_instances();
    axum::Json(
        instances
            .iter()
            .map(|i| HandInstanceInfo {
                instance_id: i.instance_id.to_string(),
                hand_id: i.hand_id.clone(),
                status: i.status.to_string(),
                agent_name: i.agent_name.clone(),
            })
            .collect(),
    )
}

async fn api_list_channels(
    state: axum::extract::State<Arc<AppState>>,
) -> axum::Json<Vec<ChannelInfo>> {
    axum::Json(vec![
        ChannelInfo {
            name: "Telegram".into(),
            channel_type: "telegram".into(),
            connected: state.telegram_connected,
        },
        ChannelInfo {
            name: "CLI".into(),
            channel_type: "cli".into(),
            connected: true,
        },
    ])
}

#[derive(Serialize)]
struct UsageStats {
    total_requests: u64,
    total_tokens_used: u64,
    uptime_secs: u64,
}

async fn api_usage(state: axum::extract::State<Arc<AppState>>) -> axum::Json<UsageStats> {
    axum::Json(UsageStats {
        total_requests: 0,
        total_tokens_used: 0,
        uptime_secs: state.started_at.elapsed().as_secs(),
    })
}

#[derive(Serialize)]
struct AuditEntry {
    timestamp: String,
    action: String,
    agent: String,
    details: String,
}

async fn api_audit() -> axum::Json<Vec<AuditEntry>> {
    axum::Json(vec![AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "daemon_start".into(),
        agent: "system".into(),
        details: "Sovereign Kernel daemon started".into(),
    }])
}

// ─── Router builder ──────────────────────────────────────────────────────────

/// Build the full dashboard router with all routes + state (follows OpenFang's `build_router`).
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Static assets
        .route("/", get(dashboard_page))
        .route("/logo.png", get(logo_png))
        .route("/favicon.ico", get(favicon_ico))
        // Core API
        .route("/api/health", get(api_health))
        .route("/api/version", get(api_version))
        .route("/api/status", get(api_status))
        // Agents
        .route("/api/agents", get(api_list_agents))
        // Hands
        .route("/api/hands", get(api_list_hands))
        .route("/api/hands/active", get(api_list_hand_instances))
        // Channels
        .route("/api/channels", get(api_list_channels))
        // Usage tracking (from OpenFang)
        .route("/api/usage", get(api_usage))
        .route("/api/usage/summary", get(api_usage))
        // Audit log (from OpenFang)
        .route("/api/audit/recent", get(api_audit))
        // OpenAI-compatible API (from OpenFang)
        .route(
            "/v1/chat/completions",
            axum::routing::post(crate::openai_compat::chat_completions),
        )
        .route("/v1/models", get(crate::openai_compat::list_models))
        // Middleware (from OpenFang)
        .layer(axum::middleware::from_fn(crate::middleware::auth))
        .layer(axum::middleware::from_fn(
            crate::middleware::security_headers,
        ))
        // CORS
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Start the dashboard web server on the given port.
pub async fn start_server(state: Arc<AppState>, port: u16) {
    let app = build_router(state);

    let addr = format!("0.0.0.0:{port}");
    info!("⚡ Live Canvas dashboard at http://localhost:{port}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
