//! File operations tools.
use sk_types::ToolDefinition;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Tool Definitions
// ---------------------------------------------------------------------------

pub fn read_file_tool() -> ToolDefinition {
    ToolDefinition {
        name: "read_file".into(),
        description:
            "Read a file's contents. Supports a maximum of 1MB to prevent context overflow.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
    }
}

pub fn write_file_tool() -> ToolDefinition {
    ToolDefinition {
        name: "write_file".into(),
        description:
            "Write content to a file. Optionally set append=true to add instead of overwrite."
                .into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"},"append":{"type":"boolean"}},"required":["path","content"]}),
    }
}

pub fn list_dir_tool() -> ToolDefinition {
    ToolDefinition {
        name: "list_dir".into(),
        description: "List directory contents with rich metadata (size, type).".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
    }
}

pub fn delete_file_tool() -> ToolDefinition {
    ToolDefinition {
        name: "delete_file".into(),
        description: "Delete a file or empty directory.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
    }
}

pub fn move_file_tool() -> ToolDefinition {
    ToolDefinition {
        name: "move_file".into(),
        description: "Move or rename a file or directory.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"source":{"type":"string"},"destination":{"type":"string"}},"required":["source","destination"]}),
    }
}

pub fn copy_file_tool() -> ToolDefinition {
    ToolDefinition {
        name: "copy_file".into(),
        description: "Copy a file or directory.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"source":{"type":"string"},"destination":{"type":"string"}},"required":["source","destination"]}),
    }
}

// ---------------------------------------------------------------------------
pub fn validate_safe_path(
    root: &Path,
    path: &str,
    unrestricted: bool,
) -> Result<PathBuf, sk_types::SovereignError> {
    // Ensure the workspace root directory exists; create it if missing.
    if !root.exists() {
        std::fs::create_dir_all(root).map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!(
                "Failed to create workspace root: {e}"
            ))
        })?;
    }
    let base = root.canonicalize().map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("Invalid workspace root: {e}"))
    })?;

    let requested = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        base.join(path)
    };

    let canonical_path = if requested.exists() {
        requested.canonicalize().map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!(
                "Security error: Failed to canonicalize path: {e}"
            ))
        })?
    } else {
        match requested.parent() {
            Some(parent) if parent.exists() => {
                let canon_parent = parent.canonicalize().map_err(|e| {
                    sk_types::SovereignError::ToolExecutionError(format!(
                        "Security error: Failed to canonicalize parent: {e}"
                    ))
                })?;
                if !unrestricted && !canon_parent.starts_with(&base) {
                    return Err(sk_types::SovereignError::ToolExecutionError(format!(
                        "🛡️ SECURITY VIOLATION: Path '{}' is outside the permitted workspace '{}'",
                        path,
                        base.display()
                    )));
                }
                requested
            }
            _ => requested,
        }
    };

    if !unrestricted && !canonical_path.starts_with(&base) {
        return Err(sk_types::SovereignError::ToolExecutionError(format!(
            "🛡️ SECURITY VIOLATION: Path '{}' is outside the permitted workspace '{}'",
            path,
            base.display()
        )));
    }

    Ok(canonical_path)
}

// ---------------------------------------------------------------------------
// Tool Handlers
// ---------------------------------------------------------------------------

const MAX_FILE_SIZE_BYTES: u64 = 1024 * 1024; // 1MB limit for reads

pub fn handle_read_file(
    root: &Path,
    path: &str,
    unrestricted: bool,
) -> Result<String, sk_types::SovereignError> {
    let safe_path = validate_safe_path(root, path, unrestricted)?;

    let metadata = fs::metadata(&safe_path)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;

    if metadata.len() > MAX_FILE_SIZE_BYTES {
        return Err(sk_types::SovereignError::ToolExecutionError(format!(
            "File size {} bytes exceeds maximum allowed read size of {} bytes.",
            metadata.len(),
            MAX_FILE_SIZE_BYTES
        )));
    }

    fs::read_to_string(&safe_path).map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("Failed to read file as text: {}", e))
    })
}

pub fn handle_write_file(
    root: &Path,
    path: &str,
    content: &str,
    append: bool,
    unrestricted: bool,
) -> Result<String, sk_types::SovereignError> {
    let safe_path = validate_safe_path(root, path, unrestricted)?;

    // Ensure parent directories exist
    if let Some(parent) = safe_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
    }

    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(append)
        .truncate(!append)
        .open(&safe_path)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;

    file.write_all(content.as_bytes())
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;

    Ok(format!(
        "Successfully {} to {}",
        if append { "appended" } else { "wrote" },
        path
    ))
}
pub fn handle_list_dir(
    root: &Path,
    path: &str,
    unrestricted: bool,
) -> Result<String, sk_types::SovereignError> {
    let safe_path = validate_safe_path(root, path, unrestricted)?;
    let mut entries = Vec::new();

    entries.push(format!(
        "{:<30} | {:<10} | {:<12}",
        "Name", "Type", "Size (bytes)"
    ));
    entries.push("-".repeat(58));

    for entry in fs::read_dir(safe_path)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?
    {
        let entry =
            entry.map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
        let match_name = entry
            .file_name()
            .into_string()
            .unwrap_or_else(|_| "INVALID_UTF8".to_string());

        let metadata = entry
            .metadata()
            .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
        let size = metadata.len();
        let (type_str, indicator) = if metadata.is_dir() {
            ("DIR", "/")
        } else {
            ("FILE", "")
        };

        entries.push(format!(
            "{:<30} | {:<10} | {:<12}",
            format!("{}{}", match_name, indicator),
            type_str,
            size
        ));
    }

    if entries.len() <= 2 {
        return Ok(format!("SUCCESS: Directory '{}' is empty.", path));
    }

    Ok(entries.join("\n"))
}

pub fn handle_delete_file(
    root: &Path,
    path: &str,
    unrestricted: bool,
) -> Result<String, sk_types::SovereignError> {
    let safe_path = validate_safe_path(root, path, unrestricted)?;
    let metadata = fs::metadata(&safe_path)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;

    if metadata.is_dir() {
        fs::remove_dir_all(&safe_path)
            .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
    } else {
        fs::remove_file(&safe_path)
            .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
    }

    Ok(format!("Successfully deleted {}", path))
}

pub fn handle_move_file(
    root: &Path,
    source: &str,
    dest: &str,
    unrestricted: bool,
) -> Result<String, sk_types::SovereignError> {
    let safe_src = validate_safe_path(root, source, unrestricted)?;
    let safe_dst = validate_safe_path(root, dest, unrestricted)?;

    if let Some(parent) = safe_dst.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
    }

    fs::rename(&safe_src, &safe_dst)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
    Ok(format!("Successfully moved {} to {}", source, dest))
}

pub fn handle_copy_file(
    root: &Path,
    source: &str,
    dest: &str,
    unrestricted: bool,
) -> Result<String, sk_types::SovereignError> {
    let safe_src = validate_safe_path(root, source, unrestricted)?;
    let safe_dst = validate_safe_path(root, dest, unrestricted)?;

    if let Some(parent) = safe_dst.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
    }

    let metadata = fs::metadata(&safe_src)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
    if metadata.is_dir() {
        return Err(sk_types::SovereignError::ToolExecutionError("Copying entire directories is not supported natively via this tool point yet. Use shell_exec for cp -r.".into()));
    }

    fs::copy(&safe_src, &safe_dst)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
    Ok(format!("Successfully copied {} to {}", source, dest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_validate_safe_path() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        // On Windows, canonicalize adds \\?\ prefix, so we canonicalize root too
        let canon_root = root.canonicalize().unwrap();

        // Valid relative path (for a new file, validate_safe_path returns base.join(path))
        let safe = validate_safe_path(root, "test.txt", false).unwrap();
        assert!(
            safe.starts_with(&canon_root),
            "Expected {:?} to start with {:?}",
            safe,
            canon_root
        );

        // Valid sub-directory path
        fs::create_dir(root.join("sub")).unwrap();
        let safe_sub = validate_safe_path(root, "sub/file.txt", false).unwrap();
        assert!(
            safe_sub.starts_with(&canon_root),
            "Expected {:?} to start with {:?}",
            safe_sub,
            canon_root
        );

        // Invalid: directory traversal attempt
        let result = validate_safe_path(root, "../outside.txt", false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("SECURITY VIOLATION"));
    }

    #[test]
    fn test_file_handlers() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Write file
        handle_write_file(root, "hello.txt", "world", false, false).unwrap();
        let content = handle_read_file(root, "hello.txt", false).unwrap();
        assert_eq!(content, "world");

        // Append file
        handle_write_file(root, "hello.txt", " again", true, false).unwrap();
        let content_appended = handle_read_file(root, "hello.txt", false).unwrap();
        assert_eq!(content_appended, "world again");

        // List dir
        let list = handle_list_dir(root, ".", false).unwrap();
        assert!(list.contains("hello.txt"));

        // Copy file
        handle_copy_file(root, "hello.txt", "hello_copy.txt", false).unwrap();
        assert!(root.join("hello_copy.txt").exists());

        // Move file
        handle_move_file(root, "hello_copy.txt", "hello_moved.txt", false).unwrap();
        assert!(!root.join("hello_copy.txt").exists());
        assert!(root.join("hello_moved.txt").exists());

        // Delete file
        handle_delete_file(root, "hello_moved.txt", false).unwrap();
        assert!(!root.join("hello_moved.txt").exists());
    }

    #[test]
    fn test_file_ops_unrestricted() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // This would normally fail, but with unrestricted=true it should allow absolute outside paths
        let outside_path = if cfg!(windows) { "C:\\" } else { "/" };
        let result = validate_safe_path(root, outside_path, true);
        assert!(result.is_ok());
    }
}
