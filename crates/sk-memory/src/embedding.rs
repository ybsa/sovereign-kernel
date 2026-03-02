//! Embedding driver trait and OpenAI-compatible implementation.
//!
//! Based on OpenFang's embedding.rs — works with any provider offering
//! a /v1/embeddings endpoint (OpenAI, Ollama, Together, etc.).

use async_trait::async_trait;
use sk_types::SovereignError;

/// Trait for computing text embeddings.
#[async_trait]
pub trait EmbeddingDriver: Send + Sync {
    /// Compute embeddings for multiple texts.
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SovereignError>;

    /// Compute embedding for a single text.
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>, SovereignError> {
        let mut results = self.embed(&[text]).await?;
        results
            .pop()
            .ok_or_else(|| SovereignError::Memory("Empty embedding response".into()))
    }

    /// Get the embedding dimensions.
    fn dimensions(&self) -> usize;
}

/// Stub for a local embedding driver (future: mistral.rs integration).
pub struct LocalEmbeddingDriver {
    dimensions: usize,
}

impl LocalEmbeddingDriver {
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

#[async_trait]
impl EmbeddingDriver for LocalEmbeddingDriver {
    async fn embed(&self, _texts: &[&str]) -> Result<Vec<Vec<f32>>, SovereignError> {
        // TODO: Integrate with mistral.rs or llama-cpp-rs for local embeddings
        Err(SovereignError::Internal(
            "Local embedding not yet implemented — use OpenAI-compatible provider".into(),
        ))
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
}
