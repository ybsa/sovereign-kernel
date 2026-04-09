# 👑 Sovereign Kernel v1.0

![Rust](https://img.shields.io/badge/language-Rust-orange?style=flat-square)
![MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)
![Status](https://img.shields.io/badge/status-v1.0.0--stable-brightgreen?style=flat-square)

> **A local-first, memory-safe AI Operating System. Single Rust binary. Runs everywhere.**

Sovereign Kernel is a production-grade Agentic Operating System and **Agent Development Kit (ADK)** built entirely in Rust. It provides a modular, strategy-based framework for building autonomous agents with deep repository awareness via **The Librarian** (background semantic indexing).

```text
┌─────────────────────────────────────────────────────────────┐
│                  Sovereign Kernel (Rust)                    │
│               The Agentic Operating System                  │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ 50+ LLM  │  │ Terminal │  │  100+    │  │ Security │   │
│  │ Providers│  │   CLI    │  │  Skills  │  │ Sandbox  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Memory  │  │  Agent   │  │ Modular  │  │   MCP    │   │
│  │ Substrate│  │ Registry │  │  Tools   │  │ Protocol │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 🏗️ Architecture

A 9-crate Rust workspace with clean separation of concerns:

| Crate | Purpose |
| :--- | :--- |
| **sk-types** | Shared types, errors, capabilities, configuration schema |
| **sk-soul** | Agent identity parser (SOUL.md, IDENTITY.md) |
| **sk-memory** | SQLite-backed memory substrate (structured KV, semantic vectors, BM25, sessions, audit) |
| **sk-engine** | LLM orchestration, agent loops, driver catalog, sandboxing |
| **sk-mcp** | Model Context Protocol server/client integration |
| **sk-kernel** | Core daemon: security, approval gates, event bus, cron, supervisor, tool registry |
| **sk-tools** | Tool implementations (shell, file ops, browser, code exec, skills) |
| **sk-hands** | Pre-built agent capability packages |
| **sk-cli** | CLI surface and interactive terminal |

---

## 🚀 Verified LLM Providers

| Provider | Status | Notes |
| :--- | :--- | :--- |
| **OpenAI** | ✅ Verified | GPT-4o, o1-preview |
| **Anthropic** | ✅ Verified | Claude 3.5/4 Sonnet (Native Tool Use) |
| **Google Gemini** | ✅ Verified | Gemini 2.5 Flash/Pro |
| **NVIDIA NIM** | ✅ Verified | Mistral, Llama via NIM API |
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
- **Audit Trail** — Merkle chain of every agent action for forensic analysis
- **Budget Enforcement** — Real-time cost tracking with hard-kill on overspend

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

## ⚙️ Quick Start

```bash
git clone https://github.com/OpenEris/sovereign-kernel.git
cd sovereign-kernel
cp config.toml.example config.toml   # Edit with your provider
cp .env.example .env                  # Add your API keys
cargo build --release --workspace
cargo run --release -- run "Hello, what can you do?"
```

## ⚙️ Requirements
- **Rust 1.75+** (Required)
- **SQLite3** (Bundled via rusqlite)
- Windows 10+, macOS, or Linux

## 📜 License
Licensed under the [MIT License](LICENSE).
