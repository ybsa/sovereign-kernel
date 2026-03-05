//! Kernel-specific error types.

use sk_types::error::SovereignError;
use thiserror::Error;

/// Kernel error type wrapping SovereignError with kernel-specific context.
#[derive(Error, Debug)]
pub enum KernelError {
    /// A wrapped SovereignError.
    #[error(transparent)]
    SovereignKernel(#[from] SovereignError),

    /// The kernel failed to boot.
    #[error("Boot failed: {0}")]
    BootFailed(String),
}

/// Alias for kernel results.
pub type KernelResult<T> = Result<T, KernelError>;
