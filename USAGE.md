# 📖 Sovereign Kernel — Complete User Guide

Welcome to the **Sovereign Kernel**. This guide provides a complete overview of what the project is, where it can be deployed, and how to set it up from scratch.

---

## 🧐 What is the Sovereign Kernel?

The **Sovereign Kernel** is an Agentic Operating System. Simply put, it is a high-performance, single-executable engine that brings an autonomous, self-aware AI to your device. 

Instead of relying on clunky, slow Python scripts or heavy Electron frameworks, the Sovereign Kernel is built entirely in **Rust**. It runs in the background of your OS (Windows, Mac, or Linux) utilizing less than 20MB of idle RAM, constantly thinking, remembering context, and polling information.

It features:
1. **Infinite Vector Memory**: A `rusqlite` database combined with BM25 algorithmic search, ensuring the AI never forgets a file, conversation, or task you give it.
2. **Universal "Soul" Identity**: You define exactly how the AI talks and acts by editing a simple `SOUL.md` markdown file.
3. **The MCP Nervous System**: It uses the Model Context Protocol (MCP) allowing it to plug into **any** app (Telegram, Google Maps, local databases) natively.

---

## 🎯 Where Can You Use It?

The Sovereign Kernel is designed to be a universal infrastructure layer. Here are the primary use cases:

* **Your Ultimate Personal Assistant (Local Desktop)**: Run the Kernel on your Windows gaming PC or Mac. It sits quietly in the background and helps you organize files, summarize emails, or control applications via MCP tools.
* **Privacy-First Corporate AI**: Deploy the daemon on a small internal server. Because it supports **local inference** (via mistral.rs/llama.cpp), companies can run LLMs completely offline, guaranteeing zero data leakage to cloud providers.
* **The "Brain" for Bots (Telegram/Discord)**: Run the daemon on a cloud VPS (like DigitalOcean or AWS). The built-in native `sk-mcp` Telegram connector allows you to expose the kernel to messaging apps, granting you an AI buddy you can text 24/7.
* **Autonomous File Organizer**: Set up the Kernel to constantly watch your `Downloads` or `Documents` folder and organize/categorize files as they drop in, using its built-in knowledge graph to understand what files map to what projects.

---

## 🛠️ Complete Setup Guide

### Step 1: Install Prerequisites

Because the Sovereign Kernel is a native Rust application, you need the Rust compiler toolchain installed.

1. **Install Rust**: Download and install `rustup` from [https://rustup.rs/](https://rustup.rs/). (On Windows, this will install the necessary MSVC build tools).
2. **(Optional) Install CUDA**: If you want to run offline, localized AI models on an NVIDIA GPU (like an RTX 40-series), ensure you have the [NVIDIA CUDA Toolkit](https://developer.nvidia.com/cuda-toolkit) installed.

### Step 2: Configure Your Agent's "Soul"

Before starting the engine, define the personality. Open the `.soul/SOUL.md` file located in the root directory.

```markdown
# SOUL.md
You are the Sovereign Agent. Your user is Sameer from Dubai. You are concise, highly competent in Rust, and prioritize local-first execution. 

## Boundaries
- Never delete files without asking for confirmation.
- Be extremely brief in your answers.
```

### Step 3: Set Your API Keys (Cloud vs Local Models)

The engine intelligently maps tasks to different capability tiers. You can use large cloud providers or completely local offline providers via the OpenAI-compatible driver.

**Option A: Cloud APIs (Anthropic/Gemini)**
If you want to use state-of-the-art cloud reasoning models, set your API keys:

On **Windows PowerShell**:
```powershell
$env:GEMINI_API_KEY="your-google-ai-studio-key"
$env:ANTHROPIC_API_KEY="your-anthropic-key"
```

On **Linux/Mac Bash**:
```bash
export GEMINI_API_KEY="your-google-ai-studio-key"
export ANTHROPIC_API_KEY="your-anthropic-key"
```

**Option B: Local Provider (Ollama / LM Studio)**
Sovereign Kernel supports any OpenAI-compatible API interface. If you are running **Ollama** or **LM Studio** locally, you can configure the Kernel to route heavy local AI tasks natively by pointing it to your local server (e.g., `http://localhost:11434/v1` for Ollama).

```bash
export OPENAI_API_BASE="http://localhost:11434/v1"
export OPENAI_API_KEY="ollama"
```

*(Note: The `sk-engine` dynamically selects between local light models for simple tasks and heavy models for complex reasoning based on what providers are active).*

### Step 4: Run the Kernel

The codebase is split into an 8-crate workspace, but the main entry point is `sk-cli`.

**To run the interactive Chat Terminal (Foreground):**
```bash
cargo run -p sk-cli -- chat
```
*This drops you into a chat interface where your conversations are automatically vectorized and saved forever into the SQLite memory substrate.*

**To run the Daemon (Background OS Mode):**
```bash
cargo run -p sk-cli -- start
```
*This starts the 24/7 background loop. It will immediately begin polling native MCP tools (like its Telegram listener, if configured), executing scheduled tasks, and indexing files, consuming practically zero resources while idle.*

---

## 👨‍💻 Advanced: Adding Your Own Tools

Because Sovereign Kernel uses the standard **Model Context Protocol (MCP)**, you don't need to rebuild the core to give it new abilities. 

To add a new tool (e.g., checking the weather, fetching crypto prices):
1. Navigate to `crates/sk-mcp/src/connectors/`.
2. Write a standard async Rust function that returns JSON.
3. Expose the tool schema inside the `sk-mcp` server registry. 
4. The Sovereign Kernel will instantly "discover" your new function at boot, understand its parameters, and decide dynamically when to invoke it during a conversation!

---

## 🛡️ Admin & Governance (Control Your AI)

Sovereign Kernel is an OS, which means you have absolute root access over what the agent can see and do. 

### How to Talk Setup
1. **Interactive Terminal**: Run `cargo run -p sk-cli -- chat`. This is the most direct way to speak to the kernel.
2. **Messengers (Telegram/Matrix)**: By enabling the native MCP connectors in `sk-mcp/src/connectors`, the background daemon will automatically poll your private Telegram bot. You can literally text your AI from your phone while you are away from your PC.

### How to Monitor the Agent
1. **The Daemon Log**: When running `cargo run -p sk-cli -- start`, the engine outputs a colored, structured `tracing` log. You can see *exactly* when it routes a task to the local GPU, what tools it is currently executing, and how many tokens it consumed.
2. **Memory Inspection**: Because the hybrid memory is just a standard SQLite database (`.sovereign/memory.db`), you can open it with any DB browser to see precisely what the agent has "remembered" and "forgotten."

### How to Control Access (RBAC)
You decide exactly which MCP tools the agent is allowed to execute.
1. **The `SOUL.md` Boundaries**: Hard boundaries are defined in the soul file (e.g., *"Under no circumstances are you allowed to execute the X tool if the user doesn't confirm it"*).
2. **The MCP Registry**: In `sk-mcp/src/registry.rs`, you explicitly register which tools the agent has access to. If you don't want the agent accessing your SQLite database, you simply don't register the SQL string tool for that specific session. If the tool isn't in the registry, the LLM physically cannot execute it.

---

## 📂 OS File Management (e.g., Deleting Screenshots)

Because Sovereign Kernel runs directly on your machine as a native binary, it doesn't need external cloud APIs to manage your computer. It uses the `sk-tools` crate to perform direct OS operations.

### How it Works
When you ask the agent: *"Delete all old screenshots from my desktop,"* the engine follows these exact steps:
1.  **Reasoning**: The LLM determines it needs to read the Desktop folder and isolate image files (e.g., `.png`).
2.  **Tool Execution**: It invokes the `list_directory` tool via its native `sk-tools` harness to find the screenshots.
3.  **Action**: It invokes the `delete_file` tool.
4.  **Verification**: The `sk-tools` module safely deletes the files using native Rust `std::fs` calls and reports success back to the context window.

### How to Configure File Access
By default, the Sovereign Kernel operates with the permissions of the user running it (e.g., your Windows admin account). 
Because OS manipulation is powerful, it is strongly recommended you set guardrails in your `SOUL.md` before deploying a Daemon that handles files autonomously:

```markdown
# SOUL.md - Autonomy Guardrails
1. You may read any file.
2. If you are asked to DELETE a file, you must first list out all the specific file paths to the user and wait for their explicit word "CONFIRM" before executing the deletion tool. 
```

*(If you wish to completely disable file deletion, you can simply unregister the `delete_file` tool inside `crates/sk-tools/src/file_ops.rs`, removing the capability entirely).*
