//! The Engine — agent execution loop, LLM drivers, and tool runner.
//!
//! Based on OpenFang's openfang-runtime, stripped of Hands/browser/Docker.
//! This is the beating heart of the Sovereign Kernel.

pub mod agent_loop;
pub mod compactor;
pub mod context_budget;
pub mod drivers;
pub mod llm_driver;
pub mod local_inference;
pub mod loop_guard;
pub mod media;
pub mod model_catalog;
pub mod prompt_builder;
pub mod retry;
pub mod routing;
pub mod sandbox;
pub mod streaming;
pub mod tool_runner;

pub mod a2a;
pub mod host_functions;
pub mod process_manager;
pub mod python_runtime;
pub mod runtime;
pub mod workspace_context;
