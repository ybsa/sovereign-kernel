//! System configuration tools (services, env vars, system info).
use sk_types::ToolDefinition;
use std::process::Command;

pub fn system_config_tool() -> ToolDefinition {
    ToolDefinition {
        name: "system_config".into(),
        description: "Manage system configurations like services, environment variables, and view system information. Windows only.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list_services", "start_service", "stop_service", "set_env_var", "get_system_info"],
                    "description": "The action to perform."
                },
                "target": {
                    "type": "string",
                    "description": "The target service name or environment variable name."
                },
                "value": {
                    "type": "string",
                    "description": "The value for setting an environment variable."
                }
            },
            "required": ["action"]
        }),
    }
}

pub fn handle_system_config(
    action: &str,
    target: Option<&str>,
    value: Option<&str>,
) -> Result<String, sk_types::SovereignError> {
    match action {
        "list_services" => {
            let output = Command::new("net").arg("start").output().map_err(|e| {
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to list services: {}",
                    e
                ))
            })?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        "start_service" => {
            let t = target.ok_or_else(|| {
                sk_types::SovereignError::ToolExecutionError(
                    "Target service name required for start_service".into(),
                )
            })?;
            let output = Command::new("net")
                .args(["start", t])
                .output()
                .map_err(|e| {
                    sk_types::SovereignError::ToolExecutionError(format!(
                        "Failed to start service {}: {}",
                        t, e
                    ))
                })?;
            if output.status.success() {
                Ok(format!("Successfully started service {}", t))
            } else {
                Err(sk_types::SovereignError::ToolExecutionError(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            }
        }
        "stop_service" => {
            let t = target.ok_or_else(|| {
                sk_types::SovereignError::ToolExecutionError(
                    "Target service name required for stop_service".into(),
                )
            })?;
            let output = Command::new("net")
                .args(["stop", t])
                .output()
                .map_err(|e| {
                    sk_types::SovereignError::ToolExecutionError(format!(
                        "Failed to stop service {}: {}",
                        t, e
                    ))
                })?;
            if output.status.success() {
                Ok(format!("Successfully stopped service {}", t))
            } else {
                Err(sk_types::SovereignError::ToolExecutionError(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            }
        }
        "set_env_var" => {
            let t = target.ok_or_else(|| {
                sk_types::SovereignError::ToolExecutionError(
                    "Target variable name required for set_env_var".into(),
                )
            })?;
            let v = value.unwrap_or("");
            let output = Command::new("setx").args([t, v]).output().map_err(|e| {
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to set env var {}: {}",
                    t, e
                ))
            })?;
            if output.status.success() {
                Ok(format!(
                    "Successfully set environment variable {} to {} (persistent)",
                    t, v
                ))
            } else {
                Err(sk_types::SovereignError::ToolExecutionError(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            }
        }
        "get_system_info" => {
            let output = Command::new("systeminfo").output().map_err(|e| {
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to get system info: {}",
                    e
                ))
            })?;
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        _ => Err(sk_types::SovereignError::ToolExecutionError(format!(
            "Unknown system_config action: {}",
            action
        ))),
    }
}
