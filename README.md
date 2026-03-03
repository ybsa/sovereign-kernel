<p align="center">
  <img src="https://img.shields.io/badge/language-Rust-orange?style=flat-square" alt="Rust" />
  <img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT" />
  <img src="https://img.shields.io/badge/status-Industrial%20Core%20Complete-brightgreen?style=flat-square" alt="Status" />
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

1.  **The Engine (OpenFang DNA)**: Provides the robust, multi-threaded core (`sk-engine`, `sk-kernel`, `sk-types`, `sk-tools`, `sk-cli`). Includes **32 ported runtime modules** for sandboxing, web scraping, media processing, and memory compaction.
2.  **The Soul (OpenClaw DNA)**: Supplies the sophisticated "human" elements (`sk-soul`, `sk-memory`, `sk-mcp`). It deeply integrates the 31k line identity logic and a dynamic **52-skill ecosystem** (Obsidian, GitHub, etc.) ported from OpenClaw.

## 🆚 The Reality Check: Sovereign Kernel vs. Ancestors

Let's be completely rigorous and transparent. While Sovereign Kernel is the *future* of this architecture, its ancestors (**OpenFang** and **OpenClaw**) represent massive engineering efforts with vastly larger feature sets. Sovereign Kernel is an early-stage, clean-slate rewrite that is currently missing most of the advanced infrastructure.

Here is the objective state of the project based on actual codebase metrics:

| Category | OpenFang Has | Sovereign Kernel Has | Gap |
| :--- | :--- | :--- | :--- |
| **Runtime modules** | 32 | 32 | **Ported** |
| **Skill Integrations** | 52 | 52 | **Ported** |
| **Kernel modules** | 22 | 15 | 7 missing |
| **Agent loop LOC** | 2,854 | 1,217 | 2x smaller (Refined) |
| **Metering LOC** | 693 | 494 | **Ported** |
| **Supervisor LOC** | 228 | 231 | **Ported** |
| **Scheduler LOC** | 169 | 251 | **Ported** |
| **LLM Drivers** | 5 (OpenAI, Anthropic, Gemini, Copilot, Fallback) | 5 (OpenAI, Anthropic, Gemini, Copilot, Fallback) | **Ported** |
| **Security** | 16 layers (WASM, Docker, shell_bleed, taint, audit) | Wasmtime + Shell Bleed + Taint + Merkle Audit Trail | **Ported** |
| **Browser** | Full Playwright CDP bridge | Ported BrowserManager + CDP Bridge | **Ported** |
| **API Bridge** | Advanced Headless API | Axum-based HTTP/JSON API + Webhooks | **Ported** |
| **Channels** | 40 adapters | 2 (Telegram, Discord) | 38 missing |

*(Sovereign Kernel is currently actively porting legacy features. It has established the core memory representations, execution guardrails, cost metering, and isolation layers, but still lacks full UI and channel ecosystems from OpenFang.)*

## ⚖️ The Honest Comparison (Best vs. Worst)

We believe in radical transparency. Here is how Sovereign Kernel stacks up against its predecessors and the broader market:

| Platform | The Best Things 🏆 | The Worst Things ⚠️ |
| :--- | :--- | :--- |
| **Sovereign Kernel** | **Clean Architecture**: 8-crate modular design.<br>**Blazingly Fast**: 100% Rust engine with minimal bloat. | **Incomplete UI ecosystem**: Missing nearly all advanced web and desktop UI features of its ancestors.<br>**No UI**: Strictly command-line. |
| **OpenClaw (Node.js)** | **Rich Interfaces**: Incredible macOS app, Canvas, and WebChat UI.<br>**Massive Reach**: Plug-and-play with WhatsApp, Telegram, iMessage, etc. | **Resource Heavy**: Node.js ecosystem consumes significant RAM.<br>**Lower Security**: Lacks process-level WASM sandboxing. |
| **OpenFang (Rust)** | **Production Ready**: 16-layer security model and highly robust agent loop.<br>**Ecosystem**: 40+ adapters and deep sandbox implementations. | **Legacy Architecture**: Massive, tightly-coupled codebase that is harder to extend safely. |

*(See [ARCHITECTURE.md](ARCHITECTURE.md) for a deep dive into the 8-crate system).*

## 🗺️ Project Status & Roadmap

-   **Phase 1-6 (Complete)**: Kernel, Engine, Security, Media, and Advanced Features.
-   **Phase 7-11 (Complete)**: Industrial Core & Skill Integration.
    - [x] Port 52 Expert Skills from OpenClaw.
    - [x] Implement BM25 Search in `sk-memory`.
    - [x] Cryptographic Merkle Audit Trail.
    - [x] Headless API Bridge & Webhook Triggers.

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
