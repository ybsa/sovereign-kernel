use sk_types::ToolDefinition;

/// Creates the tool definition for OTTO's dynamic code synthesis and execution.
pub fn ottos_outpost_tool() -> ToolDefinition {
    ToolDefinition {
        name: "ottos_outpost".into(),
        description: "Synthesize a strict, isolated environment to execute code with complex dependencies. Can run 'Inside the Box' (Zero-Pollution Docker) or 'Outside the Box' (Native Host). Use this tool whenever you need a capability or library that isn't built into the kernel.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "language": {
                    "type": "string",
                    "description": "The programming language to execute (e.g., 'python', 'node', 'bash', 'powershell')."
                },
                "execution_env": {
                    "type": "string",
                    "description": "Must be exactly 'docker' (zero-pollution isolation, default/recommended) or 'native' (if you strictly require host-level file access)."
                },
                "dependencies": {
                    "type": "array",
                    "description": "List of package manager dependencies to automatically install (e.g., ['pandas', 'requests'] or ['axios']).",
                    "items": { "type": "string" }
                },
                "code": {
                    "type": "string",
                    "description": "The complete script or code payload to execute."
                },
                "input_files": {
                    "type": "array",
                    "description": "Optional list of files to inject into the environment. Format: [{'filename': 'data.json', 'content': '...'}]",
                    "items": {
                        "type": "object",
                        "properties": {
                            "filename": { "type": "string" },
                            "content": { "type": "string" }
                        },
                        "required": ["filename", "content"]
                    }
                }
            },
            "required": ["language", "execution_env", "dependencies", "code"]
        }),
    }
}
