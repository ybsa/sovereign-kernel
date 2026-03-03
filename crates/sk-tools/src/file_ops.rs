//! File operations tools.
use sk_types::ToolDefinition;
use std::path::{Path, PathBuf};

pub fn read_file_tool() -> ToolDefinition {
    ToolDefinition {
        name: "read_file".into(),
        description: "Read a file's contents.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
    }
}
pub fn write_file_tool() -> ToolDefinition {
    ToolDefinition {
        name: "write_file".into(),
        description: "Write content to a file.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}),
    }
}
pub fn list_dir_tool() -> ToolDefinition {
    ToolDefinition {
        name: "list_dir".into(),
        description: "List directory contents.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
    }
}

/// Validates that a path is within the permitted workspace.
pub fn validate_safe_path(root: &Path, path: &str) -> Result<PathBuf, sk_types::SovereignError> {
    let base = root.canonicalize().map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("Invalid workspace root: {e}"))
    })?;

    let requested = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        base.join(path)
    };

    // For security, we canonicalize the path if it exists to resolve '..' and symlinks.
    // If it doesn't exist, we check the parent.
    let canonical_path = if requested.exists() {
        requested.canonicalize().map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!("Security error: Failed to canonicalize path: {e}"))
        })?
    } else {
        match requested.parent() {
            Some(parent) if parent.exists() => {
                let canon_parent = parent.canonicalize().map_err(|e| {
                    sk_types::SovereignError::ToolExecutionError(format!("Security error: Failed to canonicalize parent: {e}"))
                })?;
                if !canon_parent.starts_with(&base) {
                    return Err(sk_types::SovereignError::ToolExecutionError(format!(
                        "🛡️ SECURITY VIOLATION: Path '{}' is outside the permitted workspace '{}'",
                        path, base.display()
                    )));
                }
                requested
            }
            _ => requested,
        }
    };

    if !canonical_path.starts_with(&base) {
        return Err(sk_types::SovereignError::ToolExecutionError(format!(
            "🛡️ SECURITY VIOLATION: Path '{}' is outside the permitted workspace '{}'",
            path, base.display()
        )));
    }

    Ok(canonical_path)
}

pub fn handle_read_file(root: &Path, path: &str) -> Result<String, sk_types::SovereignError> {
    let safe_path = validate_safe_path(root, path)?;
    std::fs::read_to_string(safe_path)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))
}

pub fn handle_write_file(
    root: &Path,
    path: &str,
    content: &str,
) -> Result<String, sk_types::SovereignError> {
    let safe_path = validate_safe_path(root, path)?;

    // Ensure parent directories exist
    if let Some(parent) = safe_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
    }

    std::fs::write(&safe_path, content)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
    Ok(format!("Successfully wrote to {}", path))
}

pub fn handle_list_dir(root: &Path, path: &str) -> Result<String, sk_types::SovereignError> {
    let safe_path = validate_safe_path(root, path)?;
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(safe_path)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?
    {
        let entry =
            entry.map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
        let name = entry.file_name().into_string().unwrap_or_default();
        let indicator = if entry.path().is_dir() { "/" } else { "" };
        entries.push(format!("{}{}", name, indicator));
    }
    Ok(entries.join("\n"))
}
