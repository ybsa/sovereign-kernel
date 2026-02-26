# 🏛️ Sovereign Kernel Architecture (v0.1.0)

The Sovereign Kernel is engineered as a strict 8-crate Cargo workspace. This document details the internal engineering, illustrating how the DNA of OpenFang and OpenClaw interweave to form the ultimate Agentic OS.

---

## 📦 The 8-Crate Workspace

### 1. `sk-types` (The Core Ledger)
The foundational definitions for the entire OS. Contains `Message`, `Role`, `ToolCall`, `AgentManifest`, and the unified `SovereignError`. It ensures type safety and taint tracking across crate boundaries with zero circular dependencies.

### 2. `sk-engine` (The Brain & Dynamic Router)
The unyielding execution core. It manages the agent loop, context budgets, and tool dispatch.
*   **Dynamic Model Routing**: The engine intelligently evaluates `TaskComplexity`. Simple queries are routed to `LocalLight` models (via local inference) or `CloudFast` (e.g., Gemini 1.5 Flash), while heavy coding tasks are escalated to `CloudReasoning` (e.g., Claude 3.5 Sonnet).

### 3. `sk-soul` (The Identity Layer)
Replaces OpenClaw's 31k line TypeScript prompt builder. It natively parses the `SOUL.md` markdown file to extract personas, boundaries, goals, and behavioral directives. This context is seamlessly injected into the `sk-engine` at runtime, ensuring the agent's "vibe" is universally consistent.

### 4. `sk-memory` (The Infinite Substrate)
The unified memory system built atop robust SQLite connections.
*   **Native SQLite BM25 Search**: Implements lightning-fast full-text search directly within SQLite (`fts5`).
*   **Vector Embeddings & Hybrid Search**: Fuses dense vectors with BM25 keyword matching, applying Temporal Decay and Maximal Marginal Relevance (MMR) ranking for perfect context recall.

### 5. `sk-mcp` (The Nervous System)
The native Rust implementation of the Model Context Protocol (MCP) JSON-RPC 2.0 specification.
*   **Client & Server**: The kernel can expose its own tools to external systems (Server) or consume capabilities from any MCP-compliant app (Client).
*   **Native Connectors**: We scrapped fragile Python wrappers for high-speed Rust `tokio` polling loops. Telegram, SQL databases, and file systems are deeply integrated.

### 6. `sk-tools` (The Hands)
Houses the native tool implementations exposed to the LLM, including the crucial `handle_remember`, `handle_recall`, and `handle_forget` functions which directly interface with the `sk-memory` substrate.

### 7. `sk-kernel` (The Supervisor)
Manages the OS lifecycle, event bus routing, agent scheduling, and heartbeat monitoring. It ensures the background daemon stays alive 24/7.

### 8. `sk-cli` (The Interface)
The entry point. Provides the `start` command for launching the silent background observer, and `chat` for direct terminal REPL interaction.

---

## ⚡ Hardware Offloading (CUDA / Local Inference)

The Sovereign Kernel is engineered for absolute privacy and zero latency via the **LocalInferenceDriver**.

*   **`local_inference.rs` Harness**: Provides the native architectural bindings to load `.gguf` weights dynamically using backend engines like `mistral.rs` or `llama-cpp-rs`.
*   **RTX 40-Series Support**: Fully optimized to leverage CUDA toolkits for GPU offloading. When compiled with the proper feature flags (e.g., `--features cuda`), the kernel delegates heavy tensor operations directly to your localized GPU hardware, completely bypassing cloud APIs.
