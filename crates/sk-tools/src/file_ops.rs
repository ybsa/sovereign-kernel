//! File operations tools.
use sk_types::ToolDefinition;
pub fn read_file_tool() -> ToolDefinition {
    ToolDefinition {
        name: "read_file".into(),
        description: "Read a file's contents.".into(),
        parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
        source: "".into(),
        required_capabilities: vec![],
    }
}
pub fn write_file_tool() -> ToolDefinition {
    ToolDefinition {
        name: "write_file".into(),
        description: "Write content to a file.".into(),
        parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}),
        source: "".into(),
        required_capabilities: vec![],
    }
}
pub fn list_dir_tool() -> ToolDefinition {
    ToolDefinition {
        name: "list_dir".into(),
        description: "List directory contents.".into(),
        parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
        source: "".into(),
        required_capabilities: vec![],
    }
}

pub fn handle_read_file(path: &str) -> Result<String, sk_types::SovereignError> {
    std::fs::read_to_string(path)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))
}

pub fn handle_write_file(path: &str, content: &str) -> Result<String, sk_types::SovereignError> {
    std::fs::write(path, content)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
    Ok(format!("Successfully wrote to {}", path))
}

pub fn handle_list_dir(path: &str) -> Result<String, sk_types::SovereignError> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?
    {
        let entry =
            entry.map_err(|e| sk_types::SovereignError::ToolExecutionError(e.to_string()))?;
        entries.push(entry.file_name().into_string().unwrap_or_default());
    }
    Ok(entries.join("\n"))
}
