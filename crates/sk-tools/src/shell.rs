//! Shell command execution tool.
use sk_types::ToolDefinition;
pub fn shell_exec_tool() -> ToolDefinition {
    ToolDefinition {
        name: "shell_exec".into(),
        description: "Execute a shell command.".into(),
        parameters: serde_json::json!({"type":"object","properties":{"command":{"type":"string"}},"required":["command"]}),
        source: "builtin".into(),
        required_capabilities: vec![sk_types::security::Capability::ShellExec],
    }
}

pub fn handle_shell_exec(command: &str) -> Result<String, sk_types::SovereignError> {
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
