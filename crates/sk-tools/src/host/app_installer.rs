//! Application installer tool.
//!
//! Uses winget on Windows, Homebrew on macOS, and the first available
//! package manager (apt-get / dnf / pacman / zypper) on Linux.
use sk_types::ToolDefinition;
use std::process::Command;

pub fn app_installer_tool() -> ToolDefinition {
    ToolDefinition {
        name: "app_installer".into(),
        description: "Install applications on the host system. Uses winget on Windows, \
                      Homebrew on macOS, and apt-get/dnf/pacman/zypper on Linux. \
                      Requires Unrestricted mode."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "package_id": {
                    "type": "string",
                    "description": "Package identifier. Use winget ID on Windows (e.g. 'Mozilla.Firefox'), \
                                    Homebrew formula on macOS (e.g. 'firefox'), \
                                    or distro package name on Linux (e.g. 'firefox')."
                }
            },
            "required": ["package_id"]
        }),
    }
}

pub fn handle_app_installer(package_id: &str) -> Result<String, sk_types::SovereignError> {
    #[cfg(windows)]
    {
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
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to execute winget: {}",
                    e
                ))
            })?;

        return if output.status.success() {
            Ok(format!("Successfully installed {}", package_id))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Err(sk_types::SovereignError::ToolExecutionError(format!(
                "winget failed:\n{}\n{}",
                stdout, stderr
            )))
        };
    }

    #[cfg(target_os = "macos")]
    {
        let output = Command::new("brew")
            .args(["install", package_id])
            .output()
            .map_err(|e| {
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to execute brew: {}. Is Homebrew installed?",
                    e
                ))
            })?;

        return if output.status.success() {
            Ok(format!("Successfully installed {} via Homebrew", package_id))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(sk_types::SovereignError::ToolExecutionError(format!(
                "brew install failed: {}",
                stderr
            )))
        };
    }

    #[cfg(target_os = "linux")]
    {
        // Try package managers in preference order.
        let managers: &[(&str, &[&str])] = &[
            ("apt-get", &["install", "-y", package_id]),
            ("dnf", &["install", "-y", package_id]),
            ("pacman", &["-S", "--noconfirm", package_id]),
            ("zypper", &["install", "-y", package_id]),
        ];

        for (mgr, args) in managers {
            if bin_exists(mgr) {
                let output = Command::new(mgr).args(*args).output().map_err(|e| {
                    sk_types::SovereignError::ToolExecutionError(format!(
                        "Failed to execute {}: {}",
                        mgr, e
                    ))
                })?;

                return if output.status.success() {
                    Ok(format!(
                        "Successfully installed {} via {}",
                        package_id, mgr
                    ))
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    Err(sk_types::SovereignError::ToolExecutionError(format!(
                        "{} failed: {}",
                        mgr, stderr
                    )))
                };
            }
        }

        return Err(sk_types::SovereignError::ToolExecutionError(
            "No supported package manager found (tried apt-get, dnf, pacman, zypper).".into(),
        ));
    }

    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    Err(sk_types::SovereignError::ToolExecutionError(
        "app_installer is not supported on this platform.".into(),
    ))
}

#[cfg(target_os = "linux")]
fn bin_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
