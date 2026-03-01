//! Hybrid search — combines BM25 keyword search with vector similarity.
//!
//! Ported from OpenClaw's hybrid.ts. Uses Reciprocal Rank Fusion (RRF)
//! to merge results from both search backends.

/// A hybrid search result combining BM25 and vector scores.
#[derive(Debug, Clone)]
pub struct HybridResult {
    pub memory_id: String,
    pub content: String,
    /// Combined RRF score (higher = more relevant).
    pub score: f64,
    /// BM25 rank (if matched keyword search).
    pub bm25_rank: Option<f64>,
    /// Vector similarity (if matched semantic search).
    pub vector_similarity: Option<f32>,
}

/// Weight configuration for hybrid search.
#[derive(Debug, Clone)]
pub struct HybridWeights {
    /// Weight for BM25 keyword results (0.0 to 1.0).
    pub bm25_weight: f64,
    /// Weight for vector similarity results (0.0 to 1.0).
    pub vector_weight: f64,
    /// RRF constant (typically 60).
    pub rrf_k: f64,
}

impl Default for HybridWeights {
    fn default() -> Self {
        Self {
            bm25_weight: 0.4,
            vector_weight: 0.6,
            rrf_k: 60.0,
        }
    }
}

/// Merge BM25 and vector results using Reciprocal Rank Fusion.
///
/// RRF score = Σ (weight / (k + rank)) across both result sets.
/// This is robust against different score distributions between the two backends.
pub fn reciprocal_rank_fusion(
    bm25_results: &[(String, String, f64)], // (id, content, rank)
    vector_results: &[(String, String, f32)], // (id, content, similarity)
    weights: &HybridWeights,
    limit: usize,
) -> Vec<HybridResult> {
    use std::collections::HashMap;

    let mut scores: HashMap<String, HybridResult> = HashMap::new();

    // Add BM25 results
    for (rank_idx, (id, content, bm25_rank)) in bm25_results.iter().enumerate() {
        let rrf_score = weights.bm25_weight / (weights.rrf_k + rank_idx as f64 + 1.0);
        let entry = scores.entry(id.clone()).or_insert_with(|| HybridResult {
            memory_id: id.clone(),
            content: content.clone(),
            score: 0.0,
            bm25_rank: None,
            vector_similarity: None,
        });
        entry.score += rrf_score;
        entry.bm25_rank = Some(*bm25_rank);
    }

    // Add vector results
    for (rank_idx, (id, content, similarity)) in vector_results.iter().enumerate() {
        let rrf_score = weights.vector_weight / (weights.rrf_k + rank_idx as f64 + 1.0);
        let entry = scores.entry(id.clone()).or_insert_with(|| HybridResult {
            memory_id: id.clone(),
            content: content.clone(),
            score: 0.0,
            bm25_rank: None,
            vector_similarity: None,
        });
        entry.score += rrf_score;
        entry.vector_similarity = Some(*similarity);
    }

    // Sort by combined RRF score descending
    let mut results: Vec<HybridResult> = scores.into_values().collect();
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(limit);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rrf_empty_inputs() {
        let results = reciprocal_rank_fusion(&[], &[], &HybridWeights::default(), 10);
        assert!(results.is_empty());
    }

    #[test]
    fn rrf_bm25_only() {
        let bm25 = vec![("a".into(), "content a".into(), 1.0)];
        let results = reciprocal_rank_fusion(&bm25, &[], &HybridWeights::default(), 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory_id, "a");
        assert!(results[0].bm25_rank.is_some());
        assert!(results[0].vector_similarity.is_none());
    }

    #[test]
    fn rrf_overlap_boosts_score() {
        let bm25 = vec![
            ("a".into(), "content a".into(), 1.0),
            ("b".into(), "content b".into(), 2.0),
        ];
        let vector = vec![
            ("a".into(), "content a".into(), 0.95),
            ("c".into(), "content c".into(), 0.80),
        ];
        let results = reciprocal_rank_fusion(&bm25, &vector, &HybridWeights::default(), 10);

        // "a" appears in both → should have highest score
        assert_eq!(results[0].memory_id, "a");
        assert!(results[0].bm25_rank.is_some());
        assert!(results[0].vector_similarity.is_some());
    }
}
