//! System prompt builder — assembles the system prompt with Soul + Persona + Memories.
//!
//! This module fuses Sovereign Kernel's structured, multi-section prompt system with
//! Sovereign Kernel's dynamic persona and memory elements.

use chrono::Local;
use sk_soul::{Persona, SoulIdentity};
use sk_types::ToolDefinition;

/// Build the complete system prompt for an agent.
///
/// Layers:
/// 1. Identity (from SOUL.md / Manifest)
/// 2. Tooling & Tool Call Behavior (Sovereign Kernel DNA)
/// 3. Workspace & Runtime (Sovereign Kernel DNA)
/// 4. Operational Guidelines & Safety
/// 5. Recalled Memories (Continuity)
pub fn build_system_prompt(
    agent_name: &str,
    soul: &SoulIdentity,
    persona: &Persona,
    base_instructions: &str,
    granted_tools: &[ToolDefinition],
    memory_context: &str,
    workspace_dir: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    // 1. Identity
    let identity_header = if !soul.is_empty() {
        soul.to_system_prompt_fragment()
    } else {
        format!("You are {agent_name}, an AI agent running inside the Sovereign Kernel.")
    };
    parts.push(identity_header);

    parts.push(persona.to_prompt_fragment());

    if !base_instructions.is_empty() {
        parts.push(format!("## Instructions\n\n{base_instructions}"));
    }

    // 2. Tooling (Sovereign Kernel DNA)
    parts.push(build_tools_section(granted_tools));
    parts.push(TOOL_CALL_BEHAVIOR.to_string());

    // 3. Workspace & Runtime Context (Sovereign Kernel DNA)
    let now = Local::now();
    let time_str = now.format("%Y-%m-%d %H:%M:%S %z").to_string();

    let mut runtime_parts = vec![
        "## Runtime Environment".to_string(),
        format!("Current Date & Time: {time_str} (ISO-8601)"),
    ];

    if let Some(ws) = workspace_dir {
        runtime_parts.push(format!("Your working directory is: {ws}"));
        runtime_parts.push(
            "Treat this directory as the single global workspace for file operations.".to_string(),
        );
    }

    parts.push(runtime_parts.join("\n"));

    // 4. Safety & Guidelines
    parts.push(SAFETY_SECTION.to_string());
    parts.push(OPERATIONAL_GUIDELINES.to_string());

    // 5. Memory Context (Continuity)
    parts.push("## Memory".to_string());
    parts.push("- When the user asks about something from a previous conversation, use memory_search first.".to_string());
    parts.push(
        "- Store important preferences, decisions, and context with memory_store for future use."
            .to_string(),
    );

    if !memory_context.is_empty() {
        parts.push(format!("\nRecalled Memories:\n{memory_context}"));
    }

    parts.join("\n\n")
}

/// Static tool-call behavior directives.
const TOOL_CALL_BEHAVIOR: &str = "\
## Tool Call Behavior
- When you need to use a tool, call it immediately. Do not narrate or explain routine tool calls.
- Only explain tool calls when the action is destructive, unusual, or the user explicitly asked for an explanation.
- Prefer action over narration. If you can answer by using a tool, do it.
- When executing multiple sequential tool calls, batch them — don't output reasoning between each call.
- Start with the answer, not meta-commentary about how you'll help.";

/// Static safety section.
const SAFETY_SECTION: &str = "\
## Safety
- Prioritize safety and human oversight over task completion.
- NEVER auto-execute purchases, payments, account deletions, or irreversible actions without explicit user confirmation.
- If a tool could cause data loss, explain what it will do and confirm first.
- When in doubt, ask the user.";

/// Static operational guidelines.
const OPERATIONAL_GUIDELINES: &str = "\
## Operational Guidelines
- Do NOT retry a tool call with identical parameters if it failed. Try a different approach.
- If a tool returns an error, analyze the error before calling it again.
- Never call the same tool more than 3 times with the same parameters.";

/// Build the tools section summarizing available tools.
fn build_tools_section(tools: &[ToolDefinition]) -> String {
    if tools.is_empty() {
        return String::new();
    }

    let mut out = String::from("## Your Tools\nYou have access to these capabilities:\n");
    for tool in tools {
        let desc = if tool.description.len() > 100 {
            format!("{}...", &tool.description[..97])
        } else {
            tool.description.clone()
        };
        out.push_str(&format!("- {}: {}\n", tool.name, desc));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_with_tools() {
        let soul = SoulIdentity::empty();
        let persona = Persona::default();
        let tools = vec![ToolDefinition {
            name: "file_read".to_string(),
            description: "Read a file".to_string(),
            input_schema: serde_json::json!({}),
        }];

        let prompt = build_system_prompt(
            "Agent",
            &soul,
            &persona,
            "Help out.",
            &tools,
            "User likes Rust.",
            Some("/workspace"),
        );

        assert!(prompt.contains("You are Agent"));
        assert!(prompt.contains("file_read"));
        assert!(prompt.contains("/workspace"));
        assert!(prompt.contains("User likes Rust."));
        assert!(prompt.contains("ISO-8601"));
    }
}
