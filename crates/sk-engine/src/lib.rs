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
pub mod model_catalog;
pub mod prompt_builder;
pub mod retry;
pub mod routing;
pub mod streaming;
pub mod tool_runner;
