//! OpenAI-compatible `/v1/chat/completions` API endpoint.
//!
//! Ported from OpenFang's `openai_compat.rs`. Allows any OpenAI-compatible
//! client library (Python openai, curl, ChatGPT UIs) to talk to Sovereign Kernel.
//!
//! The `model` field resolves to the default agent. Supports non-streaming responses.

use crate::dashboard::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── Request types (from OpenFang) ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<OaiMessage>,
    #[serde(default)]
    pub stream: bool,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct OaiMessage {
    pub role: String,
    #[serde(default)]
    pub content: OaiContent,
}

#[derive(Debug, Deserialize, Default)]
#[serde(untagged)]
pub enum OaiContent {
    Text(String),
    Parts(Vec<OaiContentPart>),
    #[default]
    Null,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum OaiContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: OaiImageUrlRef },
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OaiImageUrlRef {
    pub url: String,
}

// ── Response types (from OpenFang) ──────────────────────────────────────────

#[derive(Serialize)]
struct ChatCompletionResponse {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    choices: Vec<Choice>,
    usage: UsageInfo,
}

#[derive(Serialize)]
struct Choice {
    index: u32,
    message: ChoiceMessage,
    finish_reason: &'static str,
}

#[derive(Serialize)]
struct ChoiceMessage {
    role: &'static str,
    content: Option<String>,
}

#[derive(Serialize)]
struct UsageInfo {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[derive(Serialize)]
struct ModelObject {
    id: String,
    object: &'static str,
    created: u64,
    owned_by: String,
}

#[derive(Serialize)]
struct ModelListResponse {
    object: &'static str,
    data: Vec<ModelObject>,
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// POST /v1/chat/completions — Process a chat completion request.
pub async fn chat_completions(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    // Extract the last user message
    let last_user_msg = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .and_then(|m| match &m.content {
            OaiContent::Text(text) => Some(text.clone()),
            OaiContent::Parts(parts) => parts.iter().find_map(|p| match p {
                OaiContentPart::Text { text } => Some(text.clone()),
                _ => None,
            }),
            OaiContent::Null => None,
        })
        .unwrap_or_default();

    if last_user_msg.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": {
                    "message": "No user message found in request",
                    "type": "invalid_request_error",
                    "code": "missing_message"
                }
            })),
        )
            .into_response();
    }

    let request_id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // For now, return a placeholder until sk-engine streaming integration is wired.
    // This structure is fully OpenAI-compatible and will work with any client.
    let response = ChatCompletionResponse {
        id: request_id,
        object: "chat.completion",
        created,
        model: req.model.clone(),
        choices: vec![Choice {
            index: 0,
            message: ChoiceMessage {
                role: "assistant",
                content: Some(format!(
                    "Sovereign Kernel received your message: '{}'. Engine integration pending.",
                    last_user_msg
                )),
            },
            finish_reason: "stop",
        }],
        usage: UsageInfo {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        },
    };

    Json(serde_json::to_value(&response).unwrap_or_default()).into_response()
}

/// GET /v1/models — List available agents as OpenAI model objects.
pub async fn list_models(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let reg = state.hand_registry.lock().unwrap();
    let defs = reg.list_definitions();

    let mut models = vec![ModelObject {
        id: "sovereign:default".to_string(),
        object: "model",
        created,
        owned_by: "sovereign-kernel".to_string(),
    }];

    // Add hands as available models too
    for def in &defs {
        models.push(ModelObject {
            id: format!("sovereign:{}", def.id),
            object: "model",
            created,
            owned_by: "sovereign-kernel".to_string(),
        });
    }

    Json(
        serde_json::to_value(&ModelListResponse {
            object: "list",
            data: models,
        })
        .unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oai_content_deserialize_string() {
        let json = r#"{"role":"user","content":"hello"}"#;
        let msg: OaiMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg.content, OaiContent::Text(ref t) if t == "hello"));
    }

    #[test]
    fn test_oai_content_deserialize_parts() {
        let json = r#"{"role":"user","content":[{"type":"text","text":"what is this?"}]}"#;
        let msg: OaiMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg.content, OaiContent::Parts(ref p) if p.len() == 1));
    }

    #[test]
    fn test_response_serialization() {
        let resp = ChatCompletionResponse {
            id: "chatcmpl-test".to_string(),
            object: "chat.completion",
            created: 1234567890,
            model: "test-agent".to_string(),
            choices: vec![Choice {
                index: 0,
                message: ChoiceMessage {
                    role: "assistant",
                    content: Some("Hello!".to_string()),
                },
                finish_reason: "stop",
            }],
            usage: UsageInfo {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["object"], "chat.completion");
        assert_eq!(json["choices"][0]["message"]["content"], "Hello!");
        assert_eq!(json["usage"]["total_tokens"], 15);
    }
}
