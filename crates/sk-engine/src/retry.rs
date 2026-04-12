//! Retry logic with exponential backoff (from Sovereign Kernel's retry.rs).

use std::time::Duration;

const BASE_DELAY_MS: u64 = 3000;
const MAX_RETRIES: u32 = 8;

/// Calculate backoff delay for a retry attempt.
pub fn backoff_delay(attempt: u32) -> Duration {
    let delay_ms = BASE_DELAY_MS * 2u64.pow(attempt.min(5));
    Duration::from_millis(delay_ms)
}

/// Get the maximum number of retries.
pub fn max_retries() -> u32 {
    MAX_RETRIES
}
