//! Host-level file operations that bypass workspace sandbox restrictions.
use sk_types::ToolDefinition;
use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Tool Definitions
// ---------------------------------------------------------------------------

pub fn host_read_file_tool() -> ToolDefinition {
    ToolDefinition {
        name: "host_read_file".into(),
        description:
            "Read a file's contents from ANYWHERE on the host system. Requires Unrestricted mode."
                .into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
    }
}

pub fn host_write_file_tool() -> ToolDefinition {
    ToolDefinition {
        name: "host_write_file".into(),
        description:
            "Write content to ANY file on the host system. Requires Unrestricted mode. Supports append=true.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"},"append":{"type":"boolean"}},"required":["path","content"]}),
    }
}

pub fn host_list_dir_tool() -> ToolDefinition {
    ToolDefinition {
        name: "host_list_dir".into(),
        description: "List contents of ANY directory on the host system with rich metadata.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
    }
}

// ---------------------------------------------------------------------------
// Tool Handlers
// ---------------------------------------------------------------------------

const MAX_FILE_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10MB limit for host reads

pub fn handle_host_read_file(path: &str) -> Result<String, sk_types::SovereignError> {
    let p = PathBuf::from(path);

    let metadata = fs::metadata(&p).map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("Metadata error for {}: {}", path, e))
    })?;

    if metadata.len() > MAX_FILE_SIZE_BYTES {
        return Err(sk_types::SovereignError::ToolExecutionError(format!(
            "File size {} bytes exceeds maximum allowed read size of {} bytes.",
            metadata.len(),
            MAX_FILE_SIZE_BYTES
        )));
    }

    fs::read_to_string(&p).map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!(
            "Failed to read host file {}: {}",
            path, e
        ))
    })
}

pub fn handle_host_write_file(
    path: &str,
    content: &str,
    append: bool,
) -> Result<String, sk_types::SovereignError> {
    let p = PathBuf::from(path);

    // Ensure parent directories exist
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                sk_types::SovereignError::ToolExecutionError(format!(
                    "Failed to create directories for {}: {}",
                    path, e
                ))
            })?;
        }
    }

    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .append(append)
        .truncate(!append)
        .open(&p)
        .map_err(|e| {
            sk_types::SovereignError::ToolExecutionError(format!(
                "Failed to open host file {}: {}",
                path, e
            ))
        })?;

    file.write_all(content.as_bytes()).map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!(
            "Failed to write to host file {}: {}",
            path, e
        ))
    })?;

    Ok(format!(
        "Successfully {} to host path {}",
        if append { "appended" } else { "wrote" },
        path
    ))
}

pub fn handle_host_list_dir(path: &str) -> Result<String, sk_types::SovereignError> {
    let p = PathBuf::from(path);
    let mut entries = Vec::new();

    entries.push(format!(
        "{:<30} | {:<10} | {:<12}",
        "Name", "Type", "Size (bytes)"
    ));
    entries.push("-".repeat(58));

    for entry in fs::read_dir(&p).map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!(
            "Failed to list host dir {}: {}",
            path, e
        ))
    })? {
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
    Ok(entries.join("\n"))
}
