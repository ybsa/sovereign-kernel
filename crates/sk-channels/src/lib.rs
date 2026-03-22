//! Channel Bridge Layer for the Sovereign Kernel Agent OS.
//!
//! Provides 40 pluggable messaging integrations that convert platform messages
//! into unified `ChannelMessage` events for the kernel.

pub mod adapters;
pub mod bridge;
pub mod discord;
pub mod formatter;
pub mod router;
pub mod slack;
pub mod telegram;
pub mod types;
pub mod whatsapp;
