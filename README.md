# Sovereign Kernel

![Rust](https://img.shields.io/badge/language-Rust-orange?style=flat-square)
![MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)
![Status](https://img.shields.io/badge/status-v1.0.0--stable-brightgreen?style=flat-square)

> **A local-first, terminal-native agentic operating system. Single Rust binary. Runs everywhere.**

Sovereign Kernel is a production-grade AI agent framework built entirely in Rust. It unifies [OpenClaw](https://github.com/openclaw/openclaw) (AI assistant), [NemoClaw](https://github.com/NVIDIA/NemoClaw) (sandbox), and a custom OS-level daemon into a single, memory-safe binary.

```text
┌─────────────────────────────────────────────────────────────┐
│                  Sovereign Kernel (Rust)                    │
│               The Agentic Operating System                  │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │ 50+ LLM  │  │ 30+ Chat │  │  106+    │  │ Security │   │
│  │ Providers │  │ Channels │  │  Skills  │  │ Sandbox  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
│                                                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │  Memory  │  │  Agents  │  │  Tools   │  │ Dashboard│   │
│  │ Substrate│  │ Village  │  │ (30+)    │  │  Web UI  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└─────────────────────────────────────────────────────────────┘

```

---

## Table of Contents

- [Features](#features)
- [Requirements](#requirements)
- [Installation](#installation)
- [Configuration](#configuration)
- [Quick Start](#quick-start)
- [CLI Reference](#cli-reference)
- [Memory Management](#memory-management)
- [Security Model](#security-model)
- [Architecture](#architecture)
- [Docker Deployment](#docker-deployment)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

---

## Features

### LLM Integration (The Oracle)

- **50+ LLM providers** — Anthropic Claude, OpenAI GPT-4o, Google Gemini, Groq, Ollama, NVIDIA NIM, AWS Bedrock, Together AI, HuggingFace, Mistral, DeepSeek, xAI, Perplexity, OpenRouter, and more
- **Automatic failover** — fallback chains with cooldown and health probes
- **Model discovery** — auto-detect available models per provider
- **Configurable budgets** — USD cap, token limits, and iteration limits per agent

### Multi-Agent Ecosystem (The Village)

- **Dynamic spawning** — create sandboxed worker agents for parallel tasks
- **Inter-agent messaging** — direct communication via the Agent Bus
- **Crash recovery** — auto-restart from SQLite checkpoints (The Resurrector)
- **Natural language creation** — describe a task, the kernel builds the right agent
- **Shared memory** — cross-agent knowledge pool

### Memory Substrate (The Archive)

- **Structured memory** — key-value store via SQLite
- **Semantic search** — cosine similarity on vector embeddings
- **Full-text search** — BM25 ranking via SQLite FTS5
- **Knowledge graph** — entity-relation triples
- **Export/Import** — full brain portability (JSON and Markdown)
- **106+ expert skills** — pre-bundled prompts for engineering, security, legal, DevOps, and more

### Channel Adapters (The Bridge)

- **30+ channels** — Telegram, Discord, WhatsApp, Slack, Signal, iMessage, Matrix, IRC, Twitch, Teams, Google Chat, Nostr, and more
- **In-channel commands** — `/status`, `/new`, `/compact`, `/think`, `/verbose`
- **Multi-agent routing** — route channels to isolated Village agents

### Security (The Warden)

- **Filesystem isolation** — Landlock LSM sandboxing
- **Syscall filtering** — seccomp-bpf enforcement
- **Network egress control** — policy-driven network interception
- **Approval gates** — dangerous actions require explicit approval
- **Tamper-evident audit** — Merkle chain of every agent action

### Tools & Automation

- **Shell execution** — full PTY support, process registry, background jobs
- **Browser automation** — CDP-based Chrome/Chromium control
- **File operations** — read, write, edit, glob — path-sandboxed
- **Voice** — STT/TTS via OpenAI Whisper and TTS-1
- **Code execution** — Docker/native sandbox with timeout

---

## Requirements

| Requirement | Minimum | Recommended |
| --- | --- | --- |
| **Rust** | `1.75+` | Latest stable |
| **OS** | Windows 10+, Linux (kernel 5.13+), macOS 12+ | Linux (for Landlock/seccomp) |
| **RAM** | 512 MB | 2 GB+ |
| **Disk** | 100 MB (binary) | 500 MB+ (with models) |
| **LLM API Key** | At least one provider | Anthropic or OpenAI |

### Optional Dependencies

| Dependency | Required For |
| --- | --- |
| Docker | Sandboxed code execution (`otto` hand) |
| Chrome/Chromium | Browser automation (The Forge) |
| Tailscale | Remote tunnel access (The Cartographer) |

---

## Installation

### From Source (Recommended)

```bash

# 1. Clone the repository

git clone https://github.com/your-org/sovereign-kernel.git
cd sovereign-kernel

# 2. Build the release binary

cargo build --release

# 3. (Optional) Install to PATH

cargo install --path crates/sk-cli

```

The compiled binary will be at `target/release/sovereign` (or `sovereign.exe` on Windows).

### Docker

```bash

# Build and run in one step

docker compose up -d

# Or build manually

docker build -t sovereign-kernel .
docker run -d --name sovereign \
  -e ANTHROPIC_API_KEY=your-key \
  -p 4200:4200 -p 8080:8080 \
  sovereign-kernel

```

### Verify Installation

```bash
sovereign --version
sovereign doctor          # Full diagnostic: checks API keys, system deps, config

```

---

## Configuration

### 1. Environment Variables

Copy the example file and fill in your API keys:

```bash
cp .env.example .env

```

**Required** (at least one LLM provider):

| Variable | Provider |
| --- | --- |
| `ANTHROPIC_API_KEY` | Anthropic Claude (recommended) |
| `OPENAI_API_KEY` | OpenAI GPT-4o / o1 |
| `GEMINI_API_KEY` | Google Gemini |
| `GROQ_API_KEY` | Groq (Llama 3, free tier) |

**Optional:**

| Variable | Provider |
| --- | --- |
| `DEEPSEEK_API_KEY` | DeepSeek |
| `OPENROUTER_API_KEY` | OpenRouter (multi-model gateway) |
| `MISTRAL_API_KEY` | Mistral AI |
| `TOGETHER_API_KEY` | Together AI |
| `PERPLEXITY_API_KEY` | Perplexity |
| `XAI_API_KEY` | xAI (Grok) |
| `TELEGRAM_BOT_TOKEN` | Telegram channel adapter |
| `DISCORD_BOT_TOKEN` | Discord channel adapter |

### 2. Kernel Configuration

The kernel reads `~/.sovereign-kernel/config.toml` (auto-created by `sovereign init`):

```toml
[kernel]
mode = "Sandbox"                    # "Sandbox" or "Unrestricted"

[model]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[budget]
max_iterations_per_task = 200
max_tokens_per_task = 128000
total_budget_usd = 5.00             # Global USD spending cap

[exec_policy]
allowed_commands = ["git", "cargo", "python", "node", "npm", "docker"]

```

---

## Quick Start

### Interactive Chat (REPL)

```bash

# Windows (PowerShell)

$env:GEMINI_API_KEY="your-key"
sovereign chat

# Linux / macOS

export GEMINI_API_KEY="your-key"
sovereign chat

```

### First-Run Setup Wizard

```bash
sovereign init            # Guided setup: picks provider, validates key, creates config

```

### Autonomous Task Execution

```bash
sovereign run "Analyze my project structure and generate a README"
sovereign run "Monitor CPU usage every 5 minutes" --schedule "*/5 * * * *"
sovereign run "Refactor this module" --mode unrestricted --budget-usd 1.00

```

---

## CLI Reference

### Core Commands

| Command | Description |
| --- | --- |
| `sovereign init` | First-run setup wizard |
| `sovereign chat` | Interactive terminal REPL |
| `sovereign run "<task>"` | Autonomous task execution |
| `sovereign start [--detach]` | Start as foreground/background daemon |
| `sovereign status` | Village overview (agents, jobs, daemon) |
| `sovereign kill [<agent-id>]` | Stop an agent or the daemon |
| `sovereign doctor` | Full system diagnostic |

### Memory Commands

| Command | Description |
| --- | --- |
| `sovereign memory stats` | Show agent count, memory entries |
| `sovereign memory export --format json` | Export all memory to JSON |
| `sovereign memory export --format markdown` | Export as human-readable Markdown |
| `sovereign memory import --input <file>` | Restore memory from exported file |

### Management Commands

| Command | Description |
| --- | --- |
| `sovereign hands list` | List all 30+ bundled autonomous hands |
| `sovereign hands activate <name>` | Start a capability hand |
| `sovereign dashboard [--port 8080]` | Open the embedded web dashboard |
| `sovereign audit logs` | View cryptographic audit trail |
| `sovereign audit verify` | Verify Merkle chain integrity |
| `sovereign treasury status` | View budget and spending |
| `sovereign mcp list` | List active MCP tool servers |
| `sovereign channels list` | List configured channel adapters |

### Budget Controls (CLI Flags)

```bash
sovereign chat --budget-usd 2.00 --max-tokens 50000 --max-iterations 50
sovereign run "task" --budget-usd 1.00 --max-iterations 30

```

### In-Channel Commands (The Herald)

Send these in Telegram / Discord / WhatsApp / Slack / Teams:

| Command | Action |
| --- | --- |
| `/status` | Session status (model, tokens, cost) |
| `/new` or `/reset` | Reset the session |
| `/compact` | Compact session context |
| `/think <level>` | Set thinking level (off\|low\|medium\|high) |
| `/verbose on\|off` | Toggle verbose output |
| `/usage off\|tokens\|full` | Per-response usage footer |
| `/elevated on\|off` | Toggle host access |

---

## Memory Management

The Archive provides a unified memory substrate backed by SQLite with WAL mode for concurrent access.

### Memory Stores

| Store | Engine | Use Case |
| --- | --- | --- |
| **Structured** | SQLite KV | Agent preferences, state |
| **Semantic** | Vector (cosine) | Concept recall, similarity search |
| **Full-Text** | FTS5 (BM25) | Keyword search, document retrieval |
| **Knowledge** | Entity-Relation | Facts, relationships, ontologies |

### Export & Import

```bash

# Export your agent's entire brain

sovereign memory export --format json --output backup.json

# Restore on a different machine

sovereign memory import --input backup.json

```

> **Note:** JSON export/import preserves all data types. Markdown export is human-readable but import only restores semantic memories.

---

## Security Model

### Execution Modes

| Mode | Behavior | Use Case |
| --- | --- | --- |
| **Sandbox** (default) | All file/shell actions require approval | Production, shared environments |
| **Unrestricted** | Full host access, no approval gates | Trusted local automation |

### Filesystem Isolation (Landlock LSM)

| Path | Access |
| --- | --- |
| `/sandbox`, `/tmp`, `/dev/null` | Read-write |
| `/usr`, `/lib`, `/proc`, `/app`, `/etc` | Read-only |
| Everything else | Blocked |

### Network Egress Control

All unlisted network connections are intercepted and require operator approval via the dashboard.

### Agent Budgets

Every agent runs under a configurable cost ceiling. Exceeding the budget stops the agent immediately:

```bash
sovereign run "task" --budget-usd 0.50    # Hard limit at $0.50

```

---

## Architecture

A 10-crate Rust workspace containing 29 named subsystems:

```text
sk-types       Shared types, capability gates, taint tracking
sk-soul        Agent identity (SOUL.md, AGENTS.md)
sk-memory      The Archive (SQLite + BM25 + vectors + knowledge graph)
sk-engine      The Oracle (50+ LLMs) + The Village (multi-agent) + The Treasury (budgets)
sk-mcp         The Diplomat (A2A protocol) + The Alchemist (plugin SDK)
sk-kernel      The Kernel (WS/HTTP API) + The Warden (sandbox) + The Resurrector
sk-tools       The Forge (browser) + The Voice (STT/TTS) + shell/file tools
sk-channels    The Bridge (30+ adapters) + The Herald (slash commands)
sk-hands       Hands (30+ capability packages) + The Bazaar (marketplace)
sk-cli         CLI surface + The Watchtower (web dashboard) + The Builder

```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for a deep dive into every subsystem.

---

## Docker Deployment

### Docker Compose (Recommended)

```yaml

# docker-compose.yml is included in the repo

docker compose up -d

```

This starts the daemon with persistent storage, exposing:

- **Port 4200** — API Bridge
- **Port 8080** — Web Dashboard (The Watchtower)

### Environment Variables in Docker

```bash
docker run -d \
  -e ANTHROPIC_API_KEY=your-key \
  -e TELEGRAM_BOT_TOKEN=your-token \
  -v sovereign_data:/home/sovereign/.sovereign-kernel \
  -p 4200:4200 -p 8080:8080 \
  sovereign-kernel

```

---

## Documentation

| Document | Description |
| --- | --- |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | 10-crate workspace deep dive, 29 subsystems |
| [docs/USAGE.md](docs/USAGE.md) | Detailed usage guide, tool reference, agent configuration |
| [docs/SECURITY.md](docs/SECURITY.md) | Security model, The Warden, The Gatekeeper |
| [docs/SAFETY_CONTROLS.md](docs/SAFETY_CONTROLS.md) | Budget controls, limits, forensics |
| [docs/VISION.md](docs/VISION.md) | Long-term AI Operating System vision |
| [docs/PROJECT_PLAN.md](docs/PROJECT_PLAN.md) | Full development roadmap |
| [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) | How to add Hands, adapters, providers |

---

## Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for guidelines on adding new:

- **Hands** — autonomous capability packages
- **Channel adapters** — messaging platform integrations
- **LLM providers** — model API drivers
- **MCP tools** — external tool server connections

---

## License

MIT License — open source, for the world.
