//! Host-level tools for full system access.
pub mod app_installer;
pub mod desktop_control;
pub mod system_config;

use sk_types::ToolDefinition;

/// Get all host-level tool definitions.
pub fn host_tools() -> Vec<ToolDefinition> {
    vec![
        app_installer::app_installer_tool(),
        desktop_control::desktop_control_tool(),
        system_config::system_config_tool(),
    ]
}
