# Sovereign Kernel — Architecture

## Overview

Sovereign Kernel is a 10-crate Rust workspace containing 29 named subsystems. It unifies OpenClaw (AI assistant), NemoClaw (sandbox), and the original Sovereign Kernel (OS daemon) into a single, memory-safe binary.

---

## Workspace Structure

```text
sovereign-kernel/
├── Cargo.toml              # Workspace root
├── Cargo.lock
├── Dockerfile              # Multi-stage production build
├── docker-compose.yml      # One-command deployment
├── .env.example            # Environment variable template
├── soul/                   # Agent identity files
│   ├── SOUL.md             # Agent personality and rules
│   ├── IDENTITY.md         # Communication style
│   ├── AGENTS.md           # Village hierarchy
│   ├── MEMORY.md           # Memory configuration
│   └── USER.md             # Auto-populated user preferences
├── docs/                   # Project documentation
│   ├── ARCHITECTURE.md     # This file
│   ├── USAGE.md            # Usage guide and tool reference
│   ├── SECURITY.md         # Security model
│   ├── SAFETY_CONTROLS.md  # Budget controls and forensics
│   ├── VISION.md           # Long-term roadmap
│   ├── PROJECT_PLAN.md     # Development plan
│   └── CONTRIBUTING.md     # Contribution guidelines
└── crates/                 # Rust workspace members
    ├── sk-types/            # Shared types and traits
    ├── sk-soul/             # Agent identity parser
    ├── sk-memory/           # Memory substrate (SQLite + BM25 + vectors)
    ├── sk-engine/           # LLM orchestration + multi-agent runtime
    ├── sk-mcp/              # A2A protocol + plugin SDK
    ├── sk-kernel/           # Core daemon, security, crash recovery
    ├── sk-tools/            # Tool execution (browser, shell, file, voice)
    ├── sk-channels/         # 30+ messaging adapters
    ├── sk-hands/            # Autonomous capability packages
    └── sk-cli/              # CLI surface + web dashboard

```

---

## Crate Dependency Graph

```text
                     sk-cli
                    /  |   \
                   /   |    \
            sk-kernel  |  sk-hands
             / |  \    |    |
            /  |   \   |    |
     sk-engine |  sk-channels
        |   \  |     |
        |    \ |     |
     sk-mcp  sk-tools
        |      |
        |      |
     sk-memory |
        |  \   |
        |   \  |
     sk-soul   |
        \      |
         \     |
         sk-types

```

---

## Crate Details

### sk-types

Foundation crate. Provides shared types, error definitions, capability gates, taint tracking, and the `Memory` trait interface used across all crates.

**Key exports:** `AgentId`, `SovereignError`, `SovereignResult`, `KernelConfig`, `Memory` trait, `ToolDefinition`, `ExportFormat`, `ImportReport`

---

### sk-soul — Agent Identity

Parses Soul Files (`SOUL.md`, `AGENTS.md`, `IDENTITY.md`) to construct agent personalities, workspace prompts, and behavioral constraints. Uses YAML frontmatter for structured metadata.

---

### sk-memory — The Archive + The Scribe

The memory substrate. All data is stored in a single SQLite database with WAL mode enabled for concurrent access.

| Subsystem | Role |
| --- | --- |
| **The Archive** | Unified memory substrate: Structured KV, Semantic vectors, BM25 full-text, Knowledge graph. Supports full lifecycle management — export (JSON/Markdown), import, and recall. Includes 106+ pre-bundled expert skills. |
| **The Scribe** | Session transcripts, write locks, transcript repair. Manages session state lifecycle. |

**Internal components:**

- `MemorySubstrate` — central hub implementing the `Memory` trait
- `StructuredStore` — SQLite key-value store
- `SemanticStore` — vector embeddings with cosine similarity
- `Bm25Index` — FTS5 full-text search with BM25 ranking
- `KnowledgeStore` — entity-relation triples
- `SessionStore` — conversation persistence
- `AuditStore` — Merkle chain integrity
- `CheckpointStore` — crash recovery state

---

### sk-engine — The Oracle + The Healer + The Village + The Witch + The Treasury + The Chronicler + The Sentinel

The intelligence layer. Manages LLM orchestration, multi-agent coordination, and resource accounting.

| Subsystem | Role |
| --- | --- |
| **The Oracle** | 50+ LLM provider catalog. Manages auth profiles, OAuth flows, model discovery, and API calls. Supports Anthropic, OpenAI, Gemini, Groq, NVIDIA NIM, Bedrock, Together, HuggingFace, Mistral, DeepSeek, xAI, Perplexity, OpenRouter, and 40+ more. |
| **The Healer** | Context compaction. Summarizes and prunes older conversation turns to stay within token budgets. Triggers at 80% context capacity. |
| **The Village** | Multi-agent ecosystem. Inter-Agent Bus for direct messaging, shared Village Library memory. |
| **The Witch** | Dynamic subagent spawning. Creates sandboxed workers for parallel task execution with depth limits. |
| **The Treasury** | Global USD budget cap. Tracks cost per agent, enforces limits, kills agents on overspend. |
| **The Chronicler** | Usage analytics. Records token usage, cost, and latency per agent, channel, and model. |
| **The Sentinel** | Retry policy. Exponential backoff with jitter for failed LLM calls and channel delivery. |

---

### sk-mcp — The Diplomat + The Alchemist

External integration layer.

| Subsystem | Role |
| --- | --- |
| **The Diplomat** | Cross-instance agent-to-agent protocol. Allows Sovereign Kernel instances on different machines to collaborate. |
| **The Alchemist** | Plugin SDK. Third-party extensions as WASM modules or dynamic Rust libraries (.so/.dll). |

---

### sk-kernel — The Kernel + The Warden + The Gatekeeper + The Resurrector + The Raven + The Cartographer + The Beacon + The Ledger

The operating system core. Manages daemon lifecycle, security, and infrastructure.

| Subsystem | Role |
| --- | --- |
| **The Kernel** | WebSocket control plane, HTTP API server, config system (TOML/JSON with hot-reload), daemon lifecycle (systemd/launchd). |
| **The Warden** | Security sandbox: Landlock LSM filesystem isolation, seccomp-bpf syscall filtering, network egress proxy with policy YAML. |
| **The Gatekeeper** | Exec approval manager. Intercepts dangerous commands and network requests, surfaces them in The Watchtower for approve/deny. |
| **The Resurrector** | Crash recovery. Auto-restarts crashed agents from their last SQLite checkpoint. Saves checkpoints every 30 seconds. |
| **The Raven** | Notifications. Push alerts, email, Gmail Pub/Sub triggers, inbound webhooks. |
| **The Cartographer** | Remote access. Native Tailscale Serve/Funnel and SSH tunnel support. |
| **The Beacon** | Presence system. Tracks agent online/offline/busy state and broadcasts to connected clients. |
| **The Ledger** | Merkle audit trail. Tamper-evident SHA-256 chain of every agent action. |

---

### sk-tools — The Forge + The Voice + Tool Execution

All tool implementations live here.

| Subsystem | Role |
| --- | --- |
| **The Forge** | CDP-based Chrome/Chromium browser automation. Screenshots, navigation, form filling, downloads. |
| **The Voice** | Always-on speech. OpenAI Whisper STT + OpenAI TTS-1. |
| **Shell execution** | Bash/PowerShell with full PTY support, process registry, background jobs. |
| **File operations** | Read, write, edit, glob — path-sandboxed, atomic writes. |
| **Media pipeline** | Image/audio/video processing, transcription, size caps. |
| **Device tools** | Camera snap/clip, screen record, location (when hardware available). |
| **Code execution** | Docker/native sandbox with timeout and output capture. |
| **Memory tools** | `remember`, `recall`, `forget` — wired to The Archive via BM25. |
| **Skill registry** | 106+ expert prompts loaded dynamically from `skills/*/SKILL.md` files. |

---

### sk-channels — The Bridge + The Herald

All messaging platform integrations.

| Subsystem | Role |
| --- | --- |
| **The Bridge** | 30+ channel adapters: Telegram, Discord, WhatsApp, Slack, Signal, iMessage/BlueBubbles, Matrix, IRC, Twitch, Teams, Google Chat, Nostr, WebChat, Zalo, Feishu, Line, Mattermost, Synology Chat, Nextcloud Talk, Tlon. |
| **The Herald** | In-channel slash command parser: `/status`, `/new`, `/compact`, `/think`, `/verbose`, `/usage`, `/restart`, `/activation`, `/elevated`. |

**Bridge features:** Channel Dock (routing), DM pairing, group routing, mention gating, allowlists, typing indicators, reactions, multi-agent channel routing.

---

### sk-hands — Hands + The Bazaar

Autonomous capability packages.

| Subsystem | Role |
| --- | --- |
| **Hands** | 30+ bundled capability agents: browser, researcher, web-search, clip, collector, lead, predictor, email, twitter, otto, mysql-reporter, peka, and more. |
| **The Bazaar** | Community marketplace. Publish, discover, install, version, and rate community Hands. |

---

### sk-cli — The Watchtower + The Builder + The Canvas

The user-facing surface.

| Subsystem | Role |
| --- | --- |
| **The Watchtower** | Terminal web dashboard at `localhost:8080`. Live logs, approval queue, agent management, WebChat, usage analytics. Embedded in the binary — zero external dependencies. |
| **The Builder** | `sovereign init` — interactive TUI setup wizard. Provider detection, key validation, first-run config. |
| **The Canvas** | A2UI agent-driven visual workspace, served via The Watchtower WebSocket. |
| **CLI surface** | All `sovereign` subcommands: `init`, `chat`, `run`, `status`, `kill`, `dashboard`, `hands`, `audit`, `doctor`, `memory`, `treasury`, `channels`, `mcp`, `tunnel`, `usage`. |

---

## Data Flow

```text
User sends message (Telegram / Discord / CLI / WebChat)
    → The Bridge (sk-channels) receives via adapter protocol
    → The Herald checks for slash commands (/status, /new, etc.)
    → The Kernel (sk-kernel) routes to agent session
    → The Scribe (sk-memory) loads session transcript
    → The Oracle (sk-engine) selects LLM provider + model
    → The Sentinel retries on transient failure
    → The Healer compacts if context exceeds 80% budget
    → Agent loop executes with tools (sk-tools)
    → The Gatekeeper checks for dangerous commands
    → The Forge runs browser automation if needed
    → The Ledger records every action (Merkle chain)
    → The Treasury checks budget, kills agent on overspend
    → The Chronicler logs usage (tokens, cost, latency)
    → Response flows back through The Bridge
    → The Voice synthesizes audio if in Voice Mode
    → User receives reply

```

---

## Build & Test

```bash

# Build everything

cargo build --workspace --release

# Run all tests

cargo test --workspace

# Lint

cargo clippy --workspace --all-targets

# Format

cargo fmt --all

```
