# 🏛️ Sovereign Kernel Architecture (v0.1.0)

The Sovereign Kernel is a virtual operating system for AI agents, built with a Microkernel-inspired architecture in Rust. It is engineered as a robust 8-crate Cargo workspace to ensure modularity, security, and performance.

## 📦 Workspace Overview

### 1. `sk-types` (The Core Ledger)
Defines shared data structures including `Message`, `ToolCall`, `AgentManifest`, and `SovereignError`. It ensures type safety and taint tracking across crate boundaries.

### 2. `sk-engine` (The Brain)
Manages the agent logic loop, LLM driver integration (OpenAI, Copilot, Fallback), and tool execution.
- **Runtime Engine**: Houses **32 modules** ported from Sovereign Kernel, including `browser_tools`, `docker_sandbox`, `web_fetch`, `media_understanding`, and LLM-based `compactor`.
- **Loop Guards**: Implements `loop_guard`, `tool_policy`, and `session_repair` to prevent autonomous runaway.

### 3. `sk-kernel` (The Supervisor)
The OS monitor. Manages agent lifecycles, resource quotas, heartbeat monitoring, and background scheduling.

### 4. `sk-soul` (The Identity)
Manages agent personas and behavioral directives. It parses `SOUL.md` files to inject consistent personality and boundaries into the engine.

### 5. `sk-memory` (The Substrate)
A unified memory system using SQLite and vector embeddings. Features BM25 search and hybrid recall for perfect context retrieval.

### 6. `sk-mcp` (The Nervous System)
A native Rust implementation of the Model Context Protocol (MCP). Allows the kernel to consume and expose tools via JSON-RPC 2.0.

### 7. `sk-tools` (The Hands)
Provides the interface between the agent and the host system.
- **Shell Hand** (`shell.rs`): Executes commands with per-tool timeout, working directory scoping, and separate stdout/stderr capture. Enforces an `ExecPolicy` allowlist in Sandbox mode.
- **File Hand** (`file_ops.rs`): Full filesystem operations — read (1MB limit), write, append, delete, move, copy, list (with rich metadata). Path validation prevents traversal outside the workspace root in Sandbox mode.
- **Web Hand** (`web_fetch.rs`): Fetches web pages using `reqwest` with automatic HTML-to-text stripping and response truncation.
- **Code Hand** (`code_exec.rs`): Sandboxed script runner for Python, Node.js, and Bash, with configurable timeouts and security policy gating.
- **Skills System**: A dynamic registry of **52 expert skills** (Obsidian, GitHub, Weather) parsed from `SKILL.md` files to provide on-demand instructions without prompt bloat.

### 8. `sk-cli` (The Shell)
The user interface. Provides the entry point for starting the kernel daemon and interacting via a terminal REPL.

## 🛡️ Security Architecture

1.  **Capability Gates**: Host functions are restricted by permissions defined in the `sk-types::Capability` enum.
2.  **Sandbox Isolation**: untrusted code runs in a **Wasmtime** sandbox with strictly metered fuel and memory.
3.  **Path Sanitization**: Filesystem access is strictly limited to the workspace root, preventing path traversal attacks.

## ⚡ Execution Flow

1.  **Input**: User sends a command or message via a `Channel` (CLI, Telegram, etc.).
2.  **Dispatch**: The `Bridge` routes the message to the active agent.
3.  **Inference**: `sk-engine` generates a response using the configured LLM driver.
4.  **Action**: If the LLM requests a tool, the `ToolRunner` checks capabilities and executes the action in a sandbox.
5.  **Output**: The result is formatted and returned to the user, with every step logged in the audit trail.
