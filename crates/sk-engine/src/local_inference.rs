//! Local inference via mistral.rs or llama-cpp-rs.
//!
//! Provides an LlmDriver implementation that loads model weights
//! locally into VRAM (e.g., for an RTX 40-series GPU) instead of
//! calling a cloud API.

use crate::llm_driver::{CompletionRequest, CompletionResponse, LlmDriver, LlmError};
use async_trait::async_trait;
use tracing::info;

/// Driver for running LLMs locally on the user's hardware.
pub struct LocalInferenceDriver {
    /// Path to the GGUF or safetensors model file.
    pub model_path: String,
    /// Number of GPU layers to offload (CUDA).
    pub gpu_layers: u32,
    /// Context window size.
    pub context_size: u32,
}

impl LocalInferenceDriver {
    /// Create a new local inference driver and (ideally) heat up the model.
    pub fn new(model_path: String, gpu_layers: u32, context_size: u32) -> Self {
        info!(
            model_path = %model_path,
            gpu_layers,
            "Initializing local GPU inference harness"
        );
        Self {
            model_path,
            gpu_layers,
            context_size,
        }
    }
}

#[async_trait]
impl LlmDriver for LocalInferenceDriver {
    fn provider(&self) -> &str {
        "local_gpu"
    }

    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        // TODO: Map CompletionRequest to mistral.rs or llama-cpp-rs inputs.
        // This requires dynamically downloading the model weights (e.g. Llama-3-8B-Instruct.gguf)
        // and compiling the backend with `--features cuda`.
        //
        // As a design standard for the global OS, the Kernel expects the compiled binary
        // to have CUDA enabled on Windows.
        Err(LlmError::ParseError(
            "Local inference requires downloading multi-GB model weights (e.g., .gguf) and enabling CUDA features during compile. Driver is structurally ready for mistral.rs/llama-cpp-rs binding.".into()
        ))
    }
}
