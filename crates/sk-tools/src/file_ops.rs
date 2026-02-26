//! File operations tools.
use sk_types::ToolDefinition;
pub fn read_file_tool() -> ToolDefinition {
    ToolDefinition { name: "read_file".into(), description: "Read a file's contents.".into(),
        parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}), source: "builtin".into() }
}
pub fn write_file_tool() -> ToolDefinition {
    ToolDefinition { name: "write_file".into(), description: "Write content to a file.".into(),
        parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]}), source: "builtin".into() }
}
pub fn list_dir_tool() -> ToolDefinition {
    ToolDefinition { name: "list_dir".into(), description: "List directory contents.".into(),
        parameters: serde_json::json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}), source: "builtin".into() }
}
