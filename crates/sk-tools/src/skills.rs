use serde::{Deserialize, Serialize};
use sk_types::ToolDefinition;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, warn};

/// Metadata for a skill parsed from SKILL.md frontmatter.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub homepage: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// A ported skill from Sovereign Kernel.
#[derive(Debug, Clone)]
pub struct Skill {
    pub metadata: SkillMetadata,
    pub content: String,
}

/// Dynamic registry of available skills.
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    pub dir: std::path::PathBuf,
}

impl SkillRegistry {
    /// Load all skills from the tools/skills directory.
    pub fn load_from_dir(dir: impl AsRef<Path>) -> Self {
        let mut skills = HashMap::new();
        let path = dir.as_ref();

        if !path.exists() {
            warn!(path = ?path, "Skills directory not found");
            return Self {
                skills,
                dir: path.to_path_buf(),
            };
        }

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let skill_path = entry.path();
                if skill_path.is_dir() {
                    let md_path = skill_path.join("SKILL.md");
                    if md_path.exists() {
                        if let Ok(skill) = Self::parse_skill_file(&md_path) {
                            debug!(name = %skill.metadata.name, "Loaded skill");
                            skills.insert(skill.metadata.name.clone(), skill);
                        }
                    }
                }
            }
        }

        Self {
            skills,
            dir: path.to_path_buf(),
        }
    }

    /// Reload skills from the stored directory, updating the existing registry in-place.
    pub fn reload(&mut self) {
        let new_registry = Self::load_from_dir(&self.dir);
        self.skills = new_registry.skills;
    }

    fn parse_skill_file(path: &Path) -> Result<Skill, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

        // Simple frontmatter parser: look for --- delimiters
        let parts: Vec<&str> = content.split("---").collect();
        if parts.len() < 3 {
            return Err(format!("Missing frontmatter in {}", path.display()));
        }

        let yaml = parts[1];
        let md_content = parts[2..].join("---");

        let metadata: SkillMetadata = serde_yaml::from_str(yaml)
            .map_err(|e| format!("Failed to parse YAML in {}: {}", path.display(), e))?;

        Ok(Skill {
            metadata,
            content: md_content.trim().to_string(),
        })
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&SkillMetadata> {
        self.skills.values().map(|s| &s.metadata).collect()
    }
}

/// Tool to retrieve the instructions for a specific skill.
pub fn get_skill_tool() -> ToolDefinition {
    ToolDefinition {
        name: "get_skill".to_string(),
        description: "Retrieve instructions and commands for a specific capability (e.g. 'weather', 'obsidian', 'github'). Use this when you need to know how to use a local tool or service.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The name of the skill to retrieve (e.g. 'weather', 'obsidian')"
                }
            },
            "required": ["name"]
        }),
    }
}

/// Tool to list all available skills.
pub fn list_skills_tool() -> ToolDefinition {
    ToolDefinition {
        name: "list_skills".to_string(),
        description:
            "List all available expert skills/capabilities that can be loaded via 'get_skill'."
                .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

/// Handle a get_skill call.
pub fn handle_get_skill(registry: &SkillRegistry, name: &str) -> String {
    match registry.get(name) {
        Some(skill) => {
            format!(
                "# Skill: {}\n\n{}\n\n---\n\n## Instructions\n\n{}",
                skill.metadata.name, skill.metadata.description, skill.content
            )
        }
        None => format!(
            "Skill '{}' not found. Use 'list_skills' to see available capabilities.",
            name
        ),
    }
}

/// Handle a list_skills call.
pub fn handle_list_skills(registry: &SkillRegistry) -> String {
    let mut skill_list = registry.list();
    skill_list.sort_by(|a, b| a.name.cmp(&b.name));

    if skill_list.is_empty() {
        return "No skills currently available.".to_string();
    }

    let mut output = "Available skills:\n".to_string();
    for skill in skill_list {
        output.push_str(&format!("- **{}**: {}\n", skill.name, skill.description));
    }
    output
}
