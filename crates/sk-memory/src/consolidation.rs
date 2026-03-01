//! Memory consolidation — periodic compaction and summarization.
//!
//! Old memories are consolidated into summaries to keep the total count
//! manageable, similar to how OpenFang's compactor works.

use sk_types::SovereignResult;

/// Configuration for memory consolidation.
#[derive(Debug, Clone)]
pub struct ConsolidationConfig {
    /// Trigger consolidation when memory count exceeds this.
    pub threshold: usize,
    /// Target count after consolidation.
    pub target_count: usize,
    /// Age in hours before a memory is eligible for consolidation.
    pub min_age_hours: f64,
}

impl Default for ConsolidationConfig {
    fn default() -> Self {
        Self {
            threshold: 500,
            target_count: 200,
            min_age_hours: 168.0, // 7 days
        }
    }
}

/// Check if consolidation is needed.
pub fn needs_consolidation(current_count: usize, config: &ConsolidationConfig) -> bool {
    current_count > config.threshold
}

/// Identify memories that should be consolidated (oldest, least accessed).
///
/// Returns memory IDs that are candidates for consolidation.
/// The actual summarization requires an LLM call (handled by the engine).
pub fn select_consolidation_candidates(
    memories: &[(String, f64, u32)], // (id, hours_since_access, access_count)
    config: &ConsolidationConfig,
) -> SovereignResult<Vec<String>> {
    let target_removal = memories.len().saturating_sub(config.target_count);
    if target_removal == 0 {
        return Ok(Vec::new());
    }

    // Score each memory: lower score = more consolidation-worthy
    let mut scored: Vec<(String, f64)> = memories
        .iter()
        .filter(|(_, hours, _)| *hours >= config.min_age_hours)
        .map(|(id, hours, access_count)| {
            // Preservation score: high access count + recent = keep
            let recency_score = 1.0 / (1.0 + hours / 24.0);
            let access_score = (*access_count as f64).ln_1p();
            let preservation = recency_score + access_score;
            (id.clone(), preservation)
        })
        .collect();

    // Sort by preservation score ascending (least worth keeping first)
    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take the least valuable up to target
    let candidates: Vec<String> = scored
        .into_iter()
        .take(target_removal)
        .map(|(id, _)| id)
        .collect();

    Ok(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_consolidation_check() {
        let config = ConsolidationConfig::default();
        assert!(!needs_consolidation(100, &config));
        assert!(needs_consolidation(600, &config));
    }

    #[test]
    fn select_candidates() {
        let config = ConsolidationConfig {
            threshold: 5,
            target_count: 3,
            min_age_hours: 1.0,
            ..Default::default()
        };
        let memories = vec![
            ("a".into(), 200.0, 1u32), // Old, rarely accessed
            ("b".into(), 100.0, 5),    // Medium age, some accesses
            ("c".into(), 0.5, 10),     // Very recent, lots of accesses (too young)
            ("d".into(), 300.0, 0),    // Very old, never accessed
            ("e".into(), 50.0, 3),     // Medium
        ];
        let candidates = select_consolidation_candidates(&memories, &config).unwrap();
        // Should select 2 candidates (5 - 3 = 2), not including "c" (too young)
        assert_eq!(candidates.len(), 2);
        // "d" (oldest, 0 accesses) should be first candidate
        assert!(candidates.contains(&"d".to_string()));
    }
}
