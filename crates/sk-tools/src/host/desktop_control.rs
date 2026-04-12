//! Desktop control tools (wallpaper, dark mode, notifications) for Windows.
use sk_types::ToolDefinition;
use std::process::Command;

pub fn desktop_control_tool() -> ToolDefinition {
    ToolDefinition {
        name: "desktop_control".into(),
        description: "Control desktop settings like wallpaper, dark mode, and system notifications. Windows only.".into(),
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
                    "description": "The value for the action (e.g., path to wallpaper or notification message)."
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
        "set_wallpaper" => {
            // Using PowerShell to set wallpaper
            let script = format!(
                "Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public class Wallpaper {{ [DllImport(\"user32.dll\", CharSet=CharSet.Auto)] public static extern int SystemParametersInfo(int uAction, int uParam, string lpvParam, int fuWinIni); }}'; [Wallpaper]::SystemParametersInfo(20, 0, \"{}\", 3)",
                value
            );
            let output = Command::new("powershell")
                .args(["-Command", &script])
                .output()
                .map_err(|e| {
                    sk_types::SovereignError::ToolExecutionError(format!(
                        "Failed to execute powershell: {}",
                        e
                    ))
                })?;

            if output.status.success() {
                Ok(format!("Successfully set wallpaper to {}", value))
            } else {
                Err(sk_types::SovereignError::ToolExecutionError(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            }
        }
        "toggle_dark_mode" => {
            let is_dark = value.to_lowercase() == "true" || value == "1";
            let val = if is_dark { 0 } else { 1 }; // AppsUseLightTheme: 0 is dark, 1 is light

            let output = Command::new("reg")
                .args([
                    "add",
                    "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                    "/v",
                    "AppsUseLightTheme",
                    "/t",
                    "REG_DWORD",
                    "/d",
                    &val.to_string(),
                    "/f",
                ])
                .output()
                .map_err(|e| {
                    sk_types::SovereignError::ToolExecutionError(format!(
                        "Failed to execute reg command: {}",
                        e
                    ))
                })?;

            if output.status.success() {
                Ok(format!("Successfully toggled app dark mode to {}", is_dark))
            } else {
                Err(sk_types::SovereignError::ToolExecutionError(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            }
        }
        "notify" => {
            let script = format!(
                "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show(\"{}\", \"Sovereign Kernel Notification\")",
                value
            );
            Command::new("powershell")
                .args(["-Command", &script])
                .spawn()
                .map_err(|e| {
                    sk_types::SovereignError::ToolExecutionError(format!(
                        "Failed to send notification: {}",
                        e
                    ))
                })?;

            Ok(format!("Sent notification: {}", value))
        }
        _ => Err(sk_types::SovereignError::ToolExecutionError(format!(
            "Unknown desktop action: {}",
            action
        ))),
    }
}
