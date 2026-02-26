//! Kernel orchestration — the supervisor that ties everything together.
//!
//! Based on OpenFang's openfang-kernel, stripped of whatsapp_gateway and wizard.

pub mod capabilities;
pub mod config;
pub mod event_bus;
pub mod kernel;
pub mod metering;
pub mod scheduler;
pub mod supervisor;

pub use kernel::SovereignKernel;
