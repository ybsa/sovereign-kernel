//! Desktop control tools (wallpaper, dark mode, notifications).
//!
//! Platform backends:
//! - Windows: PowerShell (wallpaper), registry (dark mode), MessageBox (notify)
//! - macOS:   osascript for all three actions
//! - Linux:   gsettings/feh (wallpaper), gsettings (dark mode), notify-send (notify)
use sk_types::ToolDefinition;
use std::process::Command;

pub fn desktop_control_tool() -> ToolDefinition {
    ToolDefinition {
        name: "desktop_control".into(),
        description: "Control desktop settings: wallpaper, dark mode, and notifications. \
                      Uses PowerShell/registry on Windows, osascript on macOS, \
                      and gsettings/notify-send on Linux (GNOME/freedesktop)."
            .into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["set_wallpaper", "toggle_dark_mode", "notify"],
                    "description": "The action to perform."
                },
                "value": {
                    "type": "string",
                    "description": "Absolute path to image for set_wallpaper; 'true'/'false' for toggle_dark_mode; message text for notify."
                }
            },
            "required": ["action", "value"]
        }),
    }
}

pub fn handle_desktop_control(
    action: &str,
    value: &str,
) -> Result<String, sk_types::SovereignError> {
    match action {
        "set_wallpaper" => set_wallpaper(value),
        "toggle_dark_mode" => toggle_dark_mode(value),
        "notify" => notify(value),
        _ => Err(sk_types::SovereignError::ToolExecutionError(format!(
            "Unknown desktop action: {}",
            action
        ))),
    }
}

// ── set_wallpaper ──────────────────────────────────────────────────────────

#[cfg(windows)]
fn set_wallpaper(path: &str) -> Result<String, sk_types::SovereignError> {
    let script = format!(
        "Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; \
         public class Wallpaper {{ \
             [DllImport(\"user32.dll\", CharSet=CharSet.Auto)] \
             public static extern int SystemParametersInfo(int uAction, int uParam, string lpvParam, int fuWinIni); \
         }}'; [Wallpaper]::SystemParametersInfo(20, 0, \"{}\", 3)",
        path
    );
    run_powershell(&script, &format!("Set wallpaper to {}", path))
}

#[cfg(target_os = "macos")]
fn set_wallpaper(path: &str) -> Result<String, sk_types::SovereignError> {
    let script = format!(
        "tell application \"Finder\" to set desktop picture to POSIX file \"{}\"",
        path
    );
    run_osascript(&script, &format!("Set wallpaper to {}", path))
}

#[cfg(target_os = "linux")]
fn set_wallpaper(path: &str) -> Result<String, sk_types::SovereignError> {
    let uri = format!("file://{}", path);

    // Try GNOME (gsettings) first.
    let gnome_ok = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.background", "picture-uri", &uri])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if gnome_ok {
        // Also set the dark-mode variant (GNOME 42+).
        let _ = Command::new("gsettings")
            .args([
                "set",
                "org.gnome.desktop.background",
                "picture-uri-dark",
                &uri,
            ])
            .output();
        return Ok(format!("Set wallpaper to {} (GNOME)", path));
    }

    // Fallback: feh — works on i3, openbox, and other minimal WMs.
    let feh = Command::new("feh")
        .args(["--bg-scale", path])
        .output()
        .map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!(
                "gsettings unavailable and feh failed: {}. \
                 Install feh (apt/pacman) or run under GNOME.",
                e
            ))
        })?;

    if feh.status.success() {
        Ok(format!("Set wallpaper to {} (feh)", path))
    } else {
        Err(sk_types::SovereignError::ToolExecutionError(
            String::from_utf8_lossy(&feh.stderr).to_string(),
        ))
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn set_wallpaper(_path: &str) -> Result<String, sk_types::SovereignError> {
    unsupported("set_wallpaper")
}

// ── toggle_dark_mode ───────────────────────────────────────────────────────

#[cfg(windows)]
fn toggle_dark_mode(value: &str) -> Result<String, sk_types::SovereignError> {
    let is_dark = value.eq_ignore_ascii_case("true") || value == "1";
    // AppsUseLightTheme: 0 = dark, 1 = light
    let reg_val = if is_dark { "0" } else { "1" };
    let out = Command::new("reg")
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
            "/v",
            "AppsUseLightTheme",
            "/t",
            "REG_DWORD",
            "/d",
            reg_val,
            "/f",
        ])
        .output()
        .map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!("Failed to run reg: {}", e))
        })?;
    if out.status.success() {
        Ok(format!("Dark mode set to {}", is_dark))
    } else {
        Err(sk_types::SovereignError::ToolExecutionError(
            String::from_utf8_lossy(&out.stderr).to_string(),
        ))
    }
}

#[cfg(target_os = "macos")]
fn toggle_dark_mode(value: &str) -> Result<String, sk_types::SovereignError> {
    let is_dark = value.eq_ignore_ascii_case("true") || value == "1";
    let script = format!(
        "tell application \"System Events\" to tell appearance preferences \
         to set dark mode to {}",
        is_dark
    );
    run_osascript(&script, &format!("Dark mode set to {}", is_dark))
}

#[cfg(target_os = "linux")]
fn toggle_dark_mode(value: &str) -> Result<String, sk_types::SovereignError> {
    let is_dark = value.eq_ignore_ascii_case("true") || value == "1";
    // GNOME 42+ color-scheme key.
    let scheme = if is_dark { "prefer-dark" } else { "prefer-light" };
    let out = Command::new("gsettings")
        .args(["set", "org.gnome.desktop.interface", "color-scheme", scheme])
        .output()
        .map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!(
                "Failed to run gsettings: {}",
                e
            ))
        })?;
    if out.status.success() {
        Ok(format!("Dark mode set to {} (GNOME color-scheme: {})", is_dark, scheme))
    } else {
        Err(sk_types::SovereignError::ToolExecutionError(
            String::from_utf8_lossy(&out.stderr).to_string(),
        ))
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn toggle_dark_mode(_value: &str) -> Result<String, sk_types::SovereignError> {
    unsupported("toggle_dark_mode")
}

// ── notify ─────────────────────────────────────────────────────────────────

#[cfg(windows)]
fn notify(message: &str) -> Result<String, sk_types::SovereignError> {
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         [System.Windows.Forms.MessageBox]::Show(\"{}\", \"Sovereign Kernel\")",
        message
    );
    // spawn() so the notification is non-blocking.
    Command::new("powershell")
        .args(["-Command", &script])
        .spawn()
        .map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!(
                "Failed to send notification: {}",
                e
            ))
        })?;
    Ok(format!("Sent notification: {}", message))
}

#[cfg(target_os = "macos")]
fn notify(message: &str) -> Result<String, sk_types::SovereignError> {
    let script = format!(
        "display notification \"{}\" with title \"Sovereign Kernel\"",
        message
    );
    run_osascript(&script, &format!("Sent notification: {}", message))
}

#[cfg(target_os = "linux")]
fn notify(message: &str) -> Result<String, sk_types::SovereignError> {
    let out = Command::new("notify-send")
        .args(["Sovereign Kernel", message])
        .output()
        .map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!(
                "Failed to run notify-send: {}. \
                 Install libnotify-bin (apt) or libnotify (pacman).",
                e
            ))
        })?;
    if out.status.success() {
        Ok(format!("Sent notification: {}", message))
    } else {
        Err(sk_types::SovereignError::ToolExecutionError(
            String::from_utf8_lossy(&out.stderr).to_string(),
        ))
    }
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn notify(_message: &str) -> Result<String, sk_types::SovereignError> {
    unsupported("notify")
}

// ── platform helpers ───────────────────────────────────────────────────────

#[cfg(windows)]
fn run_powershell(script: &str, success_msg: &str) -> Result<String, sk_types::SovereignError> {
    let out = Command::new("powershell")
        .args(["-Command", script])
        .output()
        .map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!(
                "Failed to run powershell: {}",
                e
            ))
        })?;
    if out.status.success() {
        Ok(success_msg.to_string())
    } else {
        Err(sk_types::SovereignError::ToolExecutionError(
            String::from_utf8_lossy(&out.stderr).to_string(),
        ))
    }
}

#[cfg(target_os = "macos")]
fn run_osascript(script: &str, success_msg: &str) -> Result<String, sk_types::SovereignError> {
    let out = Command::new("osascript")
        .args(["-e", script])
        .output()
        .map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!(
                "Failed to run osascript: {}",
                e
            ))
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
