//! Kernel orchestration — the supervisor that ties everything together.
//!
//! Based on OpenFang's openfang-kernel.

pub mod auto_reply;
pub mod background;
pub mod error;
pub mod pairing;
pub mod registry;
pub mod whatsapp_gateway;
pub mod wizard;
pub mod workflow;
pub mod approval;
pub mod audit;
pub mod auth;
pub mod capabilities;
pub mod config;
pub mod config_reload;
pub mod cron;
pub mod event_bus;
pub mod heartbeat;
pub mod kernel;
pub mod metering;
pub mod scheduler;
pub mod supervisor;
pub mod triggers;

pub use kernel::SovereignKernel;
