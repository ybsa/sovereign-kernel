# Sovereign Kernel: The Agentic Operating System

![Rust](https://img.shields.io/badge/language-Rust-orange?style=flat-square)
![MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)
![Status](https://img.shields.io/badge/status-v1.0.0--stable-brightgreen?style=flat-square)
![Subsystems](https://img.shields.io/badge/subsystems-29%20Named-blueviolet?style=flat-square)
![Channels](https://img.shields.io/badge/channels-30%2B-brightgreen?style=flat-square)
![Providers](https://img.shields.io/badge/LLM%20providers-50%2B-brightgreen?style=flat-square)
![Security](https://img.shields.io/badge/security-Landlock%20%2B%20seccomp-red?style=flat-square)

> **Terminal-first. Runs everywhere. Single Rust binary.**

**Sovereign Kernel** is a personal AI assistant and agentic operating system, built entirely in Rust. It is a complete port of [OpenClaw](https://github.com/openclaw/openclaw) (the AI assistant) and [NemoClaw](https://github.com/NVIDIA/NemoClaw) (the sandbox) into a single, memory-safe, production-grade binary.

It answers you on the channels you already use (Telegram, Discord, WhatsApp, Slack, Signal, iMessage, Matrix, IRC, Teams, and 20+ more), runs agents 24/7 as background daemons, and wraps every action in Landlock/seccomp security — all from your terminal.

```
Telegram / Discord / WhatsApp / Slack / Signal / iMessage / Matrix / IRC / Teams / WebChat
               │
               ▼
┌─────────────────────────────────────────────┐
│          Sovereign Kernel (Rust)            │
│         The Agentic Operating System        │
│                                             │
│  The Oracle ← 50+ LLM providers            │
│  The Bridge ← 30+ channel adapters         │
│  The Warden ← Landlock/seccomp sandbox     │
│  The Village ← multi-agent ecosystem       │
│  The Watchtower ← terminal web dashboard   │
│                                             │
│         ws://127.0.0.1:18789                │
└─────────────────────────────────────────────┘
               │
               ├─ sovereign chat          (interactive REPL)
               ├─ sovereign dashboard     (web UI)
               ├─ sovereign run "task"    (autonomous)
               └─ sovereign hands list    (capabilities)
```

---

## 🎯 What It Does

- **Answers on every channel** — Telegram, Discord, WhatsApp, Slack, Signal, iMessage, Matrix, IRC, Twitch, Teams, Google Chat, Nostr, WebChat, and more via **The Bridge**
- **Runs agents 24/7** as background OS daemons with heartbeat monitoring and auto-restart via **The Resurrector**
- **50+ LLM providers** — Anthropic, OpenAI, Gemini, Groq, Ollama, NVIDIA, Bedrock, Together, HuggingFace, and more via **The Oracle**
- **Model failover** — automatic fallback chains with cooldown and health probes via **The Sentinel**
- **Spawns agents from natural language** — describe a task, **The Builder** creates the right agent
- **Dynamic sub-agents** — **The Witch** summons sandboxed workers for parallel tasks
- **Full tool execution** — shell, file, code, browser (CDP), media — with approval gates via **The Gatekeeper**
- **Browser automation** — CDP-based Chrome/Chromium control via **The Forge**
- **Voice Wake + Talk Mode** — always-on speech (STT/TTS) via **The Voice**
- **Live Canvas** — agent-driven A2UI visual workspace via **The Canvas**
- **In-channel commands** — `/status`, `/new`, `/compact`, `/think`, `/verbose`, `/usage` via **The Herald**
- **Landlock/seccomp sandbox** — filesystem and syscall isolation via **The Warden**
- **Network egress control** — policy-driven network interception with operator approval via **The Gatekeeper**
- **Crash recovery** — **The Resurrector** auto-restarts dead agents from their last checkpoint
- **Remembers everything** — hybrid SQLite + BM25 vector memory via **The Archive**
- **Smart compaction** — context truncation and ground-truth summarization via **The Healer**
- **Tamper-evident audit** — Merkle chain of every agent action via **The Ledger**
- **Global budget control** — USD cap across all agents via **The Treasury**
- **Usage analytics** — per-agent/channel/model cost tracking via **The Chronicler**
- **Presence tracking** — online/offline/busy broadcast via **The Beacon**
- **Community marketplace** — publish and install Hands via **The Bazaar**
- **Remote access** — Tailscale Serve/Funnel and SSH tunnels via **The Cartographer**
- **Plugin SDK** — extend with WASM/dynamic libs via **The Alchemist**
- **Cross-instance collaboration** — agent-to-agent protocol across machines via **The Diplomat**
- **Push notifications** — desktop alerts, email, Gmail Pub/Sub via **The Raven**
- **Terminal web dashboard** — embedded at `localhost:8080` via **The Watchtower**
- **Ships 30+ autonomous Hands** — research, email, browser, clip, collector, lead, predictor, twitter, peka, otto, and more

---

## ⚠️ Safety & Budget Controls

Sovereign Kernel includes hard limits, global budgeting, and strict gating to prevent runaway costs and unintended damage.

- **Hard Limits**: Unlimited by default. Pass `--max-iterations`, `--max-tokens`, and `--budget-usd` to override via CLI.
- **Approval Gated**: Dangerous actions require approval — even in unrestricted mode.
- **Global Budget**: USD cap across all agents (e.g., $5.00 limit) via The Treasury.
- **Forensics**: Step-by-step JSONL dumps with secrets automatically redacted.
- **Sandbox Modes**: Toggle between `Sandbox` (strict) and `Unrestricted` (full host access).
- **Elevated Toggle**: Per-session `/elevated on|off` for host permissions.

See [docs/SAFETY_CONTROLS.md](docs/SAFETY_CONTROLS.md) for details.

---

## 🚀 Quick Start

### Requirements

- Rust `1.75+`
- An API key for at least one LLM provider

### Install and run

**Windows (PowerShell):**

```powershell
$env:GEMINI_API_KEY="your-key"
cargo run -p sk-cli -- onboard       # The Builder — first-run setup wizard
cargo run -p sk-cli -- chat          # Interactive agent REPL
cargo run -p sk-cli -- dashboard     # The Watchtower → http://localhost:8080
cargo run -p sk-cli -- hands list    # Show all bundled Hands
```

**Linux / macOS:**

```bash
export GEMINI_API_KEY="your-key"
cargo run -p sk-cli -- onboard
cargo run -p sk-cli -- chat
cargo run -p sk-cli -- dashboard
cargo run -p sk-cli -- hands list
```

### Supported LLM Providers (The Oracle — 50+)

| Provider | Env Variable |
| -------- | ----------- |
| Anthropic Claude | `ANTHROPIC_API_KEY` |
| OpenAI GPT-4o/o1 | `OPENAI_API_KEY` |
| Google Gemini | `GEMINI_API_KEY` |
| Groq (Llama 3) | `GROQ_API_KEY` |
| GitHub Copilot | `GITHUB_TOKEN` |
| NVIDIA NIM | `NVIDIA_API_KEY` |
| AWS Bedrock | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |
| Ollama (local) | `OLLAMA_HOST` |
| Together AI | `TOGETHER_API_KEY` |
| HuggingFace | `HF_TOKEN` |
| Moonshot | `MOONSHOT_API_KEY` |
| And 40+ more... | See `sovereign doctor` |

---

## ⚡ CLI Commands

```bash
sovereign onboard                          # The Builder — first-run wizard
sovereign chat                             # Interactive agent REPL
sovereign run "<task>"                     # Autonomous task execution
sovereign run "<task>" --mode unrestricted # Full host access
sovereign run "<task>" --schedule "cron"   # Scheduled recurring task
sovereign start                            # Start as foreground daemon
sovereign start --detach                   # Start as detached background daemon
sovereign status                           # Village overview (agents + jobs)
sovereign kill <agent-id>                  # Kill a specific agent
sovereign kill                             # Stop the daemon
sovereign mcp list                         # List active MCP Tool Servers
sovereign mcp add <name> <cmd>             # Add an MCP tool server dynamically
sovereign dashboard [--port 8080]          # The Watchtower
sovereign hands list                       # All bundled Hands
sovereign hands activate <name>            # Start a Hand
sovereign hands install <url>              # Install from The Bazaar
sovereign hands publish                    # Publish to The Bazaar
sovereign audit logs                       # The Ledger
sovereign audit verify                     # Verify Merkle chain integrity
sovereign doctor                           # Full diagnostic suite
sovereign tunnel --tailscale|--ssh         # The Cartographer
sovereign usage                            # The Chronicler cost report
```

### In-Channel Chat Commands (The Herald)

Send these in Telegram/Discord/WhatsApp/Slack/Teams/WebChat:

- `/status` — session status (model + tokens, cost)
- `/new` or `/reset` — reset the session
- `/compact` — compact session context (The Healer)
- `/think <level>` — off|minimal|low|medium|high|xhigh
- `/verbose on|off`
- `/usage off|tokens|full` — per-response usage footer
- `/restart` — restart the daemon (owner-only)
- `/activation mention|always` — group activation toggle
- `/elevated on|off` — toggle host access per-session

---

## 🏛️ The 29 Named Subsystems

| # | Name | Crate | Role |
|---|---|---|---|
| 1 | **The Kernel** | `sk-kernel` | WS control plane, HTTP API, daemon lifecycle |
| 2 | **The Village** | `sk-engine` | Multi-agent ecosystem: spawn, route, coordinate |
| 3 | **Hands** | `sk-hands` | Autonomous capability packages |
| 4 | **The Witch** | `sk-engine` | Dynamic subagent spawning |
| 5 | **The Resurrector** | `sk-kernel` | Crash recovery from checkpoints |
| 6 | **The Healer** | `sk-engine` | Context compaction & smart truncation |
| 7 | **The Builder** | `sk-cli` | `sovereign onboard` setup wizard |
| 8 | **The Warden** | `sk-kernel` | Landlock/seccomp/network sandbox |
| 9 | **The Gatekeeper** | `sk-kernel` | Exec approval + blocked action intercept |
| 10 | **The Bridge** | `sk-channels` | 30+ channel adapters |
| 11 | **The Oracle** | `sk-engine` | 50+ LLM provider catalog with failover |
| 12 | **The Sentinel** | `sk-engine` | Retry policy (LLM + channel delivery) |
| 13 | **The Scribe** | `sk-memory` | Session transcripts, write locks, repair |
| 14 | **The Archive** | `sk-memory` | SQLite + BM25 vector memory substrate |
| 15 | **The Forge** | `sk-tools` | CDP browser automation |
| 16 | **The Voice** | `sk-tools` | Always-on STT/TTS speech |
| 17 | **The Canvas** | `sk-cli` | A2UI agent-driven visual workspace |
| 18 | **Soul Files** | `sk-soul` | Agent identity (SOUL.md, AGENTS.md) |
| 19 | **The Watchtower** | `sk-cli` | Terminal web dashboard |
| 20 | **The Ledger** | `sk-kernel` | Merkle tamper-evident audit trail |
| 21 | **The Treasury** | `sk-engine` | Global USD budget cap + cost tracking |
| 22 | **The Herald** | `sk-channels` | In-channel slash commands |
| 23 | **The Beacon** | `sk-kernel` | Presence system (online/offline/busy) |
| 24 | **The Raven** | `sk-kernel` | Notifications, email alerts, Gmail Pub/Sub |
| 25 | **The Bazaar** | `sk-hands` | Community Hands marketplace |
| 26 | **The Cartographer** | `sk-kernel` | Tailscale/SSH tunnel for remote access |
| 27 | **The Diplomat** | `sk-mcp` | Cross-instance agent-to-agent protocol |
| 28 | **The Alchemist** | `sk-mcp` | Plugin SDK (WASM/dynamic lib extensions) |
| 29 | **The Chronicler** | `sk-engine` | Usage analytics per agent/channel/model |

---

## 📡 The Bridge — 30+ Channel Adapters

### Core Adapters

| Channel | Protocol |
|---|---|
| Telegram | Bot API (HTTPS) |
| Discord | Gateway WS + REST |
| WhatsApp | Web WS protocol |
| Slack | Socket Mode + Events API |
| Signal | signal-cli bridge |
| iMessage / BlueBubbles | BlueBubbles REST API |
| Matrix | matrix-rust-sdk |
| WebChat | Built into The Watchtower |

### Extension Adapters

Google Chat · Microsoft Teams · IRC · Twitch · Nostr · Zalo · Zalo Personal · Feishu · Line · Mattermost · Synology Chat · Nextcloud Talk · Tlon

### Bridge Features

- **Channel Dock** — unified inbound/outbound routing
- **DM Pairing** — security flow for unknown senders
- **Group Routing** — mention gating, reply tags
- **Allowlists** — per-channel sender/group allowlists
- **Typing Indicators** — real-time typing state
- **Reactions** — acknowledge/status reactions
- **Multi-Agent Routing** — route channels to isolated Village agents

---

## 🖐️ Bundled Hands (30+)

| Hand | Category | What It Does |
| ---- | -------- | ----------- |
| **browser** | Automation | CDP-based web browser automation |
| **researcher** | Research | Multi-source deep research with citations |
| **web-search** | Research | Brave/Tavily web search + reports |
| **clip** | Content | Clipboard-to-note capture |
| **collector** | Data | Structured data collection and archival |
| **lead** | Sales | Autonomous lead research |
| **predictor** | Analytics | Trend analysis and forecasting |
| **email** | Communication | SMTP/IMAP email management |
| **twitter** | Social | Twitter/X monitoring |
| **otto** | Builder | Docker-sandboxed code execution |
| **mysql-reporter** | Data | MySQL reporting and dashboards |
| **peka** | Terminal | The Terminal Master — shell expert |

---

## 🖥️ The Watchtower — Terminal Web Dashboard

Run `sovereign dashboard` to open at `http://localhost:8080`.

- **Embedded in the binary** — no Node.js, no npm, zero extra dependencies
- **Three-pane layout**: agents/hands panel | live log stream | command bar
- **Real-time monitoring**: uptime, active hands, approval queue
- **The Gatekeeper**: approve/deny blocked actions and network requests
- **WebChat**: embedded chat interface
- **The Chronicler**: usage analytics and cost breakdown
- **The Beacon**: agent presence (online/offline/busy)

---

## 🔒 The Warden — Security (NemoClaw in Rust)

Sovereign Kernel integrates [NemoClaw](https://github.com/NVIDIA/NemoClaw)'s security model natively in Rust:

### Filesystem Isolation (Landlock LSM)

| Path | Access |
|---|---|
| `/sandbox`, `/tmp`, `/dev/null` | Read-write |
| `/usr`, `/lib`, `/proc`, `/app`, `/etc` | Read-only |
| Everything else | Blocked |

### Network Egress Control

- Policy YAML defines allowed endpoints per binary
- All unlisted connections are **intercepted and blocked**
- Blocked requests surface in **The Watchtower** for operator approve/deny
- Dynamic policy reload without restarting agents

### Inference Routing

All LLM calls are proxied through **The Oracle** — agents never connect directly to model APIs.

### Sandbox Modes

- `sandbox` — strict Warden enforcement (default for spawned agents)
- `unrestricted` — full host access (requires explicit `--mode unrestricted`)

---

## 🏘️ The Village — Agent Ecosystem

| Feature | Subsystem | Description |
| ------- | --------- | ----------- |
| Agent Spawning | The Witch | Dynamically summon sandboxed workers |
| Agent Messaging | Inter-Agent Bus | Direct messages between Village agents |
| Crash Recovery | The Resurrector | Auto-restart from checkpoint |
| Shared Memory | Village Library | Global knowledge pool |
| Natural Language Build | The Builder | Create agents from plain English |
| Depth Limits | Village Guard | Prevent infinite spawn recursion |

---

## 🏗️ Architecture (10-Crate Workspace)

```text
sk-types     → Shared types, capability gates, taint tracking
sk-soul      → Soul Files: agent identity (SOUL.md, AGENTS.md)
sk-memory    → The Archive + The Scribe (SQLite + BM25 + sessions)
sk-engine    → The Oracle + The Healer + The Village + The Witch + The Treasury + The Chronicler + The Sentinel
sk-mcp       → The Diplomat + The Alchemist (A2A protocol + plugin SDK)
sk-kernel    → The Kernel + The Warden + The Gatekeeper + The Resurrector + The Raven + The Cartographer + The Beacon + The Ledger
sk-tools     → The Forge + The Voice + shell/file/media/camera/screen tools
sk-channels  → The Bridge + The Herald (30+ adapters + chat commands)
sk-hands     → Hands + The Bazaar (capability packages + marketplace)
sk-cli       → The Watchtower + The Builder + The Canvas + CLI surface
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for a deep dive.

---

## 🗺️ Development Roadmap

| Phase | Status | Milestone |
| ----- | ------ | --------- |
| 1–24 | ✅ Complete | Original Sovereign Kernel (Engine, Memory, Security, Dashboard, Village) |
| **25** | ✅ Complete | **The Great Merger: OpenClaw + NemoClaw → Rust** |

### Phase 25 Breakdown

| Sub-Phase | Milestone | Status |
|---|---|---|
| 25.1 | Core Gateway | ✅ Done |
| 25.2 | Agent Runtime | ✅ Done |
| 25.3 | Tools & Execution | ✅ Done |
| 25.4 | Channels | ✅ Done |
| 25.5 | Multi-Agent | ✅ Done |
| 25.6 | Dashboard & Canvas | ✅ Done |
| 25.7 | Security | ✅ Done |
| 25.8 | Production | ✅ Done |
| 25.9 | Extended Services | ✅ Done |
| 25.10 | Advanced | ✅ Done |

---

## 📚 Documentation

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — 10-crate workspace deep dive + 29 subsystems
- [docs/SECURITY.md](docs/SECURITY.md) — Security model, The Warden, The Gatekeeper
- [docs/SAFETY_CONTROLS.md](docs/SAFETY_CONTROLS.md) — The Treasury, limits, forensics
- [docs/USAGE.md](docs/USAGE.md) — Hands, dashboard, channels, agent configuration
- [docs/VISION.md](docs/VISION.md) — The long-term AI Operating System vision
- [docs/PROJECT_PLAN.md](docs/PROJECT_PLAN.md) — Full development roadmap

---

## 🧬 Origin

Sovereign Kernel is the Rust-native unification of three projects:

- **[OpenClaw](https://github.com/openclaw/openclaw)** — The personal AI assistant (Node.js/TypeScript). We port its 800+ file codebase: 50+ LLM providers, 30+ channel adapters, browser automation, voice, canvas, skills, and everything else — entirely into Rust.
- **[NemoClaw](https://github.com/NVIDIA/NemoClaw)** — The NVIDIA sandbox orchestrator. We port its Landlock/seccomp/network egress security policies natively into The Warden.
- **[Sovereign Kernel](https://github.com/)** — The Rust OS foundation. Provides the daemon lifecycle, memory substrate, audit trail, and dark-fantasy naming that wraps it all together.

The result: **one `sovereign` binary that does everything OpenClaw + NemoClaw can do, but faster, safer, and in pure Rust.**

---

## 🤝 Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) to learn how to add new Hands, channel adapters, LLM providers, and MCP tools.

## ⚖️ License

MIT License. Open source, for the world.
