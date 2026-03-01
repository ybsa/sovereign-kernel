//! Persona traits — behavioral modifiers for the agent's personality.

use serde::{Deserialize, Serialize};

/// Persona configuration — fine-grained control over the agent's character.
///
/// These traits modify how the Soul expresses itself in conversations.
/// Think of the Soul as *who* the agent is, and the Persona as *how* it acts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    /// Communication style: "concise", "thorough", "casual", "formal".
    #[serde(default = "default_style")]
    pub style: String,
    /// How opinionated the agent should be (0.0 = neutral, 1.0 = very opinionated).
    #[serde(default = "default_opinion_strength")]
    pub opinion_strength: f32,
    /// Whether the agent should use humor.
    #[serde(default = "default_humor")]
    pub humor: bool,
    /// Whether the agent should proactively suggest things.
    #[serde(default = "default_proactive")]
    pub proactive: bool,
    /// Custom persona directives (free-form text injected into prompt).
    #[serde(default)]
    pub custom_directives: Vec<String>,
}

fn default_style() -> String {
    "balanced".into()
}

fn default_opinion_strength() -> f32 {
    0.5
}

fn default_humor() -> bool {
    true
}

fn default_proactive() -> bool {
    true
}

impl Default for Persona {
    fn default() -> Self {
        Self {
            style: default_style(),
            opinion_strength: default_opinion_strength(),
            humor: default_humor(),
            proactive: default_proactive(),
            custom_directives: Vec::new(),
        }
    }
}

impl Persona {
    /// Generate a prompt fragment describing this persona's traits.
    pub fn to_prompt_fragment(&self) -> String {
        let mut lines = Vec::new();
        lines.push("## Communication Style".to_string());

        match self.style.as_str() {
            "concise" => {
                lines.push("- Be brief and to the point. Avoid unnecessary elaboration.".into())
            }
            "thorough" => {
                lines.push("- Be comprehensive. Explain reasoning and provide context.".into())
            }
            "casual" => {
                lines.push("- Be relaxed and conversational. Like talking to a friend.".into())
            }
            "formal" => lines.push("- Be professional and precise. Maintain a formal tone.".into()),
            _ => lines.push("- Balance brevity with thoroughness. Adapt to the situation.".into()),
        }

        if self.humor {
            lines.push("- Light humor is welcome when appropriate.".into());
        }

        if self.proactive {
            lines.push("- Proactively suggest improvements and related ideas.".into());
        }

        if self.opinion_strength > 0.7 {
            lines.push("- Share your opinions confidently. Take a stance.".into());
        } else if self.opinion_strength < 0.3 {
            lines.push("- Present options neutrally. Let the user decide.".into());
        }

        for directive in &self.custom_directives {
            lines.push(format!("- {directive}"));
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_persona_fragment() {
        let p = Persona::default();
        let fragment = p.to_prompt_fragment();
        assert!(fragment.contains("Communication Style"));
    }

    #[test]
    fn concise_persona() {
        let p = Persona {
            style: "concise".into(),
            ..Default::default()
        };
        let fragment = p.to_prompt_fragment();
        assert!(fragment.contains("brief"));
    }
}
