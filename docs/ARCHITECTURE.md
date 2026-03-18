# Architecture — Sovereign Kernel

## Overview

Sovereign Kernel is a 10-crate Rust workspace containing 29 named subsystems. It is the Rust-native unification of OpenClaw (AI assistant), NemoClaw (sandbox), and the original Sovereign Kernel (OS daemon).

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

Shared types, capability gates, taint tracking, and error definitions used across all crates.

### sk-soul — Soul Files

Agent identity system. Parses `SOUL.md`, `AGENTS.md`, and `TOOLS.md` to construct agent personalities, workspace prompts, and bootstrap files.

### sk-memory — The Archive + The Scribe

| Subsystem | Role |
|---|---|
| **The Archive** | SQLite + BM25 vector memory substrate. Persistent storage for agent knowledge across sessions. |
| **The Scribe** | Session transcripts, write locks, transcript repair. Manages session state lifecycle. |

### sk-engine — The Oracle + The Healer + The Village + The Witch + The Treasury + The Chronicler + The Sentinel

| Subsystem | Role |
|---|---|
| **The Oracle** | 50+ LLM provider catalog. Manages auth profiles, OAuth flows, model discovery, and API calls. |
| **The Healer** | Context compaction. Summarizes and prunes old conversation turns to stay within token budgets. |
| **The Village** | Multi-agent ecosystem. Inter-Agent Bus for direct messaging, shared Village Library memory. |
| **The Witch** | Dynamic subagent spawning. Creates sandboxed worker agents for parallel task execution. |
| **The Treasury** | Global USD budget cap. Tracks cost per agent, enforces limits, kills agents on overspend. |
| **The Chronicler** | Usage analytics. Records token usage, cost, and latency per agent, channel, and model. |
| **The Sentinel** | Retry policy. Exponential backoff for failed LLM calls and channel delivery attempts. |

### sk-mcp — The Diplomat + The Alchemist

| Subsystem | Role |
|---|---|
| **The Diplomat** | Cross-instance agent-to-agent protocol. Allows SK instances on different machines to collaborate. |
| **The Alchemist** | Plugin SDK. Third-party Hands as WASM modules or dynamic Rust libraries (.so/.dll). |

### sk-kernel — The Kernel + The Warden + The Gatekeeper + The Resurrector + The Raven + The Cartographer + The Beacon + The Ledger

| Subsystem | Role |
|---|---|
| **The Kernel** | WebSocket control plane, HTTP API server, config system (TOML/JSON, hot-reload), daemon lifecycle (systemd/launchd). |
| **The Warden** | Security sandbox: Landlock LSM filesystem isolation, seccomp-bpf syscall filtering, network egress proxy with policy YAML. |
| **The Gatekeeper** | Exec approval manager. Intercepts dangerous commands and network requests, surfaces them in The Watchtower for approve/deny. |
| **The Resurrector** | Crash recovery. Auto-restarts panicked or crashed agents from their last SQLite checkpoint. |
| **The Raven** | Notification system. Push notifications, email alerts, Gmail Pub/Sub triggers, inbound webhooks. |
| **The Cartographer** | Remote access. Native Tailscale Serve/Funnel and SSH tunnel support for exposing The Watchtower. |
| **The Beacon** | Presence system. Tracks agent online/offline/busy state and broadcasts to connected clients. |
| **The Ledger** | Merkle audit trail. Tamper-evident cryptographic chain of every agent action. |

### sk-tools — The Forge + The Voice + Tool Execution

| Subsystem | Role |
|---|---|
| **The Forge** | CDP-based Chrome/Chromium browser automation. Screenshots, navigation, form filling, downloads. |
| **The Voice** | Always-on speech. cpal audio capture + Whisper STT + ElevenLabs TTS integration. |
| Shell execution | Bash/PowerShell with full PTY support, process registry, background jobs. |
| File operations | Read, write, edit, glob — path-sandboxed, atomic writes. |
| Media pipeline | Image/audio/video processing, transcription, size caps. |
| Device tools | Camera snap/clip, screen record, location.get (when hardware available). |
| Code execution | Docker/native sandbox with timeout and output capture. |

### sk-channels — The Bridge + The Herald

| Subsystem | Role |
|---|---|
| **The Bridge** | 30+ channel adapters: Telegram, Discord, WhatsApp, Slack, Signal, iMessage, Matrix, IRC, Twitch, Teams, Google Chat, Nostr, WebChat, Zalo, Feishu, Line, Mattermost, Synology, Nextcloud Talk, Tlon. |
| **The Herald** | In-channel slash command parser: `/status`, `/new`, `/compact`, `/think`, `/verbose`, `/usage`, `/restart`, `/activation`, `/elevated`. |

Bridge features: Channel Dock (routing), DM pairing, group routing, mention gating, allowlists, typing indicators, reactions, multi-agent channel routing.

### sk-hands — Hands + The Bazaar

| Subsystem | Role |
|---|---|
| **Hands** | Autonomous capability packages. 30+ bundled: browser, researcher, web-search, clip, collector, lead, predictor, email, twitter, otto, mysql-reporter, peka, etc. |
| **The Bazaar** | Community marketplace. Publish, discover, install, version, and rate community Hands. |

### sk-cli — The Watchtower + The Builder + The Canvas

| Subsystem | Role |
|---|---|
| **The Watchtower** | Terminal web dashboard at `localhost:8080`. Live logs, approval queue, agent management, WebChat, usage analytics. Embedded in the binary. |
| **The Builder** | `sovereign onboard` — interactive TUI setup wizard. Provider detection, key validation, first-run config. |
| **The Canvas** | A2UI agent-driven visual workspace. Served via The Watchtower WebSocket. |
| CLI surface | `sovereign init/chat/run/status/kill/dashboard/hands/audit/doctor/tunnel/usage` |

---

## Data Flow

```text
User sends message on Telegram
    → The Bridge (sk-channels) receives via Bot API
    → The Herald checks for slash commands
    → The Kernel (sk-kernel) routes to session
    → The Scribe (sk-memory) loads session transcript
    → The Oracle (sk-engine) selects LLM provider
    → The Sentinel retries on failure
    → The Healer compacts if context too long
    → Agent loop executes with tools (sk-tools)
    → The Gatekeeper checks for dangerous commands
    → The Forge runs browser automation if needed
    → The Ledger records every action
    → The Treasury checks budget
    → The Chronicler logs usage
    → Response flows back through The Bridge
    → User receives reply on Telegram
```
