//! Built-in tools for the Sovereign Kernel.
//!
//! Subset of Sovereign Kernel's 53 tools, focused on essentials.

pub mod browser_tools;
pub mod code_exec;
pub mod file_ops;
pub mod mcp_bridge;
pub mod memory_tools;
pub mod ottos_outpost;
pub mod scheduler;
pub mod shared_memory;
pub mod shell;
pub mod skills;
pub mod web_fetch;
pub mod web_search;

use sk_types::ToolDefinition;

/// Get all built-in tool definitions.
pub fn builtin_tools() -> Vec<ToolDefinition> {
    let mut tools = vec![
        memory_tools::remember_tool(),
        memory_tools::recall_tool(),
        memory_tools::forget_tool(),
        web_search::web_search_tool(),
        web_fetch::web_fetch_tool(),
        file_ops::read_file_tool(),
        file_ops::write_file_tool(),
        file_ops::list_dir_tool(),
        shell::shell_exec_tool(),
    ];
    tools.extend(browser_tools::browser_tools());
    tools.push(skills::get_skill_tool());
    tools.push(skills::list_skills_tool());
    tools.push(ottos_outpost::ottos_outpost_tool());
    tools
}
