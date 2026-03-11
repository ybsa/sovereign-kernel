<p align="center">
  <img src="https://img.shields.io/badge/language-Rust-orange?style=flat-square" alt="Rust" />
  <img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT" />
  <img src="https://img.shields.io/badge/status-Phase%2020%20Complete-brightgreen?style=flat-square" alt="Status" />
  <img src="https://img.shields.io/badge/hands-10%20Bundled-brightgreen?style=flat-square" alt="Hands" />
  <img src="https://img.shields.io/badge/security-Merkle%20Audit%20Trail-brightgreen?style=flat-square" alt="Security" />
</p>

# Sovereign Kernel: The Agentic Operating System

> [!WARNING]
> **Active Development**: Sovereign Kernel is experimental software under active development. Expect breaking changes between versions.

**Sovereign Kernel** is a virtual operating system for AI agents, built entirely in Rust. It merges a memory-safe daemon core with a sophisticated memory substrate, 10 bundled autonomous capability packages (Hands), and a built-in terminal web dashboard.

It is not just a framework — it is the mediation layer between Autonomous Entities and the silicon they run on.

---

## 🎯 What It Does

- **Runs agents 24/7** as background OS daemons with heartbeat monitoring and auto-restart
- **Executes laboratory tools safely** — shell, file, code, web, browser — with capability gates and sandbox policy
- **Remembers everything** via a hybrid SQLite + BM25 vector memory substrate across sessions
- **Optimizes token usage** with "The Healer" (Smart Truncation & Ground-Truth Context Compaction)
- **Executes safely** in native environments or isolated **Docker sandboxes**
- **Ships 10 autonomous Hands** — pre-built capability packs for browser automation, research, email, lead generation, Docker sandbox, and more
- **Streams a terminal dashboard** at `http://localhost:8080` — manage agents, monitor live logs, approve actions
- **Integrates 30+ channels** — Telegram, Discord, WhatsApp, Signal, Slack, and more via the Channel Bridge

---

## 🚀 Quick Start

### Requirements
- Rust `1.75+`
- An API key for at least one LLM provider

### Run from source

**Windows (PowerShell):**
```powershell
$env:GEMINI_API_KEY="your-key"
cargo run -p sk-cli -- init        # First-time setup wizard
cargo run -p sk-cli -- chat        # Interactive terminal chat
cargo run -p sk-cli -- dashboard   # Terminal web dashboard → http://localhost:8080
cargo run -p sk-cli -- hands list  # Show all 9 bundled hands
```

**Linux / macOS:**
```bash
export GEMINI_API_KEY="your-key"
cargo run -p sk-cli -- init
cargo run -p sk-cli -- chat
cargo run -p sk-cli -- dashboard
cargo run -p sk-cli -- hands list
```

### Supported LLM Providers (auto-detected from env)
| Provider | Env Variable |
|----------|-------------|
| Anthropic Claude | `ANTHROPIC_API_KEY` |
| OpenAI GPT-4o | `OPENAI_API_KEY` |
| Google Gemini | `GEMINI_API_KEY` |
| Groq (Llama 3) | `GROQ_API_KEY` |
| GitHub Copilot | `GITHUB_TOKEN` |

---

## ⚡ CLI Commands

```
sovereign init                       # First-run setup wizard
sovereign chat                       # Interactive agent REPL
sovereign start                      # Start as background daemon
sovereign status                     # Check daemon status
sovereign stop                       # Stop the daemon
sovereign dashboard [--port 8080]    # Open terminal web dashboard
sovereign hands list                 # List all 10 bundled hands
sovereign hands activate <name>      # Start a hand's autonomous agent
sovereign hands status               # Show running hand instances
sovereign hands deactivate <id>      # Stop a hand instance
sovereign audit logs                 # View cryptographic audit trail
sovereign audit verify               # Verify Merkle chain integrity
```

---

## 🖐️ Bundled Hands (10 Included)

Hands are autonomous capability packages — each one is a pre-configured agent with specific tools, prompts, and dashboard metrics.

| Hand | Category | What It Does |
|------|----------|-------------|
| **browser** | Automation | Playwright-based web browser automation |
| **researcher** | Research | Multi-source deep research with citation tracking |
| **web-search** | Research | Brave/Tavily web search + intelligence reports |
| **clip** | Content | Clipboard-to-note capture and organization |
| **collector** | Data | Structured data collection and archival |
| **lead** | Sales | Autonomous lead research and qualification |
| **predictor** | Analytics | Trend analysis and forecasting |
| **email** | Communication | SMTP/IMAP email management with draft mode |
| **twitter** | Social | Twitter/X monitoring and engagement |
| **otto** | Builder | Docker-sandboxed code execution with dynamic dependencies |

```bash
sovereign hands list           # See all hands with requirements
sovereign hands activate email # Start the email hand
sovereign hands status         # Monitor running hands
```

---

## 🖥️ Terminal Web Dashboard

Run `sovereign dashboard` to open a **terminal-aesthetic web UI** at `http://localhost:8080`.

- **Embedded in the binary** — no Node.js, no npm, zero extra dependencies
- **Three-pane layout**: agents/hands panel | live log stream | command bar
- **Real-time monitoring**: uptime, active hands, approval queue
- **Live log stream**: every tool call and agent action in real-time
- **Command input**: type `sovereign` commands directly in the UI

```bash
sovereign dashboard              # Opens at http://localhost:8080
sovereign dashboard --port 9090  # Custom port
sovereign dashboard --no-open    # Don't auto-open browser
```

---

## 🤖 Multi-Agent Coordination (Phase 15)

Sovereign Kernel now supports **swarm intelligence** — agents can communicate, delegate tasks, and share knowledge.

| Feature | Laboratory Tool | Description |
|---------|------|-------------|
| **Agent Messaging** | `agent_message` | Send direct messages between agents via the Inter-Agent Bus |
| **Witch Skeleton Spawning** | `spawn_witch_skeleton` | Dynamically spawn sandboxed witch skeleton agents for parallel tasks |
| **Witch Status** | `check_witch_skeleton` | Poll the status and results of spawned witch skeletons |
| **Shared Memory Store** | `shared_memory_store` | Store facts globally for all authorized agents |
| **Shared Memory Recall** | `shared_memory_recall` | Search the global knowledge pool |

> **Security**: All spawned witch skeletons are **forced into Sandbox mode** — they must ask the user for permission on every action, regardless of the parent agent's mode.

---

## 🏗️ Architecture (10-Crate Workspace)

```
sk-types    → Shared types, capability gates, taint tracking
sk-engine   → Agent loop, LLM drivers, tool execution
sk-kernel   → Daemon lifecycle, scheduling, heartbeat
sk-soul     → Agent identity from SOUL.md
sk-memory   → SQLite + BM25 vector memory substrate
sk-mcp      → Model Context Protocol (MCP) nervous system
sk-tools    → Shell, file, web, code, browser tool implementations
sk-hands    → 10 bundled autonomous capability packages
sk-channels → 30+ channel adapters (Telegram, Discord, WhatsApp...)
sk-cli      → sovereign binary + terminal dashboard server
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for a deep dive.

---

## 🗺️ Roadmap

| Phase | Status | Milestone |
|-------|--------|-----------|
| 1–6 | ✅ Complete | Kernel, Engine, Security, Media, LLM Drivers |
| 7–11 | ✅ Complete | Industrial Core: 52 skills, BM25, Audit Trail, API Bridge |
| 12 | ✅ Complete | Terminal Web Dashboard (`sovereign dashboard`) |
| 13 | ✅ Complete | Core Tools Upgrade: Shell, File, Code, Browser Hands |
| 14 | ✅ Complete | Optional Hands: Web Search, Email |
| 15 | ✅ Complete | Multi-Agent Coordination: A2A Bus, Witch Spawning, Shared Memory |
| 16 | ⏳ Planned | Production Hardening: Log rotation, fallback LLM, load testing |
| 17 | ⚠️ Scaffolded | Channel Integrations: 30+ adapters exist, need wiring |
| 18 | ✅ Complete | Docker Sandbox Integration: OTTO + zero-pollution execution |
| 19 | ⏳ Planned | Universal Tooling: MCP client + autonomous discovery |
| 20 | ✅ Complete | Documentation & Lore: Dark fantasy naming, PROJECT_MAP |
| 21 | ⏳ Planned | Self-Refactoring & P2P Skill Graph |
| 22 | ⏳ Planned | Full GUI & Screen Control (browser, screenshots, desktop automation) |
| 23 | ⏳ Planned | Universal Cross-Platform (Windows, Linux, macOS, Raspberry Pi, ARM) |

---

## 📚 Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — 10-crate workspace deep dive
- [docs/SECURITY.md](docs/SECURITY.md) — Security model, sandboxing, audit trail
- [docs/USAGE.md](docs/USAGE.md) — Hands, dashboard, channels, and agent configuration
- [docs/VISION.md](docs/VISION.md) — The long-term AI Operating System vision
- [docs/PROJECT_PLAN.md](docs/PROJECT_PLAN.md) — Full 30-week development roadmap (23 phases)

---

## 🤝 Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) to learn how to add new Hands, MCP tools, and channel adapters.

## ⚖️ License

MIT License. Open source, for the world.
