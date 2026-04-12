//! Modular Tool Registry — dispatches tool calls to individual handlers.
//!
//! Every tool implements the `ToolHandler` trait and is registered in the `ToolRegistry`.
//! This replaces the monolithic match statement that previously lived in executor.rs.

use async_trait::async_trait;
use serde_json::Value;
use sk_types::config::ExecutionMode;
use sk_types::{AgentId, SovereignResult, ToolCall, ToolDefinition, ToolResult};
use std::collections::{HashMap, HashSet};
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
    pub fn register_all(mut self) -> Self {
        for entry in tool_catalog() {
            self = self.register((entry.handler)());
        }
        self
    }

    /// Dispatch a tool call to the appropriate handler.
    pub async fn dispatch(
        &self,
        ctx: ToolContext,
        tool_call: ToolCall,
        input: Value,
    ) -> SovereignResult<ToolResult> {
        let resolved_name = resolve_registered_tool_name(&tool_call.name);
        if let Some(handler) = self.handlers.get(resolved_name.as_str()) {
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

#[derive(Clone, Copy)]
enum ToolAvailability {
    Always,
    OnAny(&'static [&'static str]),
    OnAnyWhenNotSmall(&'static [&'static str]),
    WhenNotSmall,
}

struct ToolCatalogEntry {
    runtime_name: &'static str,
    aliases: &'static [&'static str],
    availability: ToolAvailability,
    definition: fn() -> ToolDefinition,
    handler: fn() -> Box<dyn ToolHandler>,
}

impl ToolCatalogEntry {
    fn is_enabled(&self, requested: &HashSet<String>, small_model: bool) -> bool {
        let explicitly_requested = requested.contains(self.runtime_name)
            || self.aliases.iter().any(|alias| requested.contains(*alias));

        match self.availability {
            ToolAvailability::Always => true,
            ToolAvailability::OnAny(selectors) => {
                explicitly_requested
                    || selectors
                        .iter()
                        .any(|selector| requested.contains(*selector))
            }
            ToolAvailability::OnAnyWhenNotSmall(selectors) => {
                !small_model
                    && (explicitly_requested
                        || selectors
                            .iter()
                            .any(|selector| requested.contains(*selector)))
            }
            ToolAvailability::WhenNotSmall => !small_model,
        }
    }
}

const FILE_READ_SELECTORS: &[&str] = &["file_read", "file_write", "file", "files"];
const FILE_WRITE_SELECTORS: &[&str] = &["file_write", "file", "files"];
const WEB_SELECTORS: &[&str] = &["web", "network"];
const BROWSER_SELECTORS: &[&str] = &["web", "browser", "browse"];
const SHELL_SELECTORS: &[&str] = &["shell"];
const OTTO_SELECTORS: &[&str] = &["otto"];

fn tool_catalog() -> Vec<ToolCatalogEntry> {
    vec![
        ToolCatalogEntry {
            runtime_name: "remember",
            aliases: &["memory_store"],
            availability: ToolAvailability::Always,
            definition: sk_tools::memory_tools::remember_tool,
            handler: || Box::new(RememberHandler),
        },
        ToolCatalogEntry {
            runtime_name: "recall",
            aliases: &["memory_recall"],
            availability: ToolAvailability::Always,
            definition: sk_tools::memory_tools::recall_tool,
            handler: || Box::new(RecallHandler),
        },
        ToolCatalogEntry {
            runtime_name: "read_file",
            aliases: &["file_read"],
            availability: ToolAvailability::OnAny(FILE_READ_SELECTORS),
            definition: sk_tools::file_ops::read_file_tool,
            handler: || Box::new(ReadFileHandler),
        },
        ToolCatalogEntry {
            runtime_name: "list_dir",
            aliases: &["file_list"],
            availability: ToolAvailability::OnAny(FILE_READ_SELECTORS),
            definition: sk_tools::file_ops::list_dir_tool,
            handler: || Box::new(ListDirHandler),
        },
        ToolCatalogEntry {
            runtime_name: "write_file",
            aliases: &["file_write"],
            availability: ToolAvailability::OnAny(FILE_WRITE_SELECTORS),
            definition: sk_tools::file_ops::write_file_tool,
            handler: || Box::new(WriteFileHandler),
        },
        ToolCatalogEntry {
            runtime_name: "shell_exec",
            aliases: &[],
            availability: ToolAvailability::OnAny(SHELL_SELECTORS),
            definition: sk_tools::shell::shell_exec_tool,
            handler: || Box::new(ShellExecHandler),
        },
        ToolCatalogEntry {
            runtime_name: "web_search",
            aliases: &[],
            availability: ToolAvailability::OnAny(WEB_SELECTORS),
            definition: sk_tools::web_search::web_search_tool,
            handler: || Box::new(WebSearchHandler),
        },
        ToolCatalogEntry {
            runtime_name: "web_fetch",
            aliases: &[],
            availability: ToolAvailability::OnAny(WEB_SELECTORS),
            definition: sk_tools::web_fetch::web_fetch_tool,
            handler: || Box::new(WebFetchHandler),
        },
        ToolCatalogEntry {
            runtime_name: "browser_navigate",
            aliases: &[],
            availability: ToolAvailability::OnAny(BROWSER_SELECTORS),
            definition: sk_tools::browser::browser_navigate_tool,
            handler: || Box::new(BrowserNavigateHandler),
        },
        ToolCatalogEntry {
            runtime_name: "browser_read_page",
            aliases: &[],
            availability: ToolAvailability::OnAny(BROWSER_SELECTORS),
            definition: sk_tools::browser::browser_read_page_tool,
            handler: || Box::new(BrowserReadPageHandler),
        },
        ToolCatalogEntry {
            runtime_name: "browser_screenshot",
            aliases: &[],
            availability: ToolAvailability::OnAny(BROWSER_SELECTORS),
            definition: sk_tools::browser::browser_screenshot_tool,
            handler: || Box::new(BrowserScreenshotHandler),
        },
        ToolCatalogEntry {
            runtime_name: "browser_click",
            aliases: &[],
            availability: ToolAvailability::OnAny(BROWSER_SELECTORS),
            definition: sk_tools::browser::browser_click_tool,
            handler: || Box::new(BrowserClickHandler),
        },
        ToolCatalogEntry {
            runtime_name: "browser_type",
            aliases: &[],
            availability: ToolAvailability::OnAny(BROWSER_SELECTORS),
            definition: sk_tools::browser::browser_type_tool,
            handler: || Box::new(BrowserTypeHandler),
        },
        ToolCatalogEntry {
            runtime_name: "browser_scroll",
            aliases: &[],
            availability: ToolAvailability::OnAny(BROWSER_SELECTORS),
            definition: sk_tools::browser::browser_scroll_tool,
            handler: || Box::new(BrowserScrollHandler),
        },
        ToolCatalogEntry {
            runtime_name: "browser_get_dom",
            aliases: &[],
            availability: ToolAvailability::OnAny(BROWSER_SELECTORS),
            definition: sk_tools::browser::browser_get_dom_tool,
            handler: || Box::new(BrowserGetDOMHandler),
        },
        ToolCatalogEntry {
            runtime_name: "delete_file",
            aliases: &[],
            availability: ToolAvailability::OnAnyWhenNotSmall(FILE_WRITE_SELECTORS),
            definition: sk_tools::file_ops::delete_file_tool,
            handler: || Box::new(DeleteFileHandler),
        },
        ToolCatalogEntry {
            runtime_name: "forget",
            aliases: &[],
            availability: ToolAvailability::OnAnyWhenNotSmall(FILE_WRITE_SELECTORS),
            definition: sk_tools::memory_tools::forget_tool,
            handler: || Box::new(ForgetHandler),
        },
        ToolCatalogEntry {
            runtime_name: "move_file",
            aliases: &[],
            availability: ToolAvailability::OnAnyWhenNotSmall(FILE_WRITE_SELECTORS),
            definition: sk_tools::file_ops::move_file_tool,
            handler: || Box::new(MoveFileHandler),
        },
        ToolCatalogEntry {
            runtime_name: "copy_file",
            aliases: &[],
            availability: ToolAvailability::OnAnyWhenNotSmall(FILE_WRITE_SELECTORS),
            definition: sk_tools::file_ops::copy_file_tool,
            handler: || Box::new(CopyFileHandler),
        },
        ToolCatalogEntry {
            runtime_name: "code_exec",
            aliases: &[],
            availability: ToolAvailability::WhenNotSmall,
            definition: sk_tools::code_exec::code_exec_tool,
            handler: || Box::new(CodeExecHandler),
        },
        ToolCatalogEntry {
            runtime_name: "shared_memory_store",
            aliases: &[],
            availability: ToolAvailability::WhenNotSmall,
            definition: sk_tools::shared_memory::shared_memory_store_tool,
            handler: || Box::new(SharedMemoryStoreHandler),
        },
        ToolCatalogEntry {
            runtime_name: "shared_memory_recall",
            aliases: &[],
            availability: ToolAvailability::WhenNotSmall,
            definition: sk_tools::shared_memory::shared_memory_recall_tool,
            handler: || Box::new(SharedMemoryRecallHandler),
        },
        ToolCatalogEntry {
            runtime_name: "app_installer",
            aliases: &[],
            availability: ToolAvailability::WhenNotSmall,
            definition: sk_tools::host::app_installer::app_installer_tool,
            handler: || Box::new(AppInstallerHandler),
        },
        ToolCatalogEntry {
            runtime_name: "desktop_control",
            aliases: &[],
            availability: ToolAvailability::WhenNotSmall,
            definition: sk_tools::host::desktop_control::desktop_control_tool,
            handler: || Box::new(DesktopControlHandler),
        },
        ToolCatalogEntry {
            runtime_name: "system_config",
            aliases: &[],
            availability: ToolAvailability::WhenNotSmall,
            definition: sk_tools::host::system_config::system_config_tool,
            handler: || Box::new(SystemConfigHandler),
        },
        ToolCatalogEntry {
            runtime_name: "get_skill",
            aliases: &[],
            availability: ToolAvailability::WhenNotSmall,
            definition: sk_tools::skills::get_skill_tool,
            handler: || Box::new(GetSkillHandler),
        },
        ToolCatalogEntry {
            runtime_name: "list_skills",
            aliases: &[],
            availability: ToolAvailability::WhenNotSmall,
            definition: sk_tools::skills::list_skills_tool,
            handler: || Box::new(ListSkillsHandler),
        },
        ToolCatalogEntry {
            runtime_name: "ottos_outpost",
            aliases: &[],
            availability: ToolAvailability::OnAnyWhenNotSmall(OTTO_SELECTORS),
            definition: sk_tools::ottos_outpost::ottos_outpost_tool,
            handler: || Box::new(OttosOutpostHandler),
        },
        ToolCatalogEntry {
            runtime_name: "compile_rust_skill",
            aliases: &[],
            availability: ToolAvailability::OnAnyWhenNotSmall(OTTO_SELECTORS),
            definition: sk_tools::otto::compile_rust_skill_tool,
            handler: || Box::new(CompileRustSkillHandler),
        },
    ]
}

fn runtime_tool_alias(name: &str) -> Option<&'static str> {
    match name {
        "file_read" | "read_file" => Some("read_file"),
        "file_write" | "write_file" => Some("write_file"),
        "file_list" | "list_dir" => Some("list_dir"),
        "memory_store" | "remember" => Some("remember"),
        "memory_recall" | "recall" => Some("recall"),
        _ => None,
    }
}

fn resolve_registered_tool_name(name: &str) -> String {
    if let Some(runtime_name) = runtime_tool_alias(name) {
        return runtime_name.to_string();
    }

    if let Some(mapped) = sk_types::tool_compat::map_tool_name(name) {
        if let Some(runtime_name) = runtime_tool_alias(mapped) {
            return runtime_name.to_string();
        }
        return mapped.to_string();
    }

    name.to_string()
}

fn requested_tool_set(requested: &[String]) -> HashSet<String> {
    let mut set = HashSet::new();

    for raw in requested {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        let lowered = trimmed.to_ascii_lowercase();
        set.insert(trimmed.to_string());
        set.insert(lowered.clone());

        if let Some(runtime_name) = runtime_tool_alias(trimmed) {
            set.insert(runtime_name.to_string());
        }
        if let Some(runtime_name) = runtime_tool_alias(&lowered) {
            set.insert(runtime_name.to_string());
        }

        if let Some(mapped) = sk_types::tool_compat::map_tool_name(trimmed) {
            set.insert(mapped.to_string());
            if let Some(runtime_name) = runtime_tool_alias(mapped) {
                set.insert(runtime_name.to_string());
            }
        }
        if let Some(mapped) = sk_types::tool_compat::map_tool_name(&lowered) {
            set.insert(mapped.to_string());
            if let Some(runtime_name) = runtime_tool_alias(mapped) {
                set.insert(runtime_name.to_string());
            }
        }
    }

    set
}

pub fn available_tool_definitions(requested: &[String], small_model: bool) -> Vec<ToolDefinition> {
    let requested = requested_tool_set(requested);

    tool_catalog()
        .into_iter()
        .filter(|entry| entry.is_enabled(&requested, small_model))
        .map(|entry| (entry.definition)())
        .collect()
}

// ---------------------------------------------------------------------------
// Tool Handler Implementations
// ---------------------------------------------------------------------------

struct ShellExecHandler;
#[async_trait]
impl ToolHandler for ShellExecHandler {
    fn name(&self) -> &str {
        "shell_exec"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let command = input.get("command").and_then(|v| v.as_str()).unwrap_or("");
        let cwd = input.get("cwd").and_then(|v| v.as_str());
        let timeout = input.get("timeout_secs").and_then(|v| v.as_u64());

        // 1. Interactive Approval Hook
        let mut active_policy = ctx.policy.clone();
        if ctx.kernel.approval.requires_approval("shell_exec") {
            let req = sk_types::approval::ApprovalRequest {
                id: uuid::Uuid::new_v4(),
                agent_id: ctx.agent_id.to_string(),
                tool_name: "shell_exec".into(),
                description: format!("Agent requested shell execution: {}", command),
                action_summary: command.chars().take(200).collect(),
                risk_level: sk_types::approval::RiskLevel::High,
                requested_at: chrono::Utc::now(),
                timeout_secs: ctx.policy.timeout_secs,
            };

            let decision = ctx.kernel.approval.request_approval(req).await;
            match decision {
                sk_types::approval::ApprovalDecision::ApprovedFull => {
                    // Bypass the sandbox natively per user request
                    active_policy.mode = sk_types::config::ExecSecurityMode::Full;
                }
                sk_types::approval::ApprovalDecision::ApprovedSandboxed => {
                    // Remain in strict Allowlist sandbox mode
                    active_policy.mode = sk_types::config::ExecSecurityMode::Allowlist;
                }
                sk_types::approval::ApprovalDecision::Denied => {
                    return Ok(healer_result(
                        "shell_exec",
                        "SECURITY VIOLATION: Execution denied by user.".to_string(),
                        true,
                    ));
                }
                sk_types::approval::ApprovalDecision::TimedOut => {
                    return Ok(healer_result(
                        "shell_exec",
                        "SECURITY VIOLATION: Request timed out waiting for user approval."
                            .to_string(),
                        true,
                    ));
                }
            }
        }

        // 2. Validate using the robust SubprocessSandbox engine if in Allowlist mode
        if active_policy.mode == sk_types::config::ExecSecurityMode::Allowlist {
            if let Err(e) = sk_engine::runtime::subprocess_sandbox::validate_command_allowlist(
                command,
                &active_policy,
            ) {
                return Ok(healer_result(
                    "shell_exec",
                    format!("SECURITY VIOLATION: {}", e),
                    true,
                ));
            }
        }

        // 3. Execute the command
        // We override to Full here because we already validated it robustly against our policy above.
        // Failing to do so runs the very weak legacy sandbox logic inside handle_shell_exec.
        active_policy.mode = sk_types::config::ExecSecurityMode::Full;
        match sk_tools::shell::handle_shell_exec(&active_policy, command, cwd, timeout).await {
            Ok(out) => Ok(healer_result("shell_exec", out, false)),
            Err(e) => Ok(healer_result("shell_exec", format!("Error: {}", e), true)),
        }
    }
}

struct ReadFileHandler;
#[async_trait]
impl ToolHandler for ReadFileHandler {
    fn name(&self) -> &str {
        "read_file"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::file_ops::handle_read_file(&ctx.workspaces_dir, path, ctx.is_unrestricted())
        {
            Ok(content) => Ok(healer_result("read_file", content, false)),
            Err(e) => Ok(healer_result("read_file", format!("Error: {}", e), true)),
        }
    }
}

struct WriteFileHandler;
#[async_trait]
impl ToolHandler for WriteFileHandler {
    fn name(&self) -> &str {
        "write_file"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
        let content = input.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let append = input
            .get("append")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        match sk_tools::file_ops::handle_write_file(
            &ctx.workspaces_dir,
            path,
            content,
            append,
            ctx.is_unrestricted(),
        ) {
            Ok(msg) => Ok(healer_result("write_file", msg, false)),
            Err(e) => Ok(healer_result("write_file", format!("Error: {}", e), true)),
        }
    }
}

struct ListDirHandler;
#[async_trait]
impl ToolHandler for ListDirHandler {
    fn name(&self) -> &str {
        "list_dir"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        match sk_tools::file_ops::handle_list_dir(&ctx.workspaces_dir, path, ctx.is_unrestricted())
        {
            Ok(out) => Ok(healer_result("list_dir", out, false)),
            Err(e) => Ok(healer_result("list_dir", format!("Error: {}", e), true)),
        }
    }
}

struct DeleteFileHandler;
#[async_trait]
impl ToolHandler for DeleteFileHandler {
    fn name(&self) -> &str {
        "delete_file"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let path = input.get("path").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::file_ops::handle_delete_file(
            &ctx.workspaces_dir,
            path,
            ctx.is_unrestricted(),
        ) {
            Ok(msg) => Ok(healer_result("delete_file", msg, false)),
            Err(e) => Ok(healer_result("delete_file", format!("Error: {}", e), true)),
        }
    }
}

struct MoveFileHandler;
#[async_trait]
impl ToolHandler for MoveFileHandler {
    fn name(&self) -> &str {
        "move_file"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let src = input.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let dst = input
            .get("destination")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        match sk_tools::file_ops::handle_move_file(
            &ctx.workspaces_dir,
            src,
            dst,
            ctx.is_unrestricted(),
        ) {
            Ok(msg) => Ok(healer_result("move_file", msg, false)),
            Err(e) => Ok(healer_result("move_file", format!("Error: {}", e), true)),
        }
    }
}

struct CopyFileHandler;
#[async_trait]
impl ToolHandler for CopyFileHandler {
    fn name(&self) -> &str {
        "copy_file"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let src = input.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let dst = input
            .get("destination")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        match sk_tools::file_ops::handle_copy_file(
            &ctx.workspaces_dir,
            src,
            dst,
            ctx.is_unrestricted(),
        ) {
            Ok(msg) => Ok(healer_result("copy_file", msg, false)),
            Err(e) => Ok(healer_result("copy_file", format!("Error: {}", e), true)),
        }
    }
}

struct RememberHandler;
#[async_trait]
impl ToolHandler for RememberHandler {
    fn name(&self) -> &str {
        "remember"
    }
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
    fn name(&self) -> &str {
        "recall"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let limit = input.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        match sk_tools::memory_tools::handle_recall(&ctx.kernel.memory, ctx.agent_id, query, limit)
        {
            Ok(out) => Ok(healer_result("recall", out, false)),
            Err(e) => Ok(healer_result("recall", format!("Error: {}", e), true)),
        }
    }
}

struct ForgetHandler;
#[async_trait]
impl ToolHandler for ForgetHandler {
    fn name(&self) -> &str {
        "forget"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let memory_id = input
            .get("memory_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        match sk_tools::memory_tools::handle_forget(&ctx.kernel.memory, memory_id) {
            Ok(msg) => Ok(healer_result("forget", msg, false)),
            Err(e) => Ok(healer_result("forget", format!("Error: {}", e), true)),
        }
    }
}

struct WebSearchHandler;
#[async_trait]
impl ToolHandler for WebSearchHandler {
    fn name(&self) -> &str {
        "web_search"
    }
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
    fn name(&self) -> &str {
        "web_fetch"
    }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let url = input.get("url").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::web_fetch::handle_web_fetch(url).await {
            Ok(out) => Ok(healer_result("web_fetch", out, false)),
            Err(e) => Ok(healer_result("web_fetch", format!("Error: {}", e), true)),
        }
    }
}

struct CodeExecHandler;
#[async_trait]
impl ToolHandler for CodeExecHandler {
    fn name(&self) -> &str {
        "code_exec"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let code = input.get("code").and_then(|v| v.as_str()).unwrap_or("");
        let lang = input
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("python");
        match sk_tools::code_exec::handle_code_exec(&ctx.policy, code, lang).await {
            Ok(out) => Ok(healer_result("code_exec", out, false)),
            Err(e) => Ok(healer_result("code_exec", format!("Error: {}", e), true)),
        }
    }
}

struct SharedMemoryStoreHandler;
#[async_trait]
impl ToolHandler for SharedMemoryStoreHandler {
    fn name(&self) -> &str {
        "shared_memory_store"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let content = input.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let topic = input
            .get("topic")
            .and_then(|v| v.as_str())
            .unwrap_or("general");
        match ctx.kernel.memory.shared.store(ctx.agent_id, content, topic) {
            Ok(_) => Ok(healer_result(
                "shared_memory_store",
                "Stored in shared memory.".into(),
                false,
            )),
            Err(e) => Ok(healer_result(
                "shared_memory_store",
                format!("Error: {}", e),
                true,
            )),
        }
    }
}

struct SharedMemoryRecallHandler;
#[async_trait]
impl ToolHandler for SharedMemoryRecallHandler {
    fn name(&self) -> &str {
        "shared_memory_recall"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let query = input.get("query").and_then(|v| v.as_str()).unwrap_or("");
        match ctx.kernel.memory.shared.recall(query) {
            Ok(results) => {
                if results.is_empty() {
                    Ok(healer_result(
                        "shared_memory_recall",
                        "No shared memories found.".into(),
                        false,
                    ))
                } else {
                    let formatted: Vec<String> = results
                        .iter()
                        .map(|(author, content, ts)| format!("[{}] {}: {}", ts, author, content))
                        .collect();
                    Ok(healer_result(
                        "shared_memory_recall",
                        formatted.join("\n"),
                        false,
                    ))
                }
            }
            Err(e) => Ok(healer_result(
                "shared_memory_recall",
                format!("Error: {}", e),
                true,
            )),
        }
    }
}

struct GetSkillHandler;
#[async_trait]
impl ToolHandler for GetSkillHandler {
    fn name(&self) -> &str {
        "get_skill"
    }
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
    fn name(&self) -> &str {
        "list_skills"
    }
    async fn handle(&self, ctx: ToolContext, _input: Value) -> SovereignResult<ToolResult> {
        let skills = ctx.kernel.skills.read().unwrap();
        let result = sk_tools::skills::handle_list_skills(&skills);
        Ok(healer_result("list_skills", result, false))
    }
}
struct AppInstallerHandler;
#[async_trait]
impl ToolHandler for AppInstallerHandler {
    fn name(&self) -> &str {
        "app_installer"
    }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let package_id = input
            .get("package_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        match sk_tools::host::app_installer::handle_app_installer(package_id) {
            Ok(msg) => Ok(healer_result("app_installer", msg, false)),
            Err(e) => Ok(healer_result(
                "app_installer",
                format!("Error: {}", e),
                true,
            )),
        }
    }
}

struct DesktopControlHandler;
#[async_trait]
impl ToolHandler for DesktopControlHandler {
    fn name(&self) -> &str {
        "desktop_control"
    }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let action = input.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let value = input.get("value").and_then(|v| v.as_str()).unwrap_or("");
        match sk_tools::host::desktop_control::handle_desktop_control(action, value) {
            Ok(msg) => Ok(healer_result("desktop_control", msg, false)),
            Err(e) => Ok(healer_result(
                "desktop_control",
                format!("Error: {}", e),
                true,
            )),
        }
    }
}

struct SystemConfigHandler;
#[async_trait]
impl ToolHandler for SystemConfigHandler {
    fn name(&self) -> &str {
        "system_config"
    }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let action = input.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let target = input.get("target").and_then(|v| v.as_str());
        let value = input.get("value").and_then(|v| v.as_str());
        match sk_tools::host::system_config::handle_system_config(action, target, value) {
            Ok(msg) => Ok(healer_result("system_config", msg, false)),
            Err(e) => Ok(healer_result(
                "system_config",
                format!("Error: {}", e),
                true,
            )),
        }
    }
}

fn get_browser_bridge_path() -> String {
    let script = "crates/sk-tools/src/browser_bridge.py";
    if std::path::Path::new(script).exists() {
        return script.to_string();
    }
    // Fallback for when running from within a crate dir
    let fallback = "../../crates/sk-tools/src/browser_bridge.py";
    if std::path::Path::new(fallback).exists() {
        return fallback.to_string();
    }
    script.to_string()
}

struct BrowserNavigateHandler;
#[async_trait]
impl ToolHandler for BrowserNavigateHandler {
    fn name(&self) -> &str {
        "browser_navigate"
    }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let args = serde_json::to_string(&input).unwrap_or_default();
        let output = tokio::process::Command::new("python")
            .arg(get_browser_bridge_path())
            .arg("navigate")
            .arg(&args)
            .output()
            .await
            .map_err(|e| {
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to run browser bridge: {}",
                    e
                ))
            })?;

        let result = String::from_utf8_lossy(&output.stdout);
        Ok(healer_result(
            "browser_navigate",
            result.to_string(),
            !output.status.success(),
        ))
    }
}

struct BrowserReadPageHandler;
#[async_trait]
impl ToolHandler for BrowserReadPageHandler {
    fn name(&self) -> &str {
        "browser_read_page"
    }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let args = serde_json::to_string(&input).unwrap_or_default();
        let output = tokio::process::Command::new("python")
            .arg(get_browser_bridge_path())
            .arg("read_page")
            .arg(&args)
            .output()
            .await
            .map_err(|e| {
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to run browser bridge: {}",
                    e
                ))
            })?;

        let result = String::from_utf8_lossy(&output.stdout);
        Ok(healer_result(
            "browser_read_page",
            result.to_string(),
            !output.status.success(),
        ))
    }
}

struct BrowserScreenshotHandler;
#[async_trait]
impl ToolHandler for BrowserScreenshotHandler {
    fn name(&self) -> &str {
        "browser_screenshot"
    }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let args = serde_json::to_string(&input).unwrap_or_default();
        let output = tokio::process::Command::new("python")
            .arg(get_browser_bridge_path())
            .arg("screenshot")
            .arg(&args)
            .output()
            .await
            .map_err(|e| {
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to run browser bridge: {}",
                    e
                ))
            })?;

        let result = String::from_utf8_lossy(&output.stdout);
        Ok(healer_result(
            "browser_screenshot",
            result.to_string(),
            !output.status.success(),
        ))
    }
}

struct OttosOutpostHandler;
#[async_trait]
impl ToolHandler for OttosOutpostHandler {
    fn name(&self) -> &str {
        "ottos_outpost"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let language = input
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("python")
            .to_string();
        let env_str = input
            .get("execution_env")
            .and_then(|v| v.as_str())
            .unwrap_or("docker");
        let execution_env = if env_str == "native" {
            sk_engine::runtime::ottos_outpost::ExecutionEnv::Native
        } else {
            sk_engine::runtime::ottos_outpost::ExecutionEnv::Docker
        };
        let dependencies = input
            .get("dependencies")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_string())
            .collect();
        let code = input
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let req = sk_engine::runtime::ottos_outpost::OttosOutpostRequest {
            language,
            execution_env,
            dependencies,
            code,
            input_files: vec![],
        };

        match sk_engine::runtime::ottos_outpost::execute_ottos_outpost(req, &ctx.workspaces_dir)
            .await
        {
            Ok(res) => Ok(healer_result(
                "ottos_outpost",
                format!("STDOUT: {}\nSTDERR: {}", res.stdout, res.stderr),
                res.exit_code != 0,
            )),
            Err(e) => Ok(healer_result(
                "ottos_outpost",
                format!("Error: {}", e),
                true,
            )),
        }
    }
}

struct CompileRustSkillHandler;
#[async_trait]
impl ToolHandler for CompileRustSkillHandler {
    fn name(&self) -> &str {
        "compile_rust_skill"
    }
    async fn handle(&self, ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let skill_name = input
            .get("skill_name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let description = input
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let code = input.get("code").and_then(|v| v.as_str()).unwrap_or("");
        let dependencies_toml = input
            .get("dependencies_toml")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let instructions = input
            .get("instructions")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match ctx
            .kernel
            .compile_skill(
                skill_name,
                description,
                code,
                dependencies_toml,
                instructions,
            )
            .await
        {
            Ok(msg) => Ok(healer_result("compile_rust_skill", msg, false)),
            Err(e) => Ok(healer_result(
                "compile_rust_skill",
                format!("Error: {}", e),
                true,
            )),
        }
    }
}

struct BrowserClickHandler;
#[async_trait]
impl ToolHandler for BrowserClickHandler {
    fn name(&self) -> &str {
        "browser_click"
    }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let args = serde_json::to_string(&input).unwrap_or_default();
        let output = tokio::process::Command::new("python")
            .arg(get_browser_bridge_path())
            .arg("click")
            .arg(&args)
            .output()
            .await
            .map_err(|e| {
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to run browser bridge: {}",
                    e
                ))
            })?;

        let result = String::from_utf8_lossy(&output.stdout);
        Ok(healer_result(
            "browser_click",
            result.to_string(),
            !output.status.success(),
        ))
    }
}

struct BrowserTypeHandler;
#[async_trait]
impl ToolHandler for BrowserTypeHandler {
    fn name(&self) -> &str {
        "browser_type"
    }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let args = serde_json::to_string(&input).unwrap_or_default();
        let output = tokio::process::Command::new("python")
            .arg(get_browser_bridge_path())
            .arg("type")
            .arg(&args)
            .output()
            .await
            .map_err(|e| {
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to run browser bridge: {}",
                    e
                ))
            })?;

        let result = String::from_utf8_lossy(&output.stdout);
        Ok(healer_result(
            "browser_type",
            result.to_string(),
            !output.status.success(),
        ))
    }
}

struct BrowserScrollHandler;
#[async_trait]
impl ToolHandler for BrowserScrollHandler {
    fn name(&self) -> &str {
        "browser_scroll"
    }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let args = serde_json::to_string(&input).unwrap_or_default();
        let output = tokio::process::Command::new("python")
            .arg(get_browser_bridge_path())
            .arg("scroll")
            .arg(&args)
            .output()
            .await
            .map_err(|e| {
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to run browser bridge: {}",
                    e
                ))
            })?;

        let result = String::from_utf8_lossy(&output.stdout);
        Ok(healer_result(
            "browser_scroll",
            result.to_string(),
            !output.status.success(),
        ))
    }
}

struct BrowserGetDOMHandler;
#[async_trait]
impl ToolHandler for BrowserGetDOMHandler {
    fn name(&self) -> &str {
        "browser_get_dom"
    }
    async fn handle(&self, _ctx: ToolContext, input: Value) -> SovereignResult<ToolResult> {
        let args = serde_json::to_string(&input).unwrap_or_default();
        let output = tokio::process::Command::new("python")
            .arg(get_browser_bridge_path())
            .arg("get_dom")
            .arg(&args)
            .output()
            .await
            .map_err(|e| {
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to run browser bridge: {}",
                    e
                ))
            })?;

        let result = String::from_utf8_lossy(&output.stdout);
        Ok(healer_result(
            "browser_get_dom",
            result.to_string(),
            !output.status.success(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_enables_runtime_tools_for_legacy_aliases() {
        let defs = available_tool_definitions(
            &[
                "file_list".to_string(),
                "memory_store".to_string(),
                "memory_recall".to_string(),
            ],
            false,
        );
        let names: Vec<String> = defs.into_iter().map(|tool| tool.name).collect();

        assert!(names.contains(&"list_dir".to_string()));
        assert!(names.contains(&"remember".to_string()));
        assert!(names.contains(&"recall".to_string()));
    }

    #[test]
    fn catalog_honors_explicit_high_power_tools_when_requested() {
        let defs = available_tool_definitions(&["ottos_outpost".to_string()], false);
        let names: Vec<String> = defs.into_iter().map(|tool| tool.name).collect();

        assert!(names.contains(&"ottos_outpost".to_string()));
    }

    #[test]
    fn catalog_keeps_high_power_tools_off_for_small_models() {
        let defs = available_tool_definitions(&["ottos_outpost".to_string()], true);
        let names: Vec<String> = defs.into_iter().map(|tool| tool.name).collect();

        assert!(!names.contains(&"ottos_outpost".to_string()));
    }

    #[test]
    fn dispatch_name_resolution_maps_canonical_aliases_to_runtime_handlers() {
        assert_eq!(resolve_registered_tool_name("file_list"), "list_dir");
        assert_eq!(resolve_registered_tool_name("memory_store"), "remember");
        assert_eq!(resolve_registered_tool_name("Bash"), "shell_exec");
    }
}
