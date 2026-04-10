//! Modular Tool Registry — dispatches tool calls to individual handlers.
//!
//! Every tool implements the `ToolHandler` trait and is registered in the `ToolRegistry`.
//! This replaces the monolithic match statement that previously lived in executor.rs.

use async_trait::async_trait;
use serde_json::Value;
use sk_types::config::ExecutionMode;
use sk_types::{AgentId, SovereignResult, ToolCall, ToolResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::executor::healer_result;
use crate::SovereignKernel;

/// Context passed to every tool handler.
pub struct ToolContext {
    pub kernel: Arc<SovereignKernel>,
    pub agent_id: AgentId,
    pub mode: ExecutionMode,
    pub workspaces_dir: PathBuf,
    pub policy: sk_types::config::ExecPolicy,
}

impl ToolContext {
    /// Whether the agent is in unrestricted mode.
    fn is_unrestricted(&self) -> bool {
        self.mode == ExecutionMode::Unrestricted
    }
}

/// Trait that every tool handler must implement.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    fn name(&self) -> &str;
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult>;
}

/// Central registry for tool dispatch.
pub struct ToolRegistry {
    handlers: HashMap<String, Box<dyn ToolHandler>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a single handler.
    pub fn register(mut self, handler: Box<dyn ToolHandler>) -> Self {
        self.handlers.insert(handler.name().to_string(), handler);
        self
    }

    /// Register all standard tool handlers.
    pub fn register_all(self) -> Self {
        self.register(Box::new(ShellExecHandler))
            .register(Box::new(ReadFileHandler))
            .register(Box::new(WriteFileHandler))
            .register(Box::new(ListDirHandler))
            .register(Box::new(DeleteFileHandler))
            .register(Box::new(MoveFileHandler))
            .register(Box::new(CopyFileHandler))
            .register(Box::new(RememberHandler))
            .register(Box::new(RecallHandler))
            .register(Box::new(ForgetHandler))
            .register(Box::new(CodeExecHandler))
            .register(Box::new(WebSearchHandler))
            .register(Box::new(WebFetchHandler))
            .register(Box::new(SharedMemoryStoreHandler))
            .register(Box::new(SharedMemoryRecallHandler))
            .register(Box::new(GetSkillHandler))
            .register(Box::new(ListSkillsHandler))
    }

    /// Dispatch a tool call to the appropriate handler.
    pub async fn dispatch(
        &self,
        ctx: ToolContext,
        tool_call: ToolCall,
        input: Value,
    ) -> SovereignResult<ToolResult> {
        if let Some(handler) = self.handlers.get(&tool_call.name) {
            handler.handle(ctx, input).await
        } else {
            Ok(healer_result(
                &tool_call.name,
                format!("Unknown tool: '{}'", tool_call.name),
                true,
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Tool Handler Implementations
// ---------------------------------------------------------------------------

struct ShellExecHandler;
#[async_trait]
impl ToolHandler for ShellExecHandler {
    fn name(&self) -> &str { "shell_exec" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let command = input.get("command").and_then(|v| v.as_str()).unwrap_or("");
        let cwd = input.get("cwd").and_then(|v| v.as_str());
        let timeout = input.get("timeout_secs").and_then(|v| v.as_u64());
        match sk_tools::shell::handle_shell_exec(&ctx.policy, command, cwd, timeout).await {
            Ok(out) => Ok(healer_result("shell_exec", out, false)),
            Err(e) => Ok(healer_result("shell_exec", format!("Error: {}", e), true)),
        }
    }
}

struct ReadFileHandler;
#[async_trait]
impl ToolHandler for ReadFileHandler {
    fn name(&self) -> &str { "read_file" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::file_ops::handle_read_file(&ctx.workspaces_dir, path, ctx.is_unrestricted()) {
            Ok(content) => Ok(healer_result("read_file", content, false)),
            Err(e) => Ok(healer_result("read_file", format!("Error: {}", e), true)),
        }
    }
}

struct WriteFileHandler;
#[async_trait]
impl ToolHandler for WriteFileHandler {
    fn name(&self) -> &str { "write_file" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let content = input.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let append = input.get("append").and_then(|v| v.as_bool()).unwrap_or(false);
        match sk_tools::file_ops::handle_write_file(&ctx.workspaces_dir, path, content, append, ctx.is_unrestricted()) {
            Ok(msg) => Ok(healer_result("write_file", msg, false)),
            Err(e) => Ok(healer_result("write_file", format!("Error: {}", e), true)),
        }
    }
}

struct ListDirHandler;
#[async_trait]
impl ToolHandler for ListDirHandler {
    fn name(&self) -> &str { "list_dir" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        match sk_tools::file_ops::handle_list_dir(&ctx.workspaces_dir, path, ctx.is_unrestricted()) {
            Ok(out) => Ok(healer_result("list_dir", out, false)),
            Err(e) => Ok(healer_result("list_dir", format!("Error: {}", e), true)),
        }
    }
}

struct DeleteFileHandler;
#[async_trait]
impl ToolHandler for DeleteFileHandler {
    fn name(&self) -> &str { "delete_file" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::file_ops::handle_delete_file(&ctx.workspaces_dir, path, ctx.is_unrestricted()) {
            Ok(msg) => Ok(healer_result("delete_file", msg, false)),
            Err(e) => Ok(healer_result("delete_file", format!("Error: {}", e), true)),
        }
    }
}

struct MoveFileHandler;
#[async_trait]
impl ToolHandler for MoveFileHandler {
    fn name(&self) -> &str { "move_file" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let src = input.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let dst = input.get("destination").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::file_ops::handle_move_file(&ctx.workspaces_dir, src, dst, ctx.is_unrestricted()) {
            Ok(msg) => Ok(healer_result("move_file", msg, false)),
            Err(e) => Ok(healer_result("move_file", format!("Error: {}", e), true)),
        }
    }
}

struct CopyFileHandler;
#[async_trait]
impl ToolHandler for CopyFileHandler {
    fn name(&self) -> &str { "copy_file" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let src = input.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let dst = input.get("destination").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::file_ops::handle_copy_file(&ctx.workspaces_dir, src, dst, ctx.is_unrestricted()) {
            Ok(msg) => Ok(healer_result("copy_file", msg, false)),
            Err(e) => Ok(healer_result("copy_file", format!("Error: {}", e), true)),
        }
    }
}

struct RememberHandler;
#[async_trait]
impl ToolHandler for RememberHandler {
    fn name(&self) -> &str { "remember" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let content = input.get("content").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::memory_tools::handle_remember(&ctx.kernel.memory, ctx.agent_id, content) {
            Ok(msg) => Ok(healer_result("remember", msg, false)),
            Err(e) => Ok(healer_result("remember", format!("Error: {}", e), true)),
        }
    }
}

struct RecallHandler;
#[async_trait]
impl ToolHandler for RecallHandler {
    fn name(&self) -> &str { "recall" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        match sk_tools::memory_tools::handle_recall(&ctx.kernel.memory, ctx.agent_id, query, limit) {
            Ok(out) => Ok(healer_result("recall", out, false)),
            Err(e) => Ok(healer_result("recall", format!("Error: {}", e), true)),
        }
    }
}

struct ForgetHandler;
#[async_trait]
impl ToolHandler for ForgetHandler {
    fn name(&self) -> &str { "forget" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let memory_id = input.get("memory_id").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::memory_tools::handle_forget(&ctx.kernel.memory, memory_id) {
            Ok(msg) => Ok(healer_result("forget", msg, false)),
            Err(e) => Ok(healer_result("forget", format!("Error: {}", e), true)),
        }
    }
}

struct CodeExecHandler;
#[async_trait]
impl ToolHandler for CodeExecHandler {
    fn name(&self) -> &str { "code_exec" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let language = input.get("language").and_then(|v| v.as_str()).unwrap_or("");
        let code = input.get("code").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::code_exec::handle_code_exec(&ctx.policy, language, code).await {
            Ok(out) => Ok(healer_result("code_exec", out, false)),
            Err(e) => Ok(healer_result("code_exec", format!("Error: {}", e), true)),
        }
    }
}

struct WebSearchHandler;
#[async_trait]
impl ToolHandler for WebSearchHandler {
    fn name(&self) -> &str { "web_search" }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::web_search::handle_web_search(query).await {
            Ok(out) => Ok(healer_result("web_search", out, false)),
            Err(e) => Ok(healer_result("web_search", format!("Error: {}", e), true)),
        }
    }
}

struct WebFetchHandler;
#[async_trait]
impl ToolHandler for WebFetchHandler {
    fn name(&self) -> &str { "web_fetch" }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let url = input.get("url").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::web_fetch::handle_web_fetch(url).await {
            Ok(out) => Ok(healer_result("web_fetch", out, false)),
            Err(e) => Ok(healer_result("web_fetch", format!("Error: {}", e), true)),
        }
    }
}

struct SharedMemoryStoreHandler;
#[async_trait]
impl ToolHandler for SharedMemoryStoreHandler {
    fn name(&self) -> &str { "shared_memory_store" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let content = input.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let topic = input.get("topic").and_then(|v| v.as_str()).unwrap_or("general");
        match ctx.kernel.memory.shared.store(ctx.agent_id, content, topic) {
            Ok(_) => Ok(healer_result("shared_memory_store", "Stored in shared memory.".into(), false)),
            Err(e) => Ok(healer_result("shared_memory_store", format!("Error: {}", e), true)),
        }
    }
}

struct SharedMemoryRecallHandler;
#[async_trait]
impl ToolHandler for SharedMemoryRecallHandler {
    fn name(&self) -> &str { "shared_memory_recall" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("");
        match ctx.kernel.memory.shared.recall(query) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(healer_result("shared_memory_recall", "No shared memories found.".into(), false))
                } else {
                    let formatted: Vec<String> = results.iter()
                        .map(|(author, content, ts)| format!("[{}] {}: {}", ts, author, content))
                        .collect();
                    Ok(healer_result("shared_memory_recall", formatted.join("\n"), false))
                }
            }
            Err(e) => Ok(healer_result("shared_memory_recall", format!("Error: {}", e), true)),
        }
    }
}

struct GetSkillHandler;
#[async_trait]
impl ToolHandler for GetSkillHandler {
    fn name(&self) -> &str { "get_skill" }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let name = input.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let skills = ctx.kernel.skills.read().unwrap();
        let result = sk_tools::skills::handle_get_skill(&skills, name);
        Ok(healer_result("get_skill", result, false))
    }
}

struct ListSkillsHandler;
#[async_trait]
impl ToolHandler for ListSkillsHandler {
    fn name(&self) -> &str { "list_skills" }
    async fn handle(&self, ctx: ToolContext, _input: Value) -> SovereignResult<ToolResult> {
        let skills = ctx.kernel.skills.read().unwrap();
        let result = sk_tools::skills::handle_list_skills(&skills);
        Ok(healer_result("list_skills", result, false))
    }
}
