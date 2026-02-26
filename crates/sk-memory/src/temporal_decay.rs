//! Temporal decay — time-weighted relevance scoring.
//!
//! Ported from OpenClaw's temporal-decay.ts. Memories that haven't been
//! accessed recently are gradually deprioritized, simulating natural forgetting.

/// Apply temporal decay to a relevance score.
///
/// Uses exponential decay: `score * e^(-λ * hours_since_access)`
///
/// - `base_score`: Original relevance (e.g., cosine similarity or RRF score).
/// - `hours_since_access`: Hours since this memory was last accessed.
/// - `decay_rate`: λ parameter (higher = faster decay). 0.0 = no decay.
/// - `access_count`: Number of times this memory has been recalled (reinforcement).
///
/// Frequently accessed memories decay slower (logarithmic reinforcement).
pub fn apply_temporal_decay(
    base_score: f64,
    hours_since_access: f64,
    decay_rate: f64,
    access_count: u32,
) -> f64 {
    if decay_rate <= 0.0 || hours_since_access <= 0.0 {
        return base_score;
    }

    // Reinforcement: each access reduces effective decay
    // log2(access_count + 1) so: 1 access → 1.0, 3 → 2.0, 7 → 3.0, etc.
    let reinforcement = (access_count as f64 + 1.0).log2();

    // Effective decay rate decreases with more accesses
    let effective_rate = decay_rate / reinforcement.max(1.0);

    // Exponential decay
    let decay_factor = (-effective_rate * hours_since_access / 24.0).exp();

    base_score * decay_factor
}

/// Calculate a recency boost for very recent memories.
///
/// Memories accessed within `recency_window_hours` get a boost that
/// linearly decreases to zero at the window boundary.
pub fn recency_boost(hours_since_access: f64, recency_window_hours: f64, boost_factor: f64) -> f64 {
    if hours_since_access >= recency_window_hours || recency_window_hours <= 0.0 {
        return 0.0;
    }
    boost_factor * (1.0 - hours_since_access / recency_window_hours)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_decay_when_rate_zero() {
        let score = apply_temporal_decay(1.0, 100.0, 0.0, 0);
        assert!((score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn decay_reduces_score() {
        let fresh = apply_temporal_decay(1.0, 1.0, 0.1, 0);
        let old = apply_temporal_decay(1.0, 720.0, 0.1, 0); // 30 days
        assert!(old < fresh);
    }

    #[test]
    fn access_count_reduces_decay() {
        let rarely_accessed = apply_temporal_decay(1.0, 168.0, 0.1, 1);  // 7 days, 1 access
        let often_accessed = apply_temporal_decay(1.0, 168.0, 0.1, 15);  // 7 days, 15 accesses
        assert!(often_accessed > rarely_accessed);
    }

    #[test]
    fn recency_boost_within_window() {
        let boost = recency_boost(1.0, 24.0, 0.2);
        assert!(boost > 0.0);
        assert!(boost < 0.2);
    }

    #[test]
    fn recency_boost_outside_window() {
        let boost = recency_boost(25.0, 24.0, 0.2);
        assert!((boost).abs() < 1e-10);
    }
}
