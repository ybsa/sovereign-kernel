use sk_types::ToolDefinition;
use std::collections::HashSet;

/// The Librarian — handles dynamic tool discovery based on keyword matching.
pub struct DiscoveryEngine {
    catalog: Vec<(String, String, ToolDefinition)>, // (name, search_text, definition)
}

impl DiscoveryEngine {
    pub fn new(catalog: Vec<(String, Vec<String>, ToolDefinition)>) -> Self {
        let mut processed = Vec::new();
        for (name, aliases, def) in catalog {
            let search_text = format!("{} {} {}", name, def.description, aliases.join(" "))
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .collect();
            processed.push((name, search_text, def));
        }
        Self { catalog: processed }
    }

    /// Discover relevant tools for a given query.
    pub fn discover(&self, query: &str, limit: usize) -> Vec<ToolDefinition> {
        if query.trim().is_empty() {
            return Vec::new();
        }

        let query_words: Vec<String> = query
            .to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 2)
            .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect())
            .collect();

        if query_words.is_empty() {
            return Vec::new();
        }

        let mut scored: Vec<(f32, ToolDefinition)> = self
            .catalog
            .iter()
            .map(|(name, text, def)| {
                let mut score = 0.0f32;
                for word in &query_words {
                    if name.contains(word.as_str()) {
                        score += 10.0;
                    }
                    if text.contains(word.as_str()) {
                        score += 1.0;
                    }
                }
                (score, def.clone())
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(limit).map(|(_, def)| def).collect()
    }
}

/// Tools that must always be loaded regardless of discovery.
pub fn core_tool_names() -> HashSet<String> {
    let mut set = HashSet::with_capacity(4);
    set.insert("remember".to_string());
    set.insert("recall".to_string());
    set.insert("list_skills".to_string());
    set.insert("get_skill".to_string());
    set
}
