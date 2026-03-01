# 📖 Sovereign Kernel Usage Guide

Welcome to the Sovereign Kernel. This guide covers how to set up, configure, and run your autonomous agents.

## 🚀 Quick Start

### 1. Build the Kernel
Ensure you have the Rust toolchain installed.
```bash
cargo build --release
```

### 2. Initialize Setup
Run the CLI to configure your environment and LLM providers.
```bash
./target/release/sk-cli onboard
```

### 3. Boot the System
Start the agent loop and enter the interactive REPL.
```bash
./target/release/sk-cli boot
```

## 🛠️ Configuration

The system is configured via `.env` and `config.toml` in your workspace directory (default: `~/.sovereign/`).

### LLM Providers
Supported drivers:
- **OpenAI**: GPT-4o, GPT-3.5-Turbo
- **Gemini**: Pro 1.5, Flash
- **Copilot**: Native GitHub Copilot integration
- **Fallback**: Automatic driver failover if your primary provider is down.

## 🛡️ Security Modes

Sovereign Kernel supports two primary execution modes:

### 1. Sandbox Mode (Recommended)
- **Host Isolation**: Tools run in WASM or Docker.
- **Approvals**: All file/shell actions require Y/N confirmation.
- **Resource Limits**: Metered CPU/Memory usage.

### 2. Unrestricted Mode
- **Native Execution**: Tools run directly on the host.
- **Autonomous**: No approval gates.
- **Power**: Use for trusted local automation.

## 🧰 Built-in Tools

Your agents come equipped with:
- **ShellExec**: Execute terminal commands safely.
- **FileSystem**: Read/Write files within the workspace.
- **WebBrowser**: Search and scrape the web.
- **Memory**: Persistent vector-based long-term memory.
- **MCP**: Support for the Model Context Protocol.

## 📊 Monitoring & Audits

- **Logs**: Located in `logs/kernel.log`.
- **Audit Trails**: See `audit/merkle_chain.db` for serialized, signed history.
- **REPL Commands**:
  - `/status`: Show current agent health.
  - `/history`: Review recent actions.
  - `/approve`: Manage pending action requests.
