//! Shell command execution tool.
use sk_types::ToolDefinition;
pub fn shell_exec_tool() -> ToolDefinition {
    ToolDefinition {
        name: "shell_exec".into(),
        description: "Execute a shell command.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}),
    }
}

pub fn handle_shell_exec(
    policy: &sk_types::config::ExecPolicy,
    command: &str,
) -> Result<String, sk_types::SovereignError> {
    use sk_types::config::ExecSecurityMode;

    match policy.mode {
        ExecSecurityMode::Deny => {
            return Err(sk_types::SovereignError::ToolExecutionError(
                "🛡️ SECURITY POLICY: Shell execution is disabled.".into(),
            ));
        }
        ExecSecurityMode::Allowlist => {
            let binary = command.split_whitespace().next().unwrap_or("");
            let binary_name = std::path::Path::new(binary)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(binary);

            if !policy.allowed_commands.contains(&binary.to_string())
                && !policy.safe_bins.contains(&binary_name.to_string())
            {
                return Err(sk_types::SovereignError::ToolExecutionError(format!(
                    "🛡️ SECURITY VIOLATION: Command '{}' is not in the allowlist.",
                    binary
                )));
            }
        }
        ExecSecurityMode::Full => {}
    }

    let output = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", command])
            .output()
    } else {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
    };

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            if out.status.success() {
                Ok(stdout)
            } else {
                Ok(format!("Error: {}\n{}", stderr, stdout))
            }
        }
        Err(e) => Err(sk_types::SovereignError::ToolExecutionError(e.to_string())),
    }
}
