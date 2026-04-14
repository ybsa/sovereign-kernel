//! LLM driver trait and OpenAI-compatible implementation.
//!
//! Based on Sovereign Kernel's drivers — works with Anthropic, Gemini, and any
//! OpenAI-compatible provider (OpenAI, Groq, Together, Ollama, etc.).

use async_trait::async_trait;
use sk_types::{Message, SovereignError, ToolDefinition};

/// A completion request to an LLM.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    /// Model identifier (e.g. "claude-sonnet-4-20250514").
    pub model: String,
    /// Conversation messages.
    pub messages: Vec<Message>,
    /// Available tools.
    pub tools: Vec<ToolDefinition>,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Sampling temperature.
    pub temperature: f32,
    /// Whether to stream the response.
    pub stream: bool,
}

/// A completion response from an LLM.
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    /// Generated text content.
    pub content: String,
    /// Tool calls requested by the model.
    pub tool_calls: Vec<sk_types::ToolCall>,
    /// Stop reason.
    pub stop_reason: StopReason,
    /// Token usage.
    pub usage: TokenUsage,
}

/// Why the model stopped generating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// Normal end of response.
    EndTurn,
    /// Model wants to call tools.
    ToolUse,
    /// Hit max_tokens limit.
    MaxTokens,
    /// Content filtered.
    ContentFilter,
    /// Unknown reason.
    Unknown(String),
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    /// Tokens served from the provider's prompt cache (Anthropic, OpenAI).
    /// These cost significantly less — ~10% (Anthropic) or 50% (OpenAI) of normal price.
    pub cached_tokens: u32,
}

/// LLM driver error types.
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("Rate limited: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Context overflow: {0}")]
    ContextOverflow(String),

    #[error("Model overloaded: {0}")]
    Overloaded(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

impl LlmError {
    /// Whether this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            LlmError::RateLimited { .. } | LlmError::Overloaded(_) | LlmError::NetworkError(_)
        )
    }
}

impl From<LlmError> for SovereignError {
    fn from(e: LlmError) -> Self {
        SovereignError::LlmDriver(e.to_string())
    }
}

/// A handler for streaming tokens.
pub type StreamHandler = Box<dyn Fn(&str) + Send + Sync>;

/// Trait for LLM drivers.
#[async_trait]
pub trait LlmDriver: Send + Sync {
    /// Send a completion request.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// Send a completion request with streaming support.
    async fn complete_stream(
        &self,
        request: CompletionRequest,
        _handler: &StreamHandler,
    ) -> Result<CompletionResponse, LlmError> {
        self.complete(request).await
    }

    /// Get the driver's provider name.
    fn provider(&self) -> &str;
}

/// OpenAI-compatible LLM driver.
///
/// Works with any provider that implements the `/v1/chat/completions` endpoint.
pub struct OpenAICompatDriver {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    provider_name: String,
}

impl OpenAICompatDriver {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        provider_name: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            provider_name: provider_name.into(),
        }
    }
}

#[async_trait]
impl LlmDriver for OpenAICompatDriver {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        // Build messages array
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                let role_str = match m.role {
                    sk_types::Role::System => "system",
                    sk_types::Role::User => "user",
                    sk_types::Role::Assistant => "assistant",
                };
                serde_json::json!({
                    "role": role_str,
                    "content": m.content.text_content(),
                })
            })
            .collect();

        // Build tools array
        let tools: Vec<serde_json::Value> = request
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.input_schema,
                    }
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });

        if !tools.is_empty() {
            body["tools"] = serde_json::Value::Array(tools);
            body["parallel_tool_calls"] = serde_json::Value::Bool(false);
        }

        let url = format!(
            "{}/v1/chat/completions",
            self.base_url.trim_end_matches('/')
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        let status = response.status().as_u16();

        if status == 429 {
            return Err(LlmError::RateLimited {
                retry_after_ms: 5000,
            });
        }
        if status == 529 || status == 503 {
            return Err(LlmError::Overloaded("Server overloaded".into()));
        }
        if status == 401 || status == 403 {
            return Err(LlmError::AuthError("Invalid API key".into()));
        }

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(LlmError::ApiError {
                status,
                message: text,
            });
        }

        let resp_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        // Parse response
        let choice = resp_json["choices"]
            .get(0)
            .ok_or_else(|| LlmError::ParseError("No choices in response".into()))?;

        let content = choice["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let mut tool_calls = Vec::new();
        if let Some(tcs) = choice["message"]["tool_calls"].as_array() {
            for tc in tcs {
                tool_calls.push(sk_types::ToolCall {
                    id: tc["id"].as_str().unwrap_or("").to_string(),
                    name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                    input: serde_json::from_str(
                        tc["function"]["arguments"].as_str().unwrap_or("{}"),
                    )
                    .unwrap_or(serde_json::json!({})),
                });
            }
        }

        let stop_reason = match choice["finish_reason"].as_str() {
            Some("stop") => StopReason::EndTurn,
            Some("tool_calls") => StopReason::ToolUse,
            Some("length") => StopReason::MaxTokens,
            Some("content_filter") => StopReason::ContentFilter,
            Some(other) => StopReason::Unknown(other.into()),
            None => {
                if !tool_calls.is_empty() {
                    StopReason::ToolUse
                } else {
                    StopReason::EndTurn
                }
            }
        };

        // OpenAI caches prompt prefixes automatically (≥1024 tokens) at 50% cost.
        // cached_tokens lives at usage.prompt_tokens_details.cached_tokens
        let cached_tokens = resp_json["usage"]["prompt_tokens_details"]["cached_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;

        if cached_tokens > 0 {
            tracing::debug!(
                cached_tokens,
                billed_input = resp_json["usage"]["prompt_tokens"].as_u64().unwrap_or(0),
                "OpenAI prompt cache hit"
            );
        }

        let usage = TokenUsage {
            prompt_tokens: resp_json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: resp_json["usage"]["completion_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            total_tokens: resp_json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
            cached_tokens,
        };

        Ok(CompletionResponse {
            content,
            tool_calls,
            stop_reason,
            usage,
        })
    }

    fn provider(&self) -> &str {
        &self.provider_name
    }
}
