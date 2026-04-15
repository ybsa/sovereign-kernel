//! System configuration tools (services, env vars, system info).
//!
//! Platform backends:
//! - Windows: `net` / `setx` / `systeminfo`
//! - macOS:   `launchctl` / `system_profiler`
//! - Linux:   `systemctl` / `/etc/environment` / `/etc/os-release`
use sk_types::ToolDefinition;
use std::process::Command;

pub fn system_config_tool() -> ToolDefinition {
    ToolDefinition {
        name: "system_config".into(),
        description: "Manage system configurations: list/start/stop services, set persistent \
                      environment variables, and view system info. Uses net/setx on Windows, \
                      launchctl on macOS, and systemctl on Linux."
            .into(),
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
                    "description": "Target service name or environment variable name."
                },
                "value": {
                    "type": "string",
                    "description": "Value when setting an environment variable."
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
        "list_services" => list_services(),
        "start_service" => start_service(require_target(target, "start_service")?),
        "stop_service" => stop_service(require_target(target, "stop_service")?),
        "set_env_var" => set_env_var(require_target(target, "set_env_var")?, value.unwrap_or("")),
        "get_system_info" => get_system_info(),
        _ => Err(sk_types::SovereignError::ToolExecutionError(format!(
            "Unknown system_config action: {}",
            action
        ))),
    }
}

fn require_target<'a>(
    target: Option<&'a str>,
    action: &str,
) -> Result<&'a str, sk_types::SovereignError> {
    target.ok_or_else(|| {
        sk_types::SovereignError::ToolExecutionError(format!("target required for {}", action))
    })
}

// ── list_services ──────────────────────────────────────────────────────────

#[cfg(windows)]
fn list_services() -> Result<String, sk_types::SovereignError> {
    run_cmd("net", &["start"])
}

#[cfg(target_os = "macos")]
fn list_services() -> Result<String, sk_types::SovereignError> {
    run_cmd("launchctl", &["list"])
}

#[cfg(target_os = "linux")]
fn list_services() -> Result<String, sk_types::SovereignError> {
    run_cmd("systemctl", &["list-units", "--type=service", "--no-pager"])
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn list_services() -> Result<String, sk_types::SovereignError> {
    unsupported("list_services")
}

// ── start_service ──────────────────────────────────────────────────────────

#[cfg(windows)]
fn start_service(name: &str) -> Result<String, sk_types::SovereignError> {
    run_cmd_ok("net", &["start", name], &format!("Started service {}", name))
}

#[cfg(target_os = "macos")]
fn start_service(name: &str) -> Result<String, sk_types::SovereignError> {
    run_cmd_ok(
        "launchctl",
        &["start", name],
        &format!("Started service {}", name),
    )
}

#[cfg(target_os = "linux")]
fn start_service(name: &str) -> Result<String, sk_types::SovereignError> {
    run_cmd_ok(
        "systemctl",
        &["start", name],
        &format!("Started service {}", name),
    )
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn start_service(_name: &str) -> Result<String, sk_types::SovereignError> {
    unsupported("start_service")
}

// ── stop_service ───────────────────────────────────────────────────────────

#[cfg(windows)]
fn stop_service(name: &str) -> Result<String, sk_types::SovereignError> {
    run_cmd_ok("net", &["stop", name], &format!("Stopped service {}", name))
}

#[cfg(target_os = "macos")]
fn stop_service(name: &str) -> Result<String, sk_types::SovereignError> {
    run_cmd_ok(
        "launchctl",
        &["stop", name],
        &format!("Stopped service {}", name),
    )
}

#[cfg(target_os = "linux")]
fn stop_service(name: &str) -> Result<String, sk_types::SovereignError> {
    run_cmd_ok(
        "systemctl",
        &["stop", name],
        &format!("Stopped service {}", name),
    )
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn stop_service(_name: &str) -> Result<String, sk_types::SovereignError> {
    unsupported("stop_service")
}

// ── set_env_var ────────────────────────────────────────────────────────────

#[cfg(windows)]
fn set_env_var(name: &str, value: &str) -> Result<String, sk_types::SovereignError> {
    run_cmd_ok(
        "setx",
        &[name, value],
        &format!("Set {}={} (persistent, takes effect in new shells)", name, value),
    )
}

#[cfg(target_os = "macos")]
fn set_env_var(name: &str, value: &str) -> Result<String, sk_types::SovereignError> {
    // launchctl setenv makes it visible to GUI apps in this boot session.
    // For terminal persistence the user must add it to ~/.zshrc manually.
    run_cmd_ok(
        "launchctl",
        &["setenv", name, value],
        &format!(
            "Set {}={} for the current launchd session. \
             To persist across reboots add `export {}={}` to ~/.zshrc.",
            name, value, name, value
        ),
    )
}

#[cfg(target_os = "linux")]
fn set_env_var(name: &str, value: &str) -> Result<String, sk_types::SovereignError> {
    // Write to /etc/environment — the standard PAM-sourced file on most distros.
    // Requires write permission (i.e. root).
    let path = "/etc/environment";
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let prefix = format!("{}=", name);
    let new_line = format!("{}=\"{}\"", name, value);

    let mut found = false;
    let mut lines: Vec<String> = existing
        .lines()
        .map(|l| {
            if l.starts_with(&prefix) {
                found = true;
                new_line.clone()
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        lines.push(new_line);
    }

    std::fs::write(path, lines.join("\n") + "\n").map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!(
            "Failed to write {}: {} (root permissions may be required)",
            path, e
        ))
    })?;

    Ok(format!(
        "Set {}=\"{}\" in {}. Changes take effect on next login.",
        name, value, path
    ))
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn set_env_var(_name: &str, _value: &str) -> Result<String, sk_types::SovereignError> {
    unsupported("set_env_var")
}

// ── get_system_info ────────────────────────────────────────────────────────

#[cfg(windows)]
fn get_system_info() -> Result<String, sk_types::SovereignError> {
    run_cmd("systeminfo", &[])
}

#[cfg(target_os = "macos")]
fn get_system_info() -> Result<String, sk_types::SovereignError> {
    run_cmd(
        "system_profiler",
        &["SPSoftwareDataType", "SPHardwareDataType"],
    )
}

#[cfg(target_os = "linux")]
fn get_system_info() -> Result<String, sk_types::SovereignError> {
    let uname = run_cmd("uname", &["-a"]).unwrap_or_default();
    let os_release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    Ok(format!("{}\n{}", uname, os_release))
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn get_system_info() -> Result<String, sk_types::SovereignError> {
    unsupported("get_system_info")
}

// ── helpers ────────────────────────────────────────────────────────────────

fn run_cmd(prog: &str, args: &[&str]) -> Result<String, sk_types::SovereignError> {
    let out = Command::new(prog).args(args).output().map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("Failed to run {}: {}", prog, e))
    })?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn run_cmd_ok(
    prog: &str,
    args: &[&str],
    success_msg: &str,
) -> Result<String, sk_types::SovereignError> {
    let out = Command::new(prog).args(args).output().map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("Failed to run {}: {}", prog, e))
    })?;
    if out.status.success() {
        Ok(success_msg.to_string())
    } else {
        Err(sk_types::SovereignError::ToolExecutionError(
            String::from_utf8_lossy(&out.stderr).to_string(),
        ))
    }
}

#[allow(dead_code)]
fn unsupported(action: &str) -> Result<String, sk_types::SovereignError> {
    Err(sk_types::SovereignError::ToolExecutionError(format!(
        "{} is not supported on this platform.",
        action
    )))
}
