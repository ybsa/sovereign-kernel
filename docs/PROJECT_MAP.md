# Sovereign Kernel — Project Map & Lore

This document serves as both a directory map of the Sovereign Kernel repository and a translation guide for the dark fantasy lore terminology used throughout the system.

## 📖 Lore Terminology Translation

To make the AI ecosystem feel cohesive and flavorful, we use specific lore terms in the documentation. For the sake of maintainability, the internal Rust codebase and configuration files retain standard software engineering names.

| Lore Term | Internal/Standard Term | Description |
|-----------|------------------------|-------------|
| **The King** | `Orchestrator` / `Kernel` | The central authority and supervisor. He stays in the base, ensures safety, and manages the daemon's lifecycle. |
| **The Witch** | `Orchestrator` / `NLP` | The high-level seer. She analyzes tasks and "summons" temporary workers (Skeletons). |
| **The Builder** | `Forge` / `Architect` | The master crafter. He/She "forges" the permanent expert blueprints (Hands). |
| **The Healer** | `Compactor` | The master of memory. He performs "token healing" by summarizing long histories, ensuring agents don't get overwhelmed and stay sharp. |
| **PEKA**| `Permanent Agent` / `Hand`| **The Terminal Master.** Dedicated specialist for raw shell management and long-running processes. |
| **The Grimoires (Hands)**| `Hand` (`sk-hands`) | **Permanent Expert Blueprints.** These are pre-made, high-level roles (like Researcher or Coder) with specialized tools. |
| **The Skeletons** | `Worker` / `Sub-Agent` | **Temporary Disposable Workers.** Sandboxed agents summoned by the Witch to perform a specific, isolated task until it is complete. |
| **The Laboratory** | `Tools` (`sk-tools`) | The collection of functions, actions, and capabilities that agents can execute. |
| **The Phylactery** | `Memory Substrate` | The SQLite-backed local database where all memories and facts are stored persistently. |
| **The Whispers** | `Inter-Agent Bus` | The messaging system that allows all Village members to talk to each other. |

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
