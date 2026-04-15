# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Sovereign Kernel is a Rust-based agentic operating system — a single binary (`sovereign`) that manages AI agents as first-class citizens. It exposes a CLI, runs agents in background daemon mode, and treats everything (memory, tools, identity, scheduling, security) as kernel subsystems.

## Build & Run Commands

```bash
# Build (debug)
cargo build

# Build (release — produces single binary with LTO)
cargo build --release

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p sk-engine
cargo test -p sk-hands

# Run a single test by name
cargo test -p sk-hands hand_definition_roundtrip

# Check for compile errors without building
cargo check

# Lint
cargo clippy -- -D warnings

# Run the CLI
cargo run --bin sovereign -- chat
cargo run --bin sovereign -- run "summarize my emails"
cargo run --bin sovereign -- setup
cargo run --bin sovereign -- doctor
```

## Configuration

`config.toml` in the project root is the default config location. The kernel also searches `~/.config/sovereign-kernel/config.toml`. See `examples/config/` for reference configs (`config.sandbox.toml`, `config.unrestricted.toml`).

Key config fields:
- `execution_mode`: `"sandbox"` (default) or `"unrestricted"`
- `[[llm]]`: LLM provider entries (provider, model, api_key_env, base_url)
- `[exec_policy]`: `mode = "allowlist"` or `"full"`
- `[memory]`: `decay_rate`

API keys are loaded from environment variables or a `.env` file (via `dotenvy`).

## Crate Architecture

The workspace has 9 crates with a strict dependency hierarchy (lower crates know nothing of higher ones):

```
sk-types      — shared types: AgentManifest, Message, ToolDefinition, KernelConfig, Session, errors
sk-soul       — SOUL.md parsing: agent identity/persona injection into system prompts
sk-memory     — MemorySubstrate: SQLite-backed structured KV, semantic vectors, knowledge graph, BM25, hybrid ranking, temporal decay
sk-engine     — agent execution loop, LLM drivers, tool runner, MCP client runtime, sandbox
sk-mcp        — MCP protocol (JSON-RPC 2.0), stdio/SSE transports, McpRegistry
sk-hands      — Hand definitions (HAND.toml), HandRegistry, bundled autonomous capability packages
sk-tools      — built-in tool implementations (shell, file ops, web search/fetch, browser, memory, skills)
sk-kernel     — SovereignKernel struct: composes all subsystems, approval, metering, cron, event bus, supervisor
sk-cli        — `sovereign` binary entry point, all CLI subcommands
```

**Data flow for a chat turn:**
1. `sk-cli` parses args → calls `sk-kernel::SovereignKernel::init(config)`
2. Kernel boots: loads Soul (`soul/SOUL.md`), opens SQLite (`sk-memory`), connects MCP servers (`sk-mcp`), registers tools (`sk-tools`)
3. User message → `sk-engine::agent_loop` → LLM driver → tool calls dispatched → results fed back → loop until stop
4. Memory recalled before LLM call; new memories saved after

## Key Concepts

**Hands** — pre-packaged autonomous agents defined in `HAND.toml` (in `crates/sk-hands/bundled/*/`). A Hand is activated (not chatted with) and runs indefinitely. Each has: `id`, `category`, `requires[]` (binaries/env vars), `settings[]` (user-configurable), `[agent]` config, `[dashboard]` metrics.

**Soul** — `soul/SOUL.md` or `SOUL.md` defines the kernel's persona. Auto-discovered at init. Format is YAML frontmatter + markdown body; injected into every agent's system prompt.

**Skills** — markdown-based capability descriptions in `crates/sk-tools/skills/*/SKILL.md`. Agents can `get_skill` and `list_skills` at runtime.

**MCP** — external tool servers connect via `[[mcp_servers]]` in config. Transport is stdio (subprocess) or SSE (HTTP). MCP tools are merged with built-in tools before each agent loop.

**Execution modes:**
- `sandbox` — `exec_policy.mode = "allowlist"`: shell commands require allowlist
- `unrestricted` — full host access, no approval gating

**Metering / Treasury** — every LLM call is metered; budget limits (`max_tokens_per_task`, `total_token_budget_usd_cents`) are enforced in the engine. `sovereign treasury` commands show spend.

## Rust Toolchain

Pinned to `stable` in `rust-toolchain.toml`. Minimum Rust version: 1.75.

`sk-kernel` exports three lock macros: `rlock!`, `wlock!`, `lock!` — use these instead of unwrapping `.read()`/`.write()`/`.lock()` directly.
