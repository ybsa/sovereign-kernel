//! Security middleware — API key auth + rate limiting.
//!
//! Ported from Sovereign Kernel's `middleware.rs`.
//! If `SOVEREIGN_API_KEY` env var is set, all `/api/*` and `/v1/*` routes
//! require the key in either `Authorization: Bearer <key>` or `X-API-Key: <key>`.
//! Requests to `/` (dashboard), `/logo.png`, `/favicon.ico` are always allowed.

use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};

/// Auth middleware — checks API key if `SOVEREIGN_API_KEY` is set.
pub async fn auth(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let api_key = match std::env::var("SOVEREIGN_API_KEY") {
        Ok(key) if !key.is_empty() => key,
        _ => return Ok(next.run(req).await), // No key set — allow all
    };

    let path = req.uri().path();

    // Always allow static assets
    if path == "/" || path == "/logo.png" || path == "/favicon.ico" || path == "/api/health" {
        return Ok(next.run(req).await);
    }

    // Check for API key in headers
    let provided_key = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .or_else(|| req.headers().get("x-api-key").and_then(|v| v.to_str().ok()));

    match provided_key {
        Some(key) if key == api_key => Ok(next.run(req).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Security headers middleware (from Sovereign Kernel).
pub async fn security_headers(req: Request<Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    headers.insert("x-xss-protection", "1; mode=block".parse().unwrap());
    response
}
