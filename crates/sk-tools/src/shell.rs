//! Shell command execution tool.
use sk_types::ToolDefinition;

pub fn shell_exec_tool() -> ToolDefinition {
    ToolDefinition {
        name: "shell_exec".into(),
        description: "Execute a shell command. Supports optional working directory and timeout (default 30s). Returns exit code, stdout, and stderr separately.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The command to run." },
                "working_dir": { "type": "string", "description": "Optional working directory." },
                "timeout_secs": { "type": "integer", "description": "Optional timeout in seconds. Default 30s." }
            },
            "required": ["command"]
        }),
    }
}

pub async fn handle_shell_exec(
    policy: &sk_types::config::ExecPolicy,
    command: &str,
    working_dir: Option<&str>,
    timeout_secs: Option<u64>,
) -> Result<String, sk_types::SovereignError> {
    tracing::info!(
        "Executing shell command: '{}' in dir: {:?}",
        command,
        working_dir
    );

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

    let mut cmd = if cfg!(target_os = "windows") {
        // Use Base64 EncodedCommand to ensure absolutely no quoting/escaping issues with spaces
        // in usernames or complex paths.
        use base64::{engine::general_purpose, Engine as _};
        let utf16_bytes: Vec<u8> = command
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes().into_iter())
            .collect();
        let encoded_command = general_purpose::STANDARD.encode(&utf16_bytes);

        let mut c = tokio::process::Command::new("powershell");
        c.args([
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            &encoded_command,
        ]);
        c
    } else {
        let mut c = tokio::process::Command::new("sh");
        c.arg("-c").arg(command);
        c
    };

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    let timeout_duration = std::time::Duration::from_secs(timeout_secs.unwrap_or(30));

    match tokio::time::timeout(timeout_duration, cmd.output()).await {
        Ok(Ok(out)) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            let exit_code = out.status.code().unwrap_or(-1);

            let mut response = String::new();
            response.push_str(&format!("Exit Code: {}\n", exit_code));

            if !stdout.trim().is_empty() {
                response.push_str(&format!("STDOUT:\n{}\n", stdout.trim()));
            }
            if !stderr.trim().is_empty() {
                response.push_str(&format!("STDERR:\n{}\n", stderr.trim()));
            }

            // --- THE "BLINDNESS" FIX ---
            if stdout.trim().is_empty() && stderr.trim().is_empty() {
                if out.status.success() {
                    response.push_str("SUCCESS: The command finished successfully but produced no output. (Is the target empty or requested action silent?)");
                } else {
                    response.push_str("FAILURE: The command failed but produced no error output.");
                }
            }
            // ---------------------------

            tracing::info!(
                "Shell exec finished. Exit code: {}, STDOUT length: {}, STDERR length: {}",
                exit_code,
                stdout.len(),
                stderr.len()
            );
            Ok(response)
        }
        Ok(Err(e)) => {
            tracing::error!("Shell exec failed to run: {}", e);
            Err(sk_types::SovereignError::ToolExecutionError(e.to_string()))
        }

        Err(_) => Err(sk_types::SovereignError::ToolExecutionError(format!(
            "Command timed out after {} seconds.",
            timeout_duration.as_secs()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sk_types::config::{ExecPolicy, ExecSecurityMode};

    #[tokio::test]
    async fn test_shell_exec_basic() {
        let policy = ExecPolicy {
            mode: ExecSecurityMode::Full,
            allowed_commands: vec![],
            safe_bins: vec![],
            blocked_args: vec![],
            timeout_secs: 30,
            max_output_bytes: 1024,
            no_output_timeout_secs: 10,
        };
        let cmd = "echo hello";
        let result = handle_shell_exec(&policy, cmd, None, None).await.unwrap();
        assert!(result.contains("hello"));
        assert!(result.contains("Exit Code: 0"));
    }

    #[tokio::test]
    async fn test_shell_exec_timeout() {
        let policy = ExecPolicy {
            mode: ExecSecurityMode::Full,
            allowed_commands: vec![],
            safe_bins: vec![],
            blocked_args: vec![],
            timeout_secs: 30,
            max_output_bytes: 1024,
            no_output_timeout_secs: 10,
        };
        // Command that sleeps longer than timeout
        let cmd = if cfg!(target_os = "windows") {
            "ping 127.0.0.1 -n 3"
        } else {
            "sleep 2"
        };
        let result = handle_shell_exec(&policy, cmd, None, Some(1)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_shell_exec_security_deny() {
        let policy = ExecPolicy {
            mode: ExecSecurityMode::Deny,
            allowed_commands: vec![],
            safe_bins: vec![],
            blocked_args: vec![],
            timeout_secs: 30,
            max_output_bytes: 1024,
            no_output_timeout_secs: 10,
        };
        let result = handle_shell_exec(&policy, "ls", None, None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SECURITY POLICY"));
    }
}
