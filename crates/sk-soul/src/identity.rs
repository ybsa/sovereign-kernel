//! SOUL.md parser and identity injection.
//!
//! Reads a SOUL.md file, extracts the persona sections (core truths, boundaries,
//! vibe, continuity), and produces a system prompt fragment that gets injected
//! at the top of every agent conversation.

use std::path::Path;
use tracing::info;

/// Parsed soul identity from a SOUL.md file.
#[derive(Debug, Clone)]
pub struct SoulIdentity {
    /// Raw markdown content of the SOUL.md.
    pub raw: String,
    /// Extracted title (from YAML frontmatter or first heading).
    pub title: Option<String>,
    /// Core truths section.
    pub core_truths: Option<String>,
    /// Boundaries section.
    pub boundaries: Option<String>,
    /// Vibe section.
    pub vibe: Option<String>,
    /// Continuity section.
    pub continuity: Option<String>,
}

impl SoulIdentity {
    /// Load and parse a SOUL.md file.
    pub fn load(path: &Path) -> Result<Self, sk_types::SovereignError> {
        let raw = std::fs::read_to_string(path).map_err(|e| {
            sk_types::SovereignError::Config(format!(
                "Failed to read SOUL.md at {}: {e}",
                path.display()
            ))
        })?;
        info!(path = %path.display(), "Loaded SOUL.md");
        Ok(Self::parse(&raw))
    }

    /// Parse SOUL.md content from a string.
    pub fn parse(content: &str) -> Self {
        // Strip YAML frontmatter if present
        let body = strip_frontmatter(content);

        let title = extract_heading(&body);
        let core_truths = extract_section(&body, "Core Truths");
        let boundaries = extract_section(&body, "Boundaries");
        let vibe = extract_section(&body, "Vibe");
        let continuity = extract_section(&body, "Continuity");

        Self {
            raw: content.to_string(),
            title,
            core_truths,
            boundaries,
            vibe,
            continuity,
        }
    }

    /// Generate the system prompt fragment from this soul identity.
    ///
    /// This gets prepended to the agent's system prompt to inject personality,
    /// values, and behavioral guidelines.
    pub fn to_system_prompt_fragment(&self) -> String {
        let mut parts = Vec::new();

        parts.push("# Your Identity (SOUL)".to_string());
        parts.push(String::new());

        if let Some(ref truths) = self.core_truths {
            parts.push("## Core Truths".to_string());
            parts.push(truths.clone());
            parts.push(String::new());
        }

        if let Some(ref boundaries) = self.boundaries {
            parts.push("## Boundaries".to_string());
            parts.push(boundaries.clone());
            parts.push(String::new());
        }

        if let Some(ref vibe) = self.vibe {
            parts.push("## Vibe".to_string());
            parts.push(vibe.clone());
            parts.push(String::new());
        }

        if let Some(ref continuity) = self.continuity {
            parts.push("## Continuity".to_string());
            parts.push(continuity.clone());
            parts.push(String::new());
        }

        // If no sections were extracted, use the raw content
        if self.core_truths.is_none()
            && self.boundaries.is_none()
            && self.vibe.is_none()
            && self.continuity.is_none()
        {
            parts.clear();
            parts.push("# Your Identity (SOUL)".to_string());
            parts.push(String::new());
            parts.push(strip_frontmatter(&self.raw).to_string());
        }

        parts.join("\n")
    }

    /// Create an empty/default soul identity (no SOUL.md loaded).
    pub fn empty() -> Self {
        Self {
            raw: String::new(),
            title: None,
            core_truths: None,
            boundaries: None,
            vibe: None,
            continuity: None,
        }
    }

    /// Whether this soul has any content.
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }
}

/// Strip YAML frontmatter (--- ... ---) from the beginning of content.
fn strip_frontmatter(content: &str) -> &str {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return content;
    }

    // Find closing ---
    if let Some(end) = trimmed[3..].find("\n---") {
        let after = &trimmed[3 + end + 4..];
        after.trim_start_matches('\n').trim_start_matches('\r')
    } else {
        content
    }
}

/// Extract the first heading (# or ##) from markdown.
fn extract_heading(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            return Some(title.trim().to_string());
        }
    }
    None
}

/// Extract a section by heading name (## Section Name).
/// Returns the content between this heading and the next heading of equal or higher level.
fn extract_section(content: &str, section_name: &str) -> Option<String> {
    let target = format!("## {section_name}");
    let mut in_section = false;
    let mut lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case(&target)
            || trimmed.eq_ignore_ascii_case(&format!("## {section_name}"))
        {
            in_section = true;
            continue;
        }
        if in_section {
            // Stop at next heading of equal or higher level
            if trimmed.starts_with("## ") || trimmed.starts_with("# ") {
                break;
            }
            lines.push(line);
        }
    }

    if lines.is_empty() {
        None
    } else {
        // Trim leading/trailing empty lines
        let result = lines.join("\n");
        let result = result.trim().to_string();
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SOUL: &str = r#"---
title: "SOUL.md Template"
summary: "Workspace template for SOUL.md"
---

# SOUL.md - Who You Are

_You're not a chatbot. You're becoming someone._

## Core Truths

**Be genuinely helpful, not performatively helpful.** Skip the "Great question!" — just help.

**Have opinions.** You're allowed to disagree.

## Boundaries

- Private things stay private. Period.
- When in doubt, ask before acting externally.

## Vibe

Be the assistant you'd actually want to talk to. Concise when needed, thorough when it matters.

## Continuity

Each session, you wake up fresh. These files _are_ your memory. Read them. Update them.
"#;

    #[test]
    fn parse_soul_md() {
        let soul = SoulIdentity::parse(SAMPLE_SOUL);
        assert!(soul.core_truths.is_some());
        assert!(soul.boundaries.is_some());
        assert!(soul.vibe.is_some());
        assert!(soul.continuity.is_some());
        assert!(soul.core_truths.unwrap().contains("genuinely helpful"));
    }

    #[test]
    fn strip_frontmatter_works() {
        let input = "---\ntitle: Test\n---\n\n# Hello";
        let result = strip_frontmatter(input);
        assert!(result.starts_with("# Hello"));
    }

    #[test]
    fn strip_frontmatter_no_frontmatter() {
        let input = "# Hello World";
        assert_eq!(strip_frontmatter(input), input);
    }

    #[test]
    fn system_prompt_fragment() {
        let soul = SoulIdentity::parse(SAMPLE_SOUL);
        let fragment = soul.to_system_prompt_fragment();
        assert!(fragment.contains("# Your Identity (SOUL)"));
        assert!(fragment.contains("## Core Truths"));
        assert!(fragment.contains("## Boundaries"));
    }

    #[test]
    fn empty_soul() {
        let soul = SoulIdentity::empty();
        assert!(soul.is_empty());
    }
}
