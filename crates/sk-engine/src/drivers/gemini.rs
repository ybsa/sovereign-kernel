//! Google Gemini API driver.

use crate::llm_driver::{CompletionRequest, CompletionResponse, LlmDriver, LlmError, StopReason, TokenUsage};
use async_trait::async_trait;
use sk_types::{Role, ToolCall};
use serde::{Deserialize, Serialize};

/// Google Gemini API driver.
pub struct GeminiDriver {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl GeminiDriver {
    /// Create a new Gemini driver.
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            client: reqwest::Client::new(),
        }
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
    Text { text: String },
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
                system_text.push_str(&msg.content);
                continue;
            }

            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "model",
                Role::Tool => "user", // Tool results sent as user
                Role::System => unreachable!(),
            };

            let mut parts = Vec::new();
            if msg.role == Role::Tool {
                if let Some(ref id) = msg.tool_call_id {
                    let mut obj = serde_json::Map::new();
                    obj.insert("result".to_string(), serde_json::Value::String(msg.content.clone()));
                    parts.push(GeminiPart::FunctionResponse {
                        function_response: GeminiFunctionResponseData {
                            name: request.tools.iter()
                                .find(|t| t.name == *id)
                                .map(|t| t.name.clone())
                                .unwrap_or_else(|| id.clone()),
                            response: serde_json::Value::Object(obj),
                        },
                    });
                }
            } else if !msg.content.is_empty() {
                parts.push(GeminiPart::Text { text: msg.content.clone() });
            }

            for tool_call in &msg.tool_calls {
                parts.push(GeminiPart::FunctionCall {
                    function_call: GeminiFunctionCallData {
                        name: tool_call.name.clone(),
                        args: serde_json::from_str(&tool_call.arguments).unwrap_or(serde_json::json!({})),
                    },
                });
            }

            if !parts.is_empty() {
                contents.push(GeminiContent {
                    role: Some(role.to_string()),
                    parts,
                });
            }
        }

        let system_instruction = if !system_text.is_empty() {
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
                function_declarations: request.tools.iter().map(|t| GeminiFunctionDeclaration {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.parameters.clone(),
                }).collect(),
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

        for attempt in 0..=3 {
            let url = format!("{}/v1beta/models/{}:generateContent", self.base_url.trim_end_matches('/'), request.model);
            let resp = self.client.post(&url)
                .header("x-goog-api-key", &self.api_key)
                .json(&gemini_request)
                .send()
                .await
                .map_err(|e| LlmError::NetworkError(e.to_string()))?;

            let status = resp.status().as_u16();
            if status == 429 || status == 503 {
                if attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                    continue;
                }
                return Err(LlmError::RateLimited { retry_after_ms: 5000 });
            }

            if !resp.status().is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                if let Ok(err_json) = serde_json::from_str::<GeminiErrorResponse>(&err_text) {
                    return Err(LlmError::ApiError { status, message: err_json.error.message });
                }
                return Err(LlmError::ApiError { status, message: err_text });
            }

            let resp_text = resp.text().await.map_err(|e| LlmError::NetworkError(e.to_string()))?;
            let gemini_resp: GeminiResponse = serde_json::from_str(&resp_text)
                .map_err(|e| LlmError::ParseError(e.to_string()))?;

            let candidate = gemini_resp.candidates.into_iter().next()
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
                                arguments: function_call.args.to_string(),
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

            let usage = gemini_resp.usage_metadata.map(|u| TokenUsage {
                prompt_tokens: u.prompt_token_count,
                completion_tokens: u.candidates_token_count,
                total_tokens: u.prompt_token_count + u.candidates_token_count,
            }).unwrap_or_default();

            return Ok(CompletionResponse {
                content: final_content,
                tool_calls,
                stop_reason,
                usage,
            });
        }

        Err(LlmError::NetworkError("Max retries exceeded".to_string()))
    }
}
