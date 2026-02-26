//! Native Rust MCP (Model Context Protocol) client and server.
//!
//! The Nervous System — connects the Sovereign Kernel to any MCP-compatible
//! tool server (Telegram, Google Maps, filesystem, databases, etc.).
//!
//! Supports JSON-RPC 2.0 over:
//! - **stdio**: Subprocess with stdin/stdout (most common)
//! - **SSE**: HTTP Server-Sent Events (remote servers)
//! - **Streamable HTTP**: New MCP transport (future)

pub mod client;
pub mod connectors;
pub mod protocol;
pub mod registry;
pub mod server;
pub mod transport;

pub use client::McpClient;
pub use registry::McpRegistry;
