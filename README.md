# 👑 Sovereign Kernel v1.0

![Rust](https://img.shields.io/badge/language-Rust-orange?style=flat-square)
![MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)
![Status](https://img.shields.io/badge/status-v1.0.0--stable-brightgreen?style=flat-square)

> **A local-first, memory-safe AI Operating System. Single Rust binary. Runs everywhere.**

Sovereign Kernel is a production-grade **Agentic Operating System** built entirely in Rust. It turns any LLM into an autonomous agent that can search the web, manipulate files, execute shell commands, and manage long-running background tasks — all from a single binary with built-in security sandboxing.

```text
┌─────────────────────────────────────────────────────────────┐
│                  Sovereign Kernel (Rust)                    │
│               The Agentic Operating System                  │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ 10+ LLM  │  │ Terminal │  │ Modular  │  │ Security │   │
│  │ Providers│  │   CLI    │  │  Tools   │  │ Sandbox  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Memory  │  │  Agent   │  │  Hands   │  │  Cron    │   │
│  │ Substrate│  │ Registry │  │ Packages │  │Scheduler │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## ✨ Key Features

- **Real-time Web Search** — Direct HTTP search with structured result extraction (DuckDuckGo + Google fallback)
- **File & Shell Operations** — Create, read, write, delete files and execute commands autonomously
- **Browser Bridge** — Full Playwright-based browser automation for multi-step web navigation
- **Token Streaming** — Real-time response streaming in the terminal for instant feedback
- **Security Sandbox** — Allowlist-based command execution with human-in-the-loop approval
- **Persistent Memory** — SQLite-backed memory substrate with session management
- **Multi-Agent Architecture** — "Hands" system for pre-built agent capability packages
- **Cron Scheduler** — Background autonomous task execution on schedule

---

## 🏗️ Architecture

A 9-crate Rust workspace with clean separation of concerns:

| Crate | Purpose |
| :--- | :--- |
| **sk-types** | Shared types, errors, capabilities, configuration schema |
| **sk-soul** | Agent identity parser (SOUL.md, IDENTITY.md) |
| **sk-memory** | SQLite-backed memory substrate (structured KV, sessions, audit) |
| **sk-engine** | LLM orchestration, agent loops, driver catalog, sandboxing |
| **sk-mcp** | Model Context Protocol server/client integration |
| **sk-kernel** | Core daemon: security, approval gates, event bus, cron, supervisor, tool registry |
| **sk-tools** | Tool implementations (shell, file ops, browser, web search, code exec) |
| **sk-hands** | Pre-built agent capability packages (Researcher, Clip, Email, etc.) |
| **sk-cli** | CLI surface and interactive terminal |

---

## 🚀 Verified LLM Providers

| Provider | Status | Notes |
| :--- | :--- | :--- |
| **NVIDIA NIM** | ✅ Verified | Llama 3.3 70B (Default), Mistral |
| **OpenAI** | ✅ Verified | GPT-4o, o1-preview |
| **Anthropic** | ✅ Verified | Claude 3.5/4 Sonnet (Native Tool Use) |
| **Google Gemini** | ✅ Verified | Gemini 2.5 Flash/Pro |
| **Ollama** | ✅ Verified | Any local model (OpenAI-compatible) |
| **Groq** | ✅ Verified | Ultra-fast inference |
| **DeepSeek** | ✅ Verified | DeepSeek-V2/V3 |
| **OpenRouter** | ✅ Verified | Multi-model routing |
| **xAI / Grok** | ✅ Verified | Grok models |
| **Mistral** | ✅ Verified | Mistral Large/Small |

---

## 🛡️ Security Model

Sovereign Kernel provides multi-layered protection for LLM-driven agent operations:

- **Unified Approval Manager** — Risk-based gating (Low → Medium → High → Critical) with human-in-the-loop for dangerous operations
- **Modular Tool Registry** — All tools dispatched through a type-safe `ToolHandler` trait with per-tool risk classification
- **Filesystem Sandbox** — Agents operate in isolated workspace directories
- **Allowlist Execution** — Shell commands restricted to approved binaries only
- **Audit Trail** — Merkle chain of every agent action for forensic analysis
- **Budget Enforcement** — Real-time cost tracking with hard-kill on overspend

---

## ⚙️ Quick Start

```bash
# Clone and setup
git clone https://github.com/OpenEris/sovereign-kernel.git
cd sovereign-kernel
cp config.toml.example config.toml   # Edit with your provider
cp .env.example .env                  # Add your API keys

# Build
cargo build --release --workspace

# Interactive Chat (with real-time streaming)
cargo run --release -- chat

# Autonomous Task Execution
cargo run --release -- run "Search the web for the latest AI news and summarize it"
```

### Environment Variables

At minimum, you need one LLM provider key. Example with NVIDIA NIM (free tier available):

```bash
NVIDIA_API_KEY=nvapi-your-key-here
```

---

## 📚 Documentation

| Document | Description |
| :--- | :--- |
| [🏎️ Getting Started](GETTING_STARTED.md) | Installation, config, and first run |
| [🧭 User Guide](USER_GUIDE.md) | API, tools, and advanced usage |
| [🏛️ Architecture](docs/ARCHITECTURE.md) | Deep dive into all 9 crates |
| [🗺️ Project Map](docs/PROJECT_MAP.md) | Lore terminology and directory layout |
| [🛡️ Security](SECURITY.md) | Vulnerability disclosure and security model |
| [🤝 Contributing](CONTRIBUTING.md) | How to contribute |

---

## ⚙️ Requirements

- **Rust 1.75+** (Required)
- **SQLite3** (Bundled via rusqlite)
- **Python 3.8+** (Optional, for Browser Bridge)
- Windows 10+, macOS, or Linux

## 📜 License

Licensed under the [MIT License](LICENSE).
