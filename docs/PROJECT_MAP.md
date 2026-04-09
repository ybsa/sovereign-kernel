# Sovereign Kernel — Project Map & Lore

This document serves as both a directory map of the Sovereign Kernel repository and a translation guide for the dark fantasy lore terminology used throughout the system.

## 📖 Lore Terminology Translation

The codebase uses standard software engineering names internally. The lore terms appear in documentation and user-facing output.

| Lore Term | Internal Term | Description |
| --- | --- | --- |
| **The King** | `Kernel` / `Daemon` | The central authority. Manages lifecycle, security, and agent supervision. |
| **The Witch** | `Orchestrator` | High-level task analysis using the ADK. Spawns temporary sub-agents. |
| **The Builder** | `Forge` / `SetupWizard` | Forges permanent expert blueprints (Hands) via interactive setup. |
| **The Healer** | `Compactor` | Performs "token healing" by summarizing long histories to prevent context overflow. |
| **The Grimoires (Hands)** | `Hand` (`sk-hands`) | Permanent expert blueprints — specialized agent roles with tools and prompts. |
| **The Skeletons** | `Worker` / `Sub-Agent` | Temporary disposable workers spawned for isolated tasks. |
| **The Laboratory** | `Tools` (`sk-tools`) | The tool execution library — shell, file ops, code exec, and more. |
| **The Phylactery** | `Memory Substrate` | SQLite-backed persistent memory for all agent data. |
| **The Whispers** | `Inter-Agent Bus` | Message routing between agents via their sessions. |
| **The Librarian** | `SemanticStore` / `Indexer` | Background worker that semantically indexes the codebase. |
| **The ADK** | `sk-adk` / `Strategy` | Agent Development Kit — separates reasoning strategies from tool runtimes. |
| **The Gatekeeper** | `ApprovalManager` | Unified risk-based approval system for dangerous operations. |
| **The Treasury** | `MeteringEngine` | Real-time cost tracking and budget enforcement. |
| **The Ledger** | `AuditStore` | Merkle chain of every agent action. |
| **The Tool Registry** | `ToolRegistry` | Modular dispatch system where each tool implements `ToolHandler`. |

---

## 🗺️ Project Directory Map

```text
sovereign-kernel/
├── config.toml.example               # Configuration template (copy to config.toml)
├── .env.example                      # Environment variable template (copy to .env)
├── Cargo.toml                        # Workspace root (9 crate members)
├── Cargo.lock
├── Dockerfile                        # Multi-stage production build
├── docker-compose.yml                # One-command deployment
├── README.md                         # Project overview
├── GETTING_STARTED.md                # Installation and setup guide
├── USER_GUIDE.md                     # Usage guide
├── SECURITY.md                       # Security policy
├── CHANGELOG.md                      # Version history
├── CONTRIBUTING.md                   # Contribution guidelines
├── MAINTENANCE.md                    # Maintenance procedures
├── LICENSE                           # MIT License
├── crates/                           # The core modular Rust workspace
│   ├── sk-types/                     # Shared types, errors, config schema, capabilities
│   ├── sk-soul/                      # Soul identity parser (SOUL.md, IDENTITY.md)
│   ├── sk-memory/                    # Memory substrate (SQLite, BM25, vectors, sessions)
│   ├── sk-engine/                    # LLM drivers, agent loop, sandbox runtime
│   ├── sk-mcp/                       # Model Context Protocol integration
│   ├── sk-kernel/                    # Core daemon, tool registry, approval, event bus
│   │   └── src/tools/               # Modular tool handlers (ToolHandler trait)
│   ├── sk-tools/                     # Tool definitions and implementations
│   │   └── skills/                   # 100+ expert prompts (SKILL.md files)
│   ├── sk-hands/                     # Agent capability packages (Hands)
│   │   └── bundled/                  # Built-in Hand definitions
│   └── sk-cli/                       # CLI application
├── docs/                             # Extended documentation
│   ├── ARCHITECTURE.md               # Full crate architecture deep dive
│   ├── PROJECT_MAP.md                # This file
│   ├── PROJECT_PLAN.md               # Development roadmap
│   ├── USAGE.md                      # CLI command reference
│   ├── SECURITY.md                   # Security model details
│   ├── SAFETY_CONTROLS.md            # Budget and forensic controls
│   ├── VISION.md                     # Long-term goals
│   └── CONTRIBUTING.md               # Contribution guide
├── soul/                             # Agent identity source files
│   ├── SOUL.md
│   ├── IDENTITY.md
│   ├── AGENTS.md
│   ├── MEMORY.md
│   └── USER.md
├── examples/                         # Example configurations
└── .github/workflows/                # CI/CD (cargo check, clippy, test)
```
