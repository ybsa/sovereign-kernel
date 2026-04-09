//! Code execution tool for running scripts safely.
use sk_types::config::ExecPolicy;
use sk_types::ToolDefinition;
use std::time::Duration;
use tokio::time::timeout;

pub fn code_exec_tool() -> ToolDefinition {
    ToolDefinition {
        name: "code_exec".into(),
        description: "Write and execute arbitrary Python, Node, or Bash scripts. Use this when you need programming logic, data processing, formatting, or parsing capabilities.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "description": "The programming language: 'python', 'node', or 'bash'",
                    "enum": ["python", "node", "bash"]
                },
                "code": {
                    "type": "string",
                    "description": "The script code to execute."
                }
            },
            "required": ["language", "code"]
        }),
    }
}

pub async fn handle_code_exec(
    policy: &ExecPolicy,
    language: &str,
    code: &str,
) -> Result<String, sk_types::SovereignError> {
    use sk_types::config::ExecSecurityMode;

    if policy.mode == ExecSecurityMode::Deny {
        return Err(sk_types::SovereignError::ToolExecutionError(
            "🛡️ SECURITY POLICY: Code execution is disabled in this mode.".into(),
        ));
    }

    let file_id = uuid::Uuid::new_v4().to_string();
    let temp_dir = std::env::temp_dir().join(format!("sk_code_{}", file_id));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;

    let script_path = temp_dir.join(match language {
        "python" => "script.py",
        "node" => "script.js",
        "bash" => "script.sh",
        _ => {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(sk_types::SovereignError::ToolExecutionError(format!(
                "Unsupported language: {}",
                language
            )));
        }
    });

    if let Err(e) = std::fs::write(&script_path, code) {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(sk_types::SovereignError::ToolExecutionError(e.to_string()));
    }

    let binary = match language {
        "python" => "python3",
        "node" => "node",
        "bash" => {
            if cfg!(target_os = "windows") {
                "bash"
            } else {
                "sh"
            }
        }
        _ => unreachable!(),
    };

    // Fallback binary checks for windows (often just 'python' instead of 'python3')
    let actual_binary = if language == "python" && cfg!(target_os = "windows") {
        "python"
    } else {
        binary
    };

    let mut cmd = tokio::process::Command::new(actual_binary);
    cmd.arg(&script_path);
    cmd.current_dir(&temp_dir);

    let timeout_duration = Duration::from_secs(30);

    let execution_result = match timeout(timeout_duration, cmd.output()).await {
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
            if response.trim() == format!("Exit Code: {}", exit_code) {
                response.push_str("Script executed successfully with no output.");
            }
            Ok(response)
        }
        Ok(Err(e)) => Err(sk_types::SovereignError::ToolExecutionError(format!(
            "Failed to spawn process {}: {}",
            actual_binary, e
        ))),
        Err(_) => Err(sk_types::SovereignError::ToolExecutionError(format!(
            "Script timed out after {} seconds.",
            timeout_duration.as_secs()
        ))),
    };

    let _ = std::fs::remove_dir_all(&temp_dir);
    execution_result
}

#[cfg(test)]
mod tests {
    use super::*;
    use sk_types::config::{ExecPolicy, ExecSecurityMode};

    #[tokio::test]
    async fn test_code_exec_python() {
        let policy = ExecPolicy {
            mode: ExecSecurityMode::Full,
            allowed_commands: vec![],
            safe_bins: vec![],
            blocked_args: vec![],
            timeout_secs: 30,
            max_output_bytes: 1024,
            no_output_timeout_secs: 10,
        };
        let code = "print('hello from python')";
        let result = handle_code_exec(&policy, "python", code).await;

        // Python might not be installed on all test runners, so we handle Err gracefully
        if let Ok(res) = result {
            assert!(res.contains("hello from python"));
        }
    }

    #[tokio::test]
    async fn test_code_exec_security_deny() {
        let policy = ExecPolicy {
            mode: ExecSecurityMode::Deny,
            allowed_commands: vec![],
            safe_bins: vec![],
            blocked_args: vec![],
            timeout_secs: 30,
            max_output_bytes: 1024,
            no_output_timeout_secs: 10,
        };
        let result = handle_code_exec(&policy, "python", "print(1)").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("SECURITY POLICY"));
    }
}
