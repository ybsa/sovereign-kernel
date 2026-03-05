//! Maximal Marginal Relevance (MMR) for diversity-aware result re-ranking.
//!
//! Ported from Sovereign Kernel's mmr.ts. MMR balances relevance with diversity
//! by penalizing results that are too similar to already selected results.

use crate::semantic::cosine_similarity;

/// Re-rank results using MMR to maximize both relevance and diversity.
///
/// λ controls the trade-off:
/// - λ = 1.0: Pure relevance (no diversity)
/// - λ = 0.0: Pure diversity (no relevance)
/// - λ = 0.5: Balanced (typical default)
///
/// Algorithm:
/// 1. Start with the most relevant result
/// 2. For each remaining slot, pick the result that maximizes:
///    λ * similarity(query, doc) - (1 - λ) * max(similarity(doc, selected))
pub fn mmr_rerank(
    _query_embedding: &[f32],
    candidates: &[(String, Vec<f32>, f32)], // (id, embedding, relevance_score)
    lambda: f32,
    limit: usize,
) -> Vec<(String, f32)> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let n = limit.min(candidates.len());
    let mut selected: Vec<usize> = Vec::with_capacity(n);
    let mut remaining: Vec<usize> = (0..candidates.len()).collect();

    // Pick the most relevant as first
    let first = remaining
        .iter()
        .copied()
        .max_by(|&a, &b| {
            candidates[a]
                .2
                .partial_cmp(&candidates[b].2)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();

    selected.push(first);
    remaining.retain(|&i| i != first);

    // Iteratively select remaining
    while selected.len() < n && !remaining.is_empty() {
        let mut best_idx = remaining[0];
        let mut best_mmr = f32::NEG_INFINITY;

        for &candidate_idx in &remaining {
            let relevance = lambda * candidates[candidate_idx].2;

            // Max similarity to any already-selected result
            let max_sim = selected
                .iter()
                .map(|&sel_idx| {
                    cosine_similarity(&candidates[candidate_idx].1, &candidates[sel_idx].1)
                })
                .fold(f32::NEG_INFINITY, f32::max);

            let diversity_penalty = (1.0 - lambda) * max_sim;
            let mmr_score = relevance - diversity_penalty;

            if mmr_score > best_mmr {
                best_mmr = mmr_score;
                best_idx = candidate_idx;
            }
        }

        selected.push(best_idx);
        remaining.retain(|&i| i != best_idx);
    }

    selected
        .into_iter()
        .map(|idx| (candidates[idx].0.clone(), candidates[idx].2))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mmr_empty() {
        let result = mmr_rerank(&[1.0, 0.0], &[], 0.5, 5);
        assert!(result.is_empty());
    }

    #[test]
    fn mmr_single_candidate() {
        let candidates = vec![("a".into(), vec![1.0, 0.0], 0.9)];
        let result = mmr_rerank(&[1.0, 0.0], &candidates, 0.5, 5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "a");
    }

    #[test]
    fn mmr_diverse_selection() {
        let query = vec![1.0, 0.0];
        let candidates = vec![
            ("a".into(), vec![0.99, 0.01], 0.95), // Very similar to query
            ("b".into(), vec![0.98, 0.02], 0.94), // Almost identical to "a"
            ("c".into(), vec![0.0, 1.0], 0.70),   // Very different direction
        ];

        // With diversity (λ=0.5), "c" should be preferred over "b"
        let result = mmr_rerank(&query, &candidates, 0.5, 3);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, "a"); // Most relevant first
                                      // "c" should come before "b" due to diversity
        assert_eq!(result[1].0, "c");
        assert_eq!(result[2].0, "b");
    }
}
