# Sovereign Kernel: The Universal Agentic OS

**Sovereign Kernel** is a high-performance, single-binary Agentic Operating System. It merging the stripped-down, indestructible Rust daemon core of **OpenFang** with the advanced identity and memory systems of **OpenClaw**. The result is a universal infrastructure layer for the next generation of AI.

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

## 🆚 Is it a Framework? (vs. The Competitors)

Yes, Sovereign Kernel is a **Framework and an OS**. While tools like LangChain or Microsoft AutoGen are libraries you import into Python scripts, Sovereign Kernel is a compiled binary engine designed to run indefinitely. 

| Feature | Sovereign Kernel | LangChain / AutoGen |
| :--- | :--- | :--- |
| **Language** | 100% Rust (Memory-safe, blazingly fast) | Python / TypeScript |
| **Execution** | Runs as a background OS Daemon | Runs inside temporary scripts |
| **Memory** | Built-in Infinite SQLite + BM25 Vector Hybrid | Requires connecting external vector databases (Pinecone, Chroma) |
| **Extensibility** | **Native MCP** (Model Context Protocol). Instantly plug-and-play with any standardized MCP tool globally. | Requires writing custom Python wrapper code for every API. |
| **Resource Usage** | ~20MB RAM at idle | 500MB+ depending on Python environment |

*(See [ARCHITECTURE.md](ARCHITECTURE.md) for a deep dive into the 8-crate system).*
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
