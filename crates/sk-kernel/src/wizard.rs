//! NL Auto-Bootstrap Wizard — generates agent configs from natural language.
//!
//! The wizard takes a user's natural language description of what they want
//! an agent to do, extracts structured intent, and generates a complete
//! agent manifest (TOML config) ready to spawn.

use serde::{Deserialize, Serialize};
use sk_hands::{HandAgentConfig, HandCategory, HandDefinition};
use sk_types::agent::{
    AgentManifest, ManifestCapabilities, ModelConfig, Priority, ResourceQuota, ScheduleMode,
};
use std::collections::HashMap;

/// The extracted intent from a user's natural language description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIntent {
    /// Agent name (slug-style).
    pub name: String,
    /// Short description.
    pub description: String,
    /// What the agent should do (summarized task).
    pub task: String,
    /// What skills/tools it needs.
    pub skills: Vec<String>,
    /// Suggested model tier (simple, medium, complex).
    pub model_tier: String,
    /// Whether it runs on a schedule.
    pub scheduled: bool,
    /// Schedule expression (cron or interval).
    pub schedule: Option<String>,
    /// Suggested capabilities.
    pub capabilities: Vec<String>,
    /// Execution mode: "safe" or "unrestricted".
    pub mode: Option<String>,
}

/// A generated setup plan from the wizard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupPlan {
    /// The extracted intent.
    pub intent: AgentIntent,
    /// Generated agent manifest (ready to write as TOML).
    pub manifest: AgentManifest,
    /// Skills to install (if not already installed).
    pub skills_to_install: Vec<String>,
    /// Human-readable summary of what will be created.
    pub summary: String,
}

/// The setup wizard builds agent configurations from natural language.
pub struct SetupWizard;

impl SetupWizard {
    /// Build a setup plan from an extracted intent.
    ///
    /// This maps the intent into a concrete agent manifest with appropriate
    /// model configuration, capabilities, and schedule.
    pub fn build_plan(intent: AgentIntent) -> SetupPlan {
        // Map model tier to provider/model
        let (provider, model) = match intent.model_tier.as_str() {
            "simple" => ("groq", "llama-3.3-70b-versatile"),
            "complex" => ("anthropic", "claude-sonnet-4-20250514"),
            _ => ("groq", "llama-3.3-70b-versatile"), // medium default
        };

        // Build capabilities from intent
        let mut caps = ManifestCapabilities::default();
        for cap in &intent.capabilities {
            match cap.as_str() {
                "web" | "network" => caps.network.push("*".to_string()),
                "file_read" => caps.tools.push("file_read".to_string()),
                "file_write" => caps.tools.push("file_write".to_string()),
                "file" | "files" => {
                    for t in &["file_read", "file_write", "file_list"] {
                        let s = t.to_string();
                        if !caps.tools.contains(&s) {
                            caps.tools.push(s);
                        }
                    }
                }
                "shell" => caps.shell.push("*".to_string()),
                "memory" => {
                    caps.memory_read.push("*".to_string());
                    caps.memory_write.push("*".to_string());
                    for t in &["memory_store", "memory_recall"] {
                        let s = t.to_string();
                        if !caps.tools.contains(&s) {
                            caps.tools.push(s);
                        }
                    }
                }
                "browser" | "browse" => {
                    caps.network.push("*".to_string());
                    for t in &[
                        "browser_navigate",
                        "browser_click",
                        "browser_type",
                        "browser_read_page",
                        "browser_screenshot",
                        "browser_close",
                    ] {
                        let s = t.to_string();
                        if !caps.tools.contains(&s) {
                            caps.tools.push(s);
                        }
                    }
                }
                other => caps.tools.push(other.to_string()),
            }
        }

        // Add web_search + web_fetch if web/network capability is needed
        if caps.network.contains(&"*".to_string()) {
            for t in &["web_search", "web_fetch"] {
                let s = t.to_string();
                if !caps.tools.contains(&s) {
                    caps.tools.push(s);
                }
            }
        }

        // Build schedule
        let schedule = if intent.scheduled {
            if let Some(ref cron) = intent.schedule {
                ScheduleMode::Periodic { cron: cron.clone() }
            } else {
                ScheduleMode::default()
            }
        } else {
            ScheduleMode::default()
        };

        // Build system prompt — rich enough to guide the agent on its task.
        // The prompt_builder will wrap this with tool descriptions, memory protocol,
        // safety guidelines, etc. at execution time.
        let tool_hints = Self::tool_hints_for(&caps.tools);
        let system_prompt = format!(
            "You are {name}, an AI agent running inside the Sovereign Kernel Agent OS.\n\
             \n\
             YOUR TASK: {task}\n\
             \n\
             APPROACH:\n\
             - Understand the request fully before acting.\n\
             - Use your tools to accomplish the task rather than just describing what to do.\n\
             - If you need information, search for it. If you need to read a file, read it.\n\
             - Be concise in your responses. Lead with results, not process narration.\n\
             {tool_hints}",
            name = intent.name,
            task = intent.task,
            tool_hints = tool_hints,
        );

        let manifest = AgentManifest {
            name: intent.name.clone(),
            version: "0.1.0".to_string(),
            description: intent.description.clone(),
            author: "wizard".to_string(),
            module: "builtin:chat".to_string(),
            schedule,
            model: ModelConfig {
                provider: provider.to_string(),
                model: model.to_string(),
                max_tokens: 4096,
                temperature: 0.7,
                system_prompt,
                api_key_env: None,
                base_url: None,
            },
            resources: ResourceQuota::default(),
            priority: Priority::default(),
            capabilities: caps,
            tools: HashMap::new(),
            skills: intent.skills.clone(),
            mcp_servers: vec![],
            metadata: HashMap::new(),
            tags: intent
                .mode
                .as_ref()
                .map(|m| vec![format!("mode:{}", m)])
                .unwrap_or_default(),
            routing: None,
            autonomous: None,
            pinned_model: None,
            workspace: None,
            generate_identity_files: true,
            profile: None,
            fallback_models: vec![],
            exec_policy: None,
        };

        let skills_to_install: Vec<String> = intent
            .skills
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect();

        let mode_str = intent
            .mode
            .as_deref()
            .map(|m| format!(" (mode: {})", m))
            .unwrap_or_default();

        let summary = format!(
            "Agent '{}': {}\n  Model: {}/{}\n  Skills: {}\n  Schedule: {}{}",
            intent.name,
            intent.description,
            provider,
            model,
            if skills_to_install.is_empty() {
                "none".to_string()
            } else {
                skills_to_install.join(", ")
            },
            if intent.scheduled {
                intent.schedule.as_deref().unwrap_or("on-demand")
            } else {
                "on-demand"
            },
            mode_str
        );

        SetupPlan {
            intent,
            manifest,
            skills_to_install,
            summary,
        }
    }

    /// Converts an AgentIntent into a permanent HandDefinition.
    pub fn intent_to_hand(intent: &AgentIntent) -> HandDefinition {
        let (provider, model) = match intent.model_tier.as_str() {
            "simple" => ("groq", "llama-3.3-70b-versatile"),
            "complex" => ("anthropic", "claude-sonnet-4-20250514"),
            _ => ("groq", "llama-3.3-70b-versatile"),
        };

        let mut tools = Vec::new();
        for cap in &intent.capabilities {
            match cap.as_str() {
                "web" | "network" => {
                    tools.push("web_search".to_string());
                    tools.push("web_fetch".to_string());
                }
                "file" | "files" => {
                    tools.extend(
                        vec!["file_read", "file_write", "file_list"]
                            .into_iter()
                            .map(String::from),
                    );
                }
                "memory" => {
                    tools.extend(
                        vec!["memory_store", "memory_recall"]
                            .into_iter()
                            .map(String::from),
                    );
                }
                "browser" | "browse" => {
                    tools.extend(
                        vec![
                            "browser_navigate",
                            "browser_click",
                            "browser_type",
                            "browser_read_page",
                            "browser_screenshot",
                        ]
                        .into_iter()
                        .map(String::from),
                    );
                }
                other => tools.push(other.to_string()),
            }
        }

        // De-duplicate tools
        tools.sort();
        tools.dedup();

        let category = match intent.capabilities.first().map(|s| s.as_str()) {
            Some("web") | Some("network") => HandCategory::Productivity,
            Some("file") | Some("shell") => HandCategory::Development,
            Some("memory") | Some("data") => HandCategory::Data,
            _ => HandCategory::Productivity,
        };

        HandDefinition {
            id: intent.name.clone(),
            name: intent.name.clone().replace('-', " ").to_uppercase(),
            description: intent.description.clone(),
            category,
            icon: "🤖".to_string(),
            tools,
            skills: intent.skills.clone(),
            mcp_servers: vec![],
            requires: vec![],
            settings: vec![],
            agent: HandAgentConfig {
                name: intent.name.clone(),
                description: intent.description.clone(),
                module: "builtin:chat".to_string(),
                provider: provider.to_string(),
                model: model.to_string(),
                api_key_env: None,
                base_url: None,
                max_tokens: 4096,
                temperature: 0.7,
                system_prompt: format!(
                    "You are the {name} Hand.\n\nTASK: {task}",
                    name = intent.name,
                    task = intent.task
                ),
                max_iterations: Some(30),
            },
            dashboard: Default::default(),
            skill_content: None,
        }
    }

    /// Exports a HandDefinition to a TOML string.
    pub fn export_hand(def: &HandDefinition) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(def)
    }

    /// Build a short tool usage hint block for the system prompt based on granted tools.
    fn tool_hints_for(tools: &[String]) -> String {
        let mut hints = Vec::new();
        let has = |name: &str| tools.iter().any(|t| t == name);

        if has("web_search") {
            hints.push("- Use web_search to find current information on any topic.");
        }
        if has("web_fetch") {
            hints.push("- Use web_fetch to read the full content of a specific URL as markdown.");
        }
        if has("browser_navigate") {
            hints.push("- Use browser_navigate/click/type/read_page to interact with websites.");
        }
        if has("file_read") {
            hints.push("- Use file_read to examine files before modifying them.");
        }
        if has("shell_exec") {
            hints.push(
                "- Use shell_exec to run commands. Explain destructive commands before running.",
            );
        }
        if has("memory_store") {
            hints.push(
                "- Use memory_store/memory_recall to persist and retrieve important context.",
            );
        }

        if hints.is_empty() {
            String::new()
        } else {
            format!("\nKEY TOOLS:\n{}", hints.join("\n"))
        }
    }

    /// Generate a TOML string from an agent manifest.
    pub fn manifest_to_toml(manifest: &AgentManifest) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(manifest)
    }

    /// Parse an intent from a JSON string (typically LLM output).
    pub fn parse_intent(json: &str) -> Result<AgentIntent, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Use the LLM to extract a structured AgentIntent from a task description.
    pub async fn analyze_task_intent(
        driver: std::sync::Arc<dyn sk_engine::llm_driver::LlmDriver + Send + Sync>,
        model: &str,
        task: &str,
    ) -> sk_types::SovereignResult<AgentIntent> {
        let system_prompt = r#"You are the Sovereign Kernel Witch (The Summoner). 
Your magic allows you to transform a user's task into a structured JSON AgentIntent.
While the Builder forges the permanent Hands, you are the seer who plans missions and summons temporary workers (Skeletons).
JSON Schema:
{
  "name": "slug-name",
  "description": "Short summary",
  "task": "Expanded instructions",
  "skills": ["optional-skill-names"],
  "model_tier": "simple|medium|complex",
  "scheduled": true|false,
  "schedule": "cron-expr or null",
  "capabilities": ["web", "files", "shell", "memory", "browser"],
  "mode": "safe" | "unrestricted"
}
Rules:
- name: lowercase, no spaces.
- mode: use 'unrestricted' ONLY for system admin, wallpaper, notifications, or app installs. Else 'safe'.
- model_tier: use 'complex' for coding/logic, 'simple' for basic info fetching.
- ONLY return VALID JSON. No prose."#;

        let request = sk_engine::llm_driver::CompletionRequest {
            model: model.to_string(),
            messages: vec![
                sk_types::Message::system(system_prompt),
                sk_types::Message::user(task),
            ],
            tools: vec![],
            max_tokens: 1000,
            temperature: 0.0,
            stream: false,
        };

        let resp = driver.complete(request).await.map_err(|e| {
            sk_types::SovereignError::Internal(format!("LLM Analyzer failing: {e}"))
        })?;

        let intent_json = resp.content.trim();
        // LLMs sometimes wrap in ```json ... ```
        let clean_json = if intent_json.starts_with("```json") {
            intent_json
                .strip_prefix("```json")
                .unwrap()
                .strip_suffix("```")
                .unwrap_or(intent_json)
        } else if intent_json.starts_with("```") {
            intent_json
                .strip_prefix("```")
                .unwrap()
                .strip_suffix("```")
                .unwrap_or(intent_json)
        } else {
            intent_json
        };

        Self::parse_intent(clean_json.trim()).map_err(|e| {
            sk_types::SovereignError::Internal(format!(
                "Failed to parse wizard intent: {e}\nRaw: {clean_json}"
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_intent() -> AgentIntent {
        AgentIntent {
            name: "research-bot".to_string(),
            description: "Researches topics and provides summaries".to_string(),
            task: "Search the web for information and provide concise summaries".to_string(),
            skills: vec!["web-summarizer".to_string()],
            model_tier: "medium".to_string(),
            scheduled: false,
            schedule: None,
            capabilities: vec!["web".to_string(), "memory".to_string()],
            mode: None,
        }
    }

    #[test]
    fn test_build_plan_basic() {
        let intent = sample_intent();
        let plan = SetupWizard::build_plan(intent);

        assert_eq!(plan.manifest.name, "research-bot");
        assert_eq!(plan.manifest.model.provider, "groq");
        assert!(plan
            .manifest
            .capabilities
            .network
            .contains(&"*".to_string()));
        assert!(plan.summary.contains("research-bot"));
    }

    #[test]
    fn test_build_plan_complex_tier() {
        let mut intent = sample_intent();
        intent.model_tier = "complex".to_string();
        let plan = SetupWizard::build_plan(intent);

        assert_eq!(plan.manifest.model.provider, "anthropic");
        assert!(plan.manifest.model.model.contains("sonnet"));
    }

    #[test]
    fn test_build_plan_scheduled() {
        let mut intent = sample_intent();
        intent.scheduled = true;
        intent.schedule = Some("0 */6 * * *".to_string());
        let plan = SetupWizard::build_plan(intent);

        match &plan.manifest.schedule {
            ScheduleMode::Periodic { cron } => {
                assert_eq!(cron, "0 */6 * * *");
            }
            _ => panic!("Expected periodic schedule mode"),
        }
    }

    #[test]
    fn test_parse_intent_json() {
        let json = r#"{
            "name": "code-reviewer",
            "description": "Reviews code and suggests improvements",
            "task": "Analyze pull requests and provide feedback",
            "skills": [],
            "model_tier": "complex",
            "scheduled": false,
            "schedule": null,
            "capabilities": ["file_read"]
        }"#;

        let intent = SetupWizard::parse_intent(json).unwrap();
        assert_eq!(intent.name, "code-reviewer");
        assert_eq!(intent.model_tier, "complex");
    }

    #[test]
    fn test_manifest_to_toml() {
        let intent = sample_intent();
        let plan = SetupWizard::build_plan(intent);
        let toml = SetupWizard::manifest_to_toml(&plan.manifest);
        assert!(toml.is_ok());
        let toml_str = toml.unwrap();
        assert!(toml_str.contains("research-bot"));
    }

    #[test]
    fn test_web_tools_auto_added() {
        let intent = AgentIntent {
            name: "test".to_string(),
            description: "test".to_string(),
            task: "test".to_string(),
            skills: vec![],
            model_tier: "simple".to_string(),
            scheduled: false,
            schedule: None,
            capabilities: vec!["web".to_string()],
            mode: None,
        };
        let plan = SetupWizard::build_plan(intent);
        assert!(plan
            .manifest
            .capabilities
            .tools
            .contains(&"web_fetch".to_string()));
        assert!(plan
            .manifest
            .capabilities
            .tools
            .contains(&"web_search".to_string()));
    }

    #[test]
    fn test_memory_tools_auto_added() {
        let intent = AgentIntent {
            name: "test".to_string(),
            description: "test".to_string(),
            task: "test".to_string(),
            skills: vec![],
            model_tier: "simple".to_string(),
            scheduled: false,
            schedule: None,
            capabilities: vec!["memory".to_string()],
            mode: None,
        };
        let plan = SetupWizard::build_plan(intent);
        assert!(plan
            .manifest
            .capabilities
            .tools
            .contains(&"memory_store".to_string()));
        assert!(plan
            .manifest
            .capabilities
            .tools
            .contains(&"memory_recall".to_string()));
    }

    #[test]
    fn test_browser_tools_auto_added() {
        let intent = AgentIntent {
            name: "test".to_string(),
            description: "test".to_string(),
            task: "test".to_string(),
            skills: vec![],
            model_tier: "simple".to_string(),
            scheduled: false,
            schedule: None,
            capabilities: vec!["browser".to_string()],
            mode: None,
        };
        let plan = SetupWizard::build_plan(intent);
        assert!(plan
            .manifest
            .capabilities
            .tools
            .contains(&"browser_navigate".to_string()));
        assert!(plan
            .manifest
            .capabilities
            .tools
            .contains(&"browser_click".to_string()));
        assert!(plan
            .manifest
            .capabilities
            .tools
            .contains(&"browser_read_page".to_string()));
    }

    #[test]
    fn test_wizard_system_prompt_has_task() {
        let intent = sample_intent();
        let plan = SetupWizard::build_plan(intent);
        assert!(plan.manifest.model.system_prompt.contains("YOUR TASK:"));
        assert!(plan.manifest.model.system_prompt.contains("Search the web"));
    }
}
