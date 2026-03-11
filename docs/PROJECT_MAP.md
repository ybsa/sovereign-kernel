# Sovereign Kernel — Project Map & Lore

This document serves as both a directory map of the Sovereign Kernel repository and a translation guide for the dark fantasy lore terminology used throughout the system.

## 📖 Lore Terminology Translation

To make the AI ecosystem feel cohesive and flavorful, we use specific lore terms in the documentation. For the sake of maintainability, the internal Rust codebase and configuration files retain standard software engineering names.

| Lore Term | Internal/Standard Term | Description |
|-----------|------------------------|-------------|
| **King** | `Orchestrator` / `Kernel` | The top-level daemon that manages the entire lifecycle, schedules cron jobs, and coordinates all background activities. |
| **Witch Skeleton** | `Worker` / `Sub-Agent` | A background sandboxed agent spawned dynamically by a manager agent to perform parallel or specialized tasks. |
| **Laboratory** | `Tools` (`sk-tools`) | The collection of functions, actions, and capabilities that agents can execute (e.g., web search, file modification). An individual function is still referred to as a "tool". |
| **Grimoire / Hand** | `Hand` (`sk-hands`) | A pre-configured autonomous capability package (agent) bundled with a custom system prompt, designated tools, and settings. |
| **Incantation / Skill** | `Skill` | Modular expert prompts (like coding-agent or peekaboo) that teach an agent *how* to accomplish a specific workflow. |
| **The Phylactery** | `Memory Substrate` | The SQLite-backed local database where agent sessions, conversations, and global key-value facts are stored persistently. |
| **The Whispers** | `Inter-Agent Bus` | The messaging system that allows active agents to send data and notifications to each other. |

---

## 🗺️ Project Directory Map

```text
sovereign-kernel/
├── bin/                          # Convenience scripts and binaries
├── bundled/                      # Static assets and built tools
├── examples/                     # Examples and templates
│   └── config/                   # Example configuration files
│       ├── config.sandbox.toml
│       └── config.unrestricted.toml
├── crates/                       # The core modular Rust monorepo
│   ├── sk-cli/                   # The primary Sovereign CLI application (`sovereign`)
│   ├── sk-channels/              # Channel adapters: 30+ integrations (Telegram, Discord, etc.)
│   ├── sk-engine/                # The Execution Engine (LLM Drivers, Agent Loops, Outpost Runtime)
│   ├── sk-hands/                 # Grimoires/Hands: Pre-configured Agent packages (10 bundled)
│   ├── sk-kernel/                # The King: The top-level Daemon, Executor, Scheduler, and Bus
│   ├── sk-mcp/                   # Model Context Protocol: MCP server/client nervous system
│   ├── sk-memory/                # The Phylactery: Local SQLite memory database and state management
│   ├── sk-soul/                  # Soul loader: Agent identity and continuity from SOUL.md
│   ├── sk-tools/                 # The Laboratory: Actions that agents can execute (e.g., shell_exec)
│   │   └── skills/               # Incantations: 52 modular expert prompts ported from OpenClaw
│   └── sk-types/                 # Shared domain models, configurations, and core structures
├── docs/                         # Project Documentation
│   ├── ARCHITECTURE.md           # 10-crate workspace deep dive
│   ├── CONTRIBUTING.md           # Guide for adding new Hands or Laboratory Tools
│   ├── PROJECT_MAP.md            # (This File) Lore definitions and directory structure
│   ├── PROJECT_PLAN.md           # Full 30-week development roadmap (23 phases)
│   ├── SECURITY.md               # Breakdown of the Sandbox Policy and Safety Gates
│   ├── USAGE.md                  # Guide on how to use `sovereign` commands
│   └── VISION.md                 # Core principles and overarching goals of the project
├── soul/                         # Core system prompts (Identity, Soul, Memory, Agents)
├── .github/workflows/            # CI/CD (cargo check, clippy, test)
├── Cargo.toml                    # Master Workspace Cargo configuration
└── README.md                     # The Front Page for the Sovereign Kernel
```
