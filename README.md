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

- **Real-time Web Search** — Direct HTTP search with structured result extraction (DuckDuckGo + fallback)
- **File & Shell Operations** — Create, read, write, delete files and execute commands autonomously
- **Browser Bridge** — Full Playwright-based browser automation for multi-step web navigation
- **Token Streaming** — Real-time response streaming in the terminal for instant feedback
- **Security Sandbox** — Allowlist-based command execution with human-in-the-loop approval prompts
- **Persistent Memory** — SQLite-backed memory substrate with session management
- **Hands System** — 11 bundled agent capability packages (Researcher, Otto, Clip, Lead, Email, Collector, Predictor, Twitter, Web-Search, MySQL-Reporter, Artificer)
- **Cron Scheduler** — Background autonomous task execution on a cron or interval schedule
- **Knowledge Graph** — SQLite-backed entity-relation graph; agents can store and query structured facts
- **Prompt Caching** — Saves tokens by caching the system prompt across turns on Anthropic, OpenAI, and Gemini
- **Local Model Support** — Auto-detects Ollama, local GGUF, and localhost endpoints; adjusts context window and skips JSON schemas for small models
- **Rolling Context Window** — Sends only the last N messages to the LLM (10 standard / 6 for small/local), keeping costs low on long tasks

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
| **OpenAI** | ✅ Verified | GPT-4o, o1, o3 |
| **Anthropic** | ✅ Verified | Claude 4 Opus/Sonnet, Claude 3.5 Sonnet — native tool use + prompt caching |
| **Google Gemini** | ✅ Verified | Gemini 2.5 Pro/Flash, 2.0 Flash — prompt caching via cachedContents API |
| **Ollama** | ✅ Verified | Any local model — auto-detected, compact context window |
| **Groq** | ✅ Verified | Ultra-fast inference |
| **DeepSeek** | ✅ Verified | DeepSeek-V2/V3 |
| **OpenRouter** | ✅ Verified | Multi-model routing |
| **xAI / Grok** | ✅ Verified | Grok models |
| **Mistral** | ✅ Verified | Mistral Large/Small |

---

## 🤝 Bundled Hands (Agent Packages)

Run any Hand by prefixing your task with its ID:

```bash
sovereign run "researcher what are the best Rust async runtimes"
sovereign run "otto build me a CLI tool in Rust that converts CSV to JSON"
sovereign run "clip summarize this article https://..."
```

| Hand ID | Name | What it does |
| :--- | :--- | :--- |
| `researcher` | Researcher | Web search + browser verification, saves markdown reports |
| `otto` | Otto | Autonomous Rust/Python software builder via Otto's Outpost |
| `clip` | Clip | Content clipper: fetch, summarize, and store web articles |
| `lead` | Lead Intel | Lead research, knowledge graph for contacts and companies |
| `email` | Email Hand | SMTP/IMAP email manager with draft-mode safety |
| `collector` | Collector | Data collection pipeline with scheduling and knowledge graph |
| `predictor` | Predictor | Trend analysis and forecasting with scheduled data pulls |
| `twitter` | Twitter Hand | Tweet drafting, scheduling, and engagement tracking |
| `web-search` | Web Search | Multi-phase research pipeline with structured reports |
| `mysql-reporter` | MySQL Reporter | Daily sales reports from MySQL → email via Himalaya |
| `peka` | Artificer | System-level tool: process monitor, file ops, shell tasks |

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
