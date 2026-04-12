//! The Engine — agent execution loop, LLM drivers, and tool runner.
//!
//! Based on Sovereign Kernel's Sovereign Kernel-runtime.
//! This is the beating heart of the Sovereign Kernel.
//! Includes Hands, browser, Docker sandbox, and MCP.

pub mod agent_loop;
pub mod compactor;
pub mod context_budget;
pub mod drivers;
pub mod forensics;
pub mod llm_driver;
pub mod local_inference;
pub mod loop_guard;

pub mod model_catalog;
pub mod prompt_builder;
pub mod retry;
pub mod routing;
pub mod sentinel;

pub mod a2a;
pub mod host_functions;
pub mod process_manager;
pub mod python_runtime;
pub mod runtime;
pub mod workspace_context;
