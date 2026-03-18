//! Sentinel — robust LLM failover and retry orchestration.
//!
//! The Sentinel wraps multiple LLM drivers and models into a single
//! reliable interface. It handles:
//! 1. Automatic retries with exponential backoff.
//! 2. Seamless failover to fallback models/providers on failure.
//! 3. Case-based routing (e.g., using simpler models for simple tasks).

use crate::llm_driver::{CompletionRequest, CompletionResponse, LlmDriver, LlmError};
use crate::retry;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, warn, error};

/// A driver that manages multiple fallback options.
pub struct SentinelDriver {
    /// List of (model_name, driver) pairs to try in order.
    entries: Vec<(String, Arc<dyn LlmDriver>)>,
    /// Maximum retries per driver.
    max_retries: u32,
}

impl SentinelDriver {
    /// Create a new sentinel driver.
    pub fn new(entries: Vec<(String, Arc<dyn LlmDriver>)>) -> Self {
        Self {
            entries,
            max_retries: retry::max_retries(),
        }
    }
}

#[async_trait]
impl LlmDriver for SentinelDriver {
    async fn complete(&self, mut request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        for (i, (model_name, driver)) in self.entries.iter().enumerate() {
            let mut last_err = None;
            
            // Update request with this driver's specific model name
            request.model = model_name.clone();

            for attempt in 0..=self.max_retries {
                if attempt > 0 {
                    let delay = retry::backoff_delay(attempt - 1);
                    info!(
                        attempt,
                        provider = driver.provider(),
                        model = %model_name,
                        "Retrying LLM call after {}ms...",
                        delay.as_millis()
                    );
                    tokio::time::sleep(delay).await;
                }

                match driver.complete(request.clone()).await {
                    Ok(resp) => {
                        if i > 0 {
                            info!(
                                provider = driver.provider(),
                                model = %model_name,
                                "Failover successful after {} previous failures.",
                                i
                            );
                        }
                        return Ok(resp);
                    }
                    Err(e) => {
                        if !e.is_retryable() && attempt < self.max_retries {
                            warn!(
                                provider = driver.provider(),
                                model = %model_name,
                                error = %e,
                                "Non-retryable error, triggering immediate failover..."
                            );
                            last_err = Some(e);
                            break; // Failover to next driver
                        }
                        
                        warn!(
                            provider = driver.provider(),
                            model = %model_name,
                            attempt,
                            error = %e,
                            "LLM call failed."
                        );
                        last_err = Some(e);
                    }
                }
            }

            if i + 1 < self.entries.len() {
                warn!(
                    provider = driver.provider(),
                    "All retries failed for provider. Trying fallback..."
                );
            } else {
                error!("All primary and fallback providers failed.");
                return Err(last_err.unwrap_or(LlmError::Overloaded("Unknown failure".into())));
            }
        }

        Err(LlmError::Overloaded("Exhausted all providers".into()))
    }

    fn provider(&self) -> &str {
        "sentinel"
    }
}
