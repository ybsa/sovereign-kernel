//! Anthropic Claude API driver.

use crate::llm_driver::{
    CompletionRequest, CompletionResponse, LlmDriver, LlmError, StopReason, TokenUsage,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sk_types::{Role, ToolCall};

/// Anthropic Claude API driver.
pub struct AnthropicDriver {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl AnthropicDriver {
    /// Create a new Anthropic driver.
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            api_key,
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<ApiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ApiTool>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ApiMessage {
    role: String,
    content: Vec<ApiContentBlock>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ApiContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "image")]
    Image {
        source: ApiImageSource,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<Vec<ApiContentBlock>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

#[derive(Debug, Serialize)]
pub struct ApiImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

#[derive(Debug, Serialize)]
struct ApiTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    content: Vec<ResponseContentBlock>,
    stop_reason: String,
    usage: ApiUsage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ResponseContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
}

#[derive(Debug, Deserialize)]
struct ApiUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: String,
}

#[async_trait]
impl LlmDriver for AnthropicDriver {
    fn provider(&self) -> &str {
        "anthropic"
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        // Find system prompt
        let system = request
            .messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.text_content());

        // Map messages
        let mut api_messages = Vec::new();
        for msg in &request.messages {
            if msg.role == Role::System {
                continue;
            }

            let mut blocks = Vec::new();
            match &msg.content {
                sk_types::message::MessageContent::Text(t) => {
                    blocks.push(ApiContentBlock::Text { text: t.clone() });
                }
                sk_types::message::MessageContent::Blocks(b) => {
                    for block in b {
                        match block {
                            sk_types::message::ContentBlock::Text { text } => {
                                blocks.push(ApiContentBlock::Text { text: text.clone() });
                            }
                            sk_types::message::ContentBlock::Image { media_type, data } => {
                                blocks.push(ApiContentBlock::Image {
                                    source: ApiImageSource {
                                        source_type: "base64".to_string(),
                                        media_type: media_type.clone(),
                                        data: data.clone(),
                                    },
                                });
                            }
                            sk_types::message::ContentBlock::ToolUse { id, name, input } => {
                                blocks.push(ApiContentBlock::ToolUse {
                                    id: id.clone(),
                                    name: name.clone(),
                                    input: input.clone(),
                                });
                            }
                            sk_types::message::ContentBlock::ToolResult {
                                tool_use_id,
                                content,
                                is_error,
                            } => {
                                let result_blocks = match content {
                                    sk_types::MessageContent::Text(t) => {
                                        if t.is_empty() {
                                            None
                                        } else {
                                            Some(vec![ApiContentBlock::Text { text: t.clone() }])
                                        }
                                    }
                                    sk_types::MessageContent::Blocks(bs) => {
                                        let mut mapped = Vec::new();
                                        for b in bs {
                                            match b {
                                                sk_types::ContentBlock::Text { text } => {
                                                    mapped.push(ApiContentBlock::Text {
                                                        text: text.clone(),
                                                    });
                                                }
                                                sk_types::ContentBlock::Image {
                                                    media_type,
                                                    data,
                                                } => {
                                                    mapped.push(ApiContentBlock::Image {
                                                        source: ApiImageSource {
                                                            source_type: "base64".to_string(),
                                                            media_type: media_type.clone(),
                                                            data: data.clone(),
                                                        },
                                                    });
                                                }
                                                _ => {}
                                            }
                                        }
                                        if mapped.is_empty() {
                                            None
                                        } else {
                                            Some(mapped)
                                        }
                                    }
                                };
                                blocks.push(ApiContentBlock::ToolResult {
                                    tool_use_id: tool_use_id.clone(),
                                    content: result_blocks,
                                    is_error: if *is_error { Some(true) } else { None },
                                });
                            }
                            sk_types::message::ContentBlock::Unknown
                            | sk_types::message::ContentBlock::Thinking { .. } => {}
                        }
                    }
                }
            }

            // Anthropic requires tool results from user
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => "user",
            };

            if !blocks.is_empty() {
                api_messages.push(ApiMessage {
                    role: role.to_string(),
                    content: blocks,
                });
            }
        }

        let api_tools: Vec<ApiTool> = request
            .tools
            .iter()
            .map(|t| ApiTool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
            })
            .collect();

        let api_request = ApiRequest {
            model: request.model.clone(),
            max_tokens: request.max_tokens,
            system,
            messages: api_messages,
            tools: api_tools,
            temperature: request.temperature,
        };

        // Retry loop
        for attempt in 0..=3 {
            let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
            let resp = self
                .client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("anthropic-beta", "computer-use-2024-10-22,prompt-caching-2024-07-31")
                .json(&api_request)
                .send()
                .await
                .map_err(|e| LlmError::NetworkError(e.to_string()))?;

            let status = resp.status().as_u16();
            if status == 429 || status == 529 {
                if attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                    continue;
                }
                return Err(LlmError::RateLimited {
                    retry_after_ms: 5000,
                });
            }

            if !resp.status().is_success() {
                let err_text = resp.text().await.unwrap_or_default();
                if let Ok(err_json) = serde_json::from_str::<ApiErrorResponse>(&err_text) {
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
            let api_resp: ApiResponse = serde_json::from_str(&resp_text)
                .map_err(|e| LlmError::ParseError(e.to_string()))?;

            let mut final_content = String::new();
            let mut tool_calls = Vec::new();

            for block in api_resp.content {
                match block {
                    ResponseContentBlock::Text { text } => final_content.push_str(&text),
                    ResponseContentBlock::Thinking { thinking } => {
                        final_content.push_str(&format!("<think>\n{thinking}\n</think>\n"));
                    }
                    ResponseContentBlock::ToolUse { id, name, input } => {
                        tool_calls.push(ToolCall {
                            id,
                            name,
                            input: input.clone(),
                        });
                    }
                }
            }

            let stop_reason = match api_resp.stop_reason.as_str() {
                "end_turn" => StopReason::EndTurn,
                "tool_use" => StopReason::ToolUse,
                "max_tokens" => StopReason::MaxTokens,
                _ => StopReason::EndTurn, // fallback
            };

            return Ok(CompletionResponse {
                content: final_content,
                tool_calls,
                stop_reason,
                usage: TokenUsage {
                    prompt_tokens: api_resp.usage.input_tokens,
                    completion_tokens: api_resp.usage.output_tokens,
                    total_tokens: api_resp.usage.input_tokens + api_resp.usage.output_tokens,
                },
            });
        }

        Err(LlmError::NetworkError("Max retries exceeded".to_string()))
    }
}
