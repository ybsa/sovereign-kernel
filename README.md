<p align="center">
  <img src="https://img.shields.io/badge/language-Rust-orange?style=flat-square" alt="Rust" />
  <img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT" />
  <img src="https://img.shields.io/badge/status-Phase%206%20Complete-green?style=flat-square" alt="Status" />
  <img src="https://img.shields.io/badge/security-16--Layer%20Audit-brightgreen?style=flat-square" alt="Security" />
</p>

# Sovereign Kernel: The Agentic Operating System

> [!WARNING]
> **Active Development Notice**: Sovereign Kernel is currently highly experimental software. We are actively porting features from OpenFang and OpenClaw into this new Rust architecture. Expect breaking changes.

**Sovereign Kernel** is a virtual operating system for agents that runs on top of real operating systems. 

By merging the indestructible, memory-safe Rust daemon core of **OpenFang** with the expansive integrations and memory systems of **OpenClaw**, we are building the universal infrastructure layer for the next generation of AI. It is not just a framework; it is the mediation layer between Autonomous Entities and the silicon they run on.

---

## 🎯 Capabilities & Use Cases

Sovereign Kernel is designed for complex, long-running agentic workflows.

### Core Capabilities
*   **Persistent Execution**: Agents run as background OS daemons that don't sleep unless told to.
*   **Tool Orchestration**: Agents seamlessly juggle the filesystem, local terminals, and web browsers.
*   **Infinite Memory**: Recalls past conversations and tool outputs via hybrid SQLite + BM25 Vector search.

### Primary Use Cases
*   **Local Automations**: Have an agent monitor a specific folder, read new PDFs, summarize them, and move them—all strictly on your local machine with zero data exfiltration.
*   **Safe Code Execution**: Run an agent that writes, compiles, and tests code inside a constrained WASM Sandbox without risking your real OS.
*   **Custom Research Assistants**: Give an agent a goal (e.g., "Find the latest papers on Q-Star"), and let it crawl the web, parse the data, and build you a comprehensive report independently over the next 3 hours.

---

## 🚀 Key Features

*   **24/7 Rust Daemon**: Runs silently in the background of your OS (Windows, Mac, or Linux) with minimal RAM overhead.
*   **Hardware Offloading**: Designed for local-first execution. Includes native drivers with RTX 40-series GPU offloading (CUDA) via `mistral.rs` and `llama-cpp-rs`.
*   **Infinite Memory**: Combines SQLite, BM25 full-text search, and native vector embeddings. Your agent remembers everything across sessions indefinitely.
*   **Native MCP Support**: Built-in Model Context Protocol (MCP) Nervous System. Natively connect to any app (Telegram, SQL, filesystems) with Rust-level performance.

## 🧬 Architecture: The DNA Merge

The Sovereign Kernel is a meticulously architected 8-crate workspace that brings together the best of two worlds:

1.  **The Engine (OpenFang DNA)**: Provides the robust, multi-threaded core (`sk-engine`, `sk-kernel`, `sk-types`, `sk-tools`, `sk-cli`). Handles dynamic LLM routing, task scheduling, and the unyielding agent execution loop.
2.  **The Soul (OpenClaw DNA)**: Supplies the sophisticated "human" elements (`sk-soul`, `sk-memory`, `sk-mcp`). It deeply integrates the 31k line identity logic, temporal memory decay, maximal marginal relevance (MMR) retrieval, and the universal MCP Nervous System.

## 🆚 The Reality Check: Sovereign Kernel vs. Ancestors

Let's be completely transparent. While Sovereign Kernel is the *future* of this architecture, its ancestors (**OpenFang** and **OpenClaw**) currently have much larger, battle-tested surface areas. Sovereign Kernel is a lightweight, clean-slate rewrite that is still missing many features.

Here is the exact state of the project right now:

| Category | OpenFang (Rust) | OpenClaw (Node.js) | Sovereign Kernel | The Gap |
| :--- | :--- | :--- | :--- | :--- |
| **Runtime Modules** | 52 | N/A (NPM ecosystem) | 13 | 39 missing |
| **Kernel Modules** | 22 | N/A (Agent cluster) | 7 | 15 missing |
| **Agent Loop LOC** | 2,854 | ~4,000 (TypeScript) | 154 | 18x-25x smaller |
| **Metering LOC** | 693 | Extensive Billing API | 3 (stub) | Not implemented yet |
| **Supervisor LOC** | 228 | N/A (PM2/Docker relied) | 3 (stub) | Not implemented yet |
| **Scheduler LOC** | 169 | Full Cron Service API | 3 (stub) | Not implemented yet |
| **LLM Drivers** | 5 (OpenAI, Anthropic, etc) | 10+ (Vast support) | 2 (Gemini, Anthropic) | 3+ missing |
| **Security Layers** | 16 (WASM, Docker, Taint...) | 3 (Basic Docker Auth) | Taint + 8 Capabilities | WASM/Docker not enforced |
| **Browser Hand** | Full Playwright CDP | Deep Puppeteer Bridge | Defined in `HAND.toml` | Not functional yet |
| **Channels** | 40 adapters | 15+ (WhatsApp, iMessage) | 2 (Telegram, Discord)| 38 missing |

*(Sovereign Kernel is currently focused on establishing the core safe `sk-engine` execution loop before porting the massive feature sets of its ancestors.)*
## ⚖️ The Honest Comparison (Best vs. Worst)

We believe in transparency. Here is how Sovereign Kernel stacks up against its predecessors and the broader market:

| Platform | The Best Things 🏆 | The Worst Things ⚠️ |
| :--- | :--- | :--- |
| **Sovereign Kernel** | **Unbreakable Security**: Native WASM sandboxing and cryptographically signed audit trails.<br>**Raw Power**: Blazingly fast Rust engine with zero telemetry.<br>**Infinite Memory**: Perfect temporal decay and vector search built-in. | **Steep Learning Curve**: Requires understanding agent lifecycles.<br>**UI Pending**: Currently CLI-only (Web UI is coming in Phase 12). |
| **OpenClaw** | **Rich Interfaces**: Incredible macOS app, Canvas, and WebChat UI.<br>**Massive Reach**: Plug-and-play with WhatsApp, Telegram, iMessage, etc. | **Resource Heavy**: Node.js ecosystem consumes significant RAM.<br>**Lower Security**: Lacks process-level WASM sandboxing for scripts. |
| **OpenFang** | **Autonomous Hands**: Great built-in autonomous schedules and 40+ adapters. | **Legacy Architecture**: Massive, tightly-coupled codebase that is harder to extend natively. |
| **LangChain/AutoGen** | **Massive Community**: Huge ecosystem of tutorials and plugins.<br>**Easy to Start**: Just write a 10-line Python script. | **Fragile**: Scripts easily crash or loop infinitely in production.<br>**Dangerous**: Runs unsandboxed Python code with your API keys. |

*(See [ARCHITECTURE.md](ARCHITECTURE.md) for a deep dive into the 8-crate system).*

## 🗺️ Project Status & Roadmap

Sovereign Kernel has completed **Phase 6: Verification**. The core infrastructure, sandboxing, and LLM drivers are fully implemented and tested.

-   **Phase 1-6 (Complete)**: Kernel, Engine, Security, Media, and Advanced Features.
-   **Current Phase**: Phase 7: Execution Modes (Sandbox vs. Unrestricted).

For a deep dive into the 26-week roadmap, see [docs/PROJECT_PLAN.md](docs/PROJECT_PLAN.md).

## 📚 Documentation

-   [ARCHITECTURE.md](ARCHITECTURE.md) — Technical overview of the 8-crate system.
-   [SECURITY.md](SECURITY.md) — Security principles, sandboxing, and privacy.
-   [USAGE.md](USAGE.md) — Detailed guide on running and configuring agents.
-   [VISION.md](VISION.md) — The long-term goal of the AI Operating System.

## 📦 Installation (Pre-compiled Binaries)

You do **not** need to install Rust to run the Sovereign Kernel! 

We provide pre-built, ready-to-run executables for Windows, macOS, and Linux.
1. Go to the [GitHub Releases](https://github.com/ybsa/sovereign-kernel/releases) page.
2. Download the `.zip` or `.tar.gz` for your operating system.
3. Extract the archive and run the `sovereign` binary directly from your terminal!

## ⚡ Quick Start (Building from Source)

### 1. Requirements

*   Rust (`1.80.0+`)
*   *(Optional)* CUDA Toolkit for local GPU acceleration

### 2. Configuration

Define your agent's identity by modifying `./soul/SOUL.md`.

### 3. Run the OS

Set your preferred API keys (if using cloud models for complex reasoning):

**Windows (PowerShell):**
```powershell
$env:GEMINI_API_KEY="your-key"
cargo run -p sk-cli -- chat   # Interactive Terminal
cargo run -p sk-cli -- start  # Background Daemon
```

**Linux / macOS:**
```bash
export GEMINI_API_KEY="your-key"
cargo run -p sk-cli -- chat   # Interactive Terminal
cargo run -p sk-cli -- start  # Background Daemon
```

## 🤝 Contributing

We are building a global standard. See [CONTRIBUTING.md](CONTRIBUTING.md) to learn how to inject your own MCP tools and enhance the Sovereign Kernel.

## ⚖️ License

MIT License. Open Source, for the world.
