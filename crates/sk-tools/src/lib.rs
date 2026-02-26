//! Built-in tools for the Sovereign Kernel.
//!
//! Subset of OpenFang's 53 tools, focused on essentials.

pub mod file_ops;
pub mod mcp_bridge;
pub mod memory_tools;
pub mod shell;
pub mod web_fetch;
pub mod web_search;

use sk_types::ToolDefinition;

/// Get all built-in tool definitions.
pub fn builtin_tools() -> Vec<ToolDefinition> {
    vec![
        memory_tools::remember_tool(),
        memory_tools::recall_tool(),
        memory_tools::forget_tool(),
        web_search::web_search_tool(),
        web_fetch::web_fetch_tool(),
        file_ops::read_file_tool(),
        file_ops::write_file_tool(),
        file_ops::list_dir_tool(),
        shell::shell_exec_tool(),
    ]
}
