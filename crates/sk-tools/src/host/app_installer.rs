//! Application installer tool via winget for Windows.
use sk_types::ToolDefinition;
use std::process::Command;

pub fn app_installer_tool() -> ToolDefinition {
    ToolDefinition {
        name: "host_install_app".into(),
        description:
            "Install applications on the host system using winget. Requires Unrestricted mode."
                .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "package_id": {
                    "type": "string",
                    "description": "The winget package ID to install (e.g., 'Mozilla.Firefox')."
                }
            },
            "required": ["package_id"]
        }),
    }
}

pub fn handle_app_installer(package_id: &str) -> Result<String, sk_types::SovereignError> {
    let output = Command::new("winget")
        .args([
            "install",
            "--id",
            package_id,
            "--silent",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ])
        .output()
        .map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!("Failed to execute winget: {}", e))
        })?;

    if output.status.success() {
        Ok(format!("Successfully installed {}", package_id))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Err(sk_types::SovereignError::ToolExecutionError(format!(
            "Winget failed: {}\n{}",
            stdout, stderr
        )))
    }
}
