use sk_types::ToolDefinition;

/// Creates the tool definition for OTTO's dynamic Rust compilation skill.
pub fn compile_rust_skill_tool() -> ToolDefinition {
    ToolDefinition {
        name: "compile_rust_skill".into(),
        description: "Permanently compile and add a new Rust native skill to the Sovereign Kernel. This will generate a Cargo project, compile it in the Docker sandbox for safety, extract the release binary, create a SKILL.md file, and hot-reload the system's Skill Registry so the new skill is instantly available to all agents via `get_skill`.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "skill_name": {
                    "type": "string",
                    "description": "The unique name of the skill (e.g., 'fast_data_parser'). Must be alphanumeric with underscores."
                },
                "description": {
                    "type": "string",
                    "description": "A short, one-sentence description of what the skill does."
                },
                "dependencies_toml": {
                    "type": "string",
                    "description": "The custom Rust dependencies to inject into Cargo.toml under [dependencies]. E.g. 'serde = \"1.0\"\\ntokio = { version = \"1\", features = [\"full\"] }'"
                },
                "code": {
                    "type": "string",
                    "description": "The raw Rust source code for src/main.rs. This must be a cohesive, working binary that takes inputs via CLI arguments or stdin, and outputs cleanly to stdout."
                },
                "instructions": {
                    "type": "string",
                    "description": "The markdown instructions that will be placed in SKILL.md. This teaches agents *how* to use the newly compiled binary (e.g. 'Use shell_exec to run ~/.sovereign/skills/my_skill/my_skill --args')."
                }
            },
            "required": ["skill_name", "description", "dependencies_toml", "code", "instructions"]
        }),
    }
}
