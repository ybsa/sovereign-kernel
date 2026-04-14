//! Google Gemini API driver.

use crate::llm_driver::{
    CompletionRequest, CompletionResponse, LlmDriver, LlmError, StopReason, TokenUsage,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sk_types::{Role, ToolCall};

/// Google Gemini API driver.
pub struct GeminiDriver {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
    /// Gemini context cache: maps sha256(system_prompt) → (cache_name, expires_at_unix_secs)
    /// Gemini requires you to POST a cachedContents object first, then reference it by name.
    /// Min cacheable tokens: 4,096 (Flash) / 32,768 (Pro). TTL default: 1 hour.
    cache: std::sync::Mutex<std::collections::HashMap<String, (String, u64)>>,
}

impl GeminiDriver {
    /// Create a new Gemini driver.
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            client: reqwest::Client::new(),
            cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Hash a system prompt for use as a cache key.
    fn cache_key(system_text: &str) -> String {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        system_text.hash(&mut h);
        format!("{:x}", h.finish())
    }

    /// Look up an existing live cache entry for this system prompt.
    fn get_cached(&self, key: &str) -> Option<String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let cache = self.cache.lock().unwrap();
        cache.get(key).and_then(|(name, expires)| {
            // Keep a 60s safety margin before expiry
            if *expires > now + 60 { Some(name.clone()) } else { None }
        })
    }

    /// Store a new cache entry.
    fn store_cached(&self, key: String, name: String, ttl_secs: u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key, (name, now + ttl_secs));
    }

    /// Try to create a Gemini cached content object for a system prompt.
    /// Returns the cache name on success. Silently returns None on failure
    /// (caching is an optimisation — we always fall back to uncached).
    async fn ensure_cache(&self, model: &str, system_text: &str) -> Option<String> {
        // Gemini only caches if the content is large enough (≥4K tokens ≈ 16K chars rough estimate)
        if system_text.len() < 4096 {
            return None;
        }
        let key = Self::cache_key(system_text);
        if let Some(name) = self.get_cached(&key) {
            return Some(name);
        }

        let url = format!(
            "{}/v1beta/cachedContents",
            self.base_url.trim_end_matches('/')
        );

        // TTL: 1 hour
        let body = serde_json::json!({
            "model": format!("models/{}", model),
            "systemInstruction": {
                "parts": [{ "text": system_text }]
            },
            "ttl": "3600s"
        });

        let resp = self.client
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .ok()?;

        if !resp.status().is_success() {
            return None;
        }

        let json: serde_json::Value = resp.json().await.ok()?;
        let name = json["name"].as_str()?.to_string();
        tracing::debug!(cache_name = %name, "Gemini context cache created");
        self.store_cached(key, name.clone(), 3600);
        Some(name)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<GeminiToolConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum GeminiPart {
    Text {
        text: String,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: GeminiFunctionCallData,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: GeminiFunctionResponseData,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GeminiFunctionCallData {
    name: String,
    args: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GeminiFunctionResponseData {
    name: String,
    response: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GeminiToolConfig {
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Debug, Serialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    #[serde(default)]
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    content: Option<GeminiContent>,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsageMetadata {
    #[serde(default)]
    prompt_token_count: u32,
    #[serde(default)]
    candidates_token_count: u32,
    #[serde(default)]
    cached_content_token_count: u32,
}

#[derive(Debug, Deserialize)]
struct GeminiErrorResponse {
    error: GeminiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct GeminiErrorDetail {
    message: String,
}

#[async_trait]
impl LlmDriver for GeminiDriver {
    fn provider(&self) -> &str {
        "gemini"
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let mut contents = Vec::new();
        let mut system_text = String::new();

        // 1. Gather all system messages and construct main contents array
        for msg in &request.messages {
            if msg.role == Role::System {
                if !system_text.is_empty() {
                    system_text.push('\n');
                }
                system_text.push_str(&msg.content.text_content());
                continue;
            }

            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "model",
                Role::System => unreachable!(),
            };

            let mut parts = Vec::new();
            match &msg.content {
                sk_types::message::MessageContent::Text(t) => {
                    if !t.is_empty() {
                        parts.push(GeminiPart::Text { text: t.clone() });
                    }
                }
                sk_types::message::MessageContent::Blocks(b) => {
                    for block in b {
                        match block {
                            sk_types::message::ContentBlock::Text { text } => {
                                parts.push(GeminiPart::Text { text: text.clone() });
                            }
                            sk_types::message::ContentBlock::ToolUse { id: _, name, input } => {
                                parts.push(GeminiPart::FunctionCall {
                                    function_call: GeminiFunctionCallData {
                                        name: name.clone(),
                                        args: input.clone(),
                                    },
                                });
                            }
                            sk_types::message::ContentBlock::ToolResult {
                                tool_use_id,
                                content,
                                ..
                            } => {
                                let mut obj = serde_json::Map::new();
                                obj.insert(
                                    "result".to_string(),
                                    serde_json::Value::String(content.text_content()),
                                );
                                parts.push(GeminiPart::FunctionResponse {
                                    function_response: GeminiFunctionResponseData {
                                        name: request
                                            .tools
                                            .iter()
                                            .find(|t| t.name == *tool_use_id)
                                            .map(|t| t.name.clone())
                                            .unwrap_or_else(|| tool_use_id.clone()),
                                        response: serde_json::Value::Object(obj),
                                    },
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }

            if !parts.is_empty() {
                contents.push(GeminiContent {
                    role: Some(role.to_string()),
                    parts,
                });
            }
        }

        // Try to use Gemini context caching for large system prompts.
        // If caching succeeds, the system_instruction is omitted from the request
        // and the cached content name is passed instead (cheaper on all subsequent calls).
        let cache_name = if !system_text.is_empty() {
            self.ensure_cache(&request.model, &system_text).await
        } else {
            None
        };

        let system_instruction = if cache_name.is_none() && !system_text.is_empty() {
            Some(GeminiContent {
                role: None,
                parts: vec![GeminiPart::Text { text: system_text }],
            })
        } else {
            None
        };

        let tools = if request.tools.is_empty() {
            Vec::new()
        } else {
            vec![GeminiToolConfig {
                function_declarations: request
                    .tools
                    .iter()
                    .map(|t| GeminiFunctionDeclaration {
                        name: t.name.clone(),
                        description: t.description.clone(),
                        parameters: t.input_schema.clone(),
                    })
                    .collect(),
            }]
        };

        let gemini_request = GeminiRequest {
            contents,
            system_instruction,
            tools,
            generation_config: Some(GenerationConfig {
                temperature: Some(request.temperature),
                max_output_tokens: Some(request.max_tokens),
            }),
        };

        // Attach the cache name if we got one — Gemini serves the cached system
        // prompt at a significant discount instead of re-processing it every call.
        let gemini_request_value = if let Some(ref name) = cache_name {
            let mut v = serde_json::to_value(&gemini_request)
                .unwrap_or(serde_json::Value::Object(Default::default()));
            v["cachedContent"] = serde_json::Value::String(name.clone());
            v
        } else {
            serde_json::to_value(&gemini_request)
                .unwrap_or(serde_json::Value::Object(Default::default()))
        };
        let _ = gemini_request; // consumed into gemini_request_value

        for attempt in 0..=5 {
            let url = format!(
                "{}/v1beta/models/{}:generateContent",
                self.base_url.trim_end_matches('/'),
                request.model
            );
            let resp = self
                .client
                .post(&url)
                .header("x-goog-api-key", &self.api_key)
                .json(&gemini_request_value)
                .send()
                .await
                .map_err(|e| LlmError::NetworkError(e.to_string()))?;

            let status = resp.status().as_u16();
            if status == 429 || status == 503 {
                if attempt < 5 {
                    // Exponential backoff: 5s, 10s, 20s, 40s, 60s
                    let delay = std::cmp::min(5000 * (1 << attempt), 60000);
                    tracing::warn!(attempt, delay_ms = delay, "Rate limited, retrying...");
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    continue;
                }
                return Err(LlmError::RateLimited {
                    retry_after_ms: 60000,
                });
            }

            if !resp.status().is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                if let Ok(err_json) = serde_json::from_str::<GeminiErrorResponse>(&err_text) {
                    return Err(LlmError::ApiError {
                        status,
                        message: err_json.error.message,
                    });
                }
                return Err(LlmError::ApiError {
                    status,
                    message: err_text,
                });
            }

            let resp_text = resp
                .text()
                .await
                .map_err(|e| LlmError::NetworkError(e.to_string()))?;
            let gemini_resp: GeminiResponse = serde_json::from_str(&resp_text)
                .map_err(|e| LlmError::ParseError(e.to_string()))?;

            let candidate = gemini_resp
                .candidates
                .into_iter()
                .next()
                .ok_or_else(|| LlmError::ParseError("No candidates in response".to_string()))?;

            let mut final_content = String::new();
            let mut tool_calls = Vec::new();

            if let Some(content) = candidate.content {
                for part in content.parts {
                    match part {
                        GeminiPart::Text { text } => {
                            final_content.push_str(&text);
                        }
                        GeminiPart::FunctionCall { function_call } => {
                            let id = format!("call_{}", uuid::Uuid::new_v4().simple());
                            tool_calls.push(ToolCall {
                                id,
                                name: function_call.name,
                                input: function_call.args.clone(),
                            });
                        }
                        _ => {}
                    }
                }
            }

            let stop_reason = if !tool_calls.is_empty() {
                StopReason::ToolUse
            } else {
                match candidate.finish_reason.as_deref() {
                    Some("MAX_TOKENS") => StopReason::MaxTokens,
                    _ => StopReason::EndTurn,
                }
            };

            let usage = gemini_resp
                .usage_metadata
                .map(|u| {
                    if u.cached_content_token_count > 0 {
                        tracing::debug!(
                            cached = u.cached_content_token_count,
                            "Gemini context cache hit"
                        );
                    }
                    TokenUsage {
                        prompt_tokens: u.prompt_token_count,
                        completion_tokens: u.candidates_token_count,
                        total_tokens: u.prompt_token_count + u.candidates_token_count,
                        cached_tokens: u.cached_content_token_count,
                    }
                })
                .unwrap_or_default();

            return Ok(CompletionResponse {
                content: final_content,
                tool_calls,
                stop_reason,
                usage,
            });
        }
        Err(LlmError::NetworkError("Max retries exceeded".to_string()))
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
        _handler: &crate::llm_driver::StreamHandler,
    ) -> Result<CompletionResponse, LlmError> {
        // Fallback to non-streaming for now
        self.complete(request).await
    }
}
