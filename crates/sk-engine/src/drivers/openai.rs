//! OpenAI-compatible API driver.
//!
//! Works with OpenAI, Ollama, vLLM, and any other OpenAI-compatible endpoint.

use crate::llm_driver::{
    CompletionRequest, CompletionResponse, LlmDriver, LlmError, StopReason, StreamHandler,
    TokenUsage,
};
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sk_types::tool::ToolCall;
use tracing::{debug, warn};

/// OpenAI-compatible API driver.
pub struct OpenAIDriver {
    api_key: String,
    base_url: String,
    provider_name: String,
    client: reqwest::Client,
}

impl OpenAIDriver {
    /// Create a new OpenAI-compatible driver.
    pub fn new(api_key: String, base_url: String, provider_name: String) -> Self {
        Self {
            api_key,
            base_url,
            provider_name,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct OaiRequest {
    model: String,
    messages: Vec<OaiMessage>,
    max_tokens: u32,
    temperature: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OaiTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    stream: bool,
    /// Force the model to only call one tool at a time.
    /// Many providers (Ollama, Groq, local models) don't support parallel tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    parallel_tool_calls: Option<bool>,
}

#[derive(Debug, Serialize)]
struct OaiMessage {
    role: String,
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OaiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OaiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OaiFunction,
}

#[derive(Debug, Serialize, Deserialize)]
struct OaiFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct OaiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OaiToolDef,
}

#[derive(Debug, Serialize)]
struct OaiToolDef {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OaiResponse {
    choices: Vec<OaiChoice>,
    #[allow(dead_code)]
    usage: Option<OaiUsage>,
}

#[derive(Debug, Deserialize)]
struct OaiChoice {
    message: OaiResponseMessage,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OaiResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OaiToolCall>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OaiUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[async_trait]
impl LlmDriver for OpenAIDriver {
    fn provider(&self) -> &str {
        &self.provider_name
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let oai_request = self.prepare_oai_request(&request);

        if self.provider_name == "nvidia" {
            debug!(
                request = %serde_json::to_string_pretty(&oai_request).unwrap_or_default(),
                "NVIDIA Request Body"
            );
        }

        let max_retries = 8;
        for attempt in 0..=max_retries {
            let url = format!("{}/chat/completions", self.base_url);
            let mut req_builder = self
                .client
                .post(&url)
                .header("content-type", "application/json")
                .json(&oai_request);

            if !self.api_key.is_empty() {
                req_builder =
                    req_builder.header("authorization", format!("Bearer {}", self.api_key));
            }

            let resp = req_builder
                .send()
                .await
                .map_err(|e| LlmError::NetworkError(e.to_string()))?;

            let status = resp.status().as_u16();
            if status == 429 {
                if attempt < max_retries {
                    let retry_ms = (attempt + 1) as u64 * 5000;
                    tokio::time::sleep(std::time::Duration::from_millis(retry_ms)).await;
                    continue;
                }
                return Err(LlmError::RateLimited {
                    retry_after_ms: 5000,
                });
            }

            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(LlmError::ApiError {
                    status,
                    message: body,
                });
            }

            let body = resp
                .text()
                .await
                .map_err(|e| LlmError::NetworkError(e.to_string()))?;

            let oai_response: OaiResponse =
                serde_json::from_str(&body).map_err(|e| LlmError::ParseError(e.to_string()))?;

            let choice = oai_response
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| LlmError::ParseError("No choices in response".to_string()))?;

            let content = choice.message.content.clone().unwrap_or_default();
            let mut tool_calls = Vec::new();

            if let Some(calls) = choice.message.tool_calls {
                for call in calls {
                    tool_calls.push(ToolCall {
                        id: call.id,
                        name: call.function.name,
                        input: serde_json::from_str(&call.function.arguments)
                            .unwrap_or(serde_json::json!({})),
                    });
                }
            }

            return Ok(CompletionResponse {
                content,
                stop_reason: StopReason::EndTurn,
                tool_calls,
                usage: TokenUsage::default(),
            });
        }

        Err(LlmError::ApiError {
            status: 0_u16,
            message: "Max retries exceeded".to_string(),
        })
    }

    async fn complete_stream(
        &self,
        request: CompletionRequest,
        handler: &StreamHandler,
    ) -> Result<CompletionResponse, LlmError> {
        let mut oai_request = self.prepare_oai_request(&request);
        oai_request.stream = true;

        let url = format!("{}/chat/completions", self.base_url);
        let mut req_builder = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&oai_request);

        if !self.api_key.is_empty() {
            req_builder =
                req_builder.header("authorization", format!("Bearer {}", self.api_key));
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| LlmError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::ApiError { status, message: body });
        }

        let mut full_content = String::new();
        let mut stream = resp.bytes_stream();
        let mut accumulated_text = String::new();

        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|e| LlmError::NetworkError(e.to_string()))?;
            let text = String::from_utf8_lossy(&chunk);
            accumulated_text.push_str(&text);

            while let Some(pos) = accumulated_text.find('\n') {
                let line = accumulated_text[..pos].trim().to_string();
                let remaining = accumulated_text[pos + 1..].to_string();
                accumulated_text = remaining;

                if line.is_empty() || !line.starts_with("data: ") {
                    continue;
                }

                let data = &line[6..];
                if data == "[DONE]" {
                    break;
                }

                if let Ok(val) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = val["choices"][0]["delta"]["content"].as_str() {
                        full_content.push_str(content);
                        handler(content);
                    }
                }
            }
        }

        Ok(CompletionResponse {
            content: full_content,
            stop_reason: StopReason::EndTurn,
            tool_calls: vec![],
            usage: TokenUsage::default(),
        })
    }
}

impl OpenAIDriver {
    fn prepare_oai_request(&self, request: &CompletionRequest) -> OaiRequest {
        let mut oai_messages: Vec<OaiMessage> = Vec::new();
        for msg in &request.messages {
            let role_str = match msg.role {
                sk_types::Role::System => "system",
                sk_types::Role::User => "user",
                sk_types::Role::Assistant => "assistant",
            };
            oai_messages.push(OaiMessage {
                role: role_str.to_string(),
                content: Some(msg.content.text_content()),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        let oai_tools: Vec<OaiTool> = request
            .tools
            .iter()
            .map(|t| OaiTool {
                tool_type: "function".to_string(),
                function: OaiToolDef {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.input_schema.clone(),
                },
            })
            .collect();

        OaiRequest {
            model: request.model.clone(),
            messages: oai_messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            tools: oai_tools,
            tool_choice: if request.tools.is_empty() { None } else { Some(serde_json::json!("auto")) },
            stream: false,
            parallel_tool_calls: if request.tools.is_empty() { None } else { Some(false) },
        }
    }
}

// Kept for potential future use with models that emit tool calls as raw JSON text.
#[allow(dead_code)]
fn recover_tool_calls_from_text(text: &str) -> Option<Vec<ToolCall>> {
    let mut tool_calls = Vec::new();
    let mut current_text = text;

    // Look for JSON-like structures that look like tool calls:
    // {"name": "...", "input": {...}} or {"name": "...", "parameters": {...}}
    while let Some(start) = current_text.find('{') {
        current_text = &current_text[start..];

        // Find matching closing brace (simple heuristic)
        let mut brace_count = 0;
        let mut end_pos = None;
        for (i, c) in current_text.char_indices() {
            if c == '{' {
                brace_count += 1;
            } else if c == '}' {
                brace_count -= 1;
                if brace_count == 0 {
                    end_pos = Some(i + 1);
                    break;
                }
            }
        }

        if let Some(end) = end_pos {
            let potential_json = &current_text[..end];
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(potential_json) {
                if let Some(obj) = val.as_object() {
                    if let Some(name) = obj.get("name").and_then(|n| n.as_str()) {
                        // Support both "input" and "parameters" (common in small models)
                        let input = obj
                            .get("input")
                            .or_else(|| obj.get("parameters"))
                            .cloned()
                            .unwrap_or(serde_json::json!({}));

                        tool_calls.push(ToolCall {
                            id: format!("recovered_{}", &uuid::Uuid::new_v4().to_string()[..8]),
                            name: name.to_string(),
                            input,
                        });
                    }
                }
            }
            current_text = &current_text[end..];
        } else {
            break;
        }
    }

    if tool_calls.is_empty() {
        None
    } else {
        Some(tool_calls)
    }
}

/// Extract the max_tokens limit from an API error message.
/// Looks for patterns like: `must be less than or equal to \`8192\``
#[allow(dead_code)]
fn extract_max_tokens_limit(body: &str) -> Option<u32> {
    // Pattern: "must be <= `N`" or "must be less than or equal to `N`"
    let patterns = [
        "less than or equal to `",
        "must be <= `",
        "maximum value for `max_tokens` is `",
    ];
    for pat in &patterns {
        if let Some(idx) = body.find(pat) {
            let after = &body[idx + pat.len()..];
            let end = after
                .find('`')
                .or_else(|| after.find('"'))
                .unwrap_or(after.len());
            if let Ok(n) = after[..end].trim().parse::<u32>() {
                return Some(n);
            }
        }
    }
    None
}

/// Some models (e.g. Llama 3.3) generate tool calls as XML: `<function=NAME ARGS></function>`
/// instead of the proper JSON format. Groq rejects these with `tool_use_failed` but includes
/// the raw generation. We parse it and construct a proper CompletionResponse.
#[allow(dead_code)]
fn parse_groq_failed_tool_call(body: &str) -> Option<CompletionResponse> {
    let json_body: serde_json::Value = serde_json::from_str(body).ok()?;
    let failed = json_body
        .pointer("/error/failed_generation")
        .and_then(|v| v.as_str())?;

    // Parse all tool calls from the failed generation.
    // Format: <function=tool_name{"arg":"val"}></function> or <function=tool_name {"arg":"val"}></function>
    let mut tool_calls = Vec::new();
    let mut remaining = failed;

    while let Some(start) = remaining.find("<function=") {
        remaining = &remaining[start + 10..]; // skip "<function="
                                              // Find the end tag
        let end = remaining.find("</function>")?;
        let mut call_content = &remaining[..end];
        remaining = &remaining[end + 11..]; // skip "</function>"

        // Strip trailing ">" from the XML opening tag close
        call_content = call_content.strip_suffix('>').unwrap_or(call_content);

        // Split into name and args: "tool_name{"arg":"val"}" or "tool_name {"arg":"val"}"
        let (name, args) = if let Some(brace_pos) = call_content.find('{') {
            let name = call_content[..brace_pos].trim();
            let args = &call_content[brace_pos..];
            (name, args)
        } else {
            // No args — just a tool name
            (call_content.trim(), "{}")
        };

        // Parse args as JSON Value
        let args_value: serde_json::Value =
            serde_json::from_str(args).unwrap_or(serde_json::json!({}));

        tool_calls.push(ToolCall {
            id: format!("groq_recovered_{}", tool_calls.len()),
            name: name.to_string(),
            input: args_value,
        });
    }

    if tool_calls.is_empty() {
        // No tool calls found — the model generated plain text but Groq rejected it.
        // Return it as a normal text response instead of failing.
        if !failed.trim().is_empty() {
            warn!("Recovering plain text from Groq failed_generation (no tool calls)");
            return Some(CompletionResponse {
                content: failed.to_string(),
                tool_calls: vec![],
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                    cached_tokens: 0,
                },
            });
        }
        return None;
    }

    Some(CompletionResponse {
        content: String::new(),
        tool_calls,
        stop_reason: StopReason::ToolUse,
        usage: TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            cached_tokens: 0,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_driver_creation() {
        let driver = OpenAIDriver::new(
            "test-key".to_string(),
            "http://localhost".to_string(),
            "openai".to_string(),
        );
        assert_eq!(driver.api_key.as_str(), "test-key");
    }

    #[test]
    fn test_parse_groq_failed_tool_call() {
        let body = r#"{"error":{"message":"Failed to call a function.","type":"invalid_request_error","code":"tool_use_failed","failed_generation":"<function=web_fetch{\"url\": \"https://example.com\"}></function>\n"}}"#;
        let result = parse_groq_failed_tool_call(body);
        assert!(result.is_some());
        let resp = result.unwrap();
        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].name, "web_fetch");
        assert!(resp.tool_calls[0]
            .input
            .to_string()
            .contains("https://example.com"));
    }

    #[test]
    fn test_parse_groq_failed_tool_call_with_space() {
        let body = r#"{"error":{"message":"Failed","type":"invalid_request_error","code":"tool_use_failed","failed_generation":"<function=shell_exec {\"command\": \"ls -la\"}></function>"}}"#;
        let result = parse_groq_failed_tool_call(body);
        assert!(result.is_some());
        let resp = result.unwrap();
        assert_eq!(resp.tool_calls[0].name, "shell_exec");
    }
}
