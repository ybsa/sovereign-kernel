//! Embedding driver trait and OpenAI-compatible implementation.
//!
//! Based on Sovereign Kernel's embedding.rs — works with any provider offering
//! a /v1/embeddings endpoint (OpenAI, Ollama, Together, etc.).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
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

/// OpenAI-compatible embedding driver.
pub struct OpenAIEmbeddingDriver {
    api_key: String,
    base_url: String,
    model: String,
    dimensions: usize,
    client: reqwest::Client,
}

impl OpenAIEmbeddingDriver {
    pub fn new(api_key: String, base_url: String, model: String, dimensions: usize) -> Self {
        Self {
            api_key,
            base_url,
            model,
            dimensions,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct OaiEmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize)]
struct OaiEmbeddingResponse {
    data: Vec<OaiEmbeddingData>,
}

#[derive(Deserialize)]
struct OaiEmbeddingData {
    embedding: Vec<f32>,
}

#[async_trait]
impl EmbeddingDriver for OpenAIEmbeddingDriver {
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SovereignError> {
        let url = format!("{}/embeddings", self.base_url);
        let req = OaiEmbeddingRequest {
            model: self.model.clone(),
            input: texts.iter().map(|s| s.to_string()).collect(),
        };

        let mut builder = self.client.post(&url).json(&req);
        if !self.api_key.is_empty() {
            builder = builder.header("authorization", format!("Bearer {}", self.api_key));
        }

        let resp = builder
            .send()
            .await
            .map_err(|e| SovereignError::Internal(format!("Network error: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(SovereignError::Internal(format!(
                "Embedding API error ({}): {}",
                status, body
            )));
        }

        let data: OaiEmbeddingResponse = resp
            .json()
            .await
            .map_err(|e| SovereignError::Internal(format!("Parse error: {e}")))?;

        Ok(data.data.into_iter().map(|d| d.embedding).collect())
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }
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
